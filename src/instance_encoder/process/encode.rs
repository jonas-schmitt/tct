//! Raw token instance-encoding process orchestration.

use super::super::InstanceEncoder;
use super::super::encode_failure::EncodeFailure;
use super::super::metrics::{
    BatchAllocationMetrics, BatchEncodeHotPathMetrics, BatchEncodePhaseMetrics,
    BatchEncodeValidationPhaseMetrics, BatchParentIoMetrics, BatchPrimaryRuntimeMetrics,
    parse_batch_allocation_metrics, parse_batch_encode_hot_path_metrics,
    parse_batch_encode_phase_metrics, parse_batch_encode_validation_phase_metrics,
    parse_primary_batch_runtime_metrics,
};
use super::{ALLOCATION_METRICS_ENV, BatchProcessMetrics, elapsed_process_ns};
use rayon::prelude::*;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn failure_paths(failures: &[EncodeFailure]) -> Vec<PathBuf> {
    failures
        .iter()
        .map(|failure| failure.path.clone())
        .collect()
}

impl InstanceEncoder {
    /// Encode a single JSON instance file
    ///
    /// # Arguments
    /// * `path` - Path to JSON instance file
    ///
    /// # Returns
    /// Vector of tokens representing the encoded instance
    ///
    /// # Errors
    /// Returns an error if:
    /// - File cannot be read
    /// - JSON is invalid
    /// - Encoding fails
    pub fn encode_file(&self, path: &Path) -> Result<Vec<u32>, Box<dyn Error>> {
        let output = Command::new(&self.encoder_bin)
            .arg("encode")
            .arg(path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to encode {}: {}", path.display(), stderr).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let tokens: Vec<u32> = serde_json::from_str(stdout.trim())?;

        Ok(tokens)
    }

    /// Encode a JSON value directly (convenience method)
    ///
    /// # Arguments
    /// * `value` - JSON value to encode
    ///
    /// # Returns
    /// Vector of tokens representing the encoded instance
    ///
    /// # Errors
    /// Returns an error if encoding fails
    pub fn encode_json(&self, value: &serde_json::Value) -> Result<Vec<u32>, Box<dyn Error>> {
        let temp_path =
            std::env::temp_dir().join(format!("tct_encode_{}.json", std::process::id()));
        fs::write(&temp_path, serde_json::to_string(value)?)?;

        let result = self.encode_file(&temp_path);

        fs::remove_file(temp_path).ok();

        result
    }

    /// Encode a batch of JSON instance files efficiently using a single subprocess
    ///
    /// This method spawns a single encoder subprocess and streams file paths via stdin,
    /// reading JSONL output. This is ~100x faster than spawning a subprocess per file.
    ///
    /// # Arguments
    /// * `paths` - Slice of paths to JSON instance files
    ///
    /// # Returns
    /// Tuple of (successful_sequences, failed_paths)
    /// - successful_sequences: Vector of token sequences for successfully encoded files
    /// - failed_paths: Vector of paths that failed to encode
    ///
    /// # Note
    /// Progress is reported to stderr every 10000 files
    #[allow(clippy::type_complexity)]
    pub fn encode_batch_efficient(
        &self,
        paths: &[PathBuf],
    ) -> Result<(Vec<Vec<u32>>, Vec<PathBuf>), Box<dyn Error>> {
        let (sequences, failed, _) = self.encode_batch_efficient_inner(paths, false)?;
        Ok((sequences, failure_paths(&failed)))
    }

    /// Encode a batch and return structured failures (path + message + class).
    ///
    /// Prefer this over [`Self::encode_batch_efficient`] when building prepare
    /// unencodable inventories (unknown_field vs missing_field vs other).
    #[allow(clippy::type_complexity)]
    pub fn encode_batch_efficient_with_failures(
        &self,
        paths: &[PathBuf],
    ) -> Result<(Vec<Vec<u32>>, Vec<EncodeFailure>), Box<dyn Error>> {
        let (sequences, failed, _) = self.encode_batch_efficient_inner(paths, false)?;
        Ok((sequences, failed))
    }

    /// Encode a batch through the generated helper's profiling path.
    ///
    /// This sidecar path keeps normal `encode-batch` throughput timings stable while
    /// attributing generated helper work to JSON read/parse, base token emission,
    /// optional BPE, output serialization, and stdout writing phases.
    #[allow(clippy::type_complexity)]
    pub fn encode_batch_efficient_with_phase_metrics(
        &self,
        paths: &[PathBuf],
    ) -> Result<(Vec<Vec<u32>>, Vec<PathBuf>, BatchEncodePhaseMetrics), Box<dyn Error>> {
        let (sequences, failed, stderr, _) =
            self.run_encode_batch_command(paths, "encode-batch-profile", false)?;
        let metrics = parse_batch_encode_phase_metrics(&stderr, "encode-batch")
            .ok_or("Generated encode phase metrics were requested, but no metrics were emitted")?;
        Ok((sequences, failure_paths(&failed), metrics))
    }

    /// Encode a batch through the generated helper's no-JSONL hot-path profiling mode.
    ///
    /// This sidecar path reads and encodes the same files as `encode-batch`, but
    /// emits only aggregate token count/checksum metrics instead of per-instance
    /// token JSONL. Use it to separate tokenizer hot-path timing from measurement
    /// helper output transport.
    pub fn encode_batch_hot_path_metrics(
        &self,
        paths: &[PathBuf],
    ) -> Result<BatchEncodeHotPathMetrics, Box<dyn Error>> {
        let (metrics, _) = self.encode_batch_hot_path_metrics_inner(paths, false)?;
        Ok(metrics)
    }

    /// Encode a batch through the generated helper's no-JSONL hot-path profiling mode
    /// with generated-process allocation counters enabled.
    pub fn encode_batch_hot_path_metrics_with_allocation_metrics(
        &self,
        paths: &[PathBuf],
    ) -> Result<(BatchEncodeHotPathMetrics, Option<BatchAllocationMetrics>), Box<dyn Error>> {
        if !self.allocation_metrics_enabled {
            return Err("InstanceEncoder was not built with allocation metrics enabled".into());
        }
        self.encode_batch_hot_path_metrics_inner(paths, true)
    }

    fn encode_batch_hot_path_metrics_inner(
        &self,
        paths: &[PathBuf],
        track_allocations: bool,
    ) -> Result<(BatchEncodeHotPathMetrics, Option<BatchAllocationMetrics>), Box<dyn Error>> {
        let stderr =
            self.run_encode_summary_command(paths, "encode-batch-hot-path", track_allocations)?;
        let metrics = parse_batch_encode_hot_path_metrics(&stderr, "encode-batch").ok_or_else(
            || -> Box<dyn Error> {
                "Generated encode hot-path metrics were requested, but no metrics were emitted"
                    .into()
            },
        )?;
        let allocations = track_allocations
            .then(|| parse_batch_allocation_metrics(&stderr, "encode-batch-hot-path"))
            .flatten();
        Ok((metrics, allocations))
    }

    /// Encode a batch through a diagnostic Value -> typed validation profiling mode.
    ///
    /// This sidecar pass reports file read, JSON `Value` parse, typed serde
    /// validation/deserialization, base encode, and optional BPE attribution. It
    /// emits only aggregate token count/checksum metrics and verifies against
    /// the primary encode path at the CLI layer.
    pub fn encode_batch_validation_phase_metrics(
        &self,
        paths: &[PathBuf],
    ) -> Result<BatchEncodeValidationPhaseMetrics, Box<dyn Error>> {
        let stderr =
            self.run_encode_summary_command(paths, "encode-batch-validation-profile", false)?;
        parse_batch_encode_validation_phase_metrics(&stderr, "encode-batch").ok_or(
            "Generated encode validation phase metrics were requested, but no metrics were emitted"
                .into(),
        )
    }

    /// Encode a batch and request generated-process allocation metrics.
    #[allow(clippy::type_complexity)]
    pub fn encode_batch_efficient_with_metrics(
        &self,
        paths: &[PathBuf],
    ) -> Result<(Vec<Vec<u32>>, Vec<PathBuf>, Option<BatchAllocationMetrics>), Box<dyn Error>> {
        if !self.allocation_metrics_enabled {
            return Err("InstanceEncoder was not built with allocation metrics enabled".into());
        }
        let (sequences, failed, metrics) = self.encode_batch_efficient_inner(paths, true)?;
        Ok((sequences, failure_paths(&failed), metrics.allocations))
    }

    /// Encode a batch and return primary child and parent transport runtime metrics.
    #[allow(clippy::type_complexity)]
    pub fn encode_batch_efficient_with_runtime_metrics(
        &self,
        paths: &[PathBuf],
    ) -> Result<
        (
            Vec<Vec<u32>>,
            Vec<PathBuf>,
            Option<BatchPrimaryRuntimeMetrics>,
        ),
        Box<dyn Error>,
    > {
        let (sequences, failed, metrics) = self.encode_batch_efficient_inner(paths, false)?;
        Ok((sequences, failure_paths(&failed), metrics.runtime))
    }

    /// Encode a batch and return primary child/parent runtime metrics with allocation counters.
    #[allow(clippy::type_complexity)]
    pub fn encode_batch_efficient_with_runtime_and_allocation_metrics(
        &self,
        paths: &[PathBuf],
    ) -> Result<
        (
            Vec<Vec<u32>>,
            Vec<PathBuf>,
            Option<BatchPrimaryRuntimeMetrics>,
        ),
        Box<dyn Error>,
    > {
        if !self.allocation_metrics_enabled {
            return Err("InstanceEncoder was not built with allocation metrics enabled".into());
        }
        let (sequences, failed, metrics) = self.encode_batch_efficient_inner(paths, true)?;
        Ok((sequences, failure_paths(&failed), metrics.runtime))
    }

    #[allow(clippy::type_complexity)]
    fn encode_batch_efficient_inner(
        &self,
        paths: &[PathBuf],
        track_allocations: bool,
    ) -> Result<(Vec<Vec<u32>>, Vec<EncodeFailure>, BatchProcessMetrics), Box<dyn Error>> {
        let (sequences, failed, stderr, parent_io) =
            self.run_encode_batch_command(paths, "encode-batch", track_allocations)?;
        let allocations = track_allocations
            .then(|| parse_batch_allocation_metrics(&stderr, "encode-batch"))
            .flatten();
        let runtime = parse_primary_batch_runtime_metrics(
            &stderr,
            "encode-batch",
            track_allocations,
            parent_io,
        );
        let metrics = BatchProcessMetrics {
            runtime,
            allocations,
        };

        Ok((sequences, failed, metrics))
    }

    #[allow(clippy::type_complexity)]
    fn run_encode_batch_command(
        &self,
        paths: &[PathBuf],
        mode: &str,
        track_allocations: bool,
    ) -> Result<
        (
            Vec<Vec<u32>>,
            Vec<EncodeFailure>,
            Vec<u8>,
            BatchParentIoMetrics,
        ),
        Box<dyn Error>,
    > {
        use std::io::{BufRead, BufReader, Write};

        let mut command = Command::new(&self.encoder_bin);
        command
            .arg(mode)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if track_allocations {
            command.env(ALLOCATION_METRICS_ENV, "1");
        }
        let mut child = command.spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or("Failed to open stdin for encoder")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to open stdout for encoder")?;

        let paths_owned: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        let writer_thread = std::thread::spawn(move || {
            let stdin_write_start = std::time::Instant::now();
            for path in paths_owned {
                if writeln!(stdin, "{}", path).is_err() {
                    break;
                }
            }
            elapsed_process_ns(stdin_write_start)
        });

        let stdout_read_parse_start = std::time::Instant::now();
        let reader = BufReader::new(stdout);
        let mut sequences = Vec::new();
        let mut failed = Vec::new();
        let mut count = 0;
        let total = paths.len();
        let mut stdout_json_parse_ns = 0_u64;

        for line in reader.lines() {
            let line = line?;
            count += 1;

            if count % 10000 == 0 {
                eprintln!("  Encoded {}/{} instances", count, total);
            }

            let json_parse_start = std::time::Instant::now();
            let parsed: serde_json::Value = serde_json::from_str(&line)?;
            stdout_json_parse_ns =
                stdout_json_parse_ns.saturating_add(elapsed_process_ns(json_parse_start));

            let path = parsed
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");

            if let Some(tokens) = parsed.get("tokens") {
                let json_parse_start = std::time::Instant::now();
                let tokens: Vec<u32> = serde_json::from_value(tokens.clone())?;
                stdout_json_parse_ns =
                    stdout_json_parse_ns.saturating_add(elapsed_process_ns(json_parse_start));
                if !tokens.is_empty() {
                    sequences.push(tokens);
                } else {
                    failed.push(EncodeFailure::empty_tokens(path));
                }
            } else {
                let message = parsed
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("encode failed without error detail")
                    .to_string();
                failed.push(EncodeFailure::new(path, message));
            }
        }
        let stdout_read_parse_ns = elapsed_process_ns(stdout_read_parse_start);

        let stdin_write_ns = writer_thread.join().unwrap_or(0);

        let child_wait_start = std::time::Instant::now();
        let output = child.wait_with_output()?;
        let child_wait_ns = elapsed_process_ns(child_wait_start);
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.trim().is_empty() {
                return Err(
                    format!("Encoder batch process failed in {mode}: {}", output.status).into(),
                );
            }
            return Err(
                format!("Encoder batch process failed in {mode}: {}", stderr.trim()).into(),
            );
        }

        let parent_io = BatchParentIoMetrics {
            stdin_write_ns,
            stdout_read_parse_ns,
            stdout_json_parse_ns,
            child_wait_ns,
        };

        Ok((sequences, failed, output.stderr, parent_io))
    }

