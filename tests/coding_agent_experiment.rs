use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use bhcp::hash::format_hash;
use bhcp::model::ClauseKind;
use bhcp::pipeline::compile_source;

fn experiment() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("experiments/minimal-coding-agent")
}

fn cargo_test(manifest: &Path, target_name: &str) -> Output {
    Command::new(env!("CARGO"))
        .args([
            "test",
            "--offline",
            "--manifest-path",
            manifest.to_str().unwrap(),
        ])
        .env(
            "CARGO_TARGET_DIR",
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/experiment-fixtures")
                .join(target_name),
        )
        .output()
        .unwrap()
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn contract_compiles_to_the_expected_verifier_boundary() {
    let path = experiment().join("contract.bhcp");
    let source = fs::read_to_string(&path).unwrap();
    let compiled = compile_source(&source, path.to_str().unwrap()).unwrap();
    let goal = &compiled.ir.goals[0];

    assert_eq!(goal.symbol, "experiment/RepairBatchLedger@0");
    assert_eq!(
        format_hash(&compiled.semantic_hash),
        fs::read_to_string(experiment().join("contract.semantic-id"))
            .unwrap()
            .trim()
    );
    let verifiers: Vec<_> = goal
        .clauses
        .iter()
        .filter_map(|clause| match &clause.kind {
            ClauseKind::Verify { binding } => Some(binding.verifier.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        verifiers,
        [
            "experiment/verifier/public-rust@0",
            "experiment/verifier/ledger-invariants@0",
            "experiment/verifier/change-policy@0",
        ]
    );
}

#[test]
fn pinned_subject_passes_public_tests_while_invariant_oracle_rejects_it() {
    let public = cargo_test(&experiment().join("subject/Cargo.toml"), "subject");
    assert!(public.status.success(), "{}", output_text(&public));

    let oracle = cargo_test(&experiment().join("oracle/Cargo.toml"), "oracle");
    assert!(
        !oracle.status.success(),
        "buggy subject unexpectedly passed"
    );
    let output = output_text(&oracle);
    for invariant in [
        "later_failure_must_not_commit_earlier_transfers",
        "destination_overflow_must_not_debit_the_source",
        "aggregate_overflow_must_roll_back_the_entire_batch",
        "a_request_id_reused_with_a_different_payload_must_conflict",
        "a_failed_request_id_can_retry_against_the_original_state",
    ] {
        let failed = format!("test {invariant} ... FAILED");
        assert!(
            output.contains(&failed),
            "pinned subject did not fail {invariant}:\n{output}"
        );
    }
    assert!(
        output
            .contains("test successful_batches_conserve_balance_and_report_the_checked_sum ... ok"),
        "oracle did not independently accept the successful-batch invariant:\n{output}"
    );
}
