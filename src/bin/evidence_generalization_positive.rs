use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::experiment::{
    AgentMetrics, ExperimentArm, ExperimentController, ExperimentLimits, ExperimentPins,
    ExperimentPlan, JudgeCommand, RejectionReason, SessionStatus,
};
use bhcp::hash::HashAlgorithm;
use bhcp::model::ContentReference;
use bhcp::schema::validate_root;
use bhcp::value::Value;

const RUST_TOOLCHAIN: &str = "1.97.1";
const MODEL: &str = "gpt-5.4-mini";
const REASONING: &str = "medium";
const SANDBOX: &str = "workspace-write/no-network/read-confined";
const TOOLCHAIN: &str = "codex-cli-0.142.4+rust-1.97.1";
const OWNER_ATTESTATION_MERGE: &str = "c327d80308dbf6010321ca05ef498493b04350e7";
const SESSION_TIMEOUT_MILLIS: u64 = 15 * 60 * 1_000;
const STUDY_SESSION_LIMIT: usize = 12;
const STUDY_MODEL_MINUTES: u64 = 180;
const INPUT_STOP_AFTER: u64 = 12_000_000;
const OUTPUT_STOP_AFTER: u64 = 500_000;
const REASONING_STOP_AFTER: u64 = 500_000;
const PRODUCED_AT: &str = "2026-07-20T04:07:56Z";
const SEEDS: [&str; 3] = ["seed-01", "seed-02", "seed-03"];

#[derive(Clone, Copy)]
struct TaskSpec {
    id: &'static str,
    source: &'static str,
    goal: &'static str,
    repository: &'static str,
    task: Option<&'static str>,
    oracle_symbol: &'static str,
}

const TASKS: [TaskSpec; 4] = [
    TaskSpec {
        id: "atomic-batch",
        source: "experiments/minimal-coding-agent",
        goal: "experiment/RepairBatchLedger@0",
        repository: "batch-ledger@0",
        task: Some("atomic-idempotent-batch@0"),
        oracle_symbol: "experiment/verifier/ledger-invariants@0",
    },
    TaskSpec {
        id: "tenant-policy",
        source: "experiments/policy-resolution-agent",
        goal: "experiment/ResolveTenantPolicy@0",
        repository: "tenant-policy@0",
        task: Some("tenant-policy-resolution@0"),
        oracle_symbol: "experiment/verifier/policy-resolution@0",
    },
    TaskSpec {
        id: "contextual-policy",
        source: "experiments/contextual-policy-agent",
        goal: "experiment/ResolveContextualPolicy@0",
        repository: "contextual-policy@0",
        task: Some("contextual-policy-resolution@0"),
        oracle_symbol: "experiment/verifier/contextual-policy@0",
    },
    TaskSpec {
        id: "in-session-evidence",
        source: "experiments/in-session-evidence-agent",
        goal: "experiment/InSessionEvidence@0",
        repository: "in-session-evidence@0",
        task: None,
        oracle_symbol: "experiment/verifier/in-session-oracle@0",
    },
];

struct Runtime {
    driver: PathBuf,
    codex: PathBuf,
    codex_home: PathBuf,
    cargo_home: PathBuf,
    rustup_home: PathBuf,
    bhcp: PathBuf,
    rustup: PathBuf,
    toolchain_bin: PathBuf,
    deny_root: PathBuf,
    adapter: PathBuf,
    prepared: PathBuf,
    scratch: PathBuf,
    toolchain_executables: Vec<PathBuf>,
}

struct GitIdentity {
    head: String,
    artifacts: Vec<String>,
}

#[derive(Clone)]
struct SessionResult {
    task: &'static str,
    seed: &'static str,
    positive_use: bool,
    evidence_accepted: bool,
    independently_accepted: bool,
    accepted: bool,
    claimed_success: Option<bool>,
    calibrated: bool,
    excluded: bool,
    metrics: Option<AgentMetrics>,
    agent_millis: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("positive-use evidence study failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let mode = arguments
        .first()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "missing UTF-8 mode".to_owned())?;
    let should_prepare = mode == "prepare";
    let should_run = mode == "run";
    if !matches!(mode, "prepare" | "freeze" | "run") {
        return Err("mode must be prepare, freeze, or run".to_owned());
    }
    if arguments.len() != if should_run { 14 } else { 13 } {
        return Err("expected MODE DRIVER CODEX CODEX_HOME CARGO_HOME RUSTUP_HOME BHCP RUSTUP TOOLCHAIN_BIN DENY_ROOT ADAPTER PREPARED_ROOT SCRATCH [OUTPUT]".to_owned());
    }
    let mut runtime = runtime(&arguments)?;
    billing_preflight(&runtime.codex_home)?;
    if should_prepare {
        prepare_fixtures(&runtime.prepared, &runtime.adapter)?;
        println!("version=bhcp-evidence-generalization-positive@0");
        println!("prepared_tasks={}", TASKS.len());
        return Ok(());
    } else if !runtime.prepared.is_dir() {
        return Err("prepared root does not exist".to_owned());
    }
    runtime.prepared = fs::canonicalize(&runtime.prepared)
        .map_err(|error| format!("cannot resolve prepared root: {error}"))?;

