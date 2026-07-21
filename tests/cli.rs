use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use bhcp::cbor::decode_deterministic;
use bhcp::schema::validate_root;
use bhcp::value::Value;

fn run(command: &str, fixture: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bhcp"))
        .arg(command)
        .arg(fixture)
        .output()
        .unwrap()
}

#[test]
fn parse_and_lower_emit_cddl_validated_deterministic_cbor() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures/canonical-simple.bhcp");
    for (command, kind) in [("parse", "canonical-ast"), ("lower", "semantic-ir")] {
        let output = run(command, &fixture);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let artifact = decode_deterministic(&output.stdout).unwrap();
        assert_eq!(artifact.kind(), Some(kind));
        validate_root(&artifact, kind).unwrap();
    }
}

#[test]
fn inspect_and_hash_are_human_interfaces_over_canonical_artifacts() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures/canonical-simple.bhcp");
    let inspect = run("inspect", &fixture);
    assert!(inspect.status.success());
    let inspection = String::from_utf8(inspect.stdout).unwrap();
    assert!(inspection.contains("artifact semantic-ir"));
    assert!(inspection.contains("semantic_id bhcp.hash/sha3-512@0:"));
    assert!(inspection.contains("goal goal-1 example/Greet@0"));

    let hash = run("hash", &fixture);
    assert!(hash.status.success());
    let hash = String::from_utf8(hash.stdout).unwrap();
    let lines: Vec<_> = hash.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("bhcp.hash/sha3-512@0:"));
    assert_eq!(lines[0].rsplit_once(':').unwrap().1.len(), 128);
}

#[test]
fn released_cli_accepts_a_relative_source_without_a_project_manifest() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let trial =
        std::env::temp_dir().join(format!("bhcp-relative-cli-{}-{nonce}", std::process::id()));
    fs::create_dir(&trial).unwrap();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures/canonical-simple.bhcp");
    fs::copy(fixture, trial.join("canonical-simple.bhcp")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bhcp"))
        .current_dir(&trial)
        .args(["inspect", "canonical-simple.bhcp"])
        .output()
        .unwrap();
    fs::remove_dir_all(&trial).unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("goal goal-1 example/Greet@0")
    );
}

#[test]
fn inspect_reads_checked_in_canonical_cbor_without_another_format() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("conformance/v0/fixtures");
    let source = run("inspect", &root.join("canonical-simple.bhcp"));
    let artifact = run("inspect", &root.join("canonical-simple.ir.cbor"));
    assert!(source.status.success());
    assert!(artifact.status.success());

    let source = String::from_utf8(source.stdout).unwrap();
    let artifact = String::from_utf8(artifact.stdout).unwrap();
    assert_eq!(
        source.lines().skip(1).collect::<Vec<_>>(),
        artifact.lines().skip(1).collect::<Vec<_>>()
    );
}

#[test]
fn self_hosted_all_flows_through_canonical_cbor() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures/canonical-all.bhcp");
    let lower = run("lower", &fixture);
    assert!(
        lower.status.success(),
        "{}",
        String::from_utf8_lossy(&lower.stderr)
    );
    let artifact = decode_deterministic(&lower.stdout).unwrap();
    assert!(contains_text(&artifact, "bhcp/prelude.all-reducer-"));
    assert!(contains_text(&artifact, "kernel.has-refuted@0"));
    assert!(!contains_text(&artifact, "lower-all"));
    assert!(!contains_text(&artifact, "derived-form"));
}

#[test]
fn inspect_exposes_a_compact_structural_contract_outline() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("experiments/policy-resolution-agent/contract.bhcp");
    let output = run("inspect", &fixture);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "goal goal-1 experiment/ResolveTenantPolicy@0",
        "[clause-17] ensures \"rules remain tenant-local\": tenantIsolationPassed",
        "[clause-28] forbids \"offline task\": bhcp-effect/network@0",
        "[clause-32] verify \"withheld policy oracle\": experiment/verifier/policy-resolution@0 -> clause-17",
    ] {
        assert!(stdout.contains(expected), "inspect omitted {expected}");
    }
    assert!(!stdout.contains("expr-"));
    assert!(
        stdout.len() < 4_000,
        "inspection is too large for routine use"
    );
}

#[test]
fn inspect_expands_goal_wide_verifier_targets() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures/canonical-simple.bhcp");
    let output = run("inspect", &fixture);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("bhcp-verifier/expression@0 -> clause-4, clause-5, clause-8"));
}

fn contains_text(value: &Value, needle: &str) -> bool {
    match value {
        Value::Text(value) => value.contains(needle),
        Value::Array(values) => values.iter().any(|value| contains_text(value, needle)),
        Value::Map(entries) => entries
            .iter()
            .any(|(key, value)| key.contains(needle) || contains_text(value, needle)),
        Value::Tag(_, value) => contains_text(value, needle),
        _ => false,
    }
}
