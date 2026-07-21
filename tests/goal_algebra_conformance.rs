use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::pipeline::compile_source;
use bhcp::prelude::{ALL_FEATURE, ANY_FEATURE, CHAIN_FEATURE, GATE_FEATURE, NONE_FEATURE};
use bhcp::schema::{parse_diagnostic, validate_root};
use bhcp::value::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.as_ref().display()))
}

#[derive(Debug)]
struct Case {
    behavior: String,
    source: String,
    equivalent: String,
    ast: String,
    ir: String,
    evidence: String,
    edge_case: String,
    proof_check: String,
}

fn cases() -> Vec<Case> {
    read(root().join("conformance/v0/goal-algebra/manifest.txt"))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields = line.split('|').map(str::trim).collect::<Vec<_>>();
            assert_eq!(fields.len(), 8, "invalid goal-algebra manifest row: {line}");
            Case {
                behavior: fields[0].to_owned(),
                source: fields[1].to_owned(),
                equivalent: fields[2].to_owned(),
                ast: fields[3].to_owned(),
                ir: fields[4].to_owned(),
                evidence: fields[5].to_owned(),
                edge_case: fields[6].to_owned(),
                proof_check: fields[7].to_owned(),
            }
        })
        .collect()
}

#[test]
fn complete_goal_algebra_regenerates_and_round_trips_deterministically() {
    let repository = root();
    let fixture_root = repository.join("conformance/v0/fixtures");
    let cases = cases();
    assert_eq!(
        cases
            .iter()
            .map(|case| case.behavior.as_str())
            .collect::<Vec<_>>(),
        ["all", "any", "none", "chain", "gate"]
    );

    for case in &cases {
        let source_path = fixture_root.join(&case.source);
        let compiled = compile_source(&read(&source_path), source_path.to_str().unwrap()).unwrap();
        let expected_ast = fs::read(fixture_root.join(&case.ast)).unwrap();
        let expected_ir = fs::read(fixture_root.join(&case.ir)).unwrap();
        assert_eq!(
            compiled.ast_bytes, expected_ast,
            "{} AST drift",
            case.behavior
        );
        assert_eq!(compiled.ir_bytes, expected_ir, "{} IR drift", case.behavior);

        for (bytes, kind) in [
            (&expected_ast, "canonical-ast"),
            (&expected_ir, "semantic-ir"),
        ] {
            let value = decode_deterministic(bytes).unwrap();
            validate_root(&value, kind).unwrap();
            assert_eq!(encode_deterministic(&value).unwrap(), *bytes);
        }

        if case.equivalent != "-" {
            let equivalent_path = fixture_root.join(&case.equivalent);
            let equivalent =
                compile_source(&read(&equivalent_path), equivalent_path.to_str().unwrap()).unwrap();
            assert_eq!(compiled.ir.semantic_value(), equivalent.ir.semantic_value());
            assert_eq!(compiled.ir.semantic_id, equivalent.ir.semantic_id);
        }

        let evidence = read(repository.join(&case.evidence));
        for function in [&case.edge_case, &case.proof_check] {
            assert!(
                evidence.contains(&format!("fn {function}()")),
                "{} omits executable evidence {function}",
                case.behavior
            );
        }
    }
}

