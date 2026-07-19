use std::fs;
use std::path::PathBuf;
use std::process::Command;

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::model::ClauseKind;
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
