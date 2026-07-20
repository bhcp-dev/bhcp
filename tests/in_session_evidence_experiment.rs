use std::fs;
use std::path::PathBuf;
use std::process::Command;

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::hash::HashAlgorithm;
use bhcp::model::{ClauseKind, ContentReference};
use bhcp::pipeline::compile_source;
use bhcp::value::Value;

fn experiment() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("experiments/in-session-evidence-agent")
}

fn adapter(mode: &str, root: &std::path::Path) -> std::process::Output {
    let verifier = match mode {
        "public" => "experiment/verifier/public-rust@0",
        "oracle" => "experiment/verifier/in-session-oracle@0",
        "change-policy" => "experiment/verifier/change-policy@0",
        _ => unreachable!(),
    };
    let subject_bytes = fs::read(root.join("subject/src/lib.rs")).unwrap();
    let subject = ContentReference::from_bytes(
        "application/vnd.bhcp.subject-source",
        &subject_bytes,
        HashAlgorithm::default(),
    );
    Command::new(env!("CARGO_BIN_EXE_bhcp-in-session-evidence-adapter"))
        .arg(mode)
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(
                &encode_deterministic(&Value::map([
                    ("version", Value::Text("bhcp/adapter-request@0".to_owned())),
                    ("verifier", Value::Text(verifier.to_owned())),
                    ("obligations", Value::Array(vec![])),
                    ("payload", Value::Bytes(vec![0])),
                    ("subject", subject.to_value()),
                    ("subject_content", Value::Bytes(subject_bytes)),
                ]))
                .unwrap(),
            )?;
            child.wait_with_output()
        })
        .unwrap()
}

#[test]
fn contract_binds_all_mandatory_targets_to_project_adapters() {
    let path = experiment().join("contract.bhcp");
    let compiled =
        compile_source(&fs::read_to_string(&path).unwrap(), path.to_str().unwrap()).unwrap();
    let goal = &compiled.ir.goals[0];
    let bindings: Vec<_> = goal
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
        bindings,
        [
            ("experiment/verifier/public-rust@0", 1),
            ("experiment/verifier/in-session-oracle@0", 1),
            ("experiment/verifier/change-policy@0", 2),
        ]
    );
}

#[test]
fn bounded_adapters_reject_the_starter_before_the_forward_test() {
    for mode in ["public", "oracle", "change-policy"] {
        let output = adapter(mode, &experiment());
        assert!(output.status.success());
        let response = decode_deterministic(&output.stdout).unwrap();
        assert_eq!(
            response.get("state"),
            Some(&Value::Text("rejected".to_owned()))
        );
    }
}

#[test]
fn bounded_adapters_accept_the_exact_focused_candidate() {
    let root = std::env::temp_dir().join(format!(
        "bhcp-in-session-adapter-final-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(root.join("subject/src")).unwrap();
    fs::write(
        root.join("subject/src/lib.rs"),
        "pub fn public_ready() -> bool {\n    true\n}\n\npub fn oracle_ready() -> bool {\n    true\n}\n\npub fn policy_ready() -> bool {\n    true\n}\n",
    )
    .unwrap();
    for mode in ["public", "oracle", "change-policy"] {
        let output = adapter(mode, &root);
        assert!(output.status.success());
        let response = decode_deterministic(&output.stdout).unwrap();
        assert_eq!(
            response.get("state"),
            Some(&Value::Text("accepted".to_owned()))
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn registry_evidence_is_bound_to_the_exact_supplied_subject() {
    let root = std::env::temp_dir().join(format!(
        "bhcp-in-session-subject-binding-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(root.join("subject/src")).unwrap();
    fs::create_dir_all(root.join("subject/tools")).unwrap();
    for path in ["bhcp-project.toml", "contract.bhcp"] {
        fs::copy(experiment().join(path), root.join(path)).unwrap();
    }
    fs::write(
        root.join("candidate.cbor"),
        encode_deterministic(&Value::map([
            (
                "input",
                Value::map([(
                    "repository",
                    Value::Text("in-session-evidence@0".to_owned()),
                )]),
            ),
            (
                "output",
                Value::map([
                    ("publicPassed", Value::Bool(true)),
                    ("oraclePassed", Value::Bool(true)),
                    ("policyPassed", Value::Bool(true)),
                    (
                        "changedFiles",
                        Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(1)]),
                    ),
                ]),
            ),
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_bhcp-in-session-evidence-adapter"),
        root.join("subject/tools/in-session-evidence-adapter"),
    )
    .unwrap();
    fs::write(
        root.join("subject/src/lib.rs"),
        "pub fn public_ready() -> bool {\n    true\n}\n\npub fn oracle_ready() -> bool {\n    true\n}\n\npub fn policy_ready() -> bool {\n    true\n}\n",
    )
    .unwrap();
    let supplied = root.join("supplied-subject.rs");
    fs::write(&supplied, "").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bhcp"))
        .args([
            "verify",
            root.join("contract.bhcp").to_str().unwrap(),
            "experiment/InSessionEvidence@0",
            root.join("candidate.cbor").to_str().unwrap(),
            supplied.to_str().unwrap(),
            "2026-07-19T18:00:00Z",
        ])
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bundle = decode_deterministic(&output.stdout).unwrap();
    let Value::Array(claims) = bundle.get("claims").unwrap() else {
        panic!("evidence bundle claims must be an array")
    };
    assert!(claims.iter().all(|claim| {
        claim.get("subject").and_then(|subject| subject.get("size")) == Some(&Value::Integer(0))
    }));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn forward_001_preserves_the_unreplaced_negative_result() {
    let result = experiment().join("results/forward-001");
    let registration =
        fs::read_to_string(experiment().join("results/forward-001-registration.md")).unwrap();
    let report = fs::read_to_string(result.join("README.md")).unwrap();
    let controller = fs::read_to_string(result.join("CONTROLLER.md")).unwrap();
    assert!(registration.contains("one fixed arm, `forward-01`, with no replacement"));
    assert!(report.contains("**0/1 accepted**"));
    assert!(report.contains("included forward-test failure"));
    assert!(report.contains("no in-session evidence bundle"));
    assert!(report.contains("`claimed_success=false` was calibrated"));
    assert!(controller.contains("| forward-01 | rejected (verification-failed) | no |"));
    assert!(controller.contains("- Completed commands: 0"));
    assert_eq!(controller.matches("rejected (exit Some(").count(), 3);
    assert_eq!(
        fs::metadata(result.join("forward-01.patch")).unwrap().len(),
        0
    );
}
