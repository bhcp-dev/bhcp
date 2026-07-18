use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::schema::{parse_diagnostic, validate_root, validate_schema_inventory};

const EXPECTED: [&str; 17] = [
    "canonical-ast",
    "semantic-ir",
    "syntax",
    "profile",
    "policy",
    "waiver",
    "extension-descriptor",
    "obligation-graph",
    "capability-graph",
    "state-graph",
    "execution-graph",
    "evidence-bundle",
    "execution-result",
    "planner-request",
    "planner-result",
    "feature-manifest",
    "content-reference",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn all_seventeen_root_fixtures_parse_validate_and_round_trip() {
    let examples = root().join("schemas/v0/examples");
    let schema = fs::read_to_string(root().join("schemas/v0/bhcp-v0.cddl")).unwrap();
    let manifest = fs::read_to_string(examples.join("manifest.txt")).unwrap();
    let mut seen = HashSet::new();
    let mut count = 0;
    for line in manifest.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 2);
        let source = fs::read_to_string(examples.join(fields[0])).unwrap();
        let value = parse_diagnostic(&source).unwrap();
        validate_root(&value, fields[1]).unwrap();
        let bytes = encode_deterministic(&value).unwrap();
        assert_eq!(decode_deterministic(&bytes).unwrap(), value);
        assert!(seen.insert(fields[1]));
        count += 1;
    }
    assert_eq!(count, 17);
    assert_eq!(seen, EXPECTED.into_iter().collect());
    validate_schema_inventory(&schema, &EXPECTED).unwrap();
}

#[test]
fn compiler_artifacts_are_deterministic_and_match_root_shapes() {
    let directory = root().join("conformance/v0/fixtures");
    let schema = fs::read_to_string(root().join("schemas/v0/bhcp-v0.cddl")).unwrap();
    for (file, kind) in [
        ("canonical-simple.ast.cbor", "canonical-ast"),
        ("canonical-simple.ir.cbor", "semantic-ir"),
    ] {
        let bytes = fs::read(directory.join(file)).unwrap();
        let value = decode_deterministic(&bytes).unwrap();
        validate_root(&value, kind).unwrap();
        assert_eq!(encode_deterministic(&value).unwrap(), bytes);
    }

    validate_schema_inventory(&schema, &EXPECTED).unwrap();
}

#[test]
fn malformed_cddl_is_rejected_with_a_stable_diagnostic() {
    let diagnostic = validate_schema_inventory("root-document = [", &EXPECTED).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP5002");
    assert!(
        diagnostic
            .message
            .starts_with("CDDL schema does not parse:")
    );
}

#[test]
fn compile_time_lowering_metamodel_is_required() {
    let schema = fs::read_to_string(root().join("schemas/v0/bhcp-v0.cddl")).unwrap();
    let without_metamodel = schema
        .lines()
        .filter(|line| !line.starts_with("meta-type ="))
        .collect::<Vec<_>>()
        .join("\n");
    let diagnostic = validate_schema_inventory(&without_metamodel, &EXPECTED).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP5002");
}

#[test]
fn execution_results_reject_flat_or_mixed_category_states() {
    for source in [
        r#"{
          "version": "bhcp/v0", "features": [], "kind": "execution-result",
          "goal": "goal-1",
          "result": {"state": "satisfied", "output": "x", "evidence": ["e-1"]}
        }"#,
        r#"{
          "version": "bhcp/v0", "features": [], "kind": "execution-result",
          "goal": "goal-1",
          "result": {"state": "completed", "verdict": {"state": "faulted"}}
        }"#,
    ] {
        let value = parse_diagnostic(source).unwrap();
        let diagnostic = validate_root(&value, "execution-result").unwrap_err();
        assert_eq!(diagnostic.code, "BHCP5002");
    }
}

#[test]
fn kernel_schema_rejects_behavior_and_planner_metadata() {
    let schema = fs::read_to_string(root().join("schemas/v0/bhcp-v0.cddl")).unwrap();
    for field in [
        r#"? "quantified": quantified-family"#,
        r#"? "parallel_eligible": bool"#,
        r#"? "parallel_reasons": [* tstr]"#,
        r#"? "budgets": [* budget]"#,
    ] {
        let widened = schema.replace(
            "  \"reducer\": symbol-id\n}",
            &format!("  \"reducer\": symbol-id,\n  {field}\n}}"),
        );
        let diagnostic = validate_schema_inventory(&widened, &EXPECTED).unwrap_err();
        assert_eq!(diagnostic.code, "BHCP5002");
    }

    let behavior_rule = schema.replace(
        "  \"id\": ref-id,\n  \"premises\": [* ref-id]",
        "  \"id\": ref-id,\n  \"rule\": symbol-id,\n  \"premises\": [* ref-id]",
    );
    let diagnostic = validate_schema_inventory(&behavior_rule, &EXPECTED).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP5002");
}

#[test]
fn extension_descriptors_enforce_derived_and_native_boundaries() {
    for source in [
        r#"{
          "version": "bhcp/v0", "features": [], "kind": "extension-descriptor",
          "symbol": "example/derived@0", "extension_kind": "derived",
          "must_understand": false,
          "type_rule": {}, "effect_rule": {}, "policy_rule": {},
          "normalization_rule": {}, "evidence_rule": {}
        }"#,
        r#"{
          "version": "bhcp/v0", "features": [], "kind": "extension-descriptor",
          "symbol": "example/native@0", "extension_kind": "native",
          "must_understand": false, "payload_schema": {},
          "type_rule": {}, "effect_rule": {}, "policy_rule": {},
          "normalization_rule": {}, "evidence_rule": {}
        }"#,
        r#"{
          "version": "bhcp/v0", "features": [], "kind": "extension-descriptor",
          "symbol": "example/native@0", "extension_kind": "native",
          "must_understand": true,
          "type_rule": {}, "effect_rule": {}, "policy_rule": {},
          "normalization_rule": {}, "evidence_rule": {}
        }"#,
    ] {
        let value = parse_diagnostic(source).unwrap();
        let diagnostic = validate_root(&value, "extension-descriptor").unwrap_err();
        assert_eq!(diagnostic.code, "BHCP5002");
    }
}