    fn run_encode_summary_command(
        &self,
        paths: &[PathBuf],
        mode: &str,
        track_allocations: bool,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        use std::io::Write;

        let mut command = Command::new(&self.encoder_bin);
        command
            .arg(mode)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        if track_allocations {
            command.env(ALLOCATION_METRICS_ENV, "1");
        }
        let mut child = command.spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or("Failed to open stdin for encoder")?;

        let paths_owned: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        let writer_thread = std::thread::spawn(move || {
            for path in paths_owned {
                if writeln!(stdin, "{}", path).is_err() {
                    break;
                }
            }
        });

        writer_thread.join().ok();

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.trim().is_empty() {
                return Err(format!(
                    "Encoder summary process failed in {mode}: {}",
                    output.status
                )
                .into());
            }
            return Err(format!(
                "Encoder summary process failed in {mode}: {}",
                stderr.trim()
            )
            .into());
        }

        Ok(output.stderr)
    }

    /// Encode a batch of JSON instance files (legacy method, spawns subprocess per file)
    ///
    /// This method encodes multiple files and collects successful encodings.
    /// Failed encodings are logged to stderr and skipped.
    ///
    /// # Arguments
    /// * `paths` - Slice of paths to JSON instance files
    ///
    /// # Returns
    /// Vector of token sequences (one per successfully encoded instance)
    ///
    /// # Note
    /// Progress is reported to stderr every 1000 files.
    /// Consider using `encode_batch_efficient()` for better performance.
    pub fn encode_batch(&self, paths: &[PathBuf]) -> Result<Vec<Vec<u32>>, Box<dyn Error>> {
        let counter = AtomicUsize::new(0);
        let sequences = Mutex::new(Vec::new());
        let total = paths.len();

        paths.par_iter().for_each(|path| {
            let i = counter.fetch_add(1, Ordering::Relaxed);

            if i.is_multiple_of(1000) && i > 0 {
                eprintln!("  Encoded {}/{} instances", i, total);
            }

            match self.encode_file(path) {
                Ok(tokens) if !tokens.is_empty() => {
                    sequences
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(tokens);
                }
                Ok(_) => {
                    eprintln!(
                        "  ⚠️  Warning: {} produced empty token sequence",
                        path.display()
                    );
                }
                Err(e) => {
                    eprintln!("  ⚠️  Warning: Failed to encode {}: {}", path.display(), e);
                }
            }
        });

        Ok(sequences
            .into_inner()
            .unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    /// Replay decode through generated `role_export::token_roles` for each
    /// base-token sequence. Returns one role vector per input sequence, aligned
    /// 1:1 with the input order. Role bytes match `ROLE_STRUCTURE` /
    /// `ROLE_VALUE` / `ROLE_VALUE_END` / `ROLE_DISCRIMINATOR`.
    pub fn roles_batch(
        &self,
        token_sequences: &[Vec<u32>],
    ) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        use std::io::{BufRead, BufReader, Write};

        let mut child = Command::new(&self.encoder_bin)
            .arg("roles-batch")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or("Failed to open stdin for roles-batch")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to open stdout for roles-batch")?;

        let mut roles_by_index: Vec<Option<Vec<u8>>> = vec![None; token_sequences.len()];
        let mut errors: Vec<String> = Vec::new();

        std::thread::scope(|scope| -> Result<(), Box<dyn Error>> {
            let writer_thread = scope.spawn(move || -> Result<(), String> {
                for tokens in token_sequences {
                    serde_json::to_writer(&mut stdin, tokens).map_err(|error| error.to_string())?;
                    stdin.write_all(b"\n").map_err(|error| error.to_string())?;
                }
                Ok(())
            });

            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let line = line?;
                let parsed: serde_json::Value = serde_json::from_str(&line)?;
                let index = parsed
                    .get("index")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or_else(|| format!("roles-batch record missing index: {line}"))?;
                if let Some(error) = parsed.get("error").and_then(serde_json::Value::as_str) {
                    errors.push(format!("index {index}: {error}"));
                    continue;
                }
                let roles: Vec<u8> = parsed
                    .get("roles")
                    .ok_or_else(|| format!("roles-batch record missing roles: {line}"))
                    .and_then(|value| {
                        serde_json::from_value(value.clone()).map_err(|error| error.to_string())
                    })?;
                if index >= roles_by_index.len() {
                    return Err(format!(
                        "roles-batch index {index} out of range for {} sequences",
                        roles_by_index.len()
                    )
                    .into());
                }
                roles_by_index[index] = Some(roles);
            }

            match writer_thread.join() {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => Err(format!("roles-batch stdin writer failed: {error}").into()),
                Err(_) => Err("roles-batch stdin writer panicked".into()),
            }
        })?;

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "roles-batch process failed: {}",
                if stderr.trim().is_empty() {
                    output.status.to_string()
                } else {
                    stderr.trim().to_string()
                }
            )
            .into());
        }
        // Per-document role failures (for example a collection larger than the
        // generated codec's bound, which the encoder accepted but the role replay
        // refuses) are not fatal for the batch: the affected sequence receives an
        // EMPTY role row so consumers detect it by `roles.len() != tokens.len()`,
        // and the count is reported on stderr. Structural failures (missing index,
        // process error, length mismatch on a successful row) still abort.
        let failed_indices: std::collections::HashSet<usize> = errors
            .iter()
            .filter_map(|error| {
                error
                    .strip_prefix("index ")
                    .and_then(|rest| rest.split(':').next())
                    .and_then(|index| index.parse::<usize>().ok())
            })
            .collect();
        if !errors.is_empty() {
            eprintln!(
                "  Warning: roles-batch reported {} document-level error(s); those sequences receive empty role rows. First: {}",
                errors.len(),
                errors[0]
            );
        }

        let mut roles = Vec::with_capacity(token_sequences.len());
        for (index, entry) in roles_by_index.into_iter().enumerate() {
            let role_seq = match entry {
                Some(role_seq) => role_seq,
                None if failed_indices.contains(&index) => Vec::new(),
                None => {
                    return Err(
                        format!("roles-batch missing roles for sequence index {index}").into(),
                    );
                }
            };
            if !role_seq.is_empty() && role_seq.len() != token_sequences[index].len() {
                return Err(format!(
                    "roles-batch length mismatch at index {index}: roles {} != tokens {}",
                    role_seq.len(),
                    token_sequences[index].len()
                )
                .into());
            }
            roles.push(role_seq);
        }
        Ok(roles)
    }
}
