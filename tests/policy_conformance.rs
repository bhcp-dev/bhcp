use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::model::HashId;
use bhcp::pipeline::parse_policy_source;
use bhcp::policy::{
    EffectivePolicyDocument, PolicyDocument, PolicyLayer, PolicyWeakeningAttempt,
    SourceRuleIdentity, compose_policies, reject_policy_weakening,
};
use bhcp::schema::validate_root;
use bhcp::value::Value;

static NEXT_TEMP: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Debug)]
struct Case {
    name: String,
    mode: String,
    inputs: Vec<String>,
    expected: String,
    artifact: String,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_root() -> PathBuf {
    root().join("conformance/v0/policy")
}

fn cases() -> Vec<Case> {
    let manifest = fs::read_to_string(fixture_root().join("manifest.txt"))
        .expect("policy conformance manifest must exist");
    let mut names = BTreeSet::new();
    manifest
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<_> = line.split('|').map(str::trim).collect();
            assert_eq!(fields.len(), 5, "invalid policy conformance row: {line}");
            assert!(names.insert(fields[0].to_owned()), "duplicate case name");
            Case {
                name: fields[0].to_owned(),
                mode: fields[1].to_owned(),
                inputs: fields[2]
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty() && *value != "-")
                    .map(str::to_owned)
                    .collect(),
                expected: fields[3].to_owned(),
                artifact: fields[4].to_owned(),
            }
        })
        .collect()
}

fn case_map() -> BTreeMap<String, Case> {
    cases()
        .into_iter()
        .map(|case| (case.name.clone(), case))
        .collect()
}

fn compose(case: &Case) -> bhcp::diagnostic::Result<EffectivePolicyDocument> {
    let mut documents = Vec::new();
    for input in &case.inputs {
        let path = fixture_root().join(input);
        let source = fs::read_to_string(&path).unwrap();
        documents.extend(
            parse_policy_source(&source, path.to_str().unwrap())
                .unwrap()
                .documents,
        );
    }
    compose_policies(&documents, Default::default())
}

fn identity(id: &HashId) -> String {
    let mut digest = String::with_capacity(id.digest.len() * 2);
    for byte in &id.digest {
        write!(digest, "{byte:02x}").unwrap();
    }
    format!("{}:{digest}", id.algorithm)
}

fn policy_command(action: &str, inputs: &[PathBuf]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bhcp"));
    command.arg("policy").arg(action);
    command.args(inputs);
    command.output().unwrap()
}

fn temporary_directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "bhcp-policy-conformance-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn expected_cases() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "baseline",
        "equivalent-decomposition",
        "meaningful-change",
        "capability-widening",
        "limit-loosening",
        "type-mode-weakening",
        "incompatible-limit-units",
        "missing-inheritance",
        "remove-requirement",
        "remove-evidence",
        "allow-denied-effect",
        "unsupported-feature",
    ])
}

#[test]
fn manifest_covers_the_complete_no_waiver_policy_slice() {
    let cases = case_map();
    assert_eq!(
        cases.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        expected_cases()
    );
    assert_eq!(cases["baseline"].inputs.len(), 4);
    assert!(
        ["organization", "team", "repository", "user"]
            .iter()
            .zip(&cases["baseline"].inputs)
            .all(|(layer, input)| input == &format!("layers/{layer}.bhcp"))
    );
    for case in cases.values() {
        assert!(
            matches!(
                case.mode.as_str(),
                "valid" | "compose-error" | "weakening-error" | "unsupported-feature"
            ),
            "unknown conformance mode for {}",
            case.name
        );
    }
}

#[test]
fn valid_artifacts_are_pinned_deterministic_and_schema_round_trip() {
    let cases = case_map();
    let mut results = BTreeMap::new();
    for case in cases.values().filter(|case| case.mode == "valid") {
        let first = compose(case).unwrap();
        let second = compose(case).unwrap();
        let bytes = PolicyDocument::Effective(first.clone())
            .to_cbor(true)
            .unwrap();
        assert_eq!(
            bytes,
            PolicyDocument::Effective(second).to_cbor(true).unwrap()
        );
        assert_eq!(
            identity(first.header.semantic_id.as_ref().unwrap()),
            case.expected,
            "semantic identity drift in {}",
            case.name
        );
        assert_eq!(
            identity(first.header.artifact_id.as_ref().unwrap()),
            case.artifact,
            "artifact identity drift in {}",
            case.name
        );
        let value = decode_deterministic(&bytes).unwrap();
        validate_root(&value, "policy").unwrap();
        assert_eq!(
            PolicyDocument::from_cbor(&bytes).unwrap(),
            PolicyDocument::Effective(first.clone())
        );
        results.insert(case.name.as_str(), first);
    }
    assert_eq!(
        results["baseline"].header.semantic_id,
        results["equivalent-decomposition"].header.semantic_id
    );
    assert_ne!(
        results["baseline"].header.artifact_id,
        results["equivalent-decomposition"].header.artifact_id
    );
    assert_ne!(
        results["baseline"].header.semantic_id,
        results["meaningful-change"].header.semantic_id
    );
}

