use bhcp::formatting::format_source_bytes_with_profile_registry;
use bhcp::hash::{HashAlgorithm, artifact_hash_with};
use bhcp::pipeline::{compile_source_bytes_with_profile_registry, parse_profile_source};
use bhcp::policy::TypeMode;
use bhcp::profile::{
    FormattingRules, PresentationDocument, PresentationHeader, ProfileDocument, SyntaxDocument,
    SyntaxMapping, SyntaxMappingCategory,
};

const DEFINITIONS: &str = r##"
§policy example/policy.organization@0 {
    layer organization;
    rule a-mode: type-mode strengthen gradual nonwaivable;
}

§syntax example/syntax.base@0 {
    preamble "#!bhcp-profile";
    mappings [{ category: "keyword", canonical: "goal", surface: "outcome" }];
    formatting { indent_width: 2, line_width: 80, final_newline: true };
}

§syntax example/syntax.child@0 §extends example/syntax.base@0 {
    preamble "#!bhcp-profile";
    mappings [{ category: "sigil", canonical: "§", surface: "$" }];
    formatting { indent_width: 4, line_width: 100, final_newline: true };
}

§profile example/profile.base@0 {
    syntax example/syntax.base@0;
    type_mode gradual;
    policy_overlays [example/policy.organization@0];
}

§profile example/profile.child@0 §extends example/profile.base@0 {
    syntax example/syntax.child@0;
    type_mode infer-strict;
    policy_overlays [];
}
"##;

fn header() -> PresentationHeader {
    PresentationHeader {
        features: vec![],
        semantic_id: None,
        artifact_id: None,
        provenance: None,
        authorization: None,
    }
}

fn identify_syntax(mut document: SyntaxDocument) -> SyntaxDocument {
    document.header.artifact_id =
        Some(artifact_hash_with(&document.to_value(false), HashAlgorithm::default()).unwrap());
    document
}

fn identify_profile(mut document: ProfileDocument) -> ProfileDocument {
    document.header.artifact_id =
        Some(artifact_hash_with(&document.to_value(false), HashAlgorithm::default()).unwrap());
    document
}

