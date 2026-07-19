use std::fs;
use std::path::PathBuf;

use bhcp::cbor::encode_deterministic;
use bhcp::profile::{PresentationDocument, SyntaxMappingCategory};
use bhcp::schema::{parse_diagnostic, validate_root};
use bhcp::value::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn parse(source: &str) -> Value {
    parse_diagnostic(source).unwrap()
}

#[test]
fn every_mapping_category_and_profile_field_round_trip_deterministically() {
    let syntax = parse(
        r##"{
          "version": "bhcp/v0",
          "features": ["example/feature.a@0", "example/feature.b@0"],
          "kind": "syntax",
          "symbol": "example/syntax.words@0",
          "extends": "bhcp/syntax.canonical@0",
          "preamble": "#!bhcp-profile",
          "mappings": [
            {"category": "keyword", "canonical": "goal", "surface": "outcome"},
            {"category": "sigil", "canonical": "§", "surface": "$"},
            {"category": "open-delimiter", "canonical": "{", "surface": "<"},
            {"category": "close-delimiter", "canonical": "}", "surface": ">"},
            {"category": "terminator", "canonical": ";", "surface": "!"},
            {"category": "alias", "canonical": "example/Check@0", "surface": "check"}
          ],
          "formatting": {
            "indent_width": 2,
            "line_width": 100,
            "final_newline": true
          }
        }"##,
    );
    let document = PresentationDocument::from_value(&syntax).unwrap();
    let PresentationDocument::Syntax(syntax_document) = &document else {
        panic!("expected syntax document");
    };
    assert_eq!(
        syntax_document
            .mappings
            .iter()
            .map(|mapping| mapping.category)
            .collect::<Vec<_>>(),
        vec![
            SyntaxMappingCategory::Keyword,
            SyntaxMappingCategory::Sigil,
            SyntaxMappingCategory::OpenDelimiter,
            SyntaxMappingCategory::CloseDelimiter,
            SyntaxMappingCategory::Terminator,
            SyntaxMappingCategory::Alias,
        ]
    );
    assert_eq!(document.to_value(true), syntax);
    let bytes = document.to_cbor(true).unwrap();
    assert_eq!(bytes, encode_deterministic(&syntax).unwrap());
    assert_eq!(PresentationDocument::from_cbor(&bytes).unwrap(), document);
    assert_eq!(document.to_cbor(true).unwrap(), bytes);

    for mode in ["dynamic", "gradual", "infer-strict", "strict"] {
        let profile = parse(&format!(
            r#"{{
              "version": "bhcp/v0",
              "features": ["example/feature.profile@0"],
              "kind": "profile",
              "symbol": "example/profile.team@0",
              "extends": "bhcp/profile.canonical@0",
              "syntax": "example/syntax.words@0",
              "policy_overlays": [
                "example/policy.organization@0",
                "example/policy.team@0"
              ],
              "type_mode": "{mode}"
            }}"#
        ));
        let document = PresentationDocument::from_value(&profile).unwrap();
        let PresentationDocument::Profile(profile_document) = &document else {
            panic!("expected profile document");
        };
        assert_eq!(profile_document.type_mode.as_str(), mode);
        assert_eq!(profile_document.policy_overlays.len(), 2);
        assert_eq!(document.to_value(true), profile);
        let bytes = document.to_cbor(true).unwrap();
        assert_eq!(PresentationDocument::from_cbor(&bytes).unwrap(), document);
    }
}

#[test]
fn malformed_profile_fixtures_have_stable_model_diagnostics() {
    let directory = root().join("tests/fixtures/profile_models/invalid");
    let manifest = fs::read_to_string(directory.join("manifest.txt")).unwrap();
    for line in manifest.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.splitn(3, ' ').collect();
        assert_eq!(fields.len(), 3, "{line}");
        let value = parse(&fs::read_to_string(directory.join(fields[0])).unwrap());
        let diagnostic = PresentationDocument::from_value(&value).unwrap_err();
        assert_eq!(diagnostic.code, "BHCP9001", "{}", fields[0]);
        assert!(
            diagnostic.message.contains(fields[2]),
            "{}: {}",
            fields[0],
            diagnostic.message
        );
    }

    let duplicate_field = Value::Map(vec![
        ("version".to_owned(), Value::Text("bhcp/v0".to_owned())),
        ("features".to_owned(), Value::Array(vec![])),
        ("kind".to_owned(), Value::Text("profile".to_owned())),
        (
            "symbol".to_owned(),
            Value::Text("example/profile@0".to_owned()),
        ),
        (
            "syntax".to_owned(),
            Value::Text("bhcp/syntax.canonical@0".to_owned()),
        ),
        ("policy_overlays".to_owned(), Value::Array(vec![])),
        ("type_mode".to_owned(), Value::Text("strict".to_owned())),
        ("type_mode".to_owned(), Value::Text("dynamic".to_owned())),
    ]);
    let diagnostic = PresentationDocument::from_value(&duplicate_field).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP9001");
    assert!(diagnostic.message.contains("duplicate profile document field"));
}

#[test]
fn root_validation_uses_typed_models_without_rejecting_negotiated_features() {
    for (file, kind) in [("syntax.diag", "syntax"), ("profile.diag", "profile")] {
        let value = parse(
            &fs::read_to_string(root().join("schemas/v0/examples").join(file)).unwrap(),
        );
        validate_root(&value, kind).unwrap();
        PresentationDocument::from_value(&value).unwrap();
    }

    let with_unknown_feature = parse(
        r#"{
          "version": "bhcp/v0",
          "features": ["vendor/feature.future@7"],
          "kind": "profile",
          "symbol": "example/profile.future@0",
          "syntax": "bhcp/syntax.canonical@0",
          "policy_overlays": [],
          "type_mode": "strict"
        }"#,
    );
    validate_root(&with_unknown_feature, "profile").unwrap();

    let invalid = parse(
        r#"{
          "version": "bhcp/v0", "features": [], "kind": "profile",
          "symbol": "example/profile@0", "syntax": "bhcp/syntax.canonical@0",
          "policy_overlays": [], "type_mode": "permissive"
        }"#,
    );
    let diagnostic = validate_root(&invalid, "profile").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP5002");
    assert!(diagnostic.message.contains("profile fixture is invalid"));
}
