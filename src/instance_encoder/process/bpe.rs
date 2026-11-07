//! BPE instance-encoding process orchestration.

use super::super::InstanceEncoder;
use super::super::metrics::{BatchRuntimeMetrics, parse_batch_runtime_metrics};
use super::ALLOCATION_METRICS_ENV;
use std::error::Error;
use std::process::Command;

impl InstanceEncoder {
    /// Apply generated BPE encoding to a batch of pre-encoded base token sequences.
    ///
    /// This uses the generated helper binary's embedded `bpe::encode` function and
    /// is intended for measurement workflows that need to isolate generated BPE
    /// code from schema JSON parsing and base tokenization.
    #[allow(clippy::type_complexity)]
    pub fn bpe_encode_batch_efficient(
        &self,
        token_sequences: &[Vec<u32>],
    ) -> Result<(Vec<Vec<u32>>, Vec<usize>), Box<dyn Error>> {
        let (sequences, failed, _) =
            self.bpe_token_batch_inner("bpe-encode-batch", token_sequences, 0, false)?;
        Ok((sequences, failed))
    }

    /// Apply generated BPE encoding and return child-process runtime metrics.
    #[allow(clippy::type_complexity)]
    pub fn bpe_encode_batch_efficient_with_runtime_metrics(
        &self,
        token_sequences: &[Vec<u32>],
        warmup_iterations: u32,
    ) -> Result<(Vec<Vec<u32>>, Vec<usize>, Option<BatchRuntimeMetrics>), Box<dyn Error>> {
        self.bpe_token_batch_inner(
            "bpe-encode-batch",
            token_sequences,
            warmup_iterations,
            false,
        )
    }

    /// Apply generated BPE encoding and request generated-process allocation metrics.
    #[allow(clippy::type_complexity)]
    pub fn bpe_encode_batch_efficient_with_metrics(
        &self,
        token_sequences: &[Vec<u32>],
        warmup_iterations: u32,
    ) -> Result<(Vec<Vec<u32>>, Vec<usize>, Option<BatchRuntimeMetrics>), Box<dyn Error>> {
        if !self.allocation_metrics_enabled {
            return Err("InstanceEncoder was not built with allocation metrics enabled".into());
        }
        self.bpe_token_batch_inner("bpe-encode-batch", token_sequences, warmup_iterations, true)
    }

    /// Apply generated BPE decoding to a batch of BPE-compressed token sequences.
    ///
    /// This uses the generated helper binary's embedded `bpe::decode` function and
    /// returns base token sequences in the same order as the input records.
    #[allow(clippy::type_complexity)]
    pub fn bpe_decode_batch_efficient(
        &self,
        token_sequences: &[Vec<u32>],
    ) -> Result<(Vec<Vec<u32>>, Vec<usize>), Box<dyn Error>> {
        let (sequences, failed, _) =
            self.bpe_token_batch_inner("bpe-decode-batch", token_sequences, 0, false)?;
        Ok((sequences, failed))
    }

    /// Apply generated BPE decoding and return child-process runtime metrics.
    #[allow(clippy::type_complexity)]
    pub fn bpe_decode_batch_efficient_with_runtime_metrics(
        &self,
        token_sequences: &[Vec<u32>],
        warmup_iterations: u32,
    ) -> Result<(Vec<Vec<u32>>, Vec<usize>, Option<BatchRuntimeMetrics>), Box<dyn Error>> {
        self.bpe_token_batch_inner(
            "bpe-decode-batch",
            token_sequences,
            warmup_iterations,
            false,
        )
    }

    /// Apply generated BPE decoding and request generated-process allocation metrics.
    #[allow(clippy::type_complexity)]
    pub fn bpe_decode_batch_efficient_with_metrics(
        &self,
        token_sequences: &[Vec<u32>],
        warmup_iterations: u32,
    ) -> Result<(Vec<Vec<u32>>, Vec<usize>, Option<BatchRuntimeMetrics>), Box<dyn Error>> {
        if !self.allocation_metrics_enabled {
            return Err("InstanceEncoder was not built with allocation metrics enabled".into());
        }
        self.bpe_token_batch_inner("bpe-decode-batch", token_sequences, warmup_iterations, true)
    }

    #[allow(clippy::type_complexity)]
    fn bpe_token_batch_inner(
        &self,
        mode: &str,
        token_sequences: &[Vec<u32>],
        warmup_iterations: u32,
        track_allocations: bool,
    ) -> Result<(Vec<Vec<u32>>, Vec<usize>, Option<BatchRuntimeMetrics>), Box<dyn Error>> {
        use std::io::{BufRead, BufReader, Write};

        let mut command = Command::new(&self.encoder_bin);
        command
            .arg(mode)
            .arg(warmup_iterations.to_string())
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
            .ok_or("Failed to open stdin for generated BPE batch")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to open stdout for generated BPE batch")?;

        let mut sequences = Vec::new();
        let mut failed = Vec::new();

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
                    .and_then(|value| usize::try_from(value).ok());

                if let Some(tokens) = parsed.get("tokens") {
                    sequences.push(serde_json::from_value(tokens.clone())?);
                } else if let Some(index) = index {
                    failed.push(index);
                }
            }

            match writer_thread.join() {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => {
                    Err(format!("Generated BPE batch stdin writer failed: {error}").into())
                }
                Err(_) => Err("Generated BPE batch stdin writer panicked".into()),
            }
        })?;

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.trim().is_empty() {
                return Err(
                    format!("Generated BPE batch process failed: {}", output.status).into(),
                );
            }
            return Err(format!("Generated BPE batch process failed: {}", stderr.trim()).into());
        }

        let metrics = parse_batch_runtime_metrics(&output.stderr, mode, track_allocations);

        Ok((sequences, failed, metrics))
    }
}
