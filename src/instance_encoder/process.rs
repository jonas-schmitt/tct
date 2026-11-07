//! Process runners for generated instance-encoder helpers.

use super::InstanceEncoder;

mod bpe;
mod encode;
use super::metrics::{
    BatchAllocationMetrics, BatchDecodePrefixMetrics, BatchParentIoMetrics,
    BatchPrimaryRuntimeMetrics, parse_batch_allocation_metrics, parse_batch_decode_prefix_metrics,
    parse_primary_batch_runtime_metrics,
};
use std::error::Error;
use std::process::Command;

const ALLOCATION_METRICS_ENV: &str = "TCT_MEASURE_ALLOCATIONS";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct BatchProcessMetrics {
    runtime: Option<BatchPrimaryRuntimeMetrics>,
    allocations: Option<BatchAllocationMetrics>,
}

impl InstanceEncoder {
    /// Decode tokens back to JSON value
    ///
    /// # Arguments
    /// * `tokens` - Vector of tokens to decode
    ///
    /// # Returns
    /// Decoded JSON value
    ///
    /// # Errors
    /// Returns an error if:
    /// - Token sequence is invalid
    /// - Decoding fails
    pub fn decode_json(&self, tokens: &[u32]) -> Result<serde_json::Value, Box<dyn Error>> {
        use std::io::Write;

        let mut child = Command::new(&self.encoder_bin)
            .arg("decode")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Write tokens as JSON array to stdin
        let tokens_json = serde_json::to_string(tokens)?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(tokens_json.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to decode tokens: {}", stderr).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let decoded: serde_json::Value = serde_json::from_str(stdout.trim())?;

        Ok(decoded)
    }

    /// Decode a batch of token sequences efficiently using a single subprocess
    ///
    /// This method spawns a single decoder subprocess and streams JSON token arrays
    /// via stdin, reading JSONL output. It mirrors `encode_batch_efficient()` for
    /// measurement workflows that need generated decode timing without one
    /// subprocess per instance.
    ///
    /// # Arguments
    /// * `token_sequences` - Slice of token sequences to decode
    ///
    /// # Returns
    /// Tuple of (decoded_values, failed_indices)
    /// - decoded_values: JSON values for successfully decoded token sequences
    /// - failed_indices: Indices in the input slice that failed to decode
    #[allow(clippy::type_complexity)]
    pub fn decode_batch_efficient(
        &self,
        token_sequences: &[Vec<u32>],
    ) -> Result<(Vec<serde_json::Value>, Vec<usize>), Box<dyn Error>> {
        let (decoded, failed, _) = self.decode_batch_efficient_inner(token_sequences, false)?;
        Ok((decoded, failed))
    }

    /// Decode a batch and return primary child and parent transport runtime metrics.
    #[allow(clippy::type_complexity)]
    pub fn decode_batch_efficient_with_runtime_metrics(
        &self,
        token_sequences: &[Vec<u32>],
    ) -> Result<
        (
            Vec<serde_json::Value>,
            Vec<usize>,
            Option<BatchPrimaryRuntimeMetrics>,
        ),
        Box<dyn Error>,
    > {
        let (decoded, failed, metrics) =
            self.decode_batch_efficient_inner(token_sequences, false)?;
        Ok((decoded, failed, metrics.runtime))
    }

    /// Decode a batch and return primary child/parent runtime metrics with allocation counters.
    #[allow(clippy::type_complexity)]
    pub fn decode_batch_efficient_with_runtime_and_allocation_metrics(
        &self,
        token_sequences: &[Vec<u32>],
    ) -> Result<
        (
            Vec<serde_json::Value>,
            Vec<usize>,
            Option<BatchPrimaryRuntimeMetrics>,
        ),
        Box<dyn Error>,
    > {
        if !self.allocation_metrics_enabled {
            return Err("InstanceEncoder was not built with allocation metrics enabled".into());
        }
        let (decoded, failed, metrics) =
            self.decode_batch_efficient_inner(token_sequences, true)?;
        Ok((decoded, failed, metrics.runtime))
    }

    /// Decode a batch and request generated-process allocation metrics.
    #[allow(clippy::type_complexity)]
    pub fn decode_batch_efficient_with_metrics(
        &self,
        token_sequences: &[Vec<u32>],
    ) -> Result<
        (
            Vec<serde_json::Value>,
            Vec<usize>,
            Option<BatchAllocationMetrics>,
        ),
        Box<dyn Error>,
    > {
        if !self.allocation_metrics_enabled {
            return Err("InstanceEncoder was not built with allocation metrics enabled".into());
        }
        let (decoded, failed, metrics) =
            self.decode_batch_efficient_inner(token_sequences, true)?;
        Ok((decoded, failed, metrics.allocations))
    }

    /// Profile generated `decode_prefix` over full and truncated measured token sequences.
    ///
    /// The generated helper expands BPE-compressed sequences to raw TCT tokens
    /// before taking prefix cuts. This keeps the metrics scoped to generated
    /// `decode_prefix` behavior and avoids implying BPE-prefix streaming support.
    pub fn decode_prefix_batch_metrics(
        &self,
        token_sequences: &[Vec<u32>],
        warmup_iterations: u32,
    ) -> Result<BatchDecodePrefixMetrics, Box<dyn Error>> {
        let stderr = self.run_decode_prefix_summary_command(token_sequences, warmup_iterations)?;
        parse_batch_decode_prefix_metrics(&stderr, "decode-prefix-batch").ok_or(
            "Generated decode_prefix metrics were requested, but no metrics were emitted".into(),
        )
    }

    #[allow(clippy::type_complexity)]
    fn decode_batch_efficient_inner(
        &self,
        token_sequences: &[Vec<u32>],
        track_allocations: bool,
    ) -> Result<(Vec<serde_json::Value>, Vec<usize>, BatchProcessMetrics), Box<dyn Error>> {
        use std::io::{BufRead, BufReader, Write};

        let mut command = Command::new(&self.encoder_bin);
        command
            .arg("decode-batch")
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
            .ok_or("Failed to open stdin for decoder")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to open stdout for decoder")?;

        let sequences_owned = token_sequences.to_vec();
        let writer_thread = std::thread::spawn(move || {
            let stdin_write_start = std::time::Instant::now();
            for tokens in sequences_owned {
                let Ok(line) = serde_json::to_string(&tokens) else {
                    break;
                };
                if writeln!(stdin, "{}", line).is_err() {
                    break;
                }
            }
            elapsed_process_ns(stdin_write_start)
        });

        let stdout_read_parse_start = std::time::Instant::now();
        let reader = BufReader::new(stdout);
        let mut decoded = Vec::new();
        let mut failed = Vec::new();
        let mut stdout_json_parse_ns = 0_u64;

        for line in reader.lines() {
            let line = line?;
            let json_parse_start = std::time::Instant::now();
            let parsed: serde_json::Value = serde_json::from_str(&line)?;
            stdout_json_parse_ns =
                stdout_json_parse_ns.saturating_add(elapsed_process_ns(json_parse_start));
            let index = parsed
                .get("index")
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| usize::try_from(value).ok());

            if let Some(value) = parsed.get("json") {
                decoded.push(value.clone());
            } else if let Some(index) = index {
                failed.push(index);
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
                return Err(format!("Decoder batch process failed: {}", output.status).into());
            }
            return Err(format!("Decoder batch process failed: {}", stderr.trim()).into());
        }

        let allocations = track_allocations
            .then(|| parse_batch_allocation_metrics(&output.stderr, "decode-batch"))
            .flatten();
        let parent_io = BatchParentIoMetrics {
            stdin_write_ns,
            stdout_read_parse_ns,
            stdout_json_parse_ns,
            child_wait_ns,
        };
        let runtime = parse_primary_batch_runtime_metrics(
            &output.stderr,
            "decode-batch",
            track_allocations,
            parent_io,
        );
        let metrics = BatchProcessMetrics {
            runtime,
            allocations,
        };

        Ok((decoded, failed, metrics))
    }

