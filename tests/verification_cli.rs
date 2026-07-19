use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::schema::validate_root;
use bhcp::value::Value;

const CONTRACT: &str = r#"
§goal experiment/InSessionEvidence@0 {
    §input repository: Text;
    §output publicPassed: Bool;
    §output oraclePassed: Bool;
    §output policyPassed: Bool;

    §requires "pinned": repository == "subject@0";
    §ensures "public": publicPassed;
    §ensures "oracle": oraclePassed;
    §ensures "change policy": policyPassed;

    §allows "run registered adapters": bhcp-effect/process@0;

    §verify "public adapter": with experiment/verifier/public-rust@0 for "public";
    §verify "oracle adapter": with experiment/verifier/contextual-policy@0 for "oracle";
    §verify "change adapter": with experiment/verifier/change-policy@0 for "change policy";
}
"#;

static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(1);

struct Project {
    root: PathBuf,
    contract: PathBuf,
    candidate: PathBuf,
    subject: PathBuf,
}

impl Project {
    fn new(modes: [&str; 3]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "bhcp-verification-cli-{}-{}",
            std::process::id(),
            NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("tools")).unwrap();
        let verifier = root.join("tools/verifier");
        fs::copy(env!("CARGO_BIN_EXE_bhcp-verifier-fixture"), &verifier).unwrap();
        fs::set_permissions(
            &verifier,
            fs::metadata(env!("CARGO_BIN_EXE_bhcp-verifier-fixture"))
                .unwrap()
                .permissions(),
        )
        .unwrap();
        fs::write(
            root.join("bhcp-project.toml"),
            manifest(&[
                ("experiment/verifier/public-rust@0", modes[0]),
                ("experiment/verifier/contextual-policy@0", modes[1]),
                ("experiment/verifier/change-policy@0", modes[2]),
            ]),
        )
        .unwrap();
        let contract = root.join("contract.bhcp");
        fs::write(&contract, CONTRACT).unwrap();
        let candidate = root.join("candidate.cbor");
        fs::write(
            &candidate,
            encode_deterministic(&Value::map([
                (
                    "input",
                    Value::map([("repository", Value::Text("subject@0".to_owned()))]),
                ),
                (
                    "output",
                    Value::map([
                        ("publicPassed", Value::Bool(true)),
                        ("oraclePassed", Value::Bool(true)),
                        ("policyPassed", Value::Bool(true)),
                    ]),
                ),
            ]))
            .unwrap(),
        )
        .unwrap();
        let subject = root.join("subject.rs");
        fs::write(&subject, "candidate source\n").unwrap();
        Self {
            root,
            contract,
            candidate,
            subject,
        }
    }

    fn set_manifest(&self, adapters: &[(&str, &str)]) {
        fs::write(self.root.join("bhcp-project.toml"), manifest(adapters)).unwrap();
    }

    fn set_manifest_effect(&self, effect: &str) {
        fs::write(
            self.root.join("bhcp-project.toml"),
            manifest_with_effect(
                &[
                    ("experiment/verifier/public-rust@0", "accepted"),
                    ("experiment/verifier/contextual-policy@0", "accepted"),
                    ("experiment/verifier/change-policy@0", "accepted"),
                ],
                effect,
            ),
        )
        .unwrap();
    }

    fn verify(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_bhcp"))
            .args([
                "verify",
                self.contract.to_str().unwrap(),
                "experiment/InSessionEvidence@0",
                self.candidate.to_str().unwrap(),
                self.subject.to_str().unwrap(),
                "2026-07-19T18:00:00Z",
            ])
            .output()
            .unwrap()
    }
}

fn manifest(adapters: &[(&str, &str)]) -> String {
    manifest_with_effect(adapters, "bhcp-effect/process@0")
}

fn manifest_with_effect(adapters: &[(&str, &str)], effect: &str) -> String {
    adapters
        .iter()
        .map(|(symbol, mode)| {
            format!(
                r#"[[verifier_adapter]]
symbol = "{symbol}"
executable = "tools/verifier"
argv = ["{mode}"]
working_scope = "project"
input_media_type = "application/vnd.bhcp.verification-request+cbor"
output_media_type = "application/vnd.bhcp.verifier-result+cbor"
timeout_ms = 2000
allowed_effects = ["{effect}"]
evidence_kind = "static"
"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn verify_runs_public_oracle_and_change_policy_through_the_project_registry() {
    let project = Project::new(["accepted", "accepted", "accepted"]);
    let output = project.verify();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bundle = decode_deterministic(&output.stdout).unwrap();
    validate_root(&bundle, "evidence-bundle").unwrap();
    let Value::Map(statuses) = bundle.get("obligation_status").unwrap() else {
        panic!("obligation status must be a map")
    };
    assert!(
        statuses
            .iter()
            .all(|(_, status)| matches!(status, Value::Text(value) if value == "discharged"))
    );
    for verifier in [
        "experiment/verifier/public-rust@0",
        "experiment/verifier/contextual-policy@0",
        "experiment/verifier/change-policy@0",
    ] {
        assert!(contains_text(&bundle, verifier));
    }
}

#[test]
fn verify_keeps_rejection_unavailability_and_malicious_output_distinct() {
    let rejected = Project::new(["accepted", "rejected", "accepted"]).verify();
    assert_eq!(rejected.status.code(), Some(3));
    let rejected_bundle = decode_deterministic(&rejected.stdout).unwrap();
    assert!(contains_text(&rejected_bundle, "refuted"));

    let unavailable = Project::new(["accepted", "accepted", "accepted"]);
    unavailable.set_manifest(&[
        ("experiment/verifier/public-rust@0", "accepted"),
        ("experiment/verifier/contextual-policy-v2@0", "accepted"),
        ("experiment/verifier/change-policy@0", "accepted"),
    ]);
    let unavailable = unavailable.verify();
    assert_eq!(unavailable.status.code(), Some(4));
    let unavailable_bundle = decode_deterministic(&unavailable.stdout).unwrap();
    assert!(contains_text(
        &unavailable_bundle,
        "bhcp.reason/verifier-unregistered@0"
    ));

    let malicious = Project::new(["accepted", "malformed", "accepted"]).verify();
    assert_eq!(malicious.status.code(), Some(5));
    let malicious_bundle = decode_deterministic(&malicious.stdout).unwrap();
    assert!(contains_text(
        &malicious_bundle,
        "bhcp.evidence-gap/verifier-fault@0"
    ));
    assert!(contains_text(
        &malicious_bundle,
        "bhcp.fault/adapter-malformed-output@0"
    ));
}

#[test]
fn verify_does_not_let_the_local_manifest_self_authorize_adapter_effects() {
    let project = Project::new(["accepted", "accepted", "accepted"]);
    project.set_manifest_effect("bhcp-effect/fs.read@0");
    let output = project.verify();
    assert_eq!(output.status.code(), Some(5));
    let bundle = decode_deterministic(&output.stdout).unwrap();
    assert!(contains_text(&bundle, "bhcp.fault/adapter-boundary@0"));
}

fn contains_text(value: &Value, needle: &str) -> bool {
    match value {
        Value::Text(value) => value == needle,
        Value::Array(values) => values.iter().any(|value| contains_text(value, needle)),
        Value::Map(entries) => entries
            .iter()
            .any(|(_, value)| contains_text(value, needle)),
        Value::Tag(_, value) => contains_text(value, needle),
        _ => false,
    }
}
