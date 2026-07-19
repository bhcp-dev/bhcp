use bhcp::parser::{normalize_syntax_tokens, scan_profile_preamble};
use bhcp::pipeline::{
    ProfileSyntaxRegistry, compile_source, compile_source_bytes_with_profiles,
    parse_source_bytes_with_profiles,
};
use bhcp::profile::{
    FormattingRules, PresentationHeader, SyntaxDocument, SyntaxMapping, SyntaxMappingCategory,
};

fn syntax(mappings: Vec<SyntaxMapping>) -> SyntaxDocument {
    SyntaxDocument {
        header: PresentationHeader {
            features: vec![],
            semantic_id: None,
            artifact_id: None,
            provenance: None,
            authorization: None,
        },
        symbol: "example/syntax.words@0".to_owned(),
        extends: None,
        mappings,
        formatting: FormattingRules {
            indent_width: 4,
            line_width: 100,
            final_newline: true,
        },
    }
}

fn unresolved_child() -> SyntaxDocument {
    let mut child = syntax(vec![]);
    child.extends = Some("example/syntax.parent@0".to_owned());
    child
}

fn mapping(category: SyntaxMappingCategory, canonical: &str, surface: &str) -> SyntaxMapping {
    SyntaxMapping {
        category,
        canonical: canonical.to_owned(),
        surface: surface.to_owned(),
    }
}

fn words_syntax() -> SyntaxDocument {
    syntax(vec![
        mapping(SyntaxMappingCategory::Keyword, "goal", "outcome"),
        mapping(SyntaxMappingCategory::Keyword, "input", "accepts"),
        mapping(SyntaxMappingCategory::Keyword, "output", "yields"),
        mapping(SyntaxMappingCategory::Keyword, "requires", "needs"),
        mapping(SyntaxMappingCategory::Sigil, "§", "$"),
        mapping(SyntaxMappingCategory::OpenDelimiter, "{", "^"),
        mapping(SyntaxMappingCategory::CloseDelimiter, "}", "~"),
        mapping(SyntaxMappingCategory::Terminator, ";", "?"),
        mapping(
            SyntaxMappingCategory::Alias,
            "bhcp-verifier/expression@0",
            "proof",
        ),
    ])
}

const CANONICAL: &str = include_str!("../conformance/v0/fixtures/profile-lowering-canonical.bhcp");
const CUSTOM: &str = include_str!("../conformance/v0/fixtures/profile-lowering-words.bhcp");

#[test]
fn every_mapping_category_lowers_to_the_canonical_token_stream_once() {
    let selected = scan_profile_preamble(CUSTOM.as_bytes(), "custom.bhcp").unwrap();
    let custom =
        normalize_syntax_tokens(&selected.canonical_source, "custom.bhcp", &words_syntax())
            .unwrap();
    let canonical = normalize_syntax_tokens(CANONICAL, "canonical.bhcp", &syntax(vec![])).unwrap();

    let custom_text: Vec<_> = custom.iter().map(|token| token.text.as_str()).collect();
    let canonical_text: Vec<_> = canonical.iter().map(|token| token.text.as_str()).collect();
    assert_eq!(custom_text, canonical_text);
    assert_eq!(
        custom_text.iter().filter(|text| **text == "§input").count(),
        1
    );
    assert!(custom_text.contains(&"bhcp-verifier"));

    let input = custom
        .iter()
        .find(|token| token.text == "§input")
        .expect("mapped input keyword");
    assert_eq!((input.start.line, input.start.column), (4, 5));
    assert_eq!((input.end.line, input.end.column), (4, 13));
}

#[test]
fn selected_profile_parses_through_a_closed_registry_and_retains_original_spans() {
    let mut profiles = ProfileSyntaxRegistry::new();
    profiles
        .register("example/words@0", words_syntax())
        .unwrap();

    let ast = parse_source_bytes_with_profiles(CUSTOM.as_bytes(), "custom.bhcp", &profiles)
        .expect("registered custom profile parses through the canonical parser");
    assert_eq!(ast.profile, "example/words@0");
    assert_eq!(ast.root.children.len(), 1);
    assert_eq!(ast.root.children[0].span.start.line, 2);
    assert_eq!(ast.root.children[0].span.start.column, 1);
}

#[test]
fn custom_and_canonical_source_compile_to_the_same_semantic_identity() {
    let canonical = compile_source(CANONICAL, "canonical.bhcp").unwrap();
    let mut profiles = ProfileSyntaxRegistry::new();
    profiles
        .register("example/words@0", words_syntax())
        .unwrap();
    let custom =
        compile_source_bytes_with_profiles(CUSTOM.as_bytes(), "custom.bhcp", &profiles).unwrap();

    assert_eq!(custom.semantic_hash, canonical.semantic_hash);
    assert_ne!(custom.ast_hash, canonical.ast_hash);
}

