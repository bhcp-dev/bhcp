use bhcp::pipeline::{compile_source, parse_policy_source};
use bhcp::policy::{PolicyDocument, PolicyLayer, PolicyRule};
use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn all_rules_source(label: &str) -> String {
    format!(
        r#"§policy example/policy@0 §extends example/base@0 {{
  layer repository;
  rule a-requirement "{label}": requirement add {{
    requirement: example/requirement.lint@0,
    scope: {{ goals: [example/goal.build@0] }},
    parameters: {{ enabled: true, severity: "high" }}
  }} nonwaivable;
  rule b-evidence: evidence add {{
    obligation: example/obligation.review@0,
    classes: ["formal", "static"],
    minimum: 2
  }} nonwaivable;
  rule c-prohibition: prohibition deny {{
    effect: bhcp-effect/network@0
  }} nonwaivable;
  rule d-capability: capability narrow {{
    effect: bhcp-effect/fs.read@0,
    scope: {{ operations: [example/operation.read@0] }}
  }} waivable by ["security-team", "team-owner"];
  rule e-limit: limit tighten {{
    dimension: example/limit.memory@0,
    unit: example/unit.byte@0,
    maximum: ["integer", 4096]
  }} nonwaivable;
  rule f-type-mode: type-mode strengthen strict nonwaivable;
}}
"#
    )
}

#[test]
fn every_layer_and_typed_rule_lowers_to_validated_policy_documents() {
    for layer in ["organization", "team", "repository", "user"] {
        let source = all_rules_source("diagnostic label")
            .replace("layer repository", &format!("layer {layer}"));
        let parsed = parse_policy_source(&source, &format!("{layer}.bhcp")).unwrap();

        assert_eq!(parsed.documents.len(), 1);
        let document = &parsed.documents[0];
        assert_eq!(
            document.layer,
            match layer {
                "organization" => PolicyLayer::Organization,
                "team" => PolicyLayer::Team,
                "repository" => PolicyLayer::Repository,
                "user" => PolicyLayer::User,
                _ => unreachable!(),
            }
        );
        assert_eq!(document.extends.as_deref(), Some("example/base@0"));
        assert_eq!(document.rules.len(), 6);
        assert!(matches!(document.rules[0], PolicyRule::Requirement { .. }));
        assert!(matches!(document.rules[1], PolicyRule::Evidence { .. }));
        assert!(matches!(document.rules[2], PolicyRule::Prohibition { .. }));
        assert!(matches!(document.rules[3], PolicyRule::Capability { .. }));
        assert!(matches!(document.rules[4], PolicyRule::Limit { .. }));
        assert!(matches!(document.rules[5], PolicyRule::TypeMode { .. }));

        let policy = PolicyDocument::Source(document.clone());
        policy.validate().unwrap();
        let bytes = policy.to_cbor(false).unwrap();
        assert_eq!(PolicyDocument::from_cbor(&bytes).unwrap(), policy);
    }
}

#[test]
fn checked_in_canonical_policy_fixture_lowers_through_the_strong_model() {
    let path = root().join("conformance/v0/fixtures/canonical-policy.bhcp");
    let source = fs::read_to_string(&path).unwrap();
    let parsed = parse_policy_source(&source, path.to_str().unwrap()).unwrap();

    assert_eq!(parsed.documents.len(), 1);
    assert_eq!(parsed.documents[0].layer, PolicyLayer::Repository);
    assert_eq!(parsed.documents[0].rules.len(), 2);
    PolicyDocument::Source(parsed.documents[0].clone())
        .validate()
        .unwrap();

    let executable = compile_source(&source, path.to_str().unwrap()).unwrap_err();
    assert_eq!(executable.code, "BHCP2004");
    assert_eq!(
        executable.message,
        "policy definitions lower to policy documents, not executable goal IR"
    );
}

#[test]
fn canonical_grammar_keeps_deferred_waivers_out_of_policy_blocks() {
    let semantics = fs::read_to_string(root().join("SEMANTICS.md")).unwrap();
    assert!(semantics.contains("waiver-def      = \"§waiver\" qualified-name waiver-block ;"));
    assert!(semantics.contains("waiver-block    = meta-block ;"));
    assert!(!semantics.contains("waiver-def      = \"§waiver\" qualified-name policy-block ;"));
}

#[test]
fn policy_ast_retains_definition_and_rule_spans() {
    let parsed = parse_policy_source(&all_rules_source("shown to humans"), "policy.bhcp").unwrap();
    let policy = &parsed.ast.root.children[0];

    assert_eq!(policy.kind, "policy");
    assert_eq!(policy.span.start.line, 1);
    assert_eq!(policy.span.start.column, 1);
    assert_eq!(policy.children.len(), 6);
    assert_eq!(policy.children[0].kind, "policy-rule");
    assert_eq!(policy.children[0].span.start.line, 3);
    assert_eq!(policy.children[0].span.start.column, 3);
    assert_eq!(policy.children[5].span.start.line, 25);
}

