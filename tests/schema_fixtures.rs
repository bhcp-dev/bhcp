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
    "runtime-outcome",
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
    let schema = fs::read_to_string(root().join("schemas/v0/bhcp-v0.cddl")).unwrap();
    validate_schema_inventory(&schema, &EXPECTED).unwrap();
}

#[test]
fn compiler_artifacts_are_deterministic_and_match_root_shapes() {
    let directory = root().join("conformance/v0/fixtures");
    for (file, kind) in [
        ("canonical-simple.ast.cbor", "canonical-ast"),
        ("canonical-simple.ir.cbor", "semantic-ir"),
    ] {
        let bytes = fs::read(directory.join(file)).unwrap();
        let value = decode_deterministic(&bytes).unwrap();
        validate_root(&value, kind).unwrap();
        assert_eq!(encode_deterministic(&value).unwrap(), bytes);
    }
}
