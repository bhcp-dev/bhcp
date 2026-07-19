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
        let manifest = [
            ("experiment/verifier/public-rust@0", modes[0]),
            ("experiment/verifier/contextual-policy@0", modes[1]),
            ("experiment/verifier/change-policy@0", modes[2]),
        ]
        .into_iter()
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
allowed_effects = ["bhcp-effect/process@0"]
evidence_kind = "static"
"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
        fs::write(root.join("bhcp-project.toml"), manifest).unwrap();
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

fn contains_text(value: &Value, needle: &str) -> bool {
    match value {
        Value::Text(value) => value == needle,
        Value::Array(values) => values.iter().any(|value| contains_text(value, needle)),
        Value::Map(entries) => entries.iter().any(|(_, value)| contains_text(value, needle)),
        Value::Tag(_, value) => contains_text(value, needle),
        _ => false,
    }
}
