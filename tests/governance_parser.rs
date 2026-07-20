use bhcp::hash::HashAlgorithm;
use bhcp::model::ContentReference;
use bhcp::parser::parse_canonical;
use bhcp::pipeline::{compile_source, parse_source};
use bhcp::policy::WaiverDocument;
use bhcp::profile::PresentationDocument;
use bhcp::schema::validate_root;
use bhcp::value::Value;

fn attribute<'a>(node: &'a bhcp::model::AstNode, name: &str) -> &'a Value {
    &node
        .attributes
        .iter()
        .find(|(candidate, _)| candidate == name)
        .unwrap_or_else(|| panic!("{} omits attribute {name}", node.kind))
        .1
}

#[test]
fn complete_governance_forms_build_closed_ordered_schema_valid_ast() {
    let source = include_str!("fixtures/governance-forms.bhcp");
    let first = parse_source(source, "governance-forms.bhcp").unwrap();
    let second = parse_source(source, "governance-forms.bhcp").unwrap();
    first.validate().unwrap();
    validate_root(&first.to_value(true), "canonical-ast").unwrap();
    assert_eq!(first.to_value(true), second.to_value(true));

    assert_eq!(
        first
            .root
            .children
            .iter()
            .map(|node| node.kind.as_str())
            .collect::<Vec<_>>(),
        [
            "policy",
            "syntax",
            "profile",
            "waiver",
            "extension",
            "extension"
        ]
    );

    let syntax = &first.root.children[1];
    assert_eq!(
        attribute(syntax, "symbol"),
        &Value::Text("example/words@0".into())
    );
    assert!(matches!(attribute(syntax, "mappings"), Value::Array(values) if values.len() == 6));
    assert!(matches!(attribute(syntax, "formatting"), Value::Map(values) if values.len() == 3));

    let profile = &first.root.children[2];
    assert_eq!(
        attribute(profile, "extends"),
        &Value::Text("example/base-profile@0".into())
    );
    assert_eq!(
        attribute(profile, "type_mode"),
        &Value::Text("infer-strict".into())
    );

    let waiver = &first.root.children[3];
    assert!(matches!(attribute(waiver, "targets"), Value::Array(values) if values.len() == 1));
    assert!(matches!(attribute(waiver, "issued_at"), Value::Tag(0, _)));

    let derived = &first.root.children[4];
    assert_eq!(
        attribute(derived, "extension_kind"),
        &Value::Text("derived".into())
    );
    assert_eq!(
        attribute(derived, "lowering"),
        &Value::Text("example/lower-derived@0".into())
    );
    let native = &first.root.children[5];
    assert_eq!(
        attribute(native, "extension_kind"),
        &Value::Text("native".into())
    );
    assert_eq!(attribute(native, "must_understand"), &Value::Bool(true));
}