    let git = git_identity(should_run)?;
    let plans = plans(&runtime)?;
    let mut frozen_records = vec![
        "version=bhcp-evidence-generalization-positive@0".to_owned(),
        format!("git_head={}", git.head),
        format!("owner_attestation_merge={OWNER_ATTESTATION_MERGE}"),
        "billing_auth_mode=chatgpt".to_owned(),
        "incremental_usd=0".to_owned(),
        "concurrency=1".to_owned(),
    ];
    frozen_records.extend(git.artifacts);
    for (task, seed, plan) in &plans {
        let frozen = plan.freeze().map_err(|error| error.message)?;
        frozen_records.push(format!(
            "session|{}|{}|{}|{}|{}",
            task.id,
            seed,
            frozen.plan_digest,
            frozen.fixture_digest,
            frozen.run_order.join(",")
        ));
    }
    if should_run {
        validate_registration(&frozen_records)?;
    }
    for record in &frozen_records {
        println!("{record}");
    }
    if !should_run {
        return Ok(());
    }
    let output = absolute_path(&arguments[13], "output")?;
    run_study(&runtime, &plans, &output, &git.head)
}

fn git_identity(require_clean: bool) -> Result<GitIdentity, String> {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if require_clean {
        let status = Command::new("/usr/bin/git")
            .current_dir(&repository)
            .args(["status", "--porcelain=v1", "--untracked-files=all"])
            .output()
            .map_err(|error| format!("cannot inspect frozen Git checkout: {error}"))?;
        if !status.status.success() || !status.stdout.is_empty() {
            return Err(
                "run requires a clean Git checkout containing the frozen inputs".to_owned(),
            );
        }
    }
    let head = git_text(&repository, ["rev-parse", "HEAD"])?;
    let paths = [
        "src/bin/evidence_generalization_positive.rs",
        "src/bin/evidence_generalization_adapter.rs",
        "experiments/evidence-generalization/POSITIVE_USE_PROMPT.md",
        "experiments/evidence-generalization/withheld/atomic-batch.rs",
        "experiments/evidence-generalization/withheld/tenant-policy.rs",
        "experiments/evidence-generalization/withheld/contextual-policy.rs",
        "experiments/evidence-generalization/withheld/in-session-evidence.rs",
        ".codex/skills/interpret-bhcp-contract/SKILL.md",
    ];
    let artifacts = paths
        .into_iter()
        .map(|path| {
            let hash = git_text(&repository, ["hash-object", path])?;
            Ok(format!("artifact|{path}|gitblob={hash}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(GitIdentity { head, artifacts })
}

fn git_text<const N: usize>(repository: &Path, arguments: [&str; N]) -> Result<String, String> {
    let output = Command::new("/usr/bin/git")
        .current_dir(repository)
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot inspect frozen Git identity: {error}"))?;
    if !output.status.success() {
        return Err("cannot inspect frozen Git identity".to_owned());
    }
    String::from_utf8(output.stdout)
        .map(|text| text.trim().to_owned())
        .map_err(|_| "Git identity was not UTF-8".to_owned())
}

fn validate_registration(records: &[String]) -> Result<(), String> {
    let registration = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("experiments/evidence-generalization/positive-use-registration.txt"),
    )
    .map_err(|error| format!("cannot read positive-use registration: {error}"))?;
    for record in records
        .iter()
        .filter(|record| record.starts_with("artifact|") || record.starts_with("session|"))
    {
        if !registration.lines().any(|line| line == record) {
            return Err(format!(
                "frozen record is absent from registration: {record}"
            ));
        }
    }
    Ok(())
}

fn runtime(arguments: &[std::ffi::OsString]) -> Result<Runtime, String> {
    let driver = canonical_file(&arguments[1], "driver")?;
    let codex = canonical_file(&arguments[2], "Codex")?;
    let codex_home = canonical_directory(&arguments[3], "Codex home")?;
    let cargo_home = canonical_directory(&arguments[4], "Cargo home")?;
    let rustup_home = canonical_directory(&arguments[5], "Rustup home")?;
    let bhcp = canonical_file(&arguments[6], "BHCP")?;
    let rustup = canonical_file(&arguments[7], "Rustup")?;
    let toolchain_bin = canonical_directory(&arguments[8], "toolchain bin")?;
    let deny_root = canonical_directory(&arguments[9], "denied read root")?;
    let adapter = canonical_file(&arguments[10], "evidence adapter")?;
    let prepared = absolute_path(&arguments[11], "prepared root")?;
    let scratch = absolute_path(&arguments[12], "scratch root")?;
    let toolchain_executables = [
        "cargo",
        "rustc",
        "rustdoc",
        "rustfmt",
        "cargo-clippy",
        "clippy-driver",
    ]
    .into_iter()
    .map(|name| canonical_file(toolchain_bin.join(name).as_os_str(), name))
    .collect::<Result<Vec<_>, _>>()?;
    if !toolchain_bin.starts_with(&rustup_home) {
        return Err("frozen toolchain is outside the supplied Rustup home".to_owned());
    }
    for (name, executable) in [
        "cargo",
        "rustc",
        "rustdoc",
        "rustfmt",
        "cargo-clippy",
        "clippy-driver",
    ]
    .into_iter()
    .zip(&toolchain_executables)
    {
        verify_rustup_selection(&rustup, name, executable)?;
    }
    let adapter_sandbox = canonical_file(
        bhcp.parent()
            .ok_or_else(|| "BHCP has no parent directory".to_owned())?
            .join("bhcp-adapter-sandbox")
            .as_os_str(),
        "BHCP adapter sandbox helper",
    )?;
    let mut executables = vec![
        codex.clone(),
        bhcp.clone(),
        adapter_sandbox,
        rustup.clone(),
        adapter.clone(),
    ];
    executables.extend(toolchain_executables.iter().cloned());
    Ok(Runtime {
        driver,
        codex,
        codex_home,
        cargo_home,
        rustup_home,
        bhcp,
        rustup,
        toolchain_bin,
        deny_root,
        adapter,
        prepared,
        scratch,
        toolchain_executables: executables,
    })
}

fn billing_preflight(codex_home: &Path) -> Result<(), String> {
    for name in [
        "OPENAI_API_KEY",
        "CODEX_API_KEY",
        "AZURE_OPENAI_API_KEY",
        "OPENAI_BASE_URL",
        "OPENAI_API_BASE",
        "AZURE_OPENAI_ENDPOINT",
    ] {
        if std::env::var_os(name).is_some() {
            return Err(format!("billing preflight forbids {name}"));
        }
    }
    let auth: serde_json::Value = serde_json::from_slice(
        &fs::read(codex_home.join("auth.json"))
            .map_err(|_| "billing preflight cannot read Codex auth".to_owned())?,
    )
    .map_err(|_| "billing preflight found invalid Codex auth".to_owned())?;
    if auth.get("auth_mode").and_then(serde_json::Value::as_str) != Some("chatgpt") {
        return Err("billing preflight requires ChatGPT entitlement auth".to_owned());
    }
    if auth
        .get("OPENAI_API_KEY")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty())
    {
        return Err("billing preflight forbids stored API keys".to_owned());
    }
    Ok(())
}

fn prepare_fixtures(prepared: &Path, adapter: &Path) -> Result<(), String> {
    if prepared.exists() {
        return Err("prepared root already exists".to_owned());
    }
    fs::create_dir(prepared).map_err(|error| format!("cannot create prepared root: {error}"))?;
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let prompt = repository.join("experiments/evidence-generalization/POSITIVE_USE_PROMPT.md");
    let skill = repository.join(".codex/skills/interpret-bhcp-contract");
    for task in TASKS {
        let source = repository.join(task.source);
        let destination = prepared.join(task.id);
        fs::create_dir(&destination)
            .map_err(|error| format!("cannot create prepared task: {error}"))?;
        copy_tree(&source.join("subject"), &destination.join("subject"))?;
        copy_tree(&source.join("oracle"), &destination.join("oracle"))?;
        for name in ["TASK.md", "contract.bhcp", "contract.semantic-id"] {
            copy_tree(&source.join(name), &destination.join(name))?;
        }
        copy_tree(&prompt, &destination.join("POSITIVE_USE_PROMPT.md"))?;
        let old_skill = destination.join("subject/.agents");
        if old_skill.exists() {
            fs::remove_dir_all(&old_skill)
                .map_err(|error| format!("cannot remove historical staged skill: {error}"))?;
        }
        copy_tree(
            &skill,
            &destination.join("subject/.agents/skills/interpret-bhcp-contract"),
        )?;
        let tools = destination.join("subject/tools");
        fs::create_dir_all(&tools)
            .map_err(|error| format!("cannot create prepared tools: {error}"))?;
        let staged_adapter = tools.join("evidence-generalization-adapter");
        fs::copy(adapter, &staged_adapter)
            .map_err(|error| format!("cannot stage evidence adapter: {error}"))?;
        #[cfg(unix)]
        fs::set_permissions(&staged_adapter, fs::Permissions::from_mode(0o500))
            .map_err(|error| format!("cannot protect staged evidence adapter: {error}"))?;
        let existing_manifest = destination.join("bhcp-project.toml");
        if existing_manifest.exists() {
            fs::remove_file(&existing_manifest)
                .map_err(|error| format!("cannot replace historical project manifest: {error}"))?;
        }
        create_file(&existing_manifest, project_manifest(&task).as_bytes())?;
        create_file(
            &destination.join("candidate.cbor"),
            &encode_deterministic(&candidate(&task)).map_err(|error| error.to_string())?,
        )?;
        create_file(
            &destination.join("REGISTRY_COMMAND.txt"),
            registry_command(&task).as_bytes(),
        )?;
    }
    Ok(())
}

fn project_manifest(task: &TaskSpec) -> String {
    let adapter = "subject/tools/evidence-generalization-adapter";
    [
        ("experiment/verifier/public-rust@0", "public"),
        (task.oracle_symbol, "oracle"),
        ("experiment/verifier/change-policy@0", "change"),
    ]
    .into_iter()
    .map(|(symbol, verifier)| {
        format!(
            "[[verifier_adapter]]\nsymbol = \"{symbol}\"\nexecutable = \"{adapter}\"\nargv = [\"verify\", \"{}\", \"{verifier}\"]\nworking_scope = \"project\"\ninput_media_type = \"application/vnd.bhcp.verification-request+cbor\"\noutput_media_type = \"application/vnd.bhcp.verifier-result+cbor\"\ntimeout_ms = 2000\nallowed_effects = [\"bhcp-effect/fs.read@0\"]\nevidence_kind = \"static\"\n",
            task.id
        )
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn registry_command(task: &TaskSpec) -> String {
    format!(
        "bhcp verify ../contract.bhcp {} ../candidate.cbor src/lib.rs {PRODUCED_AT} > evidence.cbor\nbhcp inspect evidence.cbor\n",
        task.goal
    )
}

fn candidate(task: &TaskSpec) -> Value {
    let mut input = vec![(
        "repository".to_owned(),
        Value::Text(task.repository.to_owned()),
    )];
    if let Some(task_name) = task.task {
        input.push(("task".to_owned(), Value::Text(task_name.to_owned())));
    }
    let output = match task.id {
        "atomic-batch" => bool_output(&[
            "publicTestsPassed",
            "invariantOraclePassed",
            "policyCheckPassed",
        ]),
        "tenant-policy" => bool_output(&[
            "publicTestsPassed",
            "tenantIsolationPassed",
            "defaultDenyPassed",
            "specificityDominatesPriorityPassed",
            "priorityBreaksSpecificityTiesPassed",
            "denyBreaksPolicyTiesPassed",
            "ruleIdBreaksRemainingTiesPassed",
            "insertionOrderIndependentPassed",
            "policyCheckPassed",
        ]),
        "contextual-policy" => bool_output(&[
            "publicTestsPassed",
            "tenantIsolationPassed",
            "defaultDenyPassed",
            "resourceSpecificityPassed",
            "subjectSpecificityPassed",
            "actionSpecificityPassed",
            "priorityTieBreakPassed",
            "denyTieBreakPassed",
            "ruleIdTieBreakPassed",
            "insertionOrderIndependentPassed",
            "disabledRuleExclusionPassed",
            "policyCheckPassed",
        ]),
        "in-session-evidence" => bool_output(&["publicPassed", "oraclePassed", "policyPassed"]),
        _ => unreachable!(),
    };
    Value::map([("input", Value::owned_map(input)), ("output", output)])
}

fn bool_output(names: &[&str]) -> Value {
    let mut output = names
        .iter()
        .map(|name| ((*name).to_owned(), Value::Bool(true)))
        .collect::<Vec<_>>();
    if names.first() == Some(&"publicTestsPassed") {
        output.push((
            "patch".to_owned(),
            Value::Text("subject/src/lib.rs".to_owned()),
        ));
    }
    output.push((
        "changedFiles".to_owned(),
        Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(1)]),
    ));
    Value::owned_map(output)
}

fn plans(runtime: &Runtime) -> Result<Vec<(TaskSpec, &'static str, ExperimentPlan)>, String> {
    let mut plans = Vec::with_capacity(STUDY_SESSION_LIMIT);
    for task in TASKS {
        for seed in SEEDS {
            plans.push((task, seed, plan(runtime, &task, seed)?));
        }
    }
    Ok(plans)
}

fn plan(runtime: &Runtime, task: &TaskSpec, seed: &str) -> Result<ExperimentPlan, String> {
    let fixture = runtime.prepared.join(task.id);
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let oracle_probe = canonical_file(
        repository
            .join(task.source)
            .join("oracle/tests/invariants.rs")
            .as_os_str(),
        "withheld oracle probe",
    )?;
    if !oracle_probe.starts_with(&runtime.deny_root) {
        return Err("denied read root does not contain the withheld oracle".to_owned());
    }
    let mut plan = ExperimentPlan::new(
        format!("evidence-generalization-positive-{}-{seed}", task.id),
        &fixture,
        &runtime.scratch,
        ExperimentPins {
            model: MODEL.to_owned(),
            reasoning: REASONING.to_owned(),
            sandbox: SANDBOX.to_owned(),
            toolchain: TOOLCHAIN.to_owned(),
        },
    );
    plan.limits = ExperimentLimits {
        timeout_millis: SESSION_TIMEOUT_MILLIS,
        max_agent_output_bytes: 16 * 1024,
        max_judge_output_bytes: 2 * 1024 * 1024,
    };
    plan.allowed_changes = vec![
        PathBuf::from("subject/src/lib.rs"),
        PathBuf::from("subject/evidence.cbor"),
    ];
    plan.oracle_source = Some(fixture.join("oracle"));
    plan.trusted_executables = runtime.toolchain_executables.clone();
    let mut arm = ExperimentArm::new(seed, "POSITIVE_USE_PROMPT.md", &runtime.driver);
    arm.arguments = [
        &runtime.codex,
        &runtime.codex_home,
        &runtime.cargo_home,
        &runtime.rustup_home,
        &runtime.bhcp,
        &runtime.toolchain_bin,
        Path::new(RUST_TOOLCHAIN),
        &runtime.deny_root,
        &oracle_probe,
    ]
    .into_iter()
    .map(|path| path.to_string_lossy().into_owned())
    .collect();
    arm.contract_files = [
        "TASK.md",
        "contract.bhcp",
        "contract.semantic-id",
        "bhcp-project.toml",
        "candidate.cbor",
        "REGISTRY_COMMAND.txt",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();
    plan.arms = vec![arm];
    plan.judges = vec![
        cargo_judge(
            "format",
            &runtime.rustup,
            ["fmt", "--check", "--manifest-path", "subject/Cargo.toml"],
        ),
        cargo_judge(
            "clippy",
            &runtime.rustup,
            [
                "clippy",
                "--offline",
                "--manifest-path",
                "subject/Cargo.toml",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        cargo_judge(
            "public",
            &runtime.rustup,
            ["test", "--offline", "--manifest-path", "subject/Cargo.toml"],
        ),
        cargo_judge(
            "oracle",
            &runtime.rustup,
            ["test", "--offline", "--manifest-path", "oracle/Cargo.toml"],
        )
        .with_oracle(),
        JudgeCommand::new("change-policy", &runtime.adapter, ["judge-change-policy"]),
    ];
    Ok(plan)
}

fn run_study(
    runtime: &Runtime,
    plans: &[(TaskSpec, &'static str, ExperimentPlan)],
    output: &Path,
    git_head: &str,
) -> Result<(), String> {
    if output.exists() {
        return Err("output directory already exists".to_owned());
    }
    fs::create_dir(output).map_err(|error| format!("cannot create output: {error}"))?;
    create_file(
        &output.join("AUTHORITY.txt"),
        format!(
            "git-head={git_head}\nowner-attestation-merge={OWNER_ATTESTATION_MERGE}\nauth-mode=chatgpt\nincremental-usd=0\nconcurrency=1\n"
        )
        .as_bytes(),
    )?;
    let mut results = Vec::with_capacity(STUDY_SESSION_LIMIT);
    let mut usage = Usage::default();
    for (index, (task, seed, plan)) in plans.iter().enumerate() {
        prelaunch(index, &usage)?;
        let report = match ExperimentController::new().run(plan) {
            Ok(report) => report,
            Err(error) => {
                create_file(
                    &output.join("STOPPED.md"),
                    format!(
                        "# Study stopped\n\nInfrastructure failure before a closed result for `{}` / `{seed}`: {}\n\nNo replacement is authorized.\n",
                        task.id, error.message
                    )
                    .as_bytes(),
                )?;
                return Err(error.message);
            }
        };
        let outcome = report
            .arms
            .first()
            .ok_or_else(|| "controller returned no arm outcome".to_owned())?;
        let session_directory = output.join(format!("{}-{seed}", task.id));
        fs::create_dir(&session_directory)
            .map_err(|error| format!("cannot create session output: {error}"))?;
        report
            .write_markdown(session_directory.join("CONTROLLER.md"))
            .map_err(|error| error.message)?;
        let workspace = runtime
            .scratch
            .join(&plan.id)
            .join("workspaces")
            .join(seed)
            .join("subject");
        write_patch(
            &runtime.prepared.join(task.id).join("subject/src/lib.rs"),
            &workspace.join("src/lib.rs"),
            &session_directory.join("candidate.patch"),
        )?;
        let evidence_path = workspace.join("evidence.cbor");
        let assessment = assess_evidence(&evidence_path, &workspace.join("src/lib.rs"));
        if evidence_path.is_file() {
            fs::copy(&evidence_path, session_directory.join("evidence.cbor"))
                .map_err(|error| format!("cannot retain evidence bundle: {error}"))?;
        }
        let independently_accepted = outcome.status == SessionStatus::Accepted;
        let accepted = assessment.accepted && independently_accepted;
        let claimed_success = outcome
            .metrics
            .as_ref()
            .map(|metrics| metrics.claimed_success);
        let calibrated = claimed_success.is_some_and(|claim| claim == accepted);
        let excluded = outcome.metrics.is_none()
            || matches!(
                outcome.rejection,
                Some(RejectionReason::Contaminated | RejectionReason::AdaptiveOracle)
            );
        create_file(
            &session_directory.join("EVIDENCE.txt"),
            format!(
                "positive-use={}\nparseable={}\nsubject-bound={}\nadapter-items={}\nregistered-accepted={}\nindependent-accepted={}\nin-session-accepted={}\nclaimed-success={}\ncalibrated={}\nexcluded={}\n",
                assessment.positive_use,
                assessment.parseable,
                assessment.subject_bound,
                assessment.adapter_items,
                assessment.accepted,
                independently_accepted,
                accepted,
                claimed_success
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_owned()),
                calibrated,
                excluded,
            )
            .as_bytes(),
        )?;
        if let Some(metrics) = &outcome.metrics {
            usage.add(metrics, outcome.agent_elapsed_millis)?;
        }
        results.push(SessionResult {
            task: task.id,
            seed,
            positive_use: assessment.positive_use,
            evidence_accepted: assessment.accepted,
            independently_accepted,
            accepted,
            claimed_success,
            calibrated,
            excluded,
            metrics: outcome.metrics.clone(),
            agent_millis: outcome.agent_elapsed_millis,
        });
        if excluded {
            write_results(output, &results, &usage)?;
            create_file(
                &output.join("STOPPED.md"),
                format!(
                    "# Study stopped\n\nThe frozen `{}` / `{seed}` session was retained as an infrastructure exclusion. The preregistered safety rule stopped all later launches; no replacement is authorized.\n",
                    task.id
                )
                .as_bytes(),
            )?;
            return Err("an infrastructure exclusion stopped later launches".to_owned());
        }
    }
    write_results(output, &results, &usage)
}

#[derive(Default)]
struct Usage {
    input: u64,
    cached: u64,
    output: u64,
    reasoning: u64,
    model_millis: u64,
}

impl Usage {
    fn add(&mut self, metrics: &AgentMetrics, model_millis: u64) -> Result<(), String> {
        self.input = self
            .input
            .checked_add(metrics.input_tokens)
            .ok_or_else(|| "input usage overflowed".to_owned())?;
        self.cached = self
            .cached
            .checked_add(metrics.cached_input_tokens)
            .ok_or_else(|| "cached usage overflowed".to_owned())?;
        self.output = self
            .output
            .checked_add(metrics.output_tokens)
            .ok_or_else(|| "output usage overflowed".to_owned())?;
        self.reasoning = self
            .reasoning
            .checked_add(metrics.reasoning_tokens)
            .ok_or_else(|| "reasoning usage overflowed".to_owned())?;
        self.model_millis = self
            .model_millis
            .checked_add(model_millis)
            .ok_or_else(|| "model time overflowed".to_owned())?;
        Ok(())
    }
}

fn prelaunch(index: usize, usage: &Usage) -> Result<(), String> {
    if index >= STUDY_SESSION_LIMIT
        || (index as u64 + 1) * 15 > STUDY_MODEL_MINUTES
        || usage.model_millis > STUDY_MODEL_MINUTES * 60 * 1_000
    {
        return Err("the next launch exceeds the positive-use session/time budget".to_owned());
    }
    if usage.input >= INPUT_STOP_AFTER
        || usage.output >= OUTPUT_STOP_AFTER
        || usage.reasoning >= REASONING_STOP_AFTER
    {
        return Err("a post-session usage stop threshold was reached".to_owned());
    }
    Ok(())
}

struct EvidenceAssessment {
    positive_use: bool,
    parseable: bool,
    subject_bound: bool,
    adapter_items: usize,
    accepted: bool,
}

fn assess_evidence(evidence: &Path, source: &Path) -> EvidenceAssessment {
    let empty = || EvidenceAssessment {
        positive_use: false,
        parseable: false,
        subject_bound: false,
        adapter_items: 0,
        accepted: false,
    };
    let Ok(bytes) = fs::read(evidence) else {
        return empty();
    };
    if bytes.len() > 4 * 1024 * 1024 {
        return empty();
    }
    let Ok(bundle) = decode_deterministic(&bytes) else {
        return empty();
    };
    if validate_root(&bundle, "evidence-bundle").is_err() {
        return empty();
    }
    let parseable = true;
    let Ok(subject_bytes) = fs::read(source) else {
        return empty();
    };
    let subject = ContentReference::from_bytes(
        "application/vnd.bhcp.subject-source",
        &subject_bytes,
        HashAlgorithm::default(),
    )
    .to_value();
    let subject_bound = matches!(bundle.get("claims"), Some(Value::Array(claims)) if !claims.is_empty() && claims.iter().all(|claim| claim.get("subject") == Some(&subject)));
    let adapter_items = match bundle.get("items") {
        Some(Value::Array(items)) => items
            .iter()
            .filter(|item| {
                item.get("provenance")
                    .and_then(|provenance| provenance.get("source"))
                    .is_some()
            })
            .count(),
        _ => 0,
    };
    let accepted = parseable
        && subject_bound
        && matches!(bundle.get("obligation_status"), Some(Value::Map(statuses)) if !statuses.is_empty() && statuses.iter().all(|(_, status)| status == &Value::Text("discharged".to_owned())));
    EvidenceAssessment {
        positive_use: parseable && subject_bound && adapter_items >= 3,
        parseable,
        subject_bound,
        adapter_items,
        accepted,
    }
}

fn write_results(output: &Path, results: &[SessionResult], usage: &Usage) -> Result<(), String> {
    let mut ledger = String::from("version|bhcp-evidence-generalization-positive-results@0\n");
    for result in results {
        let metrics = result.metrics.as_ref();
        ledger.push_str(&format!(
            "session|{}|{}|positive-use={}|registered-accepted={}|independent-accepted={}|accepted={}|claimed={}|calibrated={}|excluded={}|input={}|cached={}|output={}|reasoning={}|model-ms={}\n",
            result.task,
            result.seed,
            result.positive_use,
            result.evidence_accepted,
            result.independently_accepted,
            result.accepted,
            result
                .claimed_success
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_owned()),
            result.calibrated,
            result.excluded,
            metrics.map_or(0, |value| value.input_tokens),
            metrics.map_or(0, |value| value.cached_input_tokens),
            metrics.map_or(0, |value| value.output_tokens),
            metrics.map_or(0, |value| value.reasoning_tokens),
            result.agent_millis,
        ));
    }
    create_file(&output.join("RESULTS.txt"), ledger.as_bytes())?;

    let included = results.iter().filter(|result| !result.excluded).count();
    let positive = results
        .iter()
        .filter(|result| !result.excluded && result.positive_use)
        .count();
    let accepted = results
        .iter()
        .filter(|result| !result.excluded && result.accepted)
        .count();
    let positive_interval = clopper_pearson(positive, included);
    let accepted_interval = clopper_pearson(accepted, included);
    let mut report = format!(
        "# Positive registered-adapter study\n\nThe frozen twelve-session study completed with `{included}` included sessions and `{}` infrastructure exclusions. Every frozen session remains in `RESULTS.txt`; no result was replaced.\n\n- Positive registry use: **{positive}/{included}** (two-sided 95% Clopper–Pearson `{:.4}..{:.4}`).\n- In-session acceptance: **{accepted}/{included}** (two-sided 95% Clopper–Pearson `{:.4}..{:.4}`).\n- Usage: {} input, {} cached input, {} output, {} reasoning tokens; {:.3} model-minutes.\n- Incremental pay-as-you-go spend authority and observed spend: **USD 0**.\n\n| Task | Seed | Positive use | Registered accepted | Independent accepted | In-session accepted | Claim | Calibrated | Excluded |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
        results.len() - included,
        positive_interval.0,
        positive_interval.1,
        accepted_interval.0,
        accepted_interval.1,
        usage.input,
        usage.cached,
        usage.output,
        usage.reasoning,
        usage.model_millis as f64 / 60_000.0,
    );
    for result in results {
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            result.task,
            result.seed,
            yes_no(result.positive_use),
            yes_no(result.evidence_accepted),
            yes_no(result.independently_accepted),
            yes_no(result.accepted),
            result.claimed_success.map(yes_no).unwrap_or("unknown"),
            yes_no(result.calibrated),
            yes_no(result.excluded),
        ));
    }
    report.push_str("\n## By-task estimates\n\n| Task | Included | Positive use (95% CI) | In-session accepted (95% CI) |\n| --- | ---: | --- | --- |\n");
    for task in TASKS {
        let task_results = results
            .iter()
            .filter(|result| result.task == task.id && !result.excluded)
            .collect::<Vec<_>>();
        let task_included = task_results.len();
        let task_positive = task_results
            .iter()
            .filter(|result| result.positive_use)
            .count();
        let task_accepted = task_results.iter().filter(|result| result.accepted).count();
        let positive_interval = clopper_pearson(task_positive, task_included);
        let accepted_interval = clopper_pearson(task_accepted, task_included);
        report.push_str(&format!(
            "| {} | {} | {}/{} ({:.4}..{:.4}) | {}/{} ({:.4}..{:.4}) |\n",
            task.id,
            task_included,
            task_positive,
            task_included,
            positive_interval.0,
            positive_interval.1,
            task_accepted,
            task_included,
            accepted_interval.0,
            accepted_interval.1,
        ));
    }
    report.push_str("\n## Resource distributions\n\nMedians and interquartile ranges use Tukey hinges over included sessions with closed usage records.\n\n| Measure | Median | IQR |\n| --- | ---: | --- |\n");
    let input = included_values(results, |metrics, _| metrics.input_tokens);
    let cached = included_values(results, |metrics, _| metrics.cached_input_tokens);
    let output_tokens = included_values(results, |metrics, _| metrics.output_tokens);
    let reasoning = included_values(results, |metrics, _| metrics.reasoning_tokens);
    let wall = included_values(results, |_, result| result.agent_millis);
    for (label, values) in [
        ("Input tokens", input),
        ("Cached-input tokens", cached),
        ("Output tokens", output_tokens),
        ("Reasoning tokens", reasoning),
        ("Model wall milliseconds", wall),
    ] {
        match median_iqr(&values) {
            Some((median, first, third)) => report.push_str(&format!(
                "| {label} | {median:.1} | {first:.1}..{third:.1} |\n"
            )),
            None => report.push_str(&format!("| {label} | unavailable | unavailable |\n")),
        }
    }
    report.push_str("\nThese are descriptive estimates for the four frozen repository fixtures under one pinned model. They do not establish a population rate, model-wide effect, developer-productivity effect, or general BHCP advantage. Null, unfavorable, and incomplete outcomes remain part of the result.\n");
    create_file(&output.join("README.md"), report.as_bytes())
}

fn included_values(
    results: &[SessionResult],
    value: impl Fn(&AgentMetrics, &SessionResult) -> u64,
) -> Vec<u64> {
    results
        .iter()
        .filter(|result| !result.excluded)
        .filter_map(|result| {
            result
                .metrics
                .as_ref()
                .map(|metrics| value(metrics, result))
        })
        .collect()
}

fn median_iqr(values: &[u64]) -> Option<(f64, f64, f64)> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let midpoint = median(&sorted);
    let middle = sorted.len() / 2;
    let (lower, upper) = if sorted.len().is_multiple_of(2) {
        (&sorted[..middle], &sorted[middle..])
    } else {
        (&sorted[..=middle], &sorted[middle..])
    };
    Some((midpoint, median(lower), median(upper)))
}

fn median(sorted: &[u64]) -> f64 {
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] as f64 + sorted[middle] as f64) / 2.0
    } else {
        sorted[middle] as f64
    }
}