#[test]
fn source_definitions_materialize_canonical_documents_and_a_validated_registry() {
    let lowered = parse_profile_source(DEFINITIONS, "profiles.bhcp").unwrap();
    let repeated = parse_profile_source(DEFINITIONS, "profiles.bhcp").unwrap();
    let expected_syntaxes = vec![
        identify_syntax(SyntaxDocument {
            header: header(),
            symbol: "example/syntax.base@0".to_owned(),
            extends: None,
            mappings: vec![SyntaxMapping {
                category: SyntaxMappingCategory::Keyword,
                canonical: "goal".to_owned(),
                surface: "outcome".to_owned(),
            }],
            formatting: FormattingRules {
                indent_width: 2,
                line_width: 80,
                final_newline: true,
            },
        }),
        identify_syntax(SyntaxDocument {
            header: header(),
            symbol: "example/syntax.child@0".to_owned(),
            extends: Some("example/syntax.base@0".to_owned()),
            mappings: vec![SyntaxMapping {
                category: SyntaxMappingCategory::Sigil,
                canonical: "§".to_owned(),
                surface: "$".to_owned(),
            }],
            formatting: FormattingRules {
                indent_width: 4,
                line_width: 100,
                final_newline: true,
            },
        }),
    ];
    let expected_profiles = vec![
        identify_profile(ProfileDocument {
            header: header(),
            symbol: "example/profile.base@0".to_owned(),
            extends: None,
            syntax: "example/syntax.base@0".to_owned(),
            policy_overlays: vec!["example/policy.organization@0".to_owned()],
            type_mode: TypeMode::Gradual,
        }),
        identify_profile(ProfileDocument {
            header: header(),
            symbol: "example/profile.child@0".to_owned(),
            extends: Some("example/profile.base@0".to_owned()),
            syntax: "example/syntax.child@0".to_owned(),
            policy_overlays: vec![],
            type_mode: TypeMode::InferStrict,
        }),
    ];

    assert_eq!(lowered.syntaxes, expected_syntaxes);
    assert_eq!(lowered.profiles, expected_profiles);
    assert_eq!(repeated.syntaxes, lowered.syntaxes);
    assert_eq!(repeated.profiles, lowered.profiles);
    for (actual, expected) in lowered.syntaxes.iter().zip(&expected_syntaxes) {
        assert_eq!(actual.to_value(true), expected.to_value(true));
        assert_eq!(
            PresentationDocument::Syntax(actual.clone())
                .to_cbor(true)
                .unwrap(),
            PresentationDocument::Syntax(expected.clone())
                .to_cbor(true)
                .unwrap()
        );
    }
    for (actual, expected) in lowered.profiles.iter().zip(&expected_profiles) {
        assert_eq!(actual.to_value(true), expected.to_value(true));
        assert_eq!(
            PresentationDocument::Profile(actual.clone())
                .to_cbor(true)
                .unwrap(),
            PresentationDocument::Profile(expected.clone())
                .to_cbor(true)
                .unwrap()
        );
    }

    let resolved = lowered
        .registry
        .resolve("example/profile.child@0", HashAlgorithm::default())
        .unwrap();
    assert_eq!(
        resolved.syntax_chain,
        ["example/syntax.base@0", "example/syntax.child@0"]
    );
    assert_eq!(resolved.type_mode, TypeMode::InferStrict);

    let custom = br#"#!bhcp-profile example/profile.child@0
$outcome example/G@0 {
    $output value: Bool;
}
"#;
    let compilation =
        compile_source_bytes_with_profile_registry(custom, "custom.bhcp", &lowered.registry)
            .unwrap();
    assert_eq!(compilation.ast.profile, "example/profile.child@0");
    assert_eq!(compilation.ir.type_mode, TypeMode::InferStrict);

    let formatted =
        format_source_bytes_with_profile_registry(custom, "custom.bhcp", &lowered.registry)
            .unwrap();
    assert!(formatted.starts_with("#!bhcp-profile example/profile.child@0\n$outcome"));
    assert!(formatted.contains("\n    $output value: Bool;\n"));
}

fn profile_source(syntaxes: &str, profiles: &str) -> String {
    format!("{syntaxes}\n{profiles}\n")
}

#[test]
fn invalid_source_registries_fail_atomically_before_custom_program_parsing() {
    let formatting = "formatting { indent_width: 2, line_width: 80, final_newline: true };";
    let cases = [
        (
            "syntax-inheritance-cycle",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax.a@0 §extends example/syntax.b@0 {{
    preamble "#!bhcp-profile"; mappings []; {formatting}
}}
§syntax example/syntax.b@0 §extends example/syntax.a@0 {{
    preamble "#!bhcp-profile"; mappings []; {formatting}
}}"##
                ),
                r#"
§profile example/profile@0 {
    syntax example/syntax.a@0; type_mode strict; policy_overlays [];
}"#,
            ),
            "BHCP9003",
        ),
        (
            "missing-syntax-parent",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax.a@0 §extends example/syntax.missing@0 {{
    preamble "#!bhcp-profile"; mappings []; {formatting}
}}"##
                ),
                r#"
§profile example/profile@0 {
    syntax example/syntax.a@0; type_mode strict; policy_overlays [];
}"#,
            ),
            "BHCP9003",
        ),
        (
            "ambiguous-surface",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax@0 {{
    preamble "#!bhcp-profile";
    mappings [
        {{ category: "keyword", canonical: "goal", surface: "same" }},
        {{ category: "keyword", canonical: "input", surface: "same" }}
    ];
    {formatting}
}}"##
                ),
                r#"
