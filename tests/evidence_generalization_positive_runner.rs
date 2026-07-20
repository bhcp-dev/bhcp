use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use bhcp::cbor::decode_deterministic;
use bhcp::value::Value;

const TASKS: [&str; 4] = [
    "atomic-batch",
    "tenant-policy",
    "contextual-policy",
    "in-session-evidence",
];

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fresh_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bhcp-positive-use-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("codex-home")).unwrap();
    fs::write(
        root.join("codex-home/auth.json"),
        br#"{"auth_mode":"chatgpt","OPENAI_API_KEY":""}"#,
    )
    .unwrap();
    root
}

fn rustup_installation() -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".cargo"));
    let rustup_home = std::env::var_os("RUSTUP_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".rustup"));
    let rustup = cargo_home.join("bin/rustup");
    let selected = Command::new(&rustup)
        .args(["which", "cargo", "--toolchain", "1.97.1"])
        .output()
        .unwrap();
    assert!(selected.status.success());
    let toolchain_bin = PathBuf::from(String::from_utf8(selected.stdout).unwrap().trim())
        .parent()
        .unwrap()
        .to_owned();
    (cargo_home, rustup_home, rustup, toolchain_bin)
}

fn runner(mode: &str, root: &Path) -> Command {
    let (cargo_home, rustup_home, rustup, toolchain_bin) = rustup_installation();
    let mut command = Command::new(env!("CARGO_BIN_EXE_evidence_generalization_positive"));
    command.args([
        mode,
        env!("CARGO_BIN_EXE_bhcp-experiment-fake-agent"),
        env!("CARGO_BIN_EXE_bhcp-experiment-fake-codex"),
        root.join("codex-home").to_str().unwrap(),
        cargo_home.to_str().unwrap(),
        rustup_home.to_str().unwrap(),
        env!("CARGO_BIN_EXE_bhcp"),
        rustup.to_str().unwrap(),
        toolchain_bin.to_str().unwrap(),
        repository().parent().unwrap().to_str().unwrap(),
        env!("CARGO_BIN_EXE_evidence_generalization_adapter"),
        root.join("prepared").to_str().unwrap(),
        root.join("scratch").to_str().unwrap(),
    ]);
    for name in [
        "OPENAI_API_KEY",
        "CODEX_API_KEY",
        "AZURE_OPENAI_API_KEY",
        "OPENAI_BASE_URL",
        "OPENAI_API_BASE",
        "AZURE_OPENAI_ENDPOINT",
    ] {
        command.env_remove(name);
    }
    command
}

fn run(mode: &str, root: &Path) -> Output {
    runner(mode, root).output().unwrap()
}