#[test]
fn composition_and_typed_weakening_diagnostics_match_the_manifest() {
    let cases = case_map();
    for case in cases.values().filter(|case| case.mode == "compose-error") {
        let diagnostic = compose(case).unwrap_err();
        assert_eq!(diagnostic.code, case.expected, "case {}", case.name);
    }

    let earlier = SourceRuleIdentity {
        policy: "example/policy.organization@0".to_owned(),
        rule: "a-baseline".to_owned(),
    };
    for case in cases.values().filter(|case| case.mode == "weakening-error") {
        let common = (
            PolicyLayer::Repository,
            "example/policy.repository@0".to_owned(),
            format!("attempt-{}", case.name),
            earlier.clone(),
            PolicyLayer::Organization,
        );
        let attempt = match case.name.as_str() {
            "remove-requirement" => PolicyWeakeningAttempt::RemoveRequirement {
                layer: common.0,
                policy: common.1,
                rule: common.2,
                requirement: "example/requirement.audit@0".to_owned(),
                earlier: common.3,
                earlier_layer: common.4,
            },
            "remove-evidence" => PolicyWeakeningAttempt::RemoveEvidence {
                layer: common.0,
                policy: common.1,
                rule: common.2,
                obligation: "example/obligation.review@0".to_owned(),
                earlier: common.3,
                earlier_layer: common.4,
            },
            "allow-denied-effect" => PolicyWeakeningAttempt::AllowDeniedEffect {
                layer: common.0,
                policy: common.1,
                rule: common.2,
                effect: "bhcp-effect/network@0".to_owned(),
                earlier: common.3,
                earlier_layer: common.4,
            },
            _ => panic!("unknown typed weakening case {}", case.name),
        };
        assert_eq!(
            reject_policy_weakening(attempt).unwrap_err().code,
            case.expected,
            "case {}",
            case.name
        );
    }
}

#[test]
fn cli_source_and_source_cbor_paths_are_identical_and_fail_atomically() {
    let cases = case_map();
    let baseline = &cases["baseline"];
    let source_paths: Vec<_> = baseline
        .inputs
        .iter()
        .map(|input| fixture_root().join(input))
        .collect();
    let source = policy_command("compose", &source_paths);
    assert!(
        source.status.success(),
        "{}",
        String::from_utf8_lossy(&source.stderr)
    );

    let temp = temporary_directory();
    let mut cbor_paths = Vec::new();
    for (index, path) in source_paths.iter().enumerate() {
        let parsed =
            parse_policy_source(&fs::read_to_string(path).unwrap(), path.to_str().unwrap())
                .unwrap();
        assert_eq!(parsed.documents.len(), 1);
        let output = temp.join(format!("source-{index}.cbor"));
        fs::write(
            &output,
            PolicyDocument::Source(parsed.documents[0].clone())
                .to_cbor(false)
                .unwrap(),
        )
        .unwrap();
        cbor_paths.push(output);
    }
    let cbor = policy_command("compose", &cbor_paths);
    assert!(
        cbor.status.success(),
        "{}",
        String::from_utf8_lossy(&cbor.stderr)
    );
    assert_eq!(source.stdout, cbor.stdout);
    assert!(source.stderr.is_empty());
    assert!(cbor.stderr.is_empty());
    validate_root(&decode_deterministic(&source.stdout).unwrap(), "policy").unwrap();

    let effective = temp.join("effective.cbor");
    fs::write(&effective, &source.stdout).unwrap();
    let inspected = policy_command("inspect", &[effective]);
    assert!(inspected.status.success());
    let outline = String::from_utf8(inspected.stdout).unwrap();
    for claim in [
        "layer organization",
        "layer team",
        "layer repository",
        "layer user",
        "requirement[",
        "evidence[",
        "prohibition[",
        "capability[",
        "limit[",
        "type-mode",
        "semantic_id",
        "artifact_id",
    ] {
        assert!(outline.contains(claim), "inspection omitted {claim}");
    }

    let mut unsupported = PolicyDocument::Source(
        parse_policy_source(
            &fs::read_to_string(&source_paths[0]).unwrap(),
            source_paths[0].to_str().unwrap(),
        )
        .unwrap()
        .documents
        .remove(0),
    )
    .to_value(false);
    let Value::Map(entries) = &mut unsupported else {
        unreachable!()
    };
    entries
        .iter_mut()
        .find(|(key, _)| key == "features")
        .unwrap()
        .1 = Value::Array(vec![Value::Text(
        "example/feature.unsupported@0".to_owned(),
    )]);
    let unsupported_path = temp.join("unsupported.cbor");
    fs::write(
        &unsupported_path,
        encode_deterministic(&unsupported).unwrap(),
    )
    .unwrap();
    let unsupported = policy_command("inspect", &[unsupported_path]);
    assert!(!unsupported.status.success());
    assert!(unsupported.stdout.is_empty());
    let error = String::from_utf8(unsupported.stderr).unwrap();
    assert!(error.contains(&cases["unsupported-feature"].expected));
    assert!(error.contains("unsupported policy feature"));

    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn every_manifest_input_exists_and_is_canonical_source() {
    for case in cases() {
        for input in case.inputs {
            let path = fixture_root().join(input);
            assert!(path.is_file(), "missing input {}", path.display());
            let source = fs::read_to_string(&path).unwrap();
            let parsed = parse_policy_source(&source, path.to_str().unwrap()).unwrap();
            assert!(!parsed.documents.is_empty());
            for document in parsed.documents {
                PolicyDocument::Source(document).validate().unwrap();
            }
        }
    }
}
