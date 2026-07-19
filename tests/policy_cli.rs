use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::pipeline::parse_policy_source;
use bhcp::policy::PolicyDocument;
use bhcp::schema::validate_root;
use bhcp::value::Value;

const ORGANIZATION: &str = r#"
§policy example/policy.org@0 {
  layer organization;
  rule a-requirement: requirement add {
    requirement: example/requirement.lint@0
  } nonwaivable;
  rule b-capability: capability narrow {
    effect: bhcp-effect/fs.read@0,
    scope: { goals: [example/goal.a@0, example/goal.b@0] }
  } nonwaivable;
  rule c-limit: limit tighten {
    dimension: example/limit.memory@0,
    unit: example/unit.byte@0,
    maximum: ["integer", 10]
  } nonwaivable;
  rule d-mode: type-mode strengthen gradual nonwaivable;
}
"#;

const REPOSITORY: &str = r#"
§policy example/policy.repo@0 {
  layer repository;
  rule a-capability: capability narrow {
    effect: bhcp-effect/fs.read@0,
    scope: { goals: [example/goal.b@0] }
  } nonwaivable;
  rule b-limit: limit tighten {
    dimension: example/limit.memory@0,
    unit: example/unit.byte@0,
    maximum: ["integer", 5]
  } nonwaivable;
  rule c-mode: type-mode strengthen strict nonwaivable;
}
"#;

const LOOSENING_REPOSITORY: &str = r#"
§policy example/policy.repo@0 {
  layer repository;
  rule a-limit: limit tighten {
    dimension: example/limit.memory@0,
    unit: example/unit.byte@0,
    maximum: ["integer", 20]
  } nonwaivable;
  rule b-mode: type-mode strengthen strict nonwaivable;
}
"#;

static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(1);

struct FixtureSet {
    root: PathBuf,
    organization: PathBuf,
    repository: PathBuf,
    loosening: PathBuf,
}