#[test]
fn formatting_comments_and_labels_do_not_change_policy_documents() {
    let first = parse_policy_source(&all_rules_source("first label"), "first.bhcp").unwrap();
    let second_source = all_rules_source("different label")
        .replace(
            "  layer repository;",
            "/* presentation only */ layer repository; // same layer",
        )
        .replace("  rule b-evidence", "\n\n  rule b-evidence");
    let second = parse_policy_source(&second_source, "second.bhcp").unwrap();

    let first = PolicyDocument::Source(first.documents[0].clone());
    let second = PolicyDocument::Source(second.documents[0].clone());
    assert_eq!(
        first.to_cbor(false).unwrap(),
        second.to_cbor(false).unwrap()
    );
}

#[test]
fn unsupported_waiver_and_profile_shorthand_fail_stably() {
    let waiver = parse_policy_source("§waiver example/waiver@0 {}", "waiver.bhcp").unwrap_err();
    assert_eq!(waiver.code, "BHCP1004");
    assert_eq!(waiver.line, 1);
    assert_eq!(waiver.column, 1);
    assert_eq!(
        waiver.message,
        "top-level syntax \"§waiver\" is outside the implemented vertical slice"
    );

    let profile = parse_policy_source(
        "§policy example/policy@0 { layer repository; profile example/profile@0; }",
        "profile.bhcp",
    )
    .unwrap_err();
    assert_eq!(profile.code, "BHCP1004");
    assert_eq!(profile.line, 1);
    assert_eq!(profile.column, 46);
    assert_eq!(
        profile.message,
        "policy clause \"profile\" is outside the implemented policy slice"
    );
}

#[test]
fn malformed_operations_values_and_issuers_have_stable_source_diagnostics() {
    let invalid_operation = parse_policy_source(
        "§policy example/policy@0 {\n  layer repository;\n  rule r: requirement deny { requirement: example/requirement.lint@0 } nonwaivable;\n}",
        "operation.bhcp",
    )
    .unwrap_err();
    assert_eq!(invalid_operation.code, "BHCP8001");
    assert_eq!(invalid_operation.line, 3);
    assert_eq!(invalid_operation.column, 3);
    assert_eq!(
        invalid_operation.message,
        "policy category \"requirement\" requires operation \"add\""
    );

    let invalid_value = parse_policy_source(
        "§policy example/policy@0 { layer repository; rule r: requirement add strict nonwaivable; }",
        "value.bhcp",
    )
    .unwrap_err();
    assert_eq!(invalid_value.code, "BHCP8001");
    assert_eq!(
        invalid_value.message,
        "requirement policy value must be a map"
    );

    let missing_issuer = parse_policy_source(
        "§policy example/policy@0 { layer repository; rule r: type-mode strengthen strict waivable; }",
        "issuer.bhcp",
    )
    .unwrap_err();
    assert_eq!(missing_issuer.code, "BHCP1001");
    assert_eq!(missing_issuer.message, "expected \"by\", found \";\"");

    let empty_issuers = parse_policy_source(
        "§policy example/policy@0 { layer repository; rule r: type-mode strengthen strict waivable by []; }",
        "empty-issuers.bhcp",
    )
    .unwrap_err();
    assert_eq!(empty_issuers.code, "BHCP8001");
    assert_eq!(
        empty_issuers.message,
        "authorized issuers must be a non-empty sorted set"
    );
}

#[test]
fn duplicate_symbols_and_noncanonical_rule_order_are_rejected_at_the_source_span() {
    let duplicate = parse_policy_source(
        "§policy example/policy@0 { layer repository; }\n§policy example/policy@0 { layer repository; }",
        "duplicate.bhcp",
    )
    .unwrap_err();
    assert_eq!(duplicate.code, "BHCP1003");
    assert_eq!(duplicate.line, 2);
    assert_eq!(duplicate.column, 1);
    assert_eq!(
        duplicate.message,
        "duplicate policy symbol example/policy@0"
    );

    let order = parse_policy_source(
        "§policy example/policy@0 {\n  layer repository;\n  rule z: type-mode strengthen strict nonwaivable;\n  rule a: type-mode strengthen strict nonwaivable;\n}",
        "order.bhcp",
    )
    .unwrap_err();
    assert_eq!(order.code, "BHCP8001");
    assert_eq!(order.line, 4);
    assert_eq!(order.column, 3);
    assert_eq!(
        order.message,
        "source policy rules must be sorted by unique rule ID"
    );
}