#[test]
fn preparation_stages_all_tasks_and_conservative_adapters() {
    let root = fresh_root("prepare");
    let prepared = run("prepare", &root);
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    let preparation = String::from_utf8(prepared.stdout).unwrap();
    assert!(preparation.contains("version=bhcp-evidence-generalization-positive@0"));
    assert!(preparation.contains("prepared_tasks=4"));
    for task in TASKS {
        let task_root = root.join("prepared").join(task);
        for relative in [
            "TASK.md",
            "contract.bhcp",
            "contract.semantic-id",
            "bhcp-project.toml",
            "candidate.cbor",
            "REGISTRY_COMMAND.txt",
            "POSITIVE_USE_PROMPT.md",
            "subject/src/lib.rs",
            "subject/tools/evidence-generalization-adapter",
            "subject/.agents/skills/interpret-bhcp-contract/SKILL.md",
            "oracle/tests/invariants.rs",
        ] {
            assert!(task_root.join(relative).is_file(), "{task}/{relative}");
        }
        let manifest = fs::read_to_string(task_root.join("bhcp-project.toml")).unwrap();
        assert_eq!(manifest.matches("[[verifier_adapter]]").count(), 3);
        let starter = fs::read(task_root.join("subject/src/lib.rs")).unwrap();
        let withheld = fs::read(
            repository()
                .join("experiments/evidence-generalization/withheld")
                .join(format!("{task}.rs")),
        )
        .unwrap();
        assert_ne!(starter, withheld, "withheld solution leaked into starter");

        let rejected = verify(&task_root);
        assert_eq!(
            rejected.status.code(),
            Some(3),
            "starter did not deterministically reject for {task}: stdout={} stderr={}",
            String::from_utf8_lossy(&rejected.stdout),
            String::from_utf8_lossy(&rejected.stderr),
        );
        fs::write(task_root.join("subject/src/lib.rs"), withheld).unwrap();
        let accepted = verify(&task_root);
        assert!(
            accepted.status.success(),
            "{}: {}",
            task,
            String::from_utf8_lossy(&accepted.stderr)
        );
        let bundle = decode_deterministic(&accepted.stdout).unwrap();
        let Value::Map(statuses) = bundle.get("obligation_status").unwrap() else {
            panic!("obligation status is not a map")
        };
        assert!(!statuses.is_empty());
        assert!(
            statuses
                .iter()
                .all(|(_, status)| status == &Value::Text("discharged".to_owned()))
        );
        let Value::Array(items) = bundle.get("items").unwrap() else {
            panic!("evidence items are not an array")
        };
        assert_eq!(
            items
                .iter()
                .filter(|item| {
                    item.get("provenance")
                        .and_then(|provenance| provenance.get("source"))
                        .is_some()
                })
                .count(),
            3
        );
    }
    fs::remove_dir_all(root).unwrap();
}

fn verify(task_root: &Path) -> Output {
    let command = fs::read_to_string(task_root.join("REGISTRY_COMMAND.txt")).unwrap();
    let first = command.lines().next().unwrap();
    let fields = first.split_ascii_whitespace().collect::<Vec<_>>();
    assert_eq!(fields[0], "bhcp");
    assert_eq!(fields[1], "verify");
    Command::new(env!("CARGO_BIN_EXE_bhcp"))
        .current_dir(task_root.join("subject"))
        .args(&fields[1..fields.len() - 2])
        .output()
        .unwrap()
}

#[test]
fn billing_preflight_rejects_api_configuration_and_non_entitlement_auth() {
    let api_root = fresh_root("api");
    let rejected = runner("prepare", &api_root)
        .env("OPENAI_API_KEY", "forbidden-test-value")
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("forbids OPENAI_API_KEY"));
    fs::remove_dir_all(api_root).unwrap();

    let auth_root = fresh_root("auth");
    fs::write(
        auth_root.join("codex-home/auth.json"),
        br#"{"auth_mode":"api","OPENAI_API_KEY":""}"#,
    )
    .unwrap();
    let rejected = run("prepare", &auth_root);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("requires ChatGPT entitlement auth")
    );
    fs::remove_dir_all(auth_root).unwrap();
}

#[test]
fn checked_in_registration_names_every_frozen_input_and_analysis_rule() {
    let root = repository();
    let prose = fs::read_to_string(
        root.join("experiments/evidence-generalization/positive-use-registration.md"),
    )
    .unwrap();
    let machine = fs::read_to_string(
        root.join("experiments/evidence-generalization/positive-use-registration.txt"),
    )
    .unwrap();
    for required in [
        "twelve sessions",
        "no replacement",
        "Clopper–Pearson",
        "Tukey hinges",
        "USD 0",
        "independent",
    ] {
        assert!(
            prose.contains(required),
            "missing registration rule: {required}"
        );
    }
    for required in [
        "version|bhcp-evidence-generalization-positive-registration@0",
        "sessions|12",
        "per-session-minutes|15",
        "concurrency|1",
        "incremental-usd|0",
        "analysis|positive-use=clopper-pearson-95",
        "resources|median+iqr|tukey-hinges",
        "no-replacement|true",
    ] {
        assert!(
            machine.contains(required),
            "missing machine pin: {required}"
        );
    }
    for task in TASKS {
        assert!(machine.contains(&format!("task|{task}|")));
        for seed in ["seed-01", "seed-02", "seed-03"] {
            assert!(machine.contains(&format!("session|{task}|{seed}|")));
        }
    }
    assert!(!machine.contains("pending-freeze"));
    assert_eq!(
        machine
            .lines()
            .filter(|line| line.starts_with("session|"))
            .count(),
        12
    );
}