#[test]
fn nfc_unicode_identifier_and_punctuation_surfaces_lower_before_ascii_lexing() {
    let unicode = syntax(vec![
        mapping(SyntaxMappingCategory::Keyword, "goal", "résultat"),
        mapping(SyntaxMappingCategory::OpenDelimiter, "{", "⟦"),
        mapping(SyntaxMappingCategory::CloseDelimiter, "}", "⟧"),
    ]);
    let source = "#!bhcp-profile example/unicode@0\n§résultat example/G@0 ⟦\n⟧\n";
    let mut profiles = ProfileSyntaxRegistry::new();
    profiles.register("example/unicode@0", unicode).unwrap();

    let ast = parse_source_bytes_with_profiles(source.as_bytes(), "unicode.bhcp", &profiles)
        .expect("NFC surface is normalized before the canonical ASCII lexer");
    assert_eq!(ast.profile, "example/unicode@0");
    assert_eq!(
        (
            ast.root.children[0].span.start.line,
            ast.root.children[0].span.start.column
        ),
        (2, 1)
    );
}

#[test]
fn parser_diagnostics_point_to_the_original_custom_spelling() {
    let source = b"#!bhcp-profile example/words@0\n$outcome ^\n";
    let mut profiles = ProfileSyntaxRegistry::new();
    profiles
        .register("example/words@0", words_syntax())
        .unwrap();

    let diagnostic =
        parse_source_bytes_with_profiles(source, "broken.bhcp", &profiles).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP1001");
    assert_eq!((diagnostic.line, diagnostic.column), (2, 10));
}

#[test]
fn unmapped_unicode_diagnostics_keep_the_original_line_and_column() {
    let source = "#!bhcp-profile example/words@0\n$outcome example/G@0 ^\n    ☃\n~\n";
    let mut profiles = ProfileSyntaxRegistry::new();
    profiles
        .register("example/words@0", words_syntax())
        .unwrap();

    let diagnostic =
        parse_source_bytes_with_profiles(source.as_bytes(), "unicode-error.bhcp", &profiles)
            .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP0002");
    assert_eq!((diagnostic.line, diagnostic.column), (3, 5));
}

#[test]
fn mapped_away_canonical_spellings_do_not_bypass_the_selected_profile() {
    let source = "#!bhcp-profile example/words@0\n§goal example/G@0 {}\n";
    let mut profiles = ProfileSyntaxRegistry::new();
    profiles
        .register("example/words@0", words_syntax())
        .unwrap();

    let diagnostic =
        parse_source_bytes_with_profiles(source.as_bytes(), "bypass.bhcp", &profiles).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP0005");
    assert!(diagnostic.message.contains("mapped-away-sigil"));
    assert_eq!((diagnostic.line, diagnostic.column), (2, 1));
}

#[test]
fn invalid_effective_maps_fail_before_accepting_any_program_token() {
    let cases = [
        ("unresolved-inheritance", unresolved_child()),
        (
            "category-mismatch",
            syntax(vec![mapping(SyntaxMappingCategory::Keyword, "{", "begin")]),
        ),
        (
            "ambiguous-surface",
            syntax(vec![mapping(
                SyntaxMappingCategory::Keyword,
                "goal",
                "input",
            )]),
        ),
        (
            "punctuation-prefix",
            syntax(vec![
                mapping(SyntaxMappingCategory::OpenDelimiter, "{", "<|"),
                mapping(SyntaxMappingCategory::CloseDelimiter, "}", "<||"),
            ]),
        ),
        (
            "recursive-alias",
            syntax(vec![
                mapping(SyntaxMappingCategory::Alias, "example/B@0", "example/A@0"),
                mapping(SyntaxMappingCategory::Alias, "example/C@0", "example/B@0"),
            ]),
        ),
        (
            "core-override",
            syntax(vec![mapping(
                SyntaxMappingCategory::Alias,
                "example/reducer@0",
                "bhcp/prelude.all-reducer@0",
            )]),
        ),
        (
            "token-capture",
            syntax(vec![mapping(
                SyntaxMappingCategory::OpenDelimiter,
                "{",
                "+",
            )]),
        ),
        (
            "invalid-surface",
            syntax(vec![mapping(
                SyntaxMappingCategory::Keyword,
                "goal",
                "re\u{301}sultat",
            )]),
        ),
    ];

    for (expected, syntax) in cases {
        let diagnostic = normalize_syntax_tokens("not even a program", "invalid.bhcp", &syntax)
            .expect_err(expected);
        assert_eq!(diagnostic.code, "BHCP9002", "{expected}");
        assert!(
            diagnostic.message.contains(expected),
            "{expected}: {diagnostic:?}"
        );
        assert_eq!((diagnostic.line, diagnostic.column), (1, 1));
    }
}

#[test]
fn unregistered_profiles_still_fail_closed_before_custom_lexing() {
    let diagnostic = parse_source_bytes_with_profiles(
        CUSTOM.as_bytes(),
        "custom.bhcp",
        &ProfileSyntaxRegistry::new(),
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP0004");
    assert!(diagnostic.message.contains("example/words@0"));
}
