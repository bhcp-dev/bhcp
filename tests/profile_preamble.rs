use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use bhcp::parser::{CANONICAL_PROFILE, scan_profile_preamble};
use bhcp::pipeline::{compile_source, parse_policy_source, parse_source};

static NEXT_TEMP: AtomicUsize = AtomicUsize::new(1);

const PROGRAM: &str = "§goal example/G@0 {}\n";
const EXPLICIT_CANONICAL_FIXTURE: &str =
    include_str!("../conformance/v0/fixtures/canonical-profile-preamble.bhcp");
const POLICY: &str = r#"§policy example/policy@0 {
  layer organization;
  rule a-mode: type-mode strengthen infer-strict nonwaivable;
}
"#;

fn explicit(profile: &str, body: &str) -> String {
    format!("#!bhcp-profile {profile}\n{body}")
}

fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "bhcp-profile-preamble-{}-{}-{name}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn omission_explicit_canonical_and_bom_select_exactly_one_profile() {
    let omitted = scan_profile_preamble(PROGRAM.as_bytes(), "omitted.bhcp").unwrap();
    assert_eq!(omitted.profile, CANONICAL_PROFILE);
    assert_eq!(omitted.body_start, 0);
    assert!(!omitted.had_preamble);
    assert_eq!(omitted.canonical_source, PROGRAM);

    let source = explicit(CANONICAL_PROFILE, PROGRAM);
    let selected = scan_profile_preamble(source.as_bytes(), "explicit.bhcp").unwrap();
    assert_eq!(selected.profile, CANONICAL_PROFILE);
    assert_eq!(selected.body_start, source.len() - PROGRAM.len());
    assert!(selected.had_preamble);
    assert_eq!(selected.canonical_source.len(), source.len());
    assert!(selected.canonical_source.ends_with(PROGRAM));
    assert_eq!(
        selected.canonical_source.as_bytes()[selected.body_start - 1],
        b'\n'
    );
    assert!(
        selected.canonical_source.as_bytes()[..selected.body_start - 1]
            .iter()
            .all(|byte| *byte == b' ')
    );

    let spaced = format!("#!bhcp-profile   {CANONICAL_PROFILE}\n{PROGRAM}");
    assert_eq!(
        scan_profile_preamble(spaced.as_bytes(), "spaced.bhcp")
            .unwrap()
            .profile,
        CANONICAL_PROFILE
    );

    let bommed = format!("\u{feff}{source}");
    let selected = scan_profile_preamble(bommed.as_bytes(), "bom.bhcp").unwrap();
    assert_eq!(selected.profile, CANONICAL_PROFILE);
    assert_eq!(selected.body_start, bommed.len() - PROGRAM.len());
    assert_eq!(selected.canonical_source.len(), bommed.len());

    let bom_only = format!("\u{feff}{PROGRAM}");
    let selected = scan_profile_preamble(bom_only.as_bytes(), "bom-only.bhcp").unwrap();
    assert_eq!(selected.profile, CANONICAL_PROFILE);
    assert_eq!(selected.body_start, 3);
    assert!(!selected.had_preamble);
}

#[test]
fn explicit_canonical_preamble_preserves_offsets_and_semantic_identity() {
    let source = format!("\u{feff}{}", explicit(CANONICAL_PROFILE, PROGRAM));
    let body_start = source.len() - PROGRAM.len();
    let ast = parse_source(&source, "profiled.bhcp").unwrap();
    assert_eq!(ast.profile, CANONICAL_PROFILE);
    assert_eq!(ast.source.size, source.len());
    assert_eq!(ast.root.span.start.byte, body_start);
    assert_eq!(ast.root.span.start.line, 2);
    assert_eq!(ast.root.span.start.column, 1);

    let plain = compile_source(PROGRAM, "plain.bhcp").unwrap();
    let profiled = compile_source(&source, "profiled.bhcp").unwrap();
    assert_eq!(plain.ir.semantic_id, profiled.ir.semantic_id);
    assert_ne!(plain.ast.artifact_id, profiled.ast.artifact_id);
    assert_eq!(profiled.ast.profile, CANONICAL_PROFILE);

    let fixture = parse_source(
        EXPLICIT_CANONICAL_FIXTURE,
        "canonical-profile-preamble.bhcp",
    )
    .unwrap();
    assert_eq!(fixture.profile, CANONICAL_PROFILE);
    assert_eq!(fixture.root.span.start.line, 2);

    let policy = explicit(CANONICAL_PROFILE, POLICY);
    assert_eq!(
        parse_policy_source(&policy, "policy.bhcp")
            .unwrap()
            .ast
            .profile,
        CANONICAL_PROFILE
    );
}

