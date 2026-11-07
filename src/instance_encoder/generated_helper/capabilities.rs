//! Detects generated-helper capabilities from rendered tokenizer source.
//!
//! The instance helper is assembled from generated Rust text before it is
//! compiled in a temporary Cargo project. These utilities keep that assembly
//! conservative: they remove repo-local debug macros that would not exist in
//! the helper crate and detect whether the generated root type exposes
//! `decode_prefix` before adding prefix-profiling commands.

pub(super) fn comment_out_log_debug_calls(code: &str) -> String {
    let mut in_log = false;
    let mut lines = Vec::new();
    for line in code.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("log_debug!(") {
            in_log = true;
            lines.push(format!("// {}", line));
            if trimmed.contains(");") {
                in_log = false;
            }
        } else if in_log {
            lines.push(format!("// {}", line));
            if trimmed.ends_with(");") {
                in_log = false;
            }
        } else {
            lines.push(line.to_string());
        }
    }
    lines.join("\n")
}

pub(super) fn generated_root_has_decode_prefix(code: &str, root_type: &str) -> bool {
    let impl_prefix = format!("impl {root_type}");
    let mut in_root_impl = false;
    let mut brace_depth = 0i64;

    for line in code.lines() {
        let trimmed = line.trim_start();
        if !in_root_impl
            && trimmed.starts_with(&impl_prefix)
            && trimmed[impl_prefix.len()..].trim_start().starts_with('{')
        {
            in_root_impl = true;
        }

        if in_root_impl {
            if trimmed.contains("pub fn decode_prefix(") {
                return true;
            }

            for ch in line.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => brace_depth -= 1,
                    _ => {}
                }
            }
            if brace_depth <= 0 {
                in_root_impl = false;
                brace_depth = 0;
            }
        }
    }

    false
}
