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

fn adapter(mode: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_bhcp-in-session-evidence-adapter"))
        .arg(mode)
        .current_dir(experiment())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(&encode_deterministic(&Value::map([(
                    "version",
                    Value::Text("fixture-request@0".to_owned()),
                )]))
                .unwrap())?;
            child.wait_with_output()
        })
        .unwrap()
}

#[test]
fn contract_binds_all_mandatory_targets_to_project_adapters() {
    let path = experiment().join("contract.bhcp");
    let compiled = compile_source(&fs::read_to_string(&path).unwrap(), path.to_str().unwrap())
        .unwrap();
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
        let output = adapter(mode);
        assert!(output.status.success());
        let response = decode_deterministic(&output.stdout).unwrap();
        assert_eq!(response.get("state"), Some(&Value::Text("rejected".to_owned())));
    }
}