#[test]
fn custom_profile_is_selected_without_aliasing_but_fails_closed_before_lexing() {
    let source = explicit("example/profile.words@0", "outcome example/G@0 <>\n");
    let selected = scan_profile_preamble(source.as_bytes(), "custom.bhcp").unwrap();
    assert_eq!(selected.profile, "example/profile.words@0");

    let diagnostic = parse_source(&source, "custom.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP0004");
    assert_eq!(diagnostic.line, 1);
    assert_eq!(diagnostic.column, 1);
    assert!(diagnostic.message.contains("example/profile.words@0"));
    assert!(
        diagnostic
            .message
            .contains("not registered for normalization")
    );
}

#[test]
fn malformed_truncated_aliased_and_non_ascii_preambles_fail_stably() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("missing-profile", b"#!bhcp-profile \n".to_vec()),
        ("missing-space", b"#!bhcp-profileexample/p@0\n".to_vec()),
        ("tab", b"#!bhcp-profile\texample/p@0\n".to_vec()),
        ("crlf", b"#!bhcp-profile example/p@0\r\n".to_vec()),
        ("truncated", b"#!bhcp-profile example/p@0".to_vec()),
        ("alias", b"#!bhcp-profile canonical\n".to_vec()),
        ("unversioned", b"#!bhcp-profile example/p\n".to_vec()),
        ("double-colon", b"#!bhcp-profile example::p@0\n".to_vec()),
        ("trailing-space", b"#!bhcp-profile example/p@0 \n".to_vec()),
        (
            "extra-token",
            b"#!bhcp-profile example/p@0 extra\n".to_vec(),
        ),
        ("aliased-marker", b"#!profile example/p@0\n".to_vec()),
        (
            "non-ascii-space",
            "#!bhcp-profile\u{00a0}example/p@0\n".as_bytes().to_vec(),
        ),
        ("invalid-utf8", vec![0xef, 0xbb, b'X']),
    ];
    for (name, source) in cases {
        let diagnostic = scan_profile_preamble(&source, name).unwrap_err();
        assert_eq!(diagnostic.code, "BHCP0003", "{name}");
        assert_eq!(diagnostic.line, 1, "{name}");
        assert_eq!(diagnostic.column, 1, "{name}");
    }
}

#[test]
fn duplicate_misplaced_and_repeated_bom_directives_name_their_source_line() {
    for (name, source, line) in [
        (
            "duplicate",
            format!(
                "#!bhcp-profile {CANONICAL_PROFILE}\n#!bhcp-profile {CANONICAL_PROFILE}\n{PROGRAM}"
            ),
            2,
        ),
        (
            "misplaced",
            format!("\n#!bhcp-profile {CANONICAL_PROFILE}\n{PROGRAM}"),
            2,
        ),
        (
            "indented",
            format!("  #!bhcp-profile {CANONICAL_PROFILE}\n{PROGRAM}"),
            1,
        ),
        (
            "after-source",
            format!("{PROGRAM}#!bhcp-profile {CANONICAL_PROFILE}\n"),
            2,
        ),
        ("duplicate-bom", format!("\u{feff}\u{feff}{PROGRAM}"), 1),
    ] {
        let diagnostic = scan_profile_preamble(source.as_bytes(), name).unwrap_err();
        assert_eq!(diagnostic.code, "BHCP0003", "{name}");
        assert_eq!(diagnostic.line, line, "{name}");
        assert!(matches!(diagnostic.column, 1 | 3), "{name}");
    }
}

#[test]
fn cli_reports_byte_level_preamble_errors_without_artifact_output() {
    let path = temp_file("invalid.bhcp");
    fs::write(&path, [0xff, 0xfe, 0xfd]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_bhcp"))
        .arg("parse")
        .arg(&path)
        .output()
        .unwrap();
    fs::remove_file(&path).unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("BHCP0003")
    );
}