    fn run_decode_prefix_summary_command(
        &self,
        token_sequences: &[Vec<u32>],
        warmup_iterations: u32,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        use std::io::Write;

        let mut child = Command::new(&self.encoder_bin)
            .arg("decode-prefix-batch")
            .arg(warmup_iterations.to_string())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or("Failed to open stdin for decode_prefix metrics")?;

        std::thread::scope(|scope| -> Result<(), Box<dyn Error>> {
            let writer_thread = scope.spawn(move || -> Result<(), String> {
                for tokens in token_sequences {
                    serde_json::to_writer(&mut stdin, tokens).map_err(|error| error.to_string())?;
                    stdin.write_all(b"\n").map_err(|error| error.to_string())?;
                }
                Ok(())
            });

            match writer_thread.join() {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => {
                    Err(format!("Generated decode_prefix stdin writer failed: {error}").into())
                }
                Err(_) => Err("Generated decode_prefix stdin writer panicked".into()),
            }
        })?;

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.trim().is_empty() {
                return Err(format!(
                    "Generated decode_prefix metrics process failed: {}",
                    output.status
                )
                .into());
            }
            return Err(format!(
                "Generated decode_prefix metrics process failed: {}",
                stderr.trim()
            )
            .into());
        }

        Ok(output.stderr)
    }
}

