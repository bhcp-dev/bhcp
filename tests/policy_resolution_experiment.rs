use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use bhcp::hash::format_hash;
use bhcp::model::ClauseKind;
use bhcp::pipeline::compile_source;

fn experiment() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("experiments/policy-resolution-agent")
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
                .join("target/policy-resolution-fixtures")
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
fn canonical_contract_pins_the_precedence_ladder_and_verifier_targets() {
    let path = experiment().join("contract.bhcp");
    let source = fs::read_to_string(&path).unwrap();
    let compiled = compile_source(&source, path.to_str().unwrap()).unwrap();
    let goal = &compiled.ir.goals[0];

    assert_eq!(goal.symbol, "experiment/ResolveTenantPolicy@0");
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
            ClauseKind::Verify {
                binding,
                obligations,
            } => Some((binding.verifier.as_str(), obligations.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        verifiers,
        [
            (
                "experiment/verifier/public-rust@0",
                vec!["clause-16".to_owned()]
            ),
            (
                "experiment/verifier/policy-resolution@0",
                (17..=23).map(|id| format!("clause-{id}")).collect()
            ),
            (
                "experiment/verifier/change-policy@0",
                vec!["clause-24".to_owned(), "clause-29".to_owned()]
            ),
        ]
    );
}

#[test]
fn prose_remains_ambiguous_where_the_contract_is_normative() {
    for path in [
        experiment().join("TASK.md"),
        experiment().join("results/pilot-004/refined-task.md"),
    ] {
        let task = fs::read_to_string(&path).unwrap();
        assert!(task.contains("tenant-policy-resolution@0"));
        for canonical_detail in [
            "specificity dominates priority",
            "lexicographically smaller",
            "more exact patterns",
        ] {
            assert!(
                !task.contains(canonical_detail),
                "{} accidentally disclosed canonical detail: {canonical_detail}",
                path.display()
            );
        }
    }
}

#[test]
fn pinned_subject_passes_public_tests_while_policy_oracle_rejects_it() {
    let public = cargo_test(&experiment().join("subject/Cargo.toml"), "subject");
    assert!(public.status.success(), "{}", output_text(&public));

    let oracle = cargo_test(&experiment().join("oracle/Cargo.toml"), "oracle");
    assert!(
        !oracle.status.success(),
        "buggy policy subject unexpectedly passed"
    );
    let output = output_text(&oracle);
    for invariant in [
        "a_rule_from_another_tenant_is_never_eligible",
        "specificity_dominates_numeric_priority",
        "deny_breaks_an_equal_specificity_and_priority_tie",
        "lexicographically_smaller_id_breaks_a_remaining_tie",
        "insertion_order_does_not_change_the_decision",
    ] {
        let failed = format!("test {invariant} ... FAILED");
        assert!(
            output.contains(&failed),
            "pinned subject did not fail {invariant}:\n{output}"
        );
    }
    assert!(
        output.contains("test higher_priority_breaks_an_equal_specificity_tie ... ok"),
        "oracle did not independently accept the priority invariant:\n{output}"
    );
    assert!(
        output.contains("test no_eligible_rule_is_denied_without_a_selected_rule ... ok"),
        "oracle did not independently accept the default-deny invariant:\n{output}"
    );
}