fn clopper_pearson(successes: usize, trials: usize) -> (f64, f64) {
    if trials == 0 {
        return (0.0, 1.0);
    }
    let alpha = 0.025;
    let lower = if successes == 0 {
        0.0
    } else {
        bisect(
            |probability| binomial_upper(trials, successes, probability),
            alpha,
            true,
        )
    };
    let upper = if successes == trials {
        1.0
    } else {
        bisect(
            |probability| binomial_lower(trials, successes, probability),
            alpha,
            false,
        )
    };
    (lower, upper)
}

fn bisect(function: impl Fn(f64) -> f64, target: f64, increasing: bool) -> f64 {
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..80 {
        let middle = (low + high) / 2.0;
        let value = function(middle);
        if (value < target) == increasing {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low + high) / 2.0
}

fn binomial_upper(trials: usize, successes: usize, probability: f64) -> f64 {
    (successes..=trials)
        .map(|value| binomial_term(trials, value, probability))
        .sum()
}

fn binomial_lower(trials: usize, successes: usize, probability: f64) -> f64 {
    (0..=successes)
        .map(|value| binomial_term(trials, value, probability))
        .sum()
}

fn binomial_term(trials: usize, successes: usize, probability: f64) -> f64 {
    let combinations = (1..=successes).fold(1.0, |value, index| {
        value * (trials + 1 - index) as f64 / index as f64
    });
    combinations
        * probability.powi(successes as i32)
        * (1.0 - probability).powi((trials - successes) as i32)
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn cargo_judge<const N: usize>(name: &str, rustup: &Path, arguments: [&str; N]) -> JudgeCommand {
    let arguments = ["run", RUST_TOOLCHAIN, "cargo"]
        .into_iter()
        .chain(arguments)
        .collect::<Vec<_>>();
    JudgeCommand::new(name, rustup, arguments)
}

fn verify_rustup_selection(rustup: &Path, name: &str, expected: &Path) -> Result<(), String> {
    let output = Command::new(rustup)
        .args(["which", name, "--toolchain", RUST_TOOLCHAIN])
        .env_clear()
        .output()
        .map_err(|error| format!("cannot ask Rustup for {name}: {error}"))?;
    if !output.status.success() {
        return Err(format!("Rustup could not select frozen {name}"));
    }
    let selected = String::from_utf8(output.stdout)
        .map_err(|_| format!("Rustup returned a non-UTF-8 {name} path"))?;
    let selected = fs::canonicalize(selected.trim())
        .map_err(|error| format!("cannot resolve Rustup-selected {name}: {error}"))?;
    if selected != expected {
        return Err(format!("Rustup did not select frozen {name}"));
    }
    Ok(())
}

fn canonical_file(value: &OsStr, name: &str) -> Result<PathBuf, String> {
    let path = absolute_path(value, name)?;
    if !path.is_file() {
        return Err(format!("{name} is not an existing file"));
    }
    fs::canonicalize(path).map_err(|error| format!("cannot resolve {name}: {error}"))
}

fn canonical_directory(value: &OsStr, name: &str) -> Result<PathBuf, String> {
    let path = absolute_path(value, name)?;
    if !path.is_dir() {
        return Err(format!("{name} is not an existing directory"));
    }
    fs::canonicalize(path).map_err(|error| format!("cannot resolve {name}: {error}"))
}

fn absolute_path(value: &OsStr, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("{name} must be absolute"));
    }
    Ok(path)
}

