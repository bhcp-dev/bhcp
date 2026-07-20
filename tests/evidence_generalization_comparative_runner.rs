use std::path::PathBuf;

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn comparative_study_is_frozen_before_any_model_turn() {
    let root = repository();
    for relative in [
        "experiments/evidence-generalization/comparative-registration.md",
        "experiments/evidence-generalization/comparative-registration.txt",
        "experiments/evidence-generalization/COMPARATIVE_BHCP_PROMPT.md",
        "experiments/evidence-generalization/COMPARATIVE_PROSE_PROMPT.md",
        "src/bin/evidence_generalization_comparative_policy.rs",
        "src/bin/evidence_generalization_comparative.rs",
    ] {
        assert!(
            root.join(relative).is_file(),
            "missing frozen comparative input: {relative}"
        );
    }
}