#[test]
fn feature_manifest_negotiates_completed_graph_builders_and_bounded_non_goals() {
    let value = parse_diagnostic(&read(
        root().join("schemas/v0/examples/feature-manifest.diag"),
    ))
    .unwrap();
    validate_root(&value, "feature-manifest").unwrap();

    let Value::Array(entries) = value.get("features_supported").unwrap() else {
        panic!("features_supported must be an array")
    };
    let mut support = BTreeMap::new();
    for entry in entries {
        let (Some(Value::Text(feature)), Some(Value::Text(level))) =
            (entry.get("feature"), entry.get("level"))
        else {
            panic!("feature support entry must name text feature and level")
        };
        let notes = match entry.get("notes") {
            Some(Value::Text(notes)) => Some(notes.as_str()),
            None => None,
            _ => panic!("feature support notes must be text"),
        };
        assert!(
            support
                .insert(feature.as_str(), (level.as_str(), notes))
                .is_none()
        );
    }
    assert_eq!(support.remove("bhcp/core@0"), Some(("required", None)));
    for feature in [
        ALL_FEATURE,
        ANY_FEATURE,
        NONE_FEATURE,
        CHAIN_FEATURE,
        GATE_FEATURE,
        "bhcp/feature.extension-resolution@0",
    ] {
        assert_eq!(
            support.remove(feature).map(|(level, _)| level),
            Some("supported"),
            "{feature}"
        );
    }
    assert_eq!(
        support.remove("bhcp/feature.complete-obligation-graph@0"),
        Some((
            "supported",
            Some(
                "Structural construction is supported; execution-time discharge remains separate."
            )
        ))
    );
    assert_eq!(
        support.remove("bhcp/feature.capability-graph-builder@0"),
        Some((
            "supported",
            Some(
                "Decision construction is supported; planning and runtime enforcement remain separate."
            )
        ))
    );
    assert_eq!(
        support
            .remove("bhcp/feature.shared-typed-graph-model@0")
            .map(|(level, _)| level),
        Some("supported")
    );
    assert!(
        support.is_empty(),
        "unclassified feature support entries: {support:?}"
    );

    let Some(Value::Array(document_entries)) = value.get("documents") else {
        panic!("documents must be an array")
    };
    let documents = document_entries
        .iter()
        .map(|document| match document {
            Value::Text(document) => document.as_str(),
            _ => panic!("document support entries must be text"),
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        documents,
        [
            "canonical-ast",
            "semantic-ir",
            "obligation-graph",
            "capability-graph",
            "state-graph",
            "execution-graph",
            "evidence-bundle",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(value.get("native_extensions"), Some(&Value::Array(vec![])));
}

#[test]
fn graph_builder_completion_claims_stay_bounded_and_consistent() {
    let repository = root();
    let readme = read(repository.join("README.md"));
    let semantics = read(repository.join("SEMANTICS.md"));
    let threat_model = read(repository.join("THREAT_MODEL.md"));
    let schema_readme = read(repository.join("schemas/v0/README.md"));
    let conformance = read(repository.join("conformance/v0/README.md"));

    for (name, document) in [
        ("README", readme.as_str()),
        ("SEMANTICS", semantics.as_str()),
        ("threat model", threat_model.as_str()),
        ("schema README", schema_readme.as_str()),
        ("conformance", conformance.as_str()),
    ] {
        let normalized = document.to_lowercase();
        assert!(
            normalized.contains("deterministic obligation")
                && normalized.contains("capability graph"),
            "{name} does not describe both completed graph-builder boundaries"
        );
        assert!(
            normalized.contains("planning") && normalized.contains("runtime"),
            "{name} omits the bounded planning/runtime non-goals"
        );
    }

    assert!(!readme.contains("does not yet build obligation"));
    assert!(!schema_readme.contains("obligation-graph construction unsupported"));
    assert!(!semantics.contains("Capability/state construction"));
}

#[test]
fn generator_and_schema_inventories_stay_exact() {
    let repository = root();
    let generator = read(repository.join("src/bin/generate_fixtures.rs"));
    for fixture in ["simple", "all", "any", "none", "chain", "gate"] {
        assert!(generator.contains(&format!("\"canonical-{fixture}\"")));
    }
    assert_eq!(generator.matches("\"canonical-").count(), 6);

    let roots = read(repository.join("schemas/v0/examples/manifest.txt"));
    let rows = roots
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 17);
    assert_eq!(
        rows.iter()
            .filter(|line| line.ends_with(" feature-manifest"))
            .count(),
        1
    );
    assert!(read(repository.join("README.md")).contains("17-root fixture invariant"));
}