§profile example/profile@0 {
    syntax example/syntax@0; type_mode strict; policy_overlays [];
}"#,
            ),
            "BHCP9002",
        ),
        (
            "punctuation-prefix",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax@0 {{
    preamble "#!bhcp-profile";
    mappings [
        {{ category: "sigil", canonical: "§", surface: "$" }},
        {{ category: "open-delimiter", canonical: "{{", surface: "$$" }}
    ];
    {formatting}
}}"##
                ),
                r#"
§profile example/profile@0 {
    syntax example/syntax@0; type_mode strict; policy_overlays [];
}"#,
            ),
            "BHCP9002",
        ),
        (
            "recursive-alias",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax@0 {{
    preamble "#!bhcp-profile";
    mappings [
        {{ category: "alias", canonical: "example/A@0", surface: "example/B@0" }},
        {{ category: "alias", canonical: "example/B@0", surface: "example/A@0" }}
    ];
    {formatting}
}}"##
                ),
                r#"
§profile example/profile@0 {
    syntax example/syntax@0; type_mode strict; policy_overlays [];
}"#,
            ),
            "BHCP9002",
        ),
        (
            "core-override",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax@0 {{
    preamble "#!bhcp-profile";
    mappings [{{ category: "alias", canonical: "example/Reducer@0", surface: "bhcp/prelude.all-reducer@0" }}];
    {formatting}
}}"##
                ),
                r#"
§profile example/profile@0 {
    syntax example/syntax@0; type_mode strict; policy_overlays [];
}"#,
            ),
            "BHCP9002",
        ),
        (
            "weaker-type-mode",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax@0 {{
    preamble "#!bhcp-profile"; mappings []; {formatting}
}}"##
                ),
                r#"
§profile example/profile.base@0 {
    syntax example/syntax@0; type_mode strict; policy_overlays [];
}
§profile example/profile.child@0 §extends example/profile.base@0 {
    syntax example/syntax@0; type_mode gradual; policy_overlays [];
}"#,
            ),
            "BHCP9003",
        ),
        (
            "missing-policy-overlay",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax@0 {{
    preamble "#!bhcp-profile"; mappings []; {formatting}
}}"##
                ),
                r#"
§profile example/profile@0 {
    syntax example/syntax@0;
    type_mode strict;
    policy_overlays [example/policy.missing@0];
}"#,
            ),
            "BHCP9003",
        ),
        (
            "policy inheritance cycle includes",
            format!(
                r##"
§policy example/policy.a@0 §extends example/policy.b@0 {{
    layer organization;
}}
§policy example/policy.b@0 §extends example/policy.a@0 {{
    layer organization;
}}
§syntax example/syntax@0 {{
    preamble "#!bhcp-profile"; mappings []; {formatting}
}}
§profile example/profile@0 {{
    syntax example/syntax@0;
    type_mode strict;
    policy_overlays [example/policy.a@0];
}}
"##
            ),
            "BHCP8110",
        ),
        (
            "profile source may contain only policy, syntax, and profile definitions",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax@0 {{
    preamble "#!bhcp-profile"; mappings []; {formatting}
}}"##
                ),
                r#"
§profile example/profile@0 {
    syntax example/syntax@0; type_mode strict; policy_overlays [];
}
§goal example/G@0 {}"#,
            ),
            "BHCP9004",
        ),
        (
            "unknown profile field",
            profile_source(
                &format!(
                    r##"
§syntax example/syntax@0 {{
    preamble "#!bhcp-profile"; mappings []; {formatting}
}}"##
                ),
                r#"
§profile example/profile@0 {
    syntax example/syntax@0;
    type_mode strict;
    policy_overlays [];
    semantic_override example/unsafe@0;
}"#,
            ),
            "BHCP1004",
        ),
    ];

    for (expected, source, code) in cases {
        let diagnostic = parse_profile_source(&source, "invalid-profiles.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, code, "{expected}: {diagnostic:?}");
        assert!(
            diagnostic.message.contains(expected),
            "{expected}: {diagnostic:?}"
        );
    }
}
