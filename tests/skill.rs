use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

fn skill() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".codex/skills/interpret-bhcp-contract")
}

fn entries(path: &Path) -> Vec<OsString> {
    let mut entries: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    entries.sort();
    entries
}

#[test]
fn interpretation_skill_stays_compact_and_dependency_free() {
    let instructions = fs::read_to_string(skill().join("SKILL.md")).unwrap();

    assert!(
        instructions.len() <= 3_600,
        "skill is too large for its routine context budget: {} bytes",
        instructions.len()
    );

    assert_eq!(
        entries(&skill()),
        [OsString::from("SKILL.md"), OsString::from("agents")],
        "skill gained an unexpected bundled dependency"
    );
    assert_eq!(
        entries(&skill().join("agents")),
        [OsString::from("openai.yaml")]
    );
}
