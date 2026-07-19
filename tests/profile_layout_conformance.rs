use std::fs;
use std::path::{Path, PathBuf};

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::formatting::format_source_bytes_with_profile_registry;
use bhcp::hash::artifact_hash;
use bhcp::pipeline::{compile_source_bytes_with_profile_registry, parse_policy_source};
use bhcp::profile::{PresentationDocument, ProfileRegistry};
use bhcp::schema::{parse_diagnostic, validate_root};

const SYMBOLIC_PROFILE: &str = "example/profile.layout-symbolic@0";
const NARRATIVE_PROFILE: &str = "example/profile.layout-narrative@0";

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("conformance/v0/profile-layout")
}

fn read(path: &Path, name: &str) -> String {
    fs::read_to_string(path.join(name)).unwrap()
}

fn registry(policy_source: &str) -> (ProfileRegistry, Vec<PresentationDocument>) {
    let path = directory();
    let documents = [
        "symbolic-syntax.diag",
        "symbolic-profile.diag",
        "narrative-syntax.diag",
        "narrative-profile.diag",
    ]
    .into_iter()
    .map(|name| {
        let value = parse_diagnostic(&read(&path, name)).unwrap();
        let document = PresentationDocument::from_value(&value).unwrap();
        let kind = match document {
            PresentationDocument::Syntax(_) => "syntax",
            PresentationDocument::Profile(_) => "profile",
        };
        validate_root(&value, kind).unwrap();
        let bytes = document.to_cbor(false).unwrap();
        assert_eq!(
            encode_deterministic(&decode_deterministic(&bytes).unwrap()).unwrap(),
            bytes
        );
        assert_eq!(PresentationDocument::from_cbor(&bytes).unwrap(), document);
        document
    })
    .collect::<Vec<_>>();

    let mut registry = ProfileRegistry::new();
    for document in &documents {
        if let PresentationDocument::Syntax(syntax) = document {
            registry.register_syntax(syntax.clone()).unwrap();
        }
    }
    for document in &documents {
        if let PresentationDocument::Profile(profile) = document {
            registry.register_profile(profile.clone()).unwrap();
        }
    }
    let policy = parse_policy_source(policy_source, "policy.bhcp")
        .unwrap()
        .documents
        .into_iter()
        .next()
        .unwrap();
    registry.register_policy(policy).unwrap();
    (registry, documents)
}

