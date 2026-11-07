//! Renders and tests the generated instance-helper binary.
//!
//! `InstanceEncoder` builds a temporary Cargo project so measurements and
//! batch encodes run through the same generated tokenizer code users would
//! compile. This module is the facade for that generated `main.rs`: it wires
//! capability detection, optional decode-prefix profiling arms, support
//! helpers, and template rendering into one helper source.

mod capabilities;
mod decode_prefix;
mod main_template;
mod support;

pub(in crate::instance_encoder) use main_template::{
    GeneratedHelperMainInput, render_generated_helper_main,
};

#[cfg(test)]
use capabilities::{comment_out_log_debug_calls, generated_root_has_decode_prefix};
#[cfg(test)]
use decode_prefix::render_decode_prefix_batch_arm;
#[cfg(test)]
use support::render_helper_support_code;

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    fn parse_generated_file(code: &str) -> syn::File {
        syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
    }

    fn generated_file_tokens_contain(code: &str, expected: &str) -> bool {
        parse_generated_file(code)
            .to_token_stream()
            .to_string()
            .contains(expected)
    }

    fn parse_decode_prefix_arm(arm: &str) -> syn::File {
        let code = format!(
            r#"
            fn __decode_prefix_arm_probe(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {{
                match args[1].as_str() {{
                    {arm}
                    _ => {{}}
                }}
                Ok(())
            }}
            "#
        );
        parse_generated_file(&code)
    }

    fn decode_prefix_arm_tokens_contain(arm: &str, expected: &str) -> bool {
        parse_decode_prefix_arm(arm)
            .to_token_stream()
            .to_string()
            .contains(expected)
    }

    #[test]
    fn comment_out_log_debug_calls_handles_single_line_macros() {
        let code = "pub fn encode() {\n    log_debug!(\"token\");\n    emit();\n}";
        let prepared = comment_out_log_debug_calls(code);

        assert!(prepared.contains("//     log_debug!(\"token\");"));
        assert!(prepared.contains("    emit();"));
    }

    #[test]
    fn comment_out_log_debug_calls_handles_multiline_macros() {
        let code = r#"fn encode() {
    log_debug!(
        "token {}",
        token
    );
    emit();
}"#;
        let prepared = comment_out_log_debug_calls(code);

        assert!(prepared.contains("//     log_debug!("));
        assert!(prepared.contains("//         \"token {}\","));
        assert!(prepared.contains("//         token"));
        assert!(prepared.contains("//     );"));
        assert!(prepared.contains("    emit();"));
    }

    #[test]
    fn render_helper_support_code_can_enable_allocation_tracking() {
        let support = render_helper_support_code(true);

        assert!(generated_file_tokens_contain(
            support,
            "unsafe impl GlobalAlloc for TctCountingAllocator"
        ));
        assert!(support.contains("TCT_ALLOCATION_METRICS"));
        assert!(support.contains("TCT_MEASURE_ALLOCATIONS"));
    }

    #[test]
    fn render_helper_support_code_can_skip_allocation_tracking() {
        let support = render_helper_support_code(false);

        assert!(support.contains("fn start_allocation_tracking() -> bool"));
        assert!(support.contains("false"));
        assert!(!support.contains("TCT_ALLOCATION_METRICS"));
        assert!(!support.contains("TctCountingAllocator"));
    }

    #[test]
    fn render_generated_helper_main_wires_base_encoding_paths() {
        let main_rs = render_generated_helper_main(GeneratedHelperMainInput {
            tokenizer_code: r#"
pub mod tokenizer {
    pub trait JsonTokenEncoder {}
}

log_debug!(
    "hidden",
    42
);
"#,
            root_type_name: "RootType",
            allocation_metrics_enabled: false,
        });

        assert!(generated_file_tokens_contain(
            &main_rs,
            "let instance : RootType = serde_json :: from_str (& content) ?"
        ));
        assert!(generated_file_tokens_contain(
            &main_rs,
            "serde_json :: from_str :: < RootType > (& content)"
        ));
        assert!(main_rs.contains("let instance = RootType::decode(&mut iter)?;"));
        assert!(main_rs.contains("\"encode-batch-profile\" =>"));
        assert!(main_rs.contains("\"encode-batch-hot-path\" =>"));
        assert!(main_rs.contains("\"decode-batch\" =>"));
        assert!(main_rs.contains("\"roles-batch\" =>"));
        assert!(main_rs.contains("token_roles"));
        assert!(main_rs.contains("false,\n            );"));
        assert!(main_rs.contains("// log_debug!("));
        assert!(!main_rs.contains("codec::encode(&instance)"));
        assert!(!main_rs.contains("TCT_ALLOCATION_METRICS"));
    }

    #[test]
    fn render_generated_helper_main_wires_bpe_paths_and_allocation_tracking() {
        let main_rs = render_generated_helper_main(GeneratedHelperMainInput {
            tokenizer_code: r#"
pub mod tokenizer {
    pub trait JsonTokenEncoder {}
}

pub mod codec {}
"#,
            root_type_name: "RootType",
            allocation_metrics_enabled: true,
        });

        assert!(main_rs.contains("let tokens = codec::encode(&instance);"));
        assert!(main_rs.contains("let (instance, _, _) = codec::decode::<RootType>(tokens)?;"));
        assert!(main_rs.contains("match codec::decode::<RootType>(tokens)"));
        assert!(generated_file_tokens_contain(
            &main_rs,
            "let tokens = bpe :: encode_owned (base_tokens , VOCAB_SIZE)"
        ));
        assert!(main_rs.contains("\"bpe-encode-batch\" =>"));
        assert!(main_rs.contains("\"bpe-decode-batch\" =>"));
        assert!(main_rs.contains("\"roles-batch\" =>"));
        assert!(main_rs.contains("finish_allocation_tracking(\"encode-batch-hot-path\""));
        assert!(main_rs.contains("true,\n            );"));
        assert!(main_rs.contains("#[global_allocator]"));
        assert!(main_rs.contains("TCT_ALLOCATION_METRICS"));
    }

    #[test]
    fn generated_root_has_decode_prefix_ignores_structural_codec_helpers() {
        let code = r#"
pub struct PrefixDecodeResult<T> { pub value: Option<T> }
pub mod codec {
    pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> Option<u32> {
        tokens.next()
    }
}
pub struct Root;
impl Root {
    pub fn decode_partial(tokens: &[u32]) -> Result<Self, ()> {
        let _ = tokens;
        Ok(Self)
    }
}
pub struct StreamingRoot;
impl StreamingRoot {
    pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
        let _ = tokens;
        PrefixDecodeResult { value: Some(Self) }
    }
}
"#;

        assert!(!generated_root_has_decode_prefix(code, "Root"));
        assert!(generated_root_has_decode_prefix(code, "StreamingRoot"));
    }

    #[test]
    fn render_decode_prefix_batch_arm_replaces_root_type_and_bpe_mode() {
        let bpe_arm = render_decode_prefix_batch_arm("RootType", true, true);
        assert!(decode_prefix_arm_tokens_contain(
            &bpe_arm,
            "let base_tokens = bpe :: decode (& tokens , VOCAB_SIZE)"
        ));
        assert!(decode_prefix_arm_tokens_contain(
            &bpe_arm,
            "RootType :: decode_prefix (& mut iter)"
        ));
        assert!(bpe_arm.contains("let warmup_iterations = args"));
        assert!(bpe_arm.contains("for _ in 0..warmup_iterations"));
        assert!(bpe_arm.contains("true,"));

        let base_arm = render_decode_prefix_batch_arm("RootType", false, true);
        assert!(base_arm.contains("let records: Vec<Result<Vec<u32>, ()>>"));
        assert!(decode_prefix_arm_tokens_contain(
            &base_arm,
            "RootType :: decode_prefix (& mut iter)"
        ));
        assert!(base_arm.contains("let warmup_iterations = args"));
        assert!(base_arm.contains("for _ in 0..warmup_iterations"));
        assert!(base_arm.contains("false,"));
    }

    #[test]
    fn render_decode_prefix_batch_arm_without_support_exits_early() {
        let arm = render_decode_prefix_batch_arm("RootType", false, false);
        assert!(decode_prefix_arm_tokens_contain(
            &arm,
            "requires generated decode_prefix support"
        ));
        assert!(!decode_prefix_arm_tokens_contain(
            &arm,
            "RootType :: decode_prefix"
        ));
    }
}