#[test]
fn checked_in_null_result_preserves_and_replays_every_registered_session() {
    let repository = repository();
    let results = repository.join("experiments/evidence-generalization/positive-use-results");
    assert!(!results.join("STOPPED.md").exists());
    assert_eq!(
        fs::read_to_string(results.join("AUTHORITY.txt")).unwrap(),
        "git-head=b9a814b7f69b6c3849109af08f58f38da83d836a\nowner-attestation-merge=c327d80308dbf6010321ca05ef498493b04350e7\nauth-mode=chatgpt\nincremental-usd=0\nconcurrency=1\n"
    );
    let ledger = fs::read_to_string(results.join("RESULTS.txt")).unwrap();
    let sessions = ledger
        .lines()
        .filter(|line| line.starts_with("session|"))
        .collect::<Vec<_>>();
    assert_eq!(sessions.len(), 12);
    let registration = fs::read_to_string(
        repository.join("experiments/evidence-generalization/positive-use-registration.txt"),
    )
    .unwrap();
    for task in TASKS {
        for seed in ["seed-01", "seed-02", "seed-03"] {
            let id = format!("{task}-{seed}");
            let ledger_prefix = format!(
                "session|{task}|{seed}|positive-use=false|registered-accepted=false|independent-accepted=false|accepted=false|claimed=false|calibrated=true|excluded=false|"
            );
            assert!(sessions.iter().any(|line| line.starts_with(&ledger_prefix)));
            let registered = registration
                .lines()
                .find(|line| line.starts_with(&format!("session|{task}|{seed}|")))
                .unwrap();
            let fields = registered.split('|').collect::<Vec<_>>();
            assert_eq!(fields.len(), 6);

            let session = results.join(id);
            assert_eq!(
                fs::metadata(session.join("candidate.patch")).unwrap().len(),
                0
            );
            assert!(!session.join("evidence.cbor").exists());
            assert_eq!(
                fs::read_to_string(session.join("EVIDENCE.txt")).unwrap(),
                "positive-use=false\nparseable=false\nsubject-bound=false\nadapter-items=0\nregistered-accepted=false\nindependent-accepted=false\nin-session-accepted=false\nclaimed-success=false\ncalibrated=true\nexcluded=false\n"
            );
            let controller = fs::read_to_string(session.join("CONTROLLER.md")).unwrap();
            assert!(controller.contains(&format!("- Plan: `{}`", fields[3])));
            assert!(controller.contains(&format!("- Fixture: `{}`", fields[4])));
            assert!(controller.contains("- Model: `gpt-5.4-mini`"));
            assert!(controller.contains("- Reasoning: `medium`"));
            assert!(controller.contains("- Sandbox: `workspace-write/no-network/read-confined`"));
            assert!(controller.contains("rejected (verification-failed) | no |"));
            for accepted in ["format", "clippy", "change-policy"] {
                assert!(controller.contains(&format!("Judge `{accepted}`: accepted")));
            }
            let public_status = if task == "in-session-evidence" {
                "rejected"
            } else {
                "accepted"
            };
            assert!(controller.contains(&format!("Judge `public`: {public_status}")));
            assert!(controller.contains("Judge `oracle`: rejected"));
        }
    }
    let report = fs::read_to_string(results.join("README.md")).unwrap();
    for exact in [
        "`12` included sessions and `0` infrastructure exclusions",
        "Positive registry use: **0/12** (two-sided 95% Clopper–Pearson `0.0000..0.2646`)",
        "In-session acceptance: **0/12** (two-sided 95% Clopper–Pearson `0.0000..0.2646`)",
        "Incremental pay-as-you-go spend authority and observed spend: **USD 0**",
    ] {
        assert!(report.contains(exact), "missing result claim: {exact}");
    }
}
