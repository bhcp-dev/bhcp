use std::fs;
use std::path::PathBuf;

const GATES: [(&str, &str, &str); 5] = [
    ("format", "Format", "cargo fmt --check"),
    (
        "clippy",
        "Clippy",
        "cargo clippy --all-targets -- -D warnings",
    ),
    ("tests", "Tests", "cargo test --all-targets"),
    ("release", "Release build", "cargo build --release"),
    (
        "schema",
        "17-root CDDL fixtures",
        "cargo test --test schema_fixtures all_seventeen_root_fixtures_parse_validate_and_round_trip -- --exact",
    ),
];

fn workflow() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    fs::read_to_string(path).expect("the required Rust CI workflow must exist")
}

#[test]
fn workflow_exposes_independent_required_gates() {
    let workflow = workflow();
    assert!(workflow.contains("pull_request:"));
    assert!(workflow.contains("push:\n    branches: [main]"));
    assert!(workflow.contains("name: Rust quality / ${{ matrix.name }}"));
    assert!(workflow.contains("run: mise exec -- ${{ matrix.command }}"));
    assert!(!workflow.contains("continue-on-error:"));

    for (id, name, command) in GATES {
        assert!(
            workflow.contains(&format!(
                "- id: {id}\n            name: {name}\n            command: {command}"
            )),
            "CI matrix omitted the {name} gate"
        );
    }
}

#[test]
fn workflow_uses_only_commit_pinned_actions_and_caches_cargo_dependencies() {
    let workflow = workflow();
    let action_lines = workflow
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("uses:"))
        .collect::<Vec<_>>();
    assert_eq!(action_lines.len(), 3);
    for line in action_lines {
        let reference = line
            .split_once('@')
            .expect("an action must have a ref")
            .1
            .split_whitespace()
            .next()
            .unwrap();
        assert_eq!(
            reference.len(),
            40,
            "action ref is not a commit SHA: {line}"
        );
        assert!(
            reference.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "action ref is not hexadecimal: {line}"
        );
    }

    for expected in [
        "~/.cargo/registry",
        "~/.cargo/git",
        "hashFiles('Cargo.lock')",
        "hashFiles('.mise.toml')",
    ] {
        assert!(workflow.contains(expected), "cache omitted {expected}");
    }
}