fn create_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("cannot inspect fixture input: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("fixture preparation rejects symbolic links".to_owned());
    }
    if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create fixture parent: {error}"))?;
        }
        fs::copy(source, destination)
            .map_err(|error| format!("cannot copy fixture file: {error}"))?;
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err("fixture preparation rejects unsupported file types".to_owned());
    }
    fs::create_dir_all(destination)
        .map_err(|error| format!("cannot create fixture directory: {error}"))?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("cannot read fixture directory: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot inspect fixture entry: {error}"))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn write_patch(original: &Path, candidate: &Path, destination: &Path) -> Result<(), String> {
    let output = Command::new("/usr/bin/git")
        .args(["diff", "--no-index", "--no-ext-diff", "--no-prefix", "--"])
        .arg(original)
        .arg(candidate)
        .output()
        .map_err(|error| format!("cannot create candidate patch: {error}"))?;
    match output.status.code() {
        Some(0) => create_file(destination, b""),
        Some(1) => {
            let text = String::from_utf8(output.stdout)
                .map_err(|_| "candidate diff was not UTF-8".to_owned())?;
            let mut lines = text.lines();
            for _ in 0..4 {
                lines
                    .next()
                    .ok_or_else(|| "candidate diff header is incomplete".to_owned())?;
            }
            let mut normalized = String::from(
                "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n",
            );
            for line in lines {
                normalized.push_str(line);
                normalized.push('\n');
            }
            create_file(destination, normalized.as_bytes())
        }
        code => Err(format!(
            "candidate diff returned unexpected status {code:?}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{clopper_pearson, median_iqr};

    #[test]
    fn exact_intervals_cover_registered_edge_cases() {
        let none = clopper_pearson(0, 3);
        let all = clopper_pearson(3, 3);
        assert_eq!(none.0, 0.0);
        assert!((none.1 - 0.707598).abs() < 0.000001);
        assert!((all.0 - 0.292402).abs() < 0.000001);
        assert_eq!(all.1, 1.0);
        assert_eq!(clopper_pearson(0, 0), (0.0, 1.0));
    }

    #[test]
    fn tukey_hinges_are_deterministic() {
        assert_eq!(median_iqr(&[]), None);
        assert_eq!(median_iqr(&[1]), Some((1.0, 1.0, 1.0)));
        assert_eq!(median_iqr(&[1, 2, 3, 4]), Some((2.5, 1.5, 3.5)));
        assert_eq!(median_iqr(&[1, 2, 3, 4, 5]), Some((3.0, 2.0, 4.0)));
    }
}
