use std::collections::BTreeMap;
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

fn mise_config() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".mise.toml");
    fs::read_to_string(path).expect("the pinned mise contract must exist")
}

fn cargo_manifest() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    fs::read_to_string(path).expect("the Cargo manifest must exist")
}

fn integration_targets() -> Vec<String> {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut targets = fs::read_dir(tests)
        .expect("the integration-test directory must exist")
        .map(|entry| entry.expect("integration-test entry must be readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .expect("integration-test filenames must be UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    targets.sort();
    targets
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
fn long_test_gate_has_time_to_finish_without_weakening_other_job_limits() {
    let workflow = workflow();
    assert!(workflow.contains("timeout-minutes: ${{ matrix.timeout }}"));
    assert!(workflow.contains(
        "- id: tests\n            name: Tests\n            command: cargo test --all-targets\n            timeout: 60"
    ));
    for id in ["format", "clippy", "release", "schema"] {
        let start = workflow
            .find(&format!("- id: {id}\n"))
            .unwrap_or_else(|| panic!("missing {id} matrix entry"));
        let entry = &workflow[start..];
        let end = entry[1..]
            .find("          - id:")
            .map_or(entry.len(), |index| index + 1);
        assert!(entry[..end].contains("timeout: 30"), "{id} timeout drifted");
    }
}

#[test]
fn hosted_test_partitions_cover_every_target_once_and_keep_the_required_context() {
    let workflow = workflow();
    assert!(
        !workflow.contains("command: cargo test --all-targets"),
        "the hosted gate still serializes every test target"
    );
    assert!(workflow.contains("name: Rust quality / Test plan"));
    assert!(workflow.contains("name: Rust quality / Test shard / ${{ matrix.name }}"));
    assert!(workflow.contains("needs: [test-plan, test-shards]"));
    assert!(workflow.contains("name: Rust quality / Tests\n"));
    assert!(workflow.contains("PLAN_RESULT: ${{ needs.test-plan.result }}"));
    assert!(workflow.contains("SHARD_RESULT: ${{ needs.test-shards.result }}"));
    assert!(workflow.matches("- id: shard-").count() >= 4);
    assert!(
        workflow
            .lines()
            .any(|line| line.trim() == "command: cargo test --lib --bins"),
        "library and binary test targets are not scheduled"
    );

    let mut scheduled = BTreeMap::<String, usize>::new();
    for line in workflow.lines().map(str::trim) {
        let Some(command) = line.strip_prefix("command: cargo test ") else {
            continue;
        };
        let fields = command.split_ascii_whitespace().collect::<Vec<_>>();
        for pair in fields.windows(2) {
            if pair[0] == "--test" {
                *scheduled.entry(pair[1].to_owned()).or_default() += 1;
            }
        }
    }
    let expected = integration_targets();
    assert_eq!(
        scheduled.keys().cloned().collect::<Vec<_>>(),
        expected,
        "hosted integration-test coverage drifted"
    );
    assert!(
        scheduled.values().all(|count| *count == 1),
        "an integration-test target is scheduled more than once: {scheduled:?}"
    );
}

#[test]
fn test_only_sha3_hot_path_is_optimised() {
    let manifest = cargo_manifest();
    for package in ["sha3", "keccak"] {
        let profile = format!("[profile.test.package.{package}]\nopt-level = 3");
        assert!(
            manifest.contains(&profile),
            "the test profile does not optimise {package}"
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

#[test]
fn pinned_rust_toolchain_includes_quality_gate_components() {
    let config = mise_config();
    assert!(config.contains("version = \"1.97.1\""));
    assert!(config.contains("components = [\"clippy\", \"rustfmt\"]"));
}
