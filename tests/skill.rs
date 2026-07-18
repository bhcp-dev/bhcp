use std::fs;
use std::path::PathBuf;

fn skill() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".codex/skills/interpret-bhcp-contract")
}

#[test]
fn interpretation_skill_is_explicit_dependency_free_and_fail_closed() {
    let instructions = fs::read_to_string(skill().join("SKILL.md")).unwrap();
    let interface = fs::read_to_string(skill().join("agents/openai.yaml")).unwrap();

    assert!(instructions.starts_with("---\nname: interpret-bhcp-contract\n"));
    assert!(instructions.contains("description: Interpret and operationalize"));
    assert!(!instructions.contains("TODO"));
    assert!(instructions.contains("Never stream the complete AST or IR"));
    assert!(instructions.contains("Implementation state | Evidence state"));
    assert!(instructions.contains("mark its obligations unresolved"));

    assert!(interface.contains("display_name: \"Interpret BHCP Contract\""));
    assert!(interface.contains("Use $interpret-bhcp-contract"));
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
