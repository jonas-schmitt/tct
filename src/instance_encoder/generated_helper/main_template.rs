//! Main template selection for generated instance-encoder helpers.

mod base;
mod bpe;

use super::capabilities::{comment_out_log_debug_calls, generated_root_has_decode_prefix};
use super::decode_prefix::render_decode_prefix_batch_arm;
use super::support::render_helper_support_code;

pub(in crate::instance_encoder) struct GeneratedHelperMainInput<'a> {
    pub(in crate::instance_encoder) tokenizer_code: &'a str,
    pub(in crate::instance_encoder) root_type_name: &'a str,
    pub(in crate::instance_encoder) allocation_metrics_enabled: bool,
}

pub(in crate::instance_encoder) fn render_generated_helper_main(
    input: GeneratedHelperMainInput<'_>,
) -> String {
    let code_str = comment_out_log_debug_calls(input.tokenizer_code);
    let root_type = input.root_type_name;

    let has_bpe = code_str.contains("pub mod codec");
    let decode_prefix_batch_arm = render_decode_prefix_batch_arm(
        root_type,
        has_bpe,
        generated_root_has_decode_prefix(&code_str, root_type),
    );
    let allocation_tracking_code = render_helper_support_code(input.allocation_metrics_enabled);

    if has_bpe {
        bpe::render_bpe_helper_main(
            &code_str,
            allocation_tracking_code,
            root_type,
            &decode_prefix_batch_arm,
        )
    } else {
        base::render_base_helper_main(
            &code_str,
            allocation_tracking_code,
            root_type,
            &decode_prefix_batch_arm,
        )
    }
}
