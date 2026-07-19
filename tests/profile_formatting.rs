use bhcp::formatting::format_source_bytes_with_profile_registry;
use bhcp::model::AstNode;
use bhcp::pipeline::{compile_source, compile_source_bytes_with_profile_registry};
use bhcp::policy::TypeMode;
use bhcp::profile::{
    FormattingRules, PresentationHeader, ProfileDocument, ProfileRegistry, SyntaxDocument,
    SyntaxMapping, SyntaxMappingCategory,
};
use bhcp::value::Value;

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

fn registry() -> ProfileRegistry {
    let mut registry = ProfileRegistry::new();
    registry
        .register_syntax(SyntaxDocument {
            header: header(),
            symbol: "example/syntax.base@0".to_owned(),
            extends: None,
            mappings: vec![
                mapping(SyntaxMappingCategory::Keyword, "goal", "outcome"),
                mapping(SyntaxMappingCategory::Keyword, "input", "given"),
                mapping(SyntaxMappingCategory::Sigil, "§", "$"),
            ],
            formatting: FormattingRules {
                indent_width: 4,
                line_width: 100,
                final_newline: true,
            },
        })
        .unwrap();
    registry
        .register_syntax(SyntaxDocument {
            header: header(),
            symbol: "example/syntax.child@0".to_owned(),
            extends: Some("example/syntax.base@0".to_owned()),
            mappings: vec![
                mapping(SyntaxMappingCategory::Keyword, "output", "résultat"),
                mapping(SyntaxMappingCategory::OpenDelimiter, "{", "^"),
                mapping(SyntaxMappingCategory::CloseDelimiter, "}", "~"),
                mapping(SyntaxMappingCategory::Terminator, ";", "?"),
            ],
            formatting: FormattingRules {
                indent_width: 2,
                line_width: 48,
                final_newline: false,
            },
        })
        .unwrap();
    registry
        .register_profile(ProfileDocument {
            header: header(),
            symbol: "example/profile.words@0".to_owned(),
            extends: None,
            syntax: "example/syntax.child@0".to_owned(),
            policy_overlays: vec![],
            type_mode: TypeMode::Dynamic,
        })
        .unwrap();
    registry
}

fn ast_shape(node: &AstNode) -> Value {
    Value::map([
        ("kind", Value::Text(node.kind.clone())),
        ("token", node.token.clone().map_or(Value::Null, Value::Text)),
        ("attributes", Value::owned_map(node.attributes.clone())),
        (
            "children",
            Value::Array(node.children.iter().map(ast_shape).collect()),
        ),
    ])
}

const COMPACT_CANONICAL: &str = "§goal example/G@0{§input value:Text;§output result:Text;}";

#[test]
fn canonical_formatting_is_deterministic_idempotent_and_semantic() {
    let registry = ProfileRegistry::new();
    let formatted = format_source_bytes_with_profile_registry(
        COMPACT_CANONICAL.as_bytes(),
        "compact.bhcp",
        &registry,
    )
    .unwrap();
    assert_eq!(
        formatted,
        "§goal example/G@0 {\n    §input value: Text;\n    §output result: Text;\n}\n"
    );
    assert_eq!(
        format_source_bytes_with_profile_registry(
            formatted.as_bytes(),
            "formatted.bhcp",
            &registry,
        )
        .unwrap(),
        formatted
    );

    let before = compile_source(COMPACT_CANONICAL, "compact.bhcp").unwrap();
    let after = compile_source(&formatted, "formatted.bhcp").unwrap();
    assert_eq!(ast_shape(&before.ast.root), ast_shape(&after.ast.root));
    assert_eq!(before.semantic_hash, after.semantic_hash);
    assert_eq!(before.ir, after.ir);
}

const CUSTOM_UNFORMATTED: &str = r#"#!bhcp-profile example/profile.words@0
$outcome example/G@0^/* retained block */$given value:Text?// retained line
$résultat result:Text?~
"#;

#[test]
fn inherited_custom_formatting_preserves_comments_unicode_and_round_trips() {
    let registry = registry();
    let formatted = format_source_bytes_with_profile_registry(
        CUSTOM_UNFORMATTED.as_bytes(),
        "custom-unformatted.bhcp",
        &registry,
    )
    .unwrap();
    assert_eq!(
        formatted,
        "#!bhcp-profile example/profile.words@0\n\
$outcome example/G@0 ^\n\
  /* retained block */\n\
  $given value: Text? // retained line\n\
  $résultat result: Text?\n\
~"
    );
    assert!(!formatted.ends_with('\n'));
    assert_eq!(
        format_source_bytes_with_profile_registry(
            formatted.as_bytes(),
            "custom-formatted.bhcp",
            &registry,
        )
        .unwrap(),
        formatted
    );

    let before = compile_source_bytes_with_profile_registry(
        CUSTOM_UNFORMATTED.as_bytes(),
        "custom-unformatted.bhcp",
        &registry,
    )
    .unwrap();
    let after = compile_source_bytes_with_profile_registry(
        formatted.as_bytes(),
        "custom-formatted.bhcp",
        &registry,
    )
    .unwrap();
    assert_eq!(ast_shape(&before.ast.root), ast_shape(&after.ast.root));
    assert_eq!(before.semantic_hash, after.semantic_hash);
    assert_eq!(before.ir, after.ir);
}

#[test]
fn invalid_or_unregistered_formatting_fails_before_output() {
    let mut registry = ProfileRegistry::new();
    let error = registry
        .register_syntax(SyntaxDocument {
            header: header(),
            symbol: "example/syntax.invalid@0".to_owned(),
            extends: None,
            mappings: vec![],
            formatting: FormattingRules {
                indent_width: 17,
                line_width: 0,
                final_newline: true,
            },
        })
        .unwrap_err();
    assert_eq!(error.code, "BHCP9001");

    let selected = "#!bhcp-profile example/profile.missing@0\n\
§goal example/G@0 { §input value: Text; §output result: Text; }";
    let error =
        format_source_bytes_with_profile_registry(selected.as_bytes(), "missing.bhcp", &registry)
            .unwrap_err();
    assert_eq!(error.code, "BHCP9003");
    assert_eq!(error.message, "missing-profile");
}
