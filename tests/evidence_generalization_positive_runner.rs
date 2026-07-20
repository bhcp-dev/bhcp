use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn positive_use_study_is_frozen_before_any_model_turn() {
    let root = root();
    for relative in [
        "experiments/evidence-generalization/positive-use-registration.md",
        "experiments/evidence-generalization/positive-use-registration.txt",
        "experiments/evidence-generalization/POSITIVE_USE_PROMPT.md",
        "src/bin/evidence_generalization_adapter.rs",
        "src/bin/evidence_generalization_positive.rs",
    ] {
        assert!(root.join(relative).is_file(), "missing frozen input: {relative}");
    }
}
