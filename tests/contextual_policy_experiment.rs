use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use bhcp::hash::format_hash;
use bhcp::model::ClauseKind;
use bhcp::pipeline::compile_source;

fn experiment() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("experiments/contextual-policy-agent")
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
                .join("target/contextual-policy-fixtures")
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

fn git_blob(path: &Path) -> String {
    let output = Command::new("git")
        .args(["hash-object", "--"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn copy_fixture_file(source: &Path, destination: &Path) {
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::copy(source, destination).unwrap();
}

fn replay_candidate(patch_name: &str, expected_blob: &str, accepted: bool) {
    let source = experiment();
    let replay = std::env::temp_dir()
        .join(format!(
            "bhcp-contextual-policy-replays-{}",
            std::process::id()
        ))
        .join(patch_name.trim_end_matches(".patch"));
    if replay.exists() {
        fs::remove_dir_all(&replay).unwrap();
    }

    for relative in [
        "subject/Cargo.toml",
        "subject/Cargo.lock",
        "subject/README.md",
        "subject/src/lib.rs",
        "subject/tests/public.rs",
        "oracle/Cargo.toml",
        "oracle/Cargo.lock",
        "oracle/src/lib.rs",
        "oracle/tests/invariants.rs",
    ] {
        copy_fixture_file(&source.join(relative), &replay.join(relative));
    }

    let patch = source.join("results/pilot-006").join(patch_name);
    let applied = Command::new("git")
        .args(["apply", "--no-index", "--unsafe-paths"])
        .arg(&patch)
        .current_dir(replay.join("subject"))
        .output()
        .unwrap();
    assert!(applied.status.success(), "{}", output_text(&applied));
    assert_eq!(git_blob(&replay.join("subject/src/lib.rs")), expected_blob);

    let public = cargo_test(&replay.join("subject/Cargo.toml"), "replays");
    assert!(public.status.success(), "{}", output_text(&public));

    let oracle = cargo_test(&replay.join("oracle/Cargo.toml"), "replays");
    assert_eq!(
        oracle.status.success(),
        accepted,
        "{}",
        output_text(&oracle)
    );
    if !accepted {
        let output = output_text(&oracle);
        for invariant in [
            "resource_specificity_dominates_other_exact_fields",
            "subject_specificity_breaks_equal_resource_scope",
        ] {
            assert!(
                output.contains(&format!("test {invariant} ... FAILED")),
                "negative candidate did not fail {invariant}:\n{output}"
            );
        }
    }

    fs::remove_dir_all(replay).unwrap();
}

#[test]
fn canonical_contract_pins_the_ordered_specificity_lattice() {
    let path = experiment().join("contract.bhcp");
    let source = fs::read_to_string(&path).unwrap();
    let compiled = compile_source(&source, path.to_str().unwrap()).unwrap();
    let goal = &compiled.ir.goals[0];

    assert_eq!(goal.symbol, "experiment/ResolveContextualPolicy@0");
    assert_eq!(
        format_hash(&compiled.semantic_hash),
        fs::read_to_string(experiment().join("contract.semantic-id"))
            .unwrap()
            .trim()
    );

    let verifier_targets: Vec<_> = goal
        .clauses
        .iter()
        .filter_map(|clause| match &clause.kind {
            ClauseKind::Verify {
                binding,
                obligations,
            } => Some((binding.verifier.as_str(), obligations.len())),
            _ => None,
        })
        .collect();
    assert_eq!(
        verifier_targets,
        [
            ("experiment/verifier/public-rust@0", 1),
            ("experiment/verifier/contextual-policy@0", 10),
            ("experiment/verifier/change-policy@0", 2),
        ]
    );

    let ensures: Vec<_> = goal
        .clauses
        .iter()
        .filter_map(|clause| match &clause.kind {
            ClauseKind::Contract {
                kind: "ensures", ..
            } => clause.label.as_deref(),
            _ => None,
        })
        .collect();
    assert_eq!(
        ensures,
        [
            "visible verification",
            "rules remain tenant-local",
            "no eligible rule is denied",
            "resource specificity dominates",
            "subject specificity breaks resource ties",
            "action specificity breaks subject ties",
            "priority breaks equal-shape ties",
            "deny breaks equal policy ties",
            "smaller rule ID breaks remaining ties",
            "insertion order has no meaning",
            "disabled rules remain ineligible",
            "change policy",
        ]
    );
}

#[test]
fn prose_does_not_disclose_the_normative_precedence_ladder() {
    let task = fs::read_to_string(experiment().join("TASK.md")).unwrap();
    assert!(task.contains("contextual-policy-resolution@0"));
    for canonical_detail in [
        "resource specificity dominates",
        "subject specificity breaks resource ties",
        "action specificity breaks subject ties",
        "lexicographically smaller",
    ] {
        assert!(
            !task.contains(canonical_detail),
            "prose accidentally disclosed canonical detail: {canonical_detail}"
        );
    }
}

#[test]
fn pinned_subject_passes_public_tests_while_oracle_exposes_multiple_defects() {
    let public = cargo_test(&experiment().join("subject/Cargo.toml"), "subject");
    assert!(public.status.success(), "{}", output_text(&public));

    let oracle = cargo_test(&experiment().join("oracle/Cargo.toml"), "oracle");
    assert!(
        !oracle.status.success(),
        "buggy subject unexpectedly passed"
    );
    let output = output_text(&oracle);

    for invariant in [
        "rules_are_tenant_local",
        "resource_specificity_dominates_other_exact_fields",
        "subject_specificity_breaks_equal_resource_scope",
        "deny_breaks_an_equal_policy_tie",
        "smaller_rule_id_breaks_the_final_tie",
        "insertion_order_is_not_semantic",
    ] {
        assert!(
            output.contains(&format!("test {invariant} ... FAILED")),
            "pinned subject did not fail {invariant}:\n{output}"
        );
    }

    for invariant in [
        "no_eligible_rule_defaults_to_deny",
        "action_specificity_breaks_remaining_shape_ties",
        "priority_breaks_equal_specificity_ties",
        "disabled_rules_remain_ineligible",
    ] {
        assert!(
            output.contains(&format!("test {invariant} ... ok")),
            "oracle did not independently accept {invariant}:\n{output}"
        );
    }
}

#[test]
fn pilot_006_preserves_the_negative_result_and_latest_skill_follow_up() {
    let result = experiment().join("results/pilot-006");
    let report = fs::read_to_string(result.join("README.md")).unwrap();

    assert!(report.contains("Raw BHCP and prose both produced independently accepted patches"));
    assert!(report.contains("| Withheld policy invariants | 10/10 | 10/10 | 8/10 |"));
    assert!(report.contains("resource_specificity_dominates_other_exact_fields"));
    assert!(report.contains("subject_specificity_breaks_equal_resource_scope"));
    assert!(report.contains("| Independently accepted | **no** | **yes** |"));

    for patch in [
        "prose.patch",
        "raw-bhcp.patch",
        "skill.patch",
        "current-skill.patch",
    ] {
        let contents = fs::read_to_string(result.join(patch)).unwrap();
        assert!(contents.starts_with("diff --git a/src/lib.rs b/src/lib.rs\n"));
    }

    for (path, blob) in [
        (
            "subject/src/lib.rs",
            "3f126bfde1c0e06309686c9c3514548759d650eb",
        ),
        ("TASK.md", "82ae3d3545ee1f73fe6ed7180a1278e4680ab420"),
        ("contract.bhcp", "dfb58210587b15abfc0d0cbaa337a653b5d6dd29"),
        (
            "oracle/tests/invariants.rs",
            "3667d107a7777a09f71a69c871802c0f4e07dde1",
        ),
        (
            "results/pilot-006/evaluated-skill/SKILL.md",
            "b1a2f5fdfb3044be679f1e947bf1a1e56957e278",
        ),
        (
            "results/pilot-006/evaluated-skill/agents/openai.yaml",
            "e6611a4fb13245035f7a9ecf191d48baac49b858",
        ),
        (
            "results/pilot-006/latest-skill/SKILL.md",
            "7c6b39df7e7682afa59e2d75d1f4cb13276ea2c7",
        ),
        (
            "results/pilot-006/latest-skill/agents/openai.yaml",
            "e6611a4fb13245035f7a9ecf191d48baac49b858",
        ),
    ] {
        assert_eq!(git_blob(&experiment().join(path)), blob);
    }

    for (patch, blob, accepted) in [
        (
            "prose.patch",
            "a5beb69383f4870da9db24d7e5ea222ee6b08938",
            true,
        ),
        (
            "raw-bhcp.patch",
            "b016d1e687ce3fa85f0ded03779f51edab67e576",
            true,
        ),
        (
            "skill.patch",
            "8536cc2fda4ba0792b9993f3c0288f49cf23e74a",
            false,
        ),
        (
            "current-skill.patch",
            "de7fd05ca696476a510354f3f37dba0585c13b4c",
            true,
        ),
    ] {
        replay_candidate(patch, blob, accepted);
    }
}