#[test]
fn governance_field_sets_order_and_duplicates_fail_stably() {
    let cases = [
        (
            "§syntax example/s@0 { mappings []; preamble \"#!bhcp-profile\"; formatting { indent_width: 2, line_width: 80, final_newline: true }; }",
            "BHCP1003",
            "field order",
        ),
        (
            "§profile example/p@0 { syntax example/s@0; syntax example/s@0; type_mode strict; policy_overlays []; }",
            "BHCP1003",
            "duplicate profile field",
        ),
        (
            "§waiver example/w@0 { issuer \"security\"; targets []; mystery true; }",
            "BHCP1004",
            "unknown waiver field",
        ),
        (
            "§extension example/e@0 derived { input example/I@0; output example/O@0; children []; }",
            "BHCP1001",
            "lowering",
        ),
        (
            "§extension example/e@0 native { lowering example/lower@0; payload_schema example/schema@0; type_rule example/t@0; effect_rule example/e@0; policy_rule example/p@0; normalization_rule example/n@0; evidence_rule example/v@0; }",
            "BHCP1004",
            "native extension",
        ),
    ];

    for (source, code, message) in cases {
        let diagnostic = parse_source(source, "bad-governance.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, code, "{source}");
        assert!(
            diagnostic.message.contains(message),
            "{}: {}",
            source,
            diagnostic.message
        );
    }
}

#[test]
fn executable_governance_payloads_and_policy_expressions_fail_before_artifacts() {
    let cases = [
        "§syntax example/s@0 { preamble \"#!bhcp-profile\"; mappings []; formatting { indent_width: 2, line_width: 80, final_newline: true }; parser_callback example/run@0; }",
        "§syntax example/s@0 { preamble \"#!bhcp-profile\"; mappings []; formatting { indent_width: 2, line_width: 80, final_newline: true }; macro example/run@0; }",
        "§profile example/p@0 { syntax example/s@0; type_mode strict; policy_overlays []; semantic_override example/unsafe@0; }",
        "§extension example/e@0 derived { lowering example/lower@0; input example/I@0; output example/O@0; children []; parser_callback example/run@0; }",
        "§policy example/p@0 { layer repository; rule a: prohibition deny { effect: bhcp-effect/network@0 } && true nonwaivable; }",
    ];
    for source in cases {
        let diagnostic = parse_source(source, "executable-payload.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, "BHCP1004", "{source}: {diagnostic}");
    }
}

#[test]
fn governance_source_stops_before_activation_or_executable_ir() {
    let diagnostic = compile_source(
        include_str!("fixtures/governance-forms.bhcp"),
        "governance-forms.bhcp",
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2004");
    assert!(diagnostic.message.contains("governance"));
}

#[test]
fn frozen_reference_governance_sources_reach_the_canonical_ast_boundary() {
    for name in ["policy", "waiver", "syntax", "profile", "extension"] {
        let source = match name {
            "policy" => include_str!("../conformance/v0/reference-program/policy.bhcp"),
            "waiver" => include_str!("../conformance/v0/reference-program/waiver.bhcp"),
            "syntax" => include_str!("../conformance/v0/reference-program/syntax.bhcp"),
            "profile" => include_str!("../conformance/v0/reference-program/profile.bhcp"),
            "extension" => include_str!("../conformance/v0/reference-program/extension.bhcp"),
            _ => unreachable!(),
        };
        let ast = parse_source(source, &format!("{name}.bhcp")).unwrap();
        ast.validate().unwrap();
        validate_root(&ast.to_value(true), "canonical-ast").unwrap();
    }
}

#[test]
fn syntax_and_profile_source_projections_round_trip_through_typed_wire_models() {
    for (name, source) in [
        (
            "syntax",
            include_str!("../conformance/v0/reference-program/syntax.bhcp"),
        ),
        (
            "profile",
            include_str!("../conformance/v0/reference-program/profile.bhcp"),
        ),
    ] {
        let source_ref =
            ContentReference::from_bytes("text/bhcp", source.as_bytes(), HashAlgorithm::default());
        let parsed = parse_canonical(source, &format!("{name}.bhcp"), source_ref).unwrap();
        let document = if name == "syntax" {
            PresentationDocument::Syntax(parsed.syntaxes[0].document.clone())
        } else {
            PresentationDocument::Profile(parsed.profiles[0].document.clone())
        };
        document.validate().unwrap();
        let bytes = document.to_cbor(false).unwrap();
        assert_eq!(PresentationDocument::from_cbor(&bytes).unwrap(), document);
    }
}

#[test]
fn waiver_source_rejects_every_reviewed_typed_boundary_violation() {
    let valid = include_str!("fixtures/governance-forms.bhcp");
    let cases = [
        valid.replace(
            "from: { dimension: example/limit.attempts@0, unit: example/unit.count@0, maximum: [integer, 3] }",
            "from: true",
        ),
        valid.replace(
            "rule: [example/repository@0, \"e-limit\"],",
            "rule: [example/repository@0, \"e-limit\"], scope: { bogus: true },",
        ),
        valid.replace(
            "justification \"one emergency retry\";",
            "justification \"one emergency retry\"; authority_chain [{ authorization: true }];",
        ),
        valid.replace(
            "targets [{",
            "targets [{ rule: [example/repository@0, \"z-limit\"], weakening: { category: \"limit\", operation: \"loosen\", from: { dimension: example/limit.attempts@0, unit: example/unit.count@0, maximum: [integer, 3] }, to: { dimension: example/limit.attempts@0, unit: example/unit.count@0, maximum: [integer, 4] } } }, {",
        ),
        valid.replace(
            "category: \"limit\",\n            operation: \"loosen\",",
            "operation: \"loosen\",\n            category: \"limit\",",
        ),
    ];

    for source in cases {
        assert!(
            parse_source(&source, "invalid-waiver.bhcp").is_err(),
            "reviewed invalid waiver unexpectedly parsed"
        );
    }
}

#[test]
fn waiver_and_extension_source_projections_round_trip_through_wire_boundaries() {
    let source = include_str!("fixtures/governance-forms.bhcp");
    let source_ref =
        ContentReference::from_bytes("text/bhcp", source.as_bytes(), HashAlgorithm::default());
    let parsed = parse_canonical(source, "governance-forms.bhcp", source_ref).unwrap();

    let waiver = parsed.waivers[0]
        .document
        .as_ref()
        .expect("inline waiver references must materialize a wire document");
    let bytes = waiver.to_cbor(false).unwrap();
    assert_eq!(WaiverDocument::from_cbor(&bytes).unwrap(), *waiver);

    for extension in &parsed.extensions {
        let descriptor = extension
            .descriptor
            .as_ref()
            .expect("complete extension source must materialize a descriptor");
        validate_root(descriptor, "extension-descriptor").unwrap();
        let bytes = bhcp::cbor::encode_deterministic(descriptor).unwrap();
        assert_eq!(
            bhcp::cbor::decode_deterministic(&bytes).unwrap(),
            *descriptor
        );
    }
}

#[test]
fn waiver_policy_parameters_remain_unrestricted_inside_closed_records() {
    let source = r#"
§waiver example/w@0 {
    issuer "security";
    targets [{
        rule: [example/p@0, "a"],
        weakening: {
            category: "requirement",
            operation: "remove",
            value: {
                requirement: example/r@0,
                parameters: { effect: true, custom: false }
            }
        }
    }];
    justification "temporary";
    issued_at time "2026-07-20T00:00:00Z";
    not_before time "2026-07-20T00:00:00Z";
    expires_at time "2026-07-21T00:00:00Z";
    authorization [example/authorization@0];
    audit_reference example/audit@0;
}
"#;
    let parsed = parse_canonical(
        source,
        "waiver-parameters.bhcp",
        ContentReference::from_bytes("text/bhcp", source.as_bytes(), HashAlgorithm::default()),
    )
    .unwrap();
    assert_eq!(parsed.waivers.len(), 1);
}
