use std::path::PathBuf;
use std::process::Command;

#[test]
fn parse_lower_inspect_and_hash_commands_work() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures/canonical-simple.bhcp");
    for command in ["parse", "lower", "inspect", "hash"] {
        let output = Command::new(env!("CARGO_BIN_EXE_bhcp"))
            .arg(command)
            .arg(&fixture)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        if command == "hash" {
            let lines: Vec<_> = stdout.lines().collect();
            assert_eq!(lines.len(), 1);
            assert!(lines[0].starts_with("bhcp.hash/sha3-512@0:"));
            assert_eq!(lines[0].rsplit_once(':').unwrap().1.len(), 128);
        } else {
            assert!(stdout.contains(if command == "parse" {
                "canonical-ast"
            } else if command == "lower" {
                "semantic-ir"
            } else {
                "semantic_id"
            }));
        }
    }
}

#[test]
fn self_hosted_all_flows_through_the_existing_cli() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures/canonical-all.bhcp");
    let lower = Command::new(env!("CARGO_BIN_EXE_bhcp"))
        .arg("lower")
        .arg(&fixture)
        .output()
        .unwrap();
    assert!(
        lower.status.success(),
        "{}",
        String::from_utf8_lossy(&lower.stderr)
    );
    let stdout = String::from_utf8(lower.stdout).unwrap();
    assert!(stdout.contains("bhcp/prelude.all-reducer-"));
    assert!(stdout.contains("kernel.has-refuted@0"));
    assert!(!stdout.contains("lower-all"));
    assert!(!stdout.contains("derived-form"));
}
