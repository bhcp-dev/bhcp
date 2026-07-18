use std::fs;
use std::path::PathBuf;

fn skill() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".codex/skills/interpret-bhcp-contract")
}

#[test]
fn interpretation_skill_is_explicit_dependency_free_and_fail_closed() {
    let instructions = fs::read_to_string(skill().join("SKILL.md")).unwrap();
    let interface = fs::read_to_string(skill().join("agents/openai.yaml")).unwrap();
    let normalized = instructions
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    assert!(instructions.starts_with("---\nname: interpret-bhcp-contract\n"));
    assert!(instructions.contains("description: Interpret and operationalize"));
    assert!(!instructions.contains("TODO"));
    assert!(
        instructions.len() <= 3_600,
        "skill is too large for its routine context budget: {} bytes",
        instructions.len()
    );
    assert!(normalized.contains("Never stream raw AST or IR bytes"));
    assert!(normalized.contains("Rust CLI"));
    assert!(normalized.contains("existing artifact boundary"));
    assert!(normalized.contains("pass the `.cbor` file back to `bhcp inspect`"));
    assert!(normalized.contains("Do not run `lower` for routine contract interpretation"));
    assert!(normalized.contains("Resolve the CLI once with `command -v bhcp`"));
    assert!(normalized.contains("Never read an executable as text"));
    assert!(normalized.contains("Do not probe `bhcp --help`"));
    assert!(normalized.contains("Do not open the contract source when `inspect` succeeds"));
    assert!(normalized.contains("Batch independent discovery and reads"));
    assert!(
        normalized.contains("Do not inventory the repository, list tests, query Cargo metadata")
    );
    assert!(normalized.contains("read the task, relevant source, and visible tests once"));
    assert!(normalized.contains("Keep the checklist internal"));
    assert!(normalized.contains("Do not print it unless the user asks"));
    assert!(normalized.contains("Never create a checklist row for a `verify` clause"));
    assert!(normalized.contains("Do not narrate each workflow transition"));
    assert!(!instructions.contains("| Structural ID |"));
    assert!(!instructions.contains("Include every `requires`"));
    assert!(normalized.contains("not evidence that any verifier ran"));
    assert!(normalized.contains("its own clause ID is not an obligation"));
    assert!(normalized.contains("do not search for alternate toolchain installs"));
    assert!(normalized.contains("implementation state separate from evidence state"));
    assert!(normalized.contains("mark its obligations unresolved"));
    assert!(normalized.contains("Never add tests or extra edits as a substitute"));
    assert!(
        normalized.contains(
            "A visible check is not adapter evidence unless it is the registered producer"
        )
    );

    assert!(interface.contains("display_name: \"Interpret BHCP Contract\""));
    assert!(interface.contains("Use $interpret-bhcp-contract"));
    assert!(interface.contains("compactly"));
    assert!(interface.contains("allow_implicit_invocation: false"));

    let files: Vec<_> = fs::read_dir(skill())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(
        files.len(),
        2,
        "skill gained an unexpected bundled dependency"
    );
}
