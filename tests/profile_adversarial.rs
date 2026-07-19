use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use bhcp::pipeline::compile_source_bytes_with_profile_registry;
use bhcp::policy::TypeMode;
use bhcp::profile::{
    FormattingRules, PresentationDocument, PresentationHeader, ProfileDocument, ProfileRegistry,
    SyntaxDocument, SyntaxMapping, SyntaxMappingCategory,
};
use bhcp::schema::parse_diagnostic;

const PROFILE: &str = "example/profile.adversarial@0";
const SYNTAX: &str = "example/syntax.adversarial@0";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn header() -> PresentationHeader {
    PresentationHeader {
        features: vec![],
        semantic_id: None,
        artifact_id: None,
        provenance: None,
        authorization: None,
    }
}

fn mapping(category: SyntaxMappingCategory, canonical: &str, surface: &str) -> SyntaxMapping {
    SyntaxMapping {
        category,
        canonical: canonical.to_owned(),
        surface: surface.to_owned(),
    }
}

fn registry(mappings: Vec<SyntaxMapping>) -> ProfileRegistry {
    let mut registry = ProfileRegistry::new();
    registry
        .register_syntax(SyntaxDocument {
            header: header(),
            symbol: SYNTAX.to_owned(),
            extends: None,
            mappings,
            formatting: FormattingRules {
                indent_width: 2,
                line_width: 80,
                final_newline: true,
            },
        })
        .unwrap();
    registry
        .register_profile(ProfileDocument {
            header: header(),
            symbol: PROFILE.to_owned(),
            extends: None,
            syntax: SYNTAX.to_owned(),
            policy_overlays: vec![],
            type_mode: TypeMode::Strict,
        })
        .unwrap();
    registry
}

#[test]
fn executable_profile_attacks_name_profile_mapping_rule_and_stable_span() {
    let cases = [
        (
            "ambiguous-surface",
            vec![
                mapping(SyntaxMappingCategory::Alias, "example/A@0", "same"),
                mapping(SyntaxMappingCategory::Alias, "example/B@0", "same"),
            ],
            "mapping=alias:example/B@0=>same",
            2,
        ),
        (
            "recursive-alias",
            vec![
                mapping(SyntaxMappingCategory::Alias, "example/A@0", "example/B@0"),
                mapping(SyntaxMappingCategory::Alias, "example/B@0", "example/A@0"),
            ],
            "mapping=alias:example/A@0=>example/B@0",
            1,
        ),
        (
            "token-capture",
            vec![mapping(
                SyntaxMappingCategory::Alias,
                "example/Condition@0",
                "if",
            )],
            "mapping=alias:example/Condition@0=>if",
            1,
        ),
        (
            "core-override",
            vec![mapping(
                SyntaxMappingCategory::Alias,
                "example/Reducer@0",
                "bhcp/prelude.all-reducer@0",
            )],
            "mapping=alias:example/Reducer@0=>bhcp/prelude.all-reducer@0",
            1,
        ),
    ];

    for (rule, mappings, expected_mapping, line) in cases {
        let registry = registry(mappings);
        let diagnostic = registry.resolve(PROFILE, Default::default()).unwrap_err();
        assert_eq!(diagnostic.code, "BHCP9002", "{rule}");
        assert_eq!(diagnostic.source, SYNTAX, "{rule}");
        assert_eq!((diagnostic.line, diagnostic.column), (line, 1), "{rule}");
        assert!(
            diagnostic.message.contains(&format!("profile={PROFILE}")),
            "{rule}: {diagnostic:?}"
        );
        assert!(
            diagnostic.message.contains(&format!("syntax={SYNTAX}")),
            "{rule}: {diagnostic:?}"
        );
        assert!(
            diagnostic.message.contains(expected_mapping),
            "{rule}: {diagnostic:?}"
        );
        assert!(
            diagnostic.message.contains(&format!("rule={rule}")),
            "{rule}: {diagnostic:?}"
        );

        let source = format!("#!bhcp-profile {PROFILE}\nnot even a program");
        let compile_diagnostic = compile_source_bytes_with_profile_registry(
            source.as_bytes(),
            "must-not-parse.bhcp",
            &registry,
        )
        .unwrap_err();
        assert_eq!(compile_diagnostic, diagnostic, "{rule}");
    }
}