impl FixtureSet {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "bhcp-policy-cli-{}-{}",
            std::process::id(),
            NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let organization = root.join("organization.bhcp");
        let repository = root.join("repository.bhcp");
        let loosening = root.join("loosening.bhcp");
        fs::write(&organization, ORGANIZATION).unwrap();
        fs::write(&repository, REPOSITORY).unwrap();
        fs::write(&loosening, LOOSENING_REPOSITORY).unwrap();
        Self {
            root,
            organization,
            repository,
            loosening,
        }
    }

    fn source_cbor(&self, source: &str, name: &str) -> PathBuf {
        let parsed = parse_policy_source(source, name).unwrap();
        assert_eq!(parsed.documents.len(), 1);
        let bytes = PolicyDocument::Source(parsed.documents[0].clone())
            .to_cbor(false)
            .unwrap();
        let path = self.root.join(format!("{name}.cbor"));
        fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for FixtureSet {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn run(arguments: &[&Path]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bhcp"));
    for argument in arguments {
        command.arg(argument);
    }
    command.output().unwrap()
}

fn policy_command(action: &str, files: &[&Path]) -> Output {
    let policy = Path::new("policy");
    let action = Path::new(action);
    let mut arguments = vec![policy, action];
    arguments.extend(files.iter().copied());
    run(&arguments)
}

#[test]
fn compose_has_source_cbor_parity_and_deterministic_validated_bytes() {
    let fixtures = FixtureSet::new();
    let source = policy_command("compose", &[&fixtures.organization, &fixtures.repository]);
    assert!(
        source.status.success(),
        "{}",
        String::from_utf8_lossy(&source.stderr)
    );
    assert!(source.stderr.is_empty());

    let organization_cbor = fixtures.source_cbor(ORGANIZATION, "organization");
    let repository_cbor = fixtures.source_cbor(REPOSITORY, "repository");
    let cbor = policy_command("compose", &[&organization_cbor, &repository_cbor]);
    assert!(
        cbor.status.success(),
        "{}",
        String::from_utf8_lossy(&cbor.stderr)
    );
    assert_eq!(source.stdout, cbor.stdout);

    let repeated = policy_command("compose", &[&fixtures.organization, &fixtures.repository]);
    assert_eq!(source.stdout, repeated.stdout);
    let value = decode_deterministic(&source.stdout).unwrap();
    assert_eq!(value.kind(), Some("policy"));
    assert_eq!(
        value.get("form"),
        Some(&Value::Text("effective".to_owned()))
    );
    validate_root(&value, "policy").unwrap();
    let document = PolicyDocument::from_cbor(&source.stdout).unwrap();
    assert_eq!(document.to_cbor(true).unwrap(), source.stdout);
}

#[test]
fn inspect_renders_source_and_effective_layers_rules_provenance_and_identities() {
    let fixtures = FixtureSet::new();
    let source = policy_command("inspect", &[&fixtures.organization]);
    assert!(source.status.success());
    let source_text = String::from_utf8(source.stdout).unwrap();
    assert!(source_text.contains("policy source example/policy.org@0 layer organization"));
    assert!(source_text.contains("[a-requirement] requirement add nonwaivable"));
    assert!(source_text.contains("requirement: \"example/requirement.lint@0\""));

    let composed = policy_command("compose", &[&fixtures.organization, &fixtures.repository]);
    assert!(composed.status.success());
    let effective_path = fixtures.root.join("effective.cbor");
    fs::write(&effective_path, composed.stdout).unwrap();
    let effective = policy_command("inspect", &[&effective_path]);
    assert!(effective.status.success());
    let effective = String::from_utf8(effective.stdout).unwrap();
    for expected in [
        "semantic_id bhcp.hash/sha3-512@0:",
        "artifact_id bhcp.hash/sha3-512@0:",
        "source-layer organization 1",
        "source example/policy.org@0",
        "source-layer repository 1",
        "source example/policy.repo@0",
        "effective requirements[0]",
        "effective capabilities[0]",
        "effective limits[0]",
        "effective capabilities[0] nonwaivable:",
        "effective type-mode strict",
        "provenance capability[0] <- example/policy.org@0#b-capability, example/policy.repo@0#a-capability",
    ] {
        assert!(
            effective.contains(expected),
            "inspection omitted {expected}"
        );
    }
}

#[test]
fn wrong_order_unsupported_features_invalid_artifacts_and_weakening_fail_without_stdout() {
    let fixtures = FixtureSet::new();

    let wrong_order = policy_command("compose", &[&fixtures.repository, &fixtures.organization]);
    assert_failure(&wrong_order, "BHCP8110", "organization-to-user order");

    let mut unsupported = PolicyDocument::Source(
        parse_policy_source(ORGANIZATION, "unsupported.bhcp")
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
    let unsupported_path = fixtures.root.join("unsupported.cbor");
    fs::write(
        &unsupported_path,
        encode_deterministic(&unsupported).unwrap(),
    )
    .unwrap();
    let unsupported = policy_command("inspect", &[&unsupported_path]);
    assert_failure(&unsupported, "BHCP8001", "unsupported policy feature");

    let invalid_path = fixtures.root.join("invalid.cbor");
    fs::write(
        &invalid_path,
        encode_deterministic(&Value::map([("kind", Value::Text("policy".to_owned()))])).unwrap(),
    )
    .unwrap();
    let invalid = policy_command("inspect", &[&invalid_path]);
    assert_failure(&invalid, "BHCP8001", "form");

    let weakening = policy_command("compose", &[&fixtures.organization, &fixtures.loosening]);
    assert_failure(&weakening, "BHCP8102", "loosens limit");
}

fn assert_failure(output: &Output, code: &str, message: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "failure emitted partial stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(code), "missing {code}: {stderr}");
    assert!(stderr.contains(message), "missing {message}: {stderr}");
}
