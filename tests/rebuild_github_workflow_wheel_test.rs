#[test]
#[ignore] // Run manually: cargo test --test rebuild_github_workflow_wheel_test rebuild -- --ignored --nocapture
fn rebuild_github_workflow_wheel() {
    use tct::wheel_builder::WheelConfig;

    eprintln!();
    eprintln!("🔧 Testing wheel builder with github-workflow schema (production test)...");
    eprintln!();

    let config = WheelConfig::new(
        "schemas/popular/github-workflow.json",
        "GithubWorkflow",
        "tct-github-workflow",
    )
    .with_version("1.0.2")
    .with_description(
        "TCT GitHub Workflow tokenizer with windowed generation and partial encoding support",
    )
    .with_max_base_vocab(512);

    let wheel_path = tct::wheel_builder::build_wheel(&config).unwrap();

    eprintln!();
    eprintln!("🎉 SUCCESS!");
    eprintln!("   Wheel: {}", wheel_path.display());
    eprintln!();
    eprintln!("📦 Install with:");
    eprintln!("   pip install {}", wheel_path.display());
    eprintln!();
    eprintln!("🐍 Python API:");
    eprintln!("   from tct_github_workflow import (");
    eprintln!("       encode, decode, vocab_size, pad_token,");
    eprintln!("       token_roles, strict_prefix_roles,");
    eprintln!("       encode_sequence, decode_sequence, sequence_metadata_json,");
    eprintln!("       decode_prefix,  # streaming wheels only");
    eprintln!("   )");
    eprintln!();
}