#[test]
fn canonical_keyword_capture_is_rejected_as_an_ambiguous_mapping() {
    let diagnostic = registry(vec![mapping(
        SyntaxMappingCategory::Keyword,
        "goal",
        "input",
    )])
    .resolve(PROFILE, Default::default())
    .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP9002");
    assert_eq!(diagnostic.source, SYNTAX);
    assert_eq!((diagnostic.line, diagnostic.column), (1, 1));
    assert!(diagnostic.message.contains(&format!("profile={PROFILE}")));
    assert!(diagnostic.message.contains("mapping=keyword:goal=>input"));
    assert!(diagnostic.message.contains("rule=ambiguous-surface"));
}

#[test]
fn mapped_away_core_spelling_reports_selected_profile_and_source_span_without_artifact() {
    let registry = registry(vec![
        mapping(SyntaxMappingCategory::Keyword, "goal", "outcome"),
        mapping(SyntaxMappingCategory::Sigil, "§", "$"),
    ]);
    let source = "#!bhcp-profile example/profile.adversarial@0\n§goal example/G@0 {}\n";
    let diagnostic = compile_source_bytes_with_profile_registry(
        source.as_bytes(),
        "meaning-change.bhcp",
        &registry,
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP0005");
    assert_eq!(diagnostic.source, "meaning-change.bhcp");
    assert_eq!((diagnostic.line, diagnostic.column), (2, 1));
    assert!(diagnostic.message.contains(&format!("profile={PROFILE}")));
    assert!(diagnostic.message.contains(&format!("syntax={SYNTAX}")));
    assert!(diagnostic.message.contains("mapping=sigil:§=>$"));
    assert!(diagnostic.message.contains("rule=mapped-away-sigil"));
}

#[test]
fn parser_macro_and_semantic_override_artifacts_fail_the_closed_model() {
    let directory = root().join("tests/fixtures/profile_adversarial");
    let manifest = fs::read_to_string(directory.join("manifest.txt")).unwrap();
    for line in manifest.lines().filter(|line| !line.is_empty()) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        assert_eq!(fields.len(), 3, "{line}");
        let value =
            parse_diagnostic(&fs::read_to_string(directory.join(fields[0])).unwrap()).unwrap();
        let diagnostic = PresentationDocument::from_value(&value).unwrap_err();
        assert_eq!(diagnostic.code, "BHCP9001", "{}", fields[0]);
        assert!(
            diagnostic.message.contains(fields[2]),
            "{}: {diagnostic:?}",
            fields[0]
        );
    }
}

#[test]
fn cli_failure_is_atomic_for_an_invalid_effective_profile() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "bhcp-profile-adversarial-{}-{suffix}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let source_path = directory.join("attack.bhcp");
    let syntax_path = directory.join("syntax.cbor");
    let profile_path = directory.join("profile.cbor");

    let syntax = SyntaxDocument {
        header: header(),
        symbol: SYNTAX.to_owned(),
        extends: None,
        mappings: vec![
            mapping(SyntaxMappingCategory::Alias, "example/A@0", "same"),
            mapping(SyntaxMappingCategory::Alias, "example/B@0", "same"),
        ],
        formatting: FormattingRules {
            indent_width: 2,
            line_width: 80,
            final_newline: true,
        },
    };
    let profile = ProfileDocument {
        header: header(),
        symbol: PROFILE.to_owned(),
        extends: None,
        syntax: SYNTAX.to_owned(),
        policy_overlays: vec![],
        type_mode: TypeMode::Strict,
    };
    fs::write(
        &syntax_path,
        PresentationDocument::Syntax(syntax).to_cbor(false).unwrap(),
    )
    .unwrap();
    fs::write(
        &profile_path,
        PresentationDocument::Profile(profile)
            .to_cbor(false)
            .unwrap(),
    )
    .unwrap();
    fs::write(
        &source_path,
        format!("#!bhcp-profile {PROFILE}\nnot even a program"),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bhcp"))
        .args([
            "format",
            source_path.to_str().unwrap(),
            syntax_path.to_str().unwrap(),
            profile_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("BHCP9002"), "{stderr}");
    assert!(stderr.contains(&format!("profile={PROFILE}")), "{stderr}");
    assert!(
        stderr.contains("mapping=alias:example/B@0=>same"),
        "{stderr}"
    );
    assert!(stderr.contains("rule=ambiguous-surface"), "{stderr}");

    fs::remove_dir_all(directory).unwrap();
}
