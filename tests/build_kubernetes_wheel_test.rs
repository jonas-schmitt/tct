#[test]
#[ignore = "requires maturin/Python wheel tooling and a long Kubernetes+BPE wheel build"]
fn build_kubernetes_wheel_with_bpe_no_max_vocab() {
    use tct::wheel_builder::WheelConfig;

    eprintln!();
    eprintln!("Building Kubernetes wheel with legacy 500-vocab BPE regression coverage...");
    eprintln!();

    // BPE table selectable via TCT_K8S_BPE; only "500" ships in this repository.
    let bpe = std::env::var("TCT_K8S_BPE").unwrap_or_else(|_| "500".to_string());
    let merges = format!("schemas/bpe/kubernetes-tct-bpe-{bpe}.json");
    eprintln!("   BPE merges: {merges}");
    let config = WheelConfig::new(
        "schemas/popular/kubernetes.json",
        "Kubernetes",
        "tct-kubernetes",
    )
    .with_version("1.0.0")
    .with_description("TCT Kubernetes tokenizer with BPE regression coverage")
    .with_bpe_merges(&merges);

    let wheel_path = tct::wheel_builder::build_wheel(&config).unwrap();

    eprintln!();
    eprintln!("SUCCESS");
    eprintln!("   Wheel: {}", wheel_path.display());
    eprintln!();
    eprintln!("Install with:");
    eprintln!("   pip install {}", wheel_path.display());
    eprintln!();
}