fn elapsed_process_ns(start: std::time::Instant) -> u64 {
    start.elapsed().as_nanos().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::super::metrics::{
        BatchEncodeHotPathMetrics, BatchEncodePhaseMetrics, BatchEncodeValidationPhaseMetrics,
        BatchRuntimeMetrics,
    };
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[cfg(unix)]
    fn write_fake_encoder(script: &str) -> (tempfile::TempDir, PathBuf) {
        use std::io::Write;

        let dir = tempdir().unwrap();
        let bin = dir.path().join("tct-encoder");
        let mut file = fs::File::create(&bin).unwrap();
        file.write_all(script.as_bytes()).unwrap();
        file.sync_all().unwrap();
        drop(file);

        let mut permissions = fs::metadata(&bin).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bin, permissions).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(25));
        (dir, bin)
    }

    #[cfg(unix)]
    fn instance_encoder_for_bin(bin: &Path) -> InstanceEncoder {
        InstanceEncoder {
            project_dir: bin.parent().unwrap().to_path_buf(),
            encoder_bin: bin.to_path_buf(),
            allocation_metrics_enabled: true,
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_file_parses_tokens_and_reports_subprocess_errors() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
if [[ "${2:-}" == *bad.json ]]; then
  echo "bad instance" >&2
  exit 7
fi
printf '[1,2,3]\n'
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        assert_eq!(
            encoder.encode_file(Path::new("ok.json")).unwrap(),
            vec![1, 2, 3]
        );

        let error = encoder.encode_file(Path::new("bad.json")).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("Failed to encode bad.json"));
        assert!(message.contains("bad instance"));
    }

    #[cfg(unix)]
    #[test]
    fn test_decode_json_writes_tokens_to_stdin_and_parses_json() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "decode" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
tokens="$(cat)"
if [[ "$tokens" != "[1,2,3]" ]]; then
  echo "unexpected tokens: $tokens" >&2
  exit 3
fi
printf '{"ok":true,"tokens":[1,2,3]}\n'
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let decoded = encoder.decode_json(&[1, 2, 3]).unwrap();

        assert_eq!(decoded["ok"], serde_json::json!(true));
        assert_eq!(decoded["tokens"], serde_json::json!([1, 2, 3]));
    }

    #[cfg(unix)]
    #[test]
    fn test_decode_batch_efficient_accounts_for_successes_and_failures() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "decode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
index=0
while IFS= read -r tokens; do
  if [[ "$tokens" == "[1,2]" ]]; then
    printf '{"index":%s,"json":{"ok":true}}\n' "$index"
  else
    printf '{"index":%s,"error":"decode: nope"}\n' "$index"
  fi
  index=$((index + 1))
done
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let (decoded, failed) = encoder
            .decode_batch_efficient(&[vec![1, 2], vec![9]])
            .unwrap();

        assert_eq!(decoded, vec![serde_json::json!({"ok": true})]);
        assert_eq!(failed, vec![1]);
    }

    #[cfg(unix)]
    #[test]
    fn test_decode_batch_efficient_with_metrics_parses_child_summary() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "decode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
if [[ "${TCT_MEASURE_ALLOCATIONS:-}" != "1" ]]; then
  echo "allocation env missing" >&2
  exit 3
fi
index=0
while IFS= read -r tokens; do
  printf '{"index":%s,"json":{"ok":true}}\n' "$index"
  index=$((index + 1))