#[test]
fn substantially_different_checked_in_layouts_preserve_governed_semantic_identity() {
    let path = directory();
    let policy = read(&path, "policy.bhcp");
    let (registry, documents) = registry(&policy);
    let symbolic_source = read(&path, "symbolic.bhcp");
    let narrative_source = read(&path, "narrative.bhcp");

    let symbolic = compile_source_bytes_with_profile_registry(
        symbolic_source.as_bytes(),
        "symbolic.bhcp",
        &registry,
    )
    .unwrap();
    let narrative = compile_source_bytes_with_profile_registry(
        narrative_source.as_bytes(),
        "narrative.bhcp",
        &registry,
    )
    .unwrap();

    assert_eq!(symbolic.semantic_hash, narrative.semantic_hash);
    assert_eq!(symbolic.ir.semantic_id, narrative.ir.semantic_id);
    assert_ne!(symbolic.ir_bytes, narrative.ir_bytes);
    assert_ne!(symbolic.ir_hash, narrative.ir_hash);
    assert_ne!(symbolic.ast_bytes, narrative.ast_bytes);
    assert_ne!(symbolic.ast_hash, narrative.ast_hash);
    assert_eq!(symbolic.ast.profile, SYMBOLIC_PROFILE);
    assert_eq!(narrative.ast.profile, NARRATIVE_PROFILE);
    assert_ne!(symbolic.ast.profile, narrative.ast.profile);
    assert_eq!(symbolic.effective_policy, narrative.effective_policy);
    assert!(symbolic.ir.goals[0].policy_decision.is_some());

    for compilation in [&symbolic, &narrative] {
        let ast = decode_deterministic(&compilation.ast_bytes).unwrap();
        validate_root(&ast, "canonical-ast").unwrap();
        assert_eq!(encode_deterministic(&ast).unwrap(), compilation.ast_bytes);
        let ir = decode_deterministic(&compilation.ir_bytes).unwrap();
        validate_root(&ir, "semantic-ir").unwrap();
        assert_eq!(encode_deterministic(&ir).unwrap(), compilation.ir_bytes);
    }

    let symbolic_resolution = registry
        .resolve(SYMBOLIC_PROFILE, Default::default())
        .unwrap();
    let narrative_resolution = registry
        .resolve(NARRATIVE_PROFILE, Default::default())
        .unwrap();
    assert_eq!(
        symbolic_resolution.policy_overlays,
        ["example/policy.layout@0"]
    );
    assert_eq!(
        symbolic_resolution.effective_policy,
        narrative_resolution.effective_policy
    );

    let artifact_ids = documents
        .iter()
        .map(|document| artifact_hash(&document.to_value(false)).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(artifact_ids.len(), 4);
    assert!(artifact_ids.iter().enumerate().all(|(index, id)| {
        artifact_ids[index + 1..]
            .iter()
            .all(|candidate| candidate != id)
    }));
}

#[test]
fn formatting_comments_labels_policy_and_diagnostics_pin_the_identity_boundary() {
    let path = directory();
    let policy = read(&path, "policy.bhcp");
    let (baseline_registry, _) = registry(&policy);
    let cases = [
        ("symbolic.bhcp", "symbolic.formatted.bhcp"),
        ("narrative.bhcp", "narrative.formatted.bhcp"),
    ];

    for (source_name, expected_name) in cases {
        let source = read(&path, source_name);
        let mut expected = read(&path, expected_name);
        if source_name == "narrative.bhcp" {
            assert_eq!(expected.pop(), Some('\n'));
        }
        let formatted = format_source_bytes_with_profile_registry(
            source.as_bytes(),
            source_name,
            &baseline_registry,
        )
        .unwrap();
        assert_eq!(formatted, expected, "{source_name}");
        assert_eq!(
            formatted.ends_with('\n'),
            source_name == "symbolic.bhcp",
            "{source_name}",
        );
        assert_eq!(
            format_source_bytes_with_profile_registry(
                formatted.as_bytes(),
                expected_name,
                &baseline_registry,
            )
            .unwrap(),
            formatted,
            "{source_name}",
        );
        let before = compile_source_bytes_with_profile_registry(
            source.as_bytes(),
            source_name,
            &baseline_registry,
        )
        .unwrap();
        let after = compile_source_bytes_with_profile_registry(
            formatted.as_bytes(),
            expected_name,
            &baseline_registry,
        )
        .unwrap();
        assert_eq!(before.semantic_hash, after.semantic_hash, "{source_name}");
        assert_ne!(before.ast_hash, after.ast_hash, "{source_name}");
    }

    let symbolic_source = read(&path, "symbolic.bhcp");
    let relabeled = symbolic_source
        .replace("/* symbolic layout */", "/* alternate explanation */")
        .replace("name supplied", "input is present")
        .replace("greeting returned", "output is shaped")
        .replace("machine check", "static expression check");
    let baseline = compile_source_bytes_with_profile_registry(
        symbolic_source.as_bytes(),
        "symbolic.bhcp",
        &baseline_registry,
    )
    .unwrap();
    let relabeled = compile_source_bytes_with_profile_registry(
        relabeled.as_bytes(),
        "symbolic-relabeled.bhcp",
        &baseline_registry,
    )
    .unwrap();
    assert_eq!(baseline.semantic_hash, relabeled.semantic_hash);
    assert_ne!(baseline.ir_bytes, relabeled.ir_bytes);
    assert_ne!(baseline.ir_hash, relabeled.ir_hash);
    assert_ne!(baseline.ast_hash, relabeled.ast_hash);

    let changed_policy = policy.replace(
        "example/requirement.review@0",
        "example/requirement.audit@0",
    );
    let (changed_registry, _) = registry(&changed_policy);
    let baseline_policy = compile_source_bytes_with_profile_registry(
        symbolic_source.as_bytes(),
        "symbolic.bhcp",
        &baseline_registry,
    )
    .unwrap();
    let changed = compile_source_bytes_with_profile_registry(
        symbolic_source.as_bytes(),
        "symbolic.bhcp",
        &changed_registry,
    )
    .unwrap();
    assert_eq!(baseline_policy.ast_hash, changed.ast_hash);
    assert_ne!(baseline_policy.semantic_hash, changed.semantic_hash);

    let symbolic_error = compile_source_bytes_with_profile_registry(
        "#!bhcp-profile example/profile.layout-symbolic@0\n§aim example/Layout@0^§arg name Text?~"
            .as_bytes(),
        "symbolic-invalid.bhcp",
        &baseline_registry,
    )
    .unwrap_err();
    let narrative_error = compile_source_bytes_with_profile_registry(
        "#!bhcp-profile example/profile.layout-narrative@0\n§objective example/Layout@0 ^~ §accepts name Text$ ~^".as_bytes(),
        "narrative-invalid.bhcp",
        &baseline_registry,
    )
    .unwrap_err();
    assert_eq!(symbolic_error.code, narrative_error.code);
    assert_eq!(symbolic_error.code, "BHCP1001");
    assert_eq!(symbolic_error.message, narrative_error.message);
    assert_eq!(symbolic_error.message, "expected \":\", found \"Text\"");
    assert_ne!(symbolic_error.source, narrative_error.source);
    assert_ne!(symbolic_error.column, narrative_error.column);
}
