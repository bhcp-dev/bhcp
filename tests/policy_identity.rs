use bhcp::hash::{HashAlgorithm, SHA3_512};
use bhcp::pipeline::parse_policy_source;
use bhcp::policy::{EffectivePolicyDocument, PolicyDocument, compose_policies};
use bhcp::value::Value;

fn compose(source: &str) -> EffectivePolicyDocument {
    let parsed = parse_policy_source(source, "identity.bhcp").unwrap();
    compose_policies(&parsed.documents, HashAlgorithm::default()).unwrap()
}

fn all_rules_source() -> &'static str {
    r#"§policy example/policy@0 {
  layer repository;
  rule a: requirement add { requirement: example/requirement.lint@0 } nonwaivable;
  rule b: evidence add { obligation: example/obligation.review@0, classes: [static], minimum: 1 } nonwaivable;
  rule c: prohibition deny { effect: bhcp-effect/network@0 } nonwaivable;
  rule d: capability narrow { effect: bhcp-effect/fs.read@0 } nonwaivable;
  rule e: limit tighten { dimension: example/limit.memory@0, unit: example/unit.byte@0, maximum: ["integer", 5] } nonwaivable;
  rule f: type-mode strengthen gradual nonwaivable;
}"#
}

#[test]
fn every_observable_effective_coordinate_changes_semantic_identity() {
    let baseline = all_rules_source();
    let variants = [
        baseline.replace("example/requirement.lint@0", "example/requirement.audit@0"),
        baseline.replace("example/obligation.review@0", "example/obligation.audit@0"),
        baseline.replace("bhcp-effect/network@0", "bhcp-effect/process.spawn@0"),
        baseline.replace("bhcp-effect/fs.read@0", "bhcp-effect/fs.write@0"),
        baseline.replace("[\"integer\", 5]", "[\"integer\", 4]"),
        baseline.replace("strengthen gradual", "strengthen strict"),
        baseline.replacen("nonwaivable", "waivable by [\"issuer-a\"]", 1),
        baseline.replacen("nonwaivable", "waivable by [\"issuer-a\", \"issuer-b\"]", 1),
    ];

    let baseline_id = compose(baseline).header.semantic_id.unwrap();
    let variant_ids = variants
        .iter()
        .map(|source| compose(source).header.semantic_id.unwrap())
        .collect::<Vec<_>>();
    assert!(variant_ids.iter().all(|identity| identity != &baseline_id));
    assert!(variant_ids.iter().enumerate().all(|(index, identity)| {
        variant_ids[index + 1..]
            .iter()
            .all(|other| other != identity)
    }));
}

#[test]
fn presentation_and_source_order_normalize_to_identical_artifact_bytes() {
    let first = r#"§policy example/a@0 {
  layer team;
  rule a "first label": requirement add { requirement: example/requirement.lint@0 } nonwaivable;
}
§policy example/b@0 {
  layer team;
  rule a: type-mode strengthen strict nonwaivable;
}"#;
    let second = r#"§policy example/b@0 {
  /* presentation only */ layer team; // same meaning
  rule a "different label": type-mode strengthen strict nonwaivable;
}

§policy example/a@0 { layer team;
  rule a: requirement add {
    requirement: example/requirement.lint@0
  } nonwaivable;
}"#;

    let first = compose(first);
    let second = compose(second);
    assert_eq!(first.header.semantic_id, second.header.semantic_id);
    assert_eq!(first.header.artifact_id, second.header.artifact_id);
    assert_eq!(
        PolicyDocument::Effective(first).to_cbor(true).unwrap(),
        PolicyDocument::Effective(second).to_cbor(true).unwrap()
    );
}

#[test]
fn decomposition_and_retained_provenance_are_artifact_only_inputs() {
    let combined = compose(
        r#"§policy example/combined@0 {
  layer team;
  rule a: requirement add { requirement: example/requirement.lint@0 } nonwaivable;
  rule b: type-mode strengthen strict nonwaivable;
}"#,
    );
    let split = compose(
        r#"§policy example/a@0 {
  layer team;
  rule a: requirement add { requirement: example/requirement.lint@0 } nonwaivable;
}
§policy example/b@0 {
  layer team;
  rule a: type-mode strengthen strict nonwaivable;
}"#,
    );
    assert_eq!(combined.header.semantic_id, split.header.semantic_id);
    assert_ne!(combined.header.artifact_id, split.header.artifact_id);

    let parsed = parse_policy_source(
        r#"§policy example/policy@0 {
  layer organization;
  rule a: requirement add { requirement: example/requirement.lint@0 } nonwaivable;
}"#,
        "provenance.bhcp",
    )
    .unwrap();
    let plain = compose_policies(&parsed.documents, HashAlgorithm::default()).unwrap();
    let mut retained = parsed.documents;
    retained[0].header.provenance = Some(Value::map([(
        "source",
        Value::Text("audit-ledger-entry".to_owned()),
    )]));
    let retained = compose_policies(&retained, HashAlgorithm::default()).unwrap();
    assert_eq!(plain.header.semantic_id, retained.header.semantic_id);
    assert_ne!(plain.header.artifact_id, retained.header.artifact_id);
    assert_ne!(plain.source_layers, retained.source_layers);
}

#[test]
fn identity_recomputation_and_algorithm_tags_match_the_materialized_document() {
    let document = compose(all_rules_source());
    assert_eq!(
        document
            .compute_semantic_id(HashAlgorithm::default())
            .unwrap(),
        document.header.semantic_id.clone().unwrap()
    );
    assert_eq!(
        document
            .compute_artifact_id(HashAlgorithm::default())
            .unwrap(),
        document.header.artifact_id.clone().unwrap()
    );
    assert_eq!(
        document.header.semantic_id.as_ref().unwrap().algorithm,
        SHA3_512
    );
    assert_eq!(
        document.header.artifact_id.as_ref().unwrap().algorithm,
        SHA3_512
    );
    assert!(document.source_layers.iter().all(|layer| {
        layer.policies.iter().all(|source| {
            source.artifact.digests.len() == 1 && source.artifact.digests[0].algorithm == SHA3_512
        })
    }));
    assert!(
        PolicyDocument::Effective(document)
            .to_cbor(true)
            .unwrap()
            .windows(SHA3_512.len())
            .any(|window| window == SHA3_512.as_bytes())
    );
}