done
printf 'TCT_ALLOCATION_METRICS\tdecode-batch\t12\t345\n' >&2
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let (decoded, failed, metrics) = encoder
            .decode_batch_efficient_with_metrics(&[vec![1, 2]])
            .unwrap();

        assert_eq!(decoded, vec![serde_json::json!({"ok": true})]);
        assert!(failed.is_empty());
        assert_eq!(
            metrics,
            Some(BatchAllocationMetrics {
                allocations: 12,
                allocated_bytes: 345,
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_decode_batch_efficient_with_runtime_metrics_parses_parent_and_child_summary() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "decode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
index=0
while IFS= read -r tokens; do
  printf '{"index":%s,"json":{"ok":true}}\n' "$index"
  index=$((index + 1))
done
printf 'TCT_BATCH_TIMING\tdecode-batch\t34\n' >&2
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let (decoded, failed, metrics) = encoder
            .decode_batch_efficient_with_runtime_metrics(&[vec![1, 2]])
            .unwrap();

        let metrics = metrics.unwrap();
        assert_eq!(decoded, vec![serde_json::json!({"ok": true})]);
        assert!(failed.is_empty());
        assert_eq!(metrics.child_wall_time_ms, 34);
        assert!(metrics.parent_io.stdout_read_parse_ns >= metrics.parent_io.stdout_json_parse_ns);
    }

    #[cfg(unix)]
    #[test]
    fn test_decode_prefix_batch_metrics_parses_summary() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "decode-prefix-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
if [[ "${2:-}" != "2" ]]; then
  echo "unexpected warmup ${2:-}" >&2
  exit 3
fi
cat >/dev/null
printf 'TCT_DECODE_PREFIX_METRICS\tdecode-prefix-batch\t{"records":1,"prefix_cuts":3,"complete_prefixes":1,"partial_prefixes":2,"failed_prefixes":0,"full_sequences_complete":1,"consumed_tokens":6,"total_prefix_tokens":7,"fields_decoded_sum":4,"partial_fields_reported":2,"profile_wall_time_ns":90,"token_input_ns":4,"bpe_decode_ns":11,"decode_prefix_ns":60}\n' >&2
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let metrics = encoder
            .decode_prefix_batch_metrics(&[vec![1, 2, 3]], 2)
            .unwrap();

        assert_eq!(
            metrics,
            BatchDecodePrefixMetrics {
                records: 1,
                prefix_cuts: 3,
                complete_prefixes: 1,
                partial_prefixes: 2,
                failed_prefixes: 0,
                full_sequences_complete: 1,
                consumed_tokens: 6,
                total_prefix_tokens: 7,
                fields_decoded_sum: 4,
                partial_fields_reported: 2,
                profile_wall_time_ns: 90,
                token_input_ns: 4,
                bpe_decode_ns: Some(11),
                decode_prefix_ns: 60,
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_bpe_encode_batch_efficient_accounts_for_successes_and_failures() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "bpe-encode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
index=0
while IFS= read -r tokens; do
  if [[ "$tokens" == "[1,2]" ]]; then
    printf '{"index":%s,"tokens":[257]}\n' "$index"
  else
    printf '{"index":%s,"error":"parse: nope"}\n' "$index"
  fi
  index=$((index + 1))
done
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let (encoded, failed) = encoder
            .bpe_encode_batch_efficient(&[vec![1, 2], vec![9]])
            .unwrap();

        assert_eq!(encoded, vec![vec![257]]);
        assert_eq!(failed, vec![1]);
    }

    #[cfg(unix)]
    #[test]
    fn test_bpe_decode_batch_efficient_with_metrics_parses_child_summary() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "bpe-decode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
if [[ "${TCT_MEASURE_ALLOCATIONS:-}" != "1" ]]; then
  echo "allocation env missing" >&2
  exit 3
fi
index=0
while IFS= read -r tokens; do
  printf '{"index":%s,"tokens":[1,2]}\n' "$index"
  index=$((index + 1))
done
printf 'TCT_ALLOCATION_METRICS\tbpe-decode-batch\t31\t987\n' >&2
printf 'TCT_BATCH_TIMING\tbpe-decode-batch\t42\n' >&2
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let (decoded, failed, metrics) = encoder
            .bpe_decode_batch_efficient_with_metrics(&[vec![257]], 0)
            .unwrap();

        assert_eq!(decoded, vec![vec![1, 2]]);
        assert!(failed.is_empty());
        assert_eq!(
            metrics,
            Some(BatchRuntimeMetrics {
                wall_time_ms: 42,
                allocation_metrics: Some(BatchAllocationMetrics {
                    allocations: 31,
                    allocated_bytes: 987,
                }),
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_decode_batch_efficient_errors_on_nonzero_child_exit() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "decode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
cat >/dev/null
echo "fatal decoder crash" >&2
exit 9
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let error = encoder.decode_batch_efficient(&[vec![1, 2]]).unwrap_err();

        assert!(error.to_string().contains("fatal decoder crash"));
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_json_delegates_and_cleans_temp_file() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
if [[ ! -f "${2:-}" ]]; then
  echo "missing temp json" >&2
  exit 3
fi
if ! grep -q '"name":"demo"' "${2:-}"; then
  echo "unexpected json content" >&2
  exit 4
fi
printf '[7]\n'
"#,
        );
        let temp_path =
            std::env::temp_dir().join(format!("tct_encode_{}.json", std::process::id()));
        fs::remove_file(&temp_path).ok();
        let encoder = instance_encoder_for_bin(&bin);

        let tokens = encoder
            .encode_json(&serde_json::json!({"name": "demo"}))
            .unwrap();

        assert_eq!(tokens, vec![7]);
        assert!(
            !temp_path.exists(),
            "encode_json should remove its temporary input file"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_batch_efficient_accounts_for_successes_and_failures() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
while IFS= read -r path; do
  if [[ "$path" == *bad.json ]]; then
    printf '{"path":"%s","error":"parse: nope"}\n' "$path"
  else
    printf '{"path":"%s","tokens":[4,5]}\n' "$path"
  fi
done
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);
        let paths = vec![PathBuf::from("good.json"), PathBuf::from("bad.json")];

        let (sequences, failed) = encoder.encode_batch_efficient(&paths).unwrap();

        assert_eq!(sequences, vec![vec![4, 5]]);
        assert_eq!(failed, vec![PathBuf::from("bad.json")]);

        let (sequences2, detailed) = encoder
            .encode_batch_efficient_with_failures(&paths)
            .unwrap();
        assert_eq!(sequences2, vec![vec![4, 5]]);
        assert_eq!(detailed.len(), 1);
        assert_eq!(detailed[0].path, PathBuf::from("bad.json"));
        assert_eq!(
            detailed[0].class,
            crate::instance_encoder::EncodeFailureClass::Other
        );
        assert!(detailed[0].message.contains("parse:"));
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_batch_efficient_with_metrics_parses_child_summary() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
if [[ "${TCT_MEASURE_ALLOCATIONS:-}" != "1" ]]; then
  echo "allocation env missing" >&2
  exit 3
fi
while IFS= read -r path; do
  printf '{"path":"%s","tokens":[4,5]}\n' "$path"
done
printf 'TCT_ALLOCATION_METRICS\tencode-batch\t23\t456\n' >&2
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let (sequences, failed, metrics) = encoder
            .encode_batch_efficient_with_metrics(&[PathBuf::from("good.json")])
            .unwrap();

        assert_eq!(sequences, vec![vec![4, 5]]);
        assert!(failed.is_empty());
        assert_eq!(
            metrics,
            Some(BatchAllocationMetrics {
                allocations: 23,
                allocated_bytes: 456,
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_batch_efficient_with_runtime_metrics_parses_parent_and_child_summary() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
while IFS= read -r path; do
  printf '{"path":"%s","tokens":[4,5]}\n' "$path"
done
printf 'TCT_BATCH_TIMING\tencode-batch\t56\n' >&2
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let (sequences, failed, metrics) = encoder
            .encode_batch_efficient_with_runtime_metrics(&[PathBuf::from("good.json")])
            .unwrap();

        let metrics = metrics.unwrap();
        assert_eq!(sequences, vec![vec![4, 5]]);
        assert!(failed.is_empty());
        assert_eq!(metrics.child_wall_time_ms, 56);
        assert!(metrics.parent_io.stdout_read_parse_ns >= metrics.parent_io.stdout_json_parse_ns);
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_batch_efficient_with_phase_metrics_parses_sidecar_summary() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode-batch-profile" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
while IFS= read -r path; do
  if [[ "$path" == *bad.json ]]; then
    printf '{"path":"%s","error":"parse: nope"}\n' "$path"
  else
    printf '{"path":"%s","tokens":[4,5]}\n' "$path"
  fi
done
printf 'TCT_ENCODE_PHASE_METRICS\tencode-batch\t{"records":2,"failures":1,"profile_wall_time_ns":100,"path_input_ns":5,"json_read_parse_ns":40,"base_encode_ns":30,"bpe_encode_ns":null,"output_serialization_ns":10,"stdout_write_ns":8}\n' >&2
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let (sequences, failed, metrics) = encoder
            .encode_batch_efficient_with_phase_metrics(&[
                PathBuf::from("good.json"),
                PathBuf::from("bad.json"),
            ])
            .unwrap();

        assert_eq!(sequences, vec![vec![4, 5]]);
        assert_eq!(failed, vec![PathBuf::from("bad.json")]);
        assert_eq!(
            metrics,
            BatchEncodePhaseMetrics {
                records: 2,
                failures: 1,
                profile_wall_time_ns: 100,
                path_input_ns: 5,
                json_read_parse_ns: 40,
                base_encode_ns: 30,
                bpe_encode_ns: None,
                output_serialization_ns: 10,
                stdout_write_ns: 8,
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_batch_hot_path_metrics_parses_summary() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode-batch-hot-path" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
cat >/dev/null
printf 'TCT_ENCODE_HOT_PATH_METRICS\tencode-batch\t{"records":2,"failures":0,"token_count":3,"token_checksum":123,"profile_wall_time_ns":100,"path_input_ns":5,"json_read_parse_ns":40,"base_encode_ns":30,"bpe_encode_ns":null}\n' >&2
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let metrics = encoder
            .encode_batch_hot_path_metrics(&[
                PathBuf::from("first.json"),
                PathBuf::from("second.json"),
            ])
            .unwrap();

        assert_eq!(
            metrics,
            BatchEncodeHotPathMetrics {
                records: 2,
                failures: 0,
                token_count: 3,
                token_checksum: 123,
                profile_wall_time_ns: 100,
                path_input_ns: 5,
                json_read_parse_ns: 40,
                base_encode_ns: 30,
                bpe_encode_ns: None,
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_batch_validation_phase_metrics_parses_summary() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode-batch-validation-profile" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
cat >/dev/null
printf 'TCT_ENCODE_VALIDATION_PHASE_METRICS\tencode-batch\t{"records":2,"failures":0,"token_count":3,"token_checksum":123,"profile_wall_time_ns":100,"path_input_ns":5,"file_read_ns":11,"json_value_parse_ns":22,"serde_from_value_ns":33,"base_encode_ns":44,"bpe_encode_ns":null}\n' >&2
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let metrics = encoder
            .encode_batch_validation_phase_metrics(&[
                PathBuf::from("first.json"),
                PathBuf::from("second.json"),
            ])
            .unwrap();

        assert_eq!(
            metrics,
            BatchEncodeValidationPhaseMetrics {
                records: 2,
                failures: 0,
                token_count: 3,
                token_checksum: 123,
                profile_wall_time_ns: 100,
                path_input_ns: 5,
                file_read_ns: 11,
                json_value_parse_ns: 22,
                serde_from_value_ns: 33,
                base_encode_ns: 44,
                bpe_encode_ns: None,
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_batch_efficient_errors_on_nonzero_child_exit() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
cat >/dev/null
echo "fatal encoder crash" >&2
exit 9
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let error = encoder
            .encode_batch_efficient(&[PathBuf::from("good.json")])
            .unwrap_err();

        assert!(error.to_string().contains("fatal encoder crash"));
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_batch_efficient_treats_empty_token_record_as_failure() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode-batch" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
while IFS= read -r path; do
  if [[ "$path" == *empty.json ]]; then
    printf '{"path":"%s","tokens":[]}\n' "$path"
  else
    printf '{"path":"%s","tokens":[4,5]}\n' "$path"
  fi
done
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);
        let paths = vec![PathBuf::from("good.json"), PathBuf::from("empty.json")];

        let (sequences, failed) = encoder.encode_batch_efficient(&paths).unwrap();

        assert_eq!(sequences, vec![vec![4, 5]]);
        assert_eq!(failed, vec![PathBuf::from("empty.json")]);
    }

    #[cfg(unix)]
    #[test]
    fn test_encode_batch_skips_failed_and_empty_sequences() {
        let (_dir, bin) = write_fake_encoder(
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "encode" ]]; then
  echo "unexpected mode ${1:-}" >&2
  exit 2
fi
case "${2:-}" in
  *good.json) printf '[9]\n' ;;
  *empty.json) printf '[]\n' ;;
  *bad.json) echo "parse failure" >&2; exit 8 ;;
  *) echo "unexpected path ${2:-}" >&2; exit 3 ;;
esac
"#,
        );
        let encoder = instance_encoder_for_bin(&bin);

        let mut sequences = encoder
            .encode_batch(&[
                PathBuf::from("good.json"),
                PathBuf::from("empty.json"),
                PathBuf::from("bad.json"),
            ])
            .unwrap();
        sequences.sort();

        assert_eq!(sequences, vec![vec![9]]);
    }
}
