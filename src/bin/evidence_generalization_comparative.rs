use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use bhcp::experiment::{
    AgentMetrics, ExperimentArm, ExperimentController, ExperimentLimits, ExperimentPins,
    ExperimentPlan, JudgeCommand, RejectionReason, SessionStatus,
};

const RUST_TOOLCHAIN: &str = "1.97.1";
const MODEL: &str = "gpt-5.4-mini";
const REASONING: &str = "medium";
const SANDBOX: &str = "workspace-write/no-network/read-confined";
const TOOLCHAIN: &str = "codex-cli-0.142.4+rust-1.97.1";
const OWNER_ATTESTATION_MERGE: &str = "c327d80308dbf6010321ca05ef498493b04350e7";
const SESSION_TIMEOUT_MILLIS: u64 = 15 * 60 * 1_000;
const STUDY_SESSION_LIMIT: usize = 24;
const STUDY_MODEL_MINUTES: u64 = 360;
const INPUT_STOP_AFTER: u64 = 12_000_000;
const OUTPUT_STOP_AFTER: u64 = 500_000;
const REASONING_STOP_AFTER: u64 = 500_000;
const PRIOR_INPUT_TOKENS: u64 = 1_961_148;
const PRIOR_OUTPUT_TOKENS: u64 = 40_755;
const PRIOR_REASONING_TOKENS: u64 = 31_916;
const SEEDS: [&str; 4] = ["seed-01", "seed-02", "seed-03", "seed-04"];

#[derive(Clone, Copy)]
struct TaskSpec {
    id: &'static str,
    source: &'static str,
    prose: &'static str,
    bhcp_first: [bool; 4],
}

const TASKS: [TaskSpec; 3] = [
    TaskSpec {
        id: "atomic-batch",
        source: "experiments/minimal-coding-agent",
        prose: "experiments/evidence-generalization/tasks/atomic-batch-prose.md",
        bhcp_first: [false, true, false, true],
    },
    TaskSpec {
        id: "tenant-policy",
        source: "experiments/policy-resolution-agent",
        prose: "experiments/evidence-generalization/tasks/tenant-policy-prose.md",
        bhcp_first: [true, false, true, false],
    },
    TaskSpec {
        id: "contextual-policy",
        source: "experiments/contextual-policy-agent",
        prose: "experiments/evidence-generalization/tasks/contextual-policy-prose.md",
        bhcp_first: [false, true, false, true],
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArmKind {
    Prose,
    Bhcp,
}

impl ArmKind {
    fn id(self) -> &'static str {
        match self {
            Self::Prose => "prose-control",
            Self::Bhcp => "bhcp-contract",
        }
    }

    fn prompt(self) -> &'static str {
        match self {
            Self::Prose => "COMPARATIVE_PROSE_PROMPT.md",
            Self::Bhcp => "COMPARATIVE_BHCP_PROMPT.md",
        }
    }

    fn contract_files(self) -> Vec<PathBuf> {
        match self {
            Self::Prose => vec![PathBuf::from("PROSE_TASK.md")],
            Self::Bhcp => ["TASK.md", "contract.bhcp", "contract.semantic-id"]
                .into_iter()
                .map(PathBuf::from)
                .collect(),
        }
    }
}

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
    policy: PathBuf,
    prepared: PathBuf,
    scratch: PathBuf,
    trusted_executables: Vec<PathBuf>,
}

struct GitIdentity {
    head: String,
    artifacts: Vec<String>,
}

struct SessionPlan {
    task: TaskSpec,
    seed: &'static str,
    position: usize,
    arm: ArmKind,
    plan: ExperimentPlan,
}

#[derive(Clone)]
struct SessionResult {
    task: &'static str,
    seed: &'static str,
    position: usize,
    arm: ArmKind,
    accepted: bool,
    claimed_success: Option<bool>,
    calibrated: bool,
    excluded: bool,
    failure: String,
    metrics: Option<AgentMetrics>,
    agent_millis: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("comparative evidence study failed: {error}");
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
        return Err("expected MODE DRIVER CODEX CODEX_HOME CARGO_HOME RUSTUP_HOME BHCP RUSTUP TOOLCHAIN_BIN DENY_ROOT POLICY PREPARED_ROOT SCRATCH [OUTPUT]".to_owned());
    }
    let mut runtime = runtime(&arguments)?;
    billing_preflight(&runtime.codex_home)?;
    if should_prepare {
        prepare_fixtures(&runtime.prepared)?;
        println!("version=bhcp-evidence-generalization-comparative@0");
        println!("prepared_tasks={}", TASKS.len());
        return Ok(());
    }
    if !runtime.prepared.is_dir() {
        return Err("prepared root does not exist".to_owned());
    }
    runtime.prepared = fs::canonicalize(&runtime.prepared)
        .map_err(|error| format!("cannot resolve prepared root: {error}"))?;

    let git = git_identity(should_run)?;
    let sessions = session_plans(&runtime)?;
    validate_symmetry(&sessions)?;
    let mut records = vec![
        "version=bhcp-evidence-generalization-comparative@0".to_owned(),
        format!("git_head={}", git.head),
        format!("owner_attestation_merge={OWNER_ATTESTATION_MERGE}"),
        "billing_auth_mode=chatgpt".to_owned(),
        "incremental_usd=0".to_owned(),
        "concurrency=1".to_owned(),
    ];
    records.extend(git.artifacts);
    for session in &sessions {
        let frozen = session.plan.freeze().map_err(|error| error.message)?;
        records.push(format!(
            "session|{}|{}|position={}|arm={}|{}|{}|{}",
            session.task.id,
            session.seed,
            session.position,
            session.arm.id(),
            frozen.plan_digest,
            frozen.fixture_digest,
            frozen.run_order.join(",")
        ));
    }
    if matches!(mode, "freeze" | "run") {
        validate_registration(&records)?;
    }
    for record in &records {
        println!("{record}");
    }
    if !should_run {
        return Ok(());
    }
    let output = absolute_path(&arguments[13], "output")?;
    run_study(&runtime, &sessions, &output, &git.head)
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
    let policy = canonical_file(&arguments[10], "change-policy judge")?;
    let prepared = absolute_path(&arguments[11], "prepared root")?;
    let scratch = absolute_path(&arguments[12], "scratch root")?;
    let rust_tools = [
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
    .zip(&rust_tools)
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
    let mut trusted_executables = vec![
        codex.clone(),
        bhcp.clone(),
        adapter_sandbox,
        rustup.clone(),
        policy.clone(),
    ];
    trusted_executables.extend(rust_tools);
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
        policy,
        prepared,
        scratch,
        trusted_executables,
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
    let head = git_text(&repository, &["rev-parse", "HEAD"])?;
    let paths = [
        "src/bin/evidence_generalization_comparative.rs",
        "src/bin/evidence_generalization_comparative_policy.rs",
        "src/bin/bhcp_codex_experiment_driver.rs",
        "experiments/evidence-generalization/COMPARATIVE_BHCP_PROMPT.md",
        "experiments/evidence-generalization/COMPARATIVE_PROSE_PROMPT.md",
        "experiments/evidence-generalization/tasks/atomic-batch-prose.md",
        "experiments/evidence-generalization/tasks/tenant-policy-prose.md",
        "experiments/evidence-generalization/tasks/contextual-policy-prose.md",
    ];
    let artifacts = paths
        .into_iter()
        .map(|path| {
            let hash = git_text(&repository, &["hash-object", path])?;
            Ok(format!("artifact|{path}|gitblob={hash}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(GitIdentity { head, artifacts })
}

fn git_text(repository: &Path, arguments: &[&str]) -> Result<String, String> {
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
            .join("experiments/evidence-generalization/comparative-registration.txt"),
    )
    .map_err(|error| format!("cannot read comparative registration: {error}"))?;
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

fn prepare_fixtures(prepared: &Path) -> Result<(), String> {
    if prepared.exists() {
        return Err("prepared root already exists".to_owned());
    }
    fs::create_dir(prepared).map_err(|error| format!("cannot create prepared root: {error}"))?;
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
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
        copy_tree(
            &repository.join(task.prose),
            &destination.join("PROSE_TASK.md"),
        )?;
        for prompt in ["COMPARATIVE_BHCP_PROMPT.md", "COMPARATIVE_PROSE_PROMPT.md"] {
            copy_tree(
                &repository
                    .join("experiments/evidence-generalization")
                    .join(prompt),
                &destination.join(prompt),
            )?;
        }
        for forbidden in [
            "subject/.agents",
            "subject/.codex",
            "subject/tools",
            "subject/bhcp-project.toml",
            "subject/evidence.cbor",
        ] {
            remove_if_exists(&destination.join(forbidden))?;
        }
        assert_registry_free(&destination.join("subject"))?;
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|error| format!("cannot remove forbidden prepared input: {error}"))
}

fn assert_registry_free(subject: &Path) -> Result<(), String> {
    for relative in [
        ".agents",
        ".codex",
        "tools",
        "bhcp-project.toml",
        "evidence.cbor",
    ] {
        if subject.join(relative).exists() {
            return Err(format!("comparative subject retained forbidden {relative}"));
        }
    }
    Ok(())
}

fn session_plans(runtime: &Runtime) -> Result<Vec<SessionPlan>, String> {
    let mut sessions = Vec::with_capacity(STUDY_SESSION_LIMIT);
    for task in TASKS {
        for (seed_index, seed) in SEEDS.into_iter().enumerate() {
            let order = if task.bhcp_first[seed_index] {
                [ArmKind::Bhcp, ArmKind::Prose]
            } else {
                [ArmKind::Prose, ArmKind::Bhcp]
            };
            for (position_index, arm) in order.into_iter().enumerate() {
                sessions.push(SessionPlan {
                    task,
                    seed,
                    position: position_index + 1,
                    arm,
                    plan: plan(runtime, &task, seed, arm)?,
                });
            }
        }
    }
    Ok(sessions)
}

fn plan(
    runtime: &Runtime,
    task: &TaskSpec,
    seed: &str,
    arm_kind: ArmKind,
) -> Result<ExperimentPlan, String> {
    let fixture = runtime.prepared.join(task.id);
    assert_registry_free(&fixture.join("subject"))?;
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
        format!(
            "evidence-generalization-comparative-{}-{seed}-{}",
            task.id,
            arm_kind.id()
        ),
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
    plan.allowed_changes = vec![PathBuf::from("subject/src/lib.rs")];
    plan.oracle_source = Some(fixture.join("oracle"));
    plan.trusted_executables = runtime.trusted_executables.clone();
    let mut arm = ExperimentArm::new(arm_kind.id(), arm_kind.prompt(), &runtime.driver);
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
    arm.contract_files = arm_kind.contract_files();
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
        JudgeCommand::new("change-policy", &runtime.policy, std::iter::empty::<&str>()),
    ];
    Ok(plan)
}

fn validate_symmetry(sessions: &[SessionPlan]) -> Result<(), String> {
    if sessions.len() != STUDY_SESSION_LIMIT {
        return Err("comparative schedule must contain exactly 24 sessions".to_owned());
    }
    let mut prose = 0;
    let mut bhcp = 0;
    let mut prose_first = 0;
    let mut bhcp_first = 0;
    for task in TASKS {
        for seed in SEEDS {
            let block = sessions
                .iter()
                .filter(|session| session.task.id == task.id && session.seed == seed)
                .collect::<Vec<_>>();
            if block.len() != 2
                || block
                    .iter()
                    .map(|session| session.position)
                    .collect::<Vec<_>>()
                    != [1, 2]
                || block[0].arm == block[1].arm
            {
                return Err(format!("invalid paired block for {} / {seed}", task.id));
            }
            match block[0].arm {
                ArmKind::Prose => prose_first += 1,
                ArmKind::Bhcp => bhcp_first += 1,
            }
        }
    }
    for session in sessions {
        match session.arm {
            ArmKind::Prose => {
                prose += 1;
                if session.plan.arms[0].contract_files != [PathBuf::from("PROSE_TASK.md")] {
                    return Err("prose arm received non-prose treatment files".to_owned());
                }
            }
            ArmKind::Bhcp => {
                bhcp += 1;
                if session.plan.arms[0].contract_files != session.arm.contract_files() {
                    return Err("BHCP arm did not receive exactly task plus contract".to_owned());
                }
            }
        }
        let judge_names = session
            .plan
            .judges
            .iter()
            .map(|judge| judge.name.as_str())
            .collect::<Vec<_>>();
        if judge_names != ["format", "clippy", "public", "oracle", "change-policy"]
            || session.plan.allowed_changes != [PathBuf::from("subject/src/lib.rs")]
        {
            return Err("comparative arms do not share the frozen judge boundary".to_owned());
        }
    }
    if (prose, bhcp, prose_first, bhcp_first) != (12, 12, 6, 6) {
        return Err("comparative arm or first-position counts are asymmetric".to_owned());
    }
    Ok(())
}

fn run_study(
    runtime: &Runtime,
    sessions: &[SessionPlan],
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
            "git-head={git_head}\nowner-attestation-merge={OWNER_ATTESTATION_MERGE}\nauth-mode=chatgpt\nincremental-usd=0\nconcurrency=1\nprior-input-tokens={PRIOR_INPUT_TOKENS}\nprior-output-tokens={PRIOR_OUTPUT_TOKENS}\nprior-reasoning-tokens={PRIOR_REASONING_TOKENS}\n"
        )
        .as_bytes(),
    )?;
    let mut results = Vec::with_capacity(STUDY_SESSION_LIMIT);
    let mut usage = Usage::default();
    for (index, session) in sessions.iter().enumerate() {
        if let Err(error) = prelaunch(index, &usage) {
            write_results(output, &results, &usage)?;
            write_stopped(output, &error)?;
            return Err(error);
        }
        let report = match ExperimentController::new().run(&session.plan) {
            Ok(report) => report,
            Err(error) => {
                write_results(output, &results, &usage)?;
                write_stopped(
                    output,
                    &format!(
                        "Infrastructure failure before a closed result for `{}` / `{}` / `{}`: {}",
                        session.task.id,
                        session.seed,
                        session.arm.id(),
                        error.message
                    ),
                )?;
                return Err(error.message);
            }
        };
        let outcome = report
            .arms
            .first()
            .ok_or_else(|| "controller returned no arm outcome".to_owned())?;
        let directory = output.join(format!(
            "{}-{}-{}",
            session.task.id,
            session.seed,
            session.arm.id()
        ));
        fs::create_dir(&directory)
            .map_err(|error| format!("cannot create session output: {error}"))?;
        report
            .write_markdown(directory.join("CONTROLLER.md"))
            .map_err(|error| error.message)?;
        let workspace = runtime
            .scratch
            .join(&session.plan.id)
            .join("workspaces")
            .join(session.arm.id())
            .join("subject");
        write_patch(
            &runtime
                .prepared
                .join(session.task.id)
                .join("subject/src/lib.rs"),
            &workspace.join("src/lib.rs"),
            &directory.join("candidate.patch"),
        )?;
        let accepted = outcome.status == SessionStatus::Accepted;
        let claimed_success = outcome
            .metrics
            .as_ref()
            .map(|metrics| metrics.claimed_success);
        let calibrated = claimed_success.is_some_and(|claim| claim == accepted);
        let excluded = matches!(
            outcome.rejection,
            Some(
                RejectionReason::Interrupted
                    | RejectionReason::Contaminated
                    | RejectionReason::AdaptiveOracle
                    | RejectionReason::Incomplete
            )
        ) || outcome.metrics.is_none();
        let failure = failure_category(outcome.rejection, &outcome.judges, accepted);
        create_file(
            &directory.join("SESSION.txt"),
            format!(
                "task={}\nseed={}\nposition={}\narm={}\naccepted={}\nclaimed-success={}\ncalibrated={}\nexcluded={}\nfailure-category={}\n",
                session.task.id,
                session.seed,
                session.position,
                session.arm.id(),
                accepted,
                claimed_success
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_owned()),
                calibrated,
                excluded,
                failure,
            )
            .as_bytes(),
        )?;
        if let Some(metrics) = &outcome.metrics {
            usage.add(metrics, outcome.agent_elapsed_millis)?;
        }
        results.push(SessionResult {
            task: session.task.id,
            seed: session.seed,
            position: session.position,
            arm: session.arm,
            accepted,
            claimed_success,
            calibrated,
            excluded,
            failure,
            metrics: outcome.metrics.clone(),
            agent_millis: outcome.agent_elapsed_millis,
        });
        if excluded {
            write_results(output, &results, &usage)?;
            write_stopped(
                output,
                &format!(
                    "The frozen `{}` / `{}` / `{}` session was retained as an infrastructure exclusion. Its paired block is excluded, later launches stopped, and no replacement is authorized.",
                    session.task.id,
                    session.seed,
                    session.arm.id()
                ),
            )?;
            return Err("an infrastructure exclusion stopped later launches".to_owned());
        }
    }
    write_results(output, &results, &usage)
}

fn write_stopped(output: &Path, detail: &str) -> Result<(), String> {
    create_file(
        &output.join("STOPPED.md"),
        format!("# Study stopped\n\n{detail}\n").as_bytes(),
    )
}

fn failure_category(
    rejection: Option<RejectionReason>,
    judges: &[bhcp::experiment::JudgeResult],
    accepted: bool,
) -> String {
    if accepted {
        return "accepted".to_owned();
    }
    match rejection {
        Some(RejectionReason::VerificationFailed) => {
            let failed = judges
                .iter()
                .filter(|judge| !judge.accepted)
                .map(|judge| judge.name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            format!("verification:{failed}")
        }
        Some(reason) => format!("infrastructure:{}", rejection_name(reason)),
        None => "rejected:unclassified".to_owned(),
    }
}

fn rejection_name(reason: RejectionReason) -> &'static str {
    match reason {
        RejectionReason::Interrupted => "interrupted",
        RejectionReason::Contaminated => "contaminated",
        RejectionReason::AdaptiveOracle => "adaptive-oracle",
        RejectionReason::Incomplete => "incomplete",
        RejectionReason::VerificationFailed => "verification-failed",
    }
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
        self.input = checked_add(self.input, metrics.input_tokens, "input usage")?;
        self.cached = checked_add(self.cached, metrics.cached_input_tokens, "cached usage")?;
        self.output = checked_add(self.output, metrics.output_tokens, "output usage")?;
        self.reasoning = checked_add(self.reasoning, metrics.reasoning_tokens, "reasoning usage")?;
        self.model_millis = checked_add(self.model_millis, model_millis, "model time")?;
        Ok(())
    }
}

fn checked_add(left: u64, right: u64, label: &str) -> Result<u64, String> {
    left.checked_add(right)
        .ok_or_else(|| format!("{label} overflowed"))
}

fn prelaunch(index: usize, usage: &Usage) -> Result<(), String> {
    if index >= STUDY_SESSION_LIMIT
        || (index as u64 + 1) * 15 > STUDY_MODEL_MINUTES
        || usage.model_millis > STUDY_MODEL_MINUTES * 60 * 1_000
    {
        return Err("the next launch exceeds the comparative session/time budget".to_owned());
    }
    if PRIOR_INPUT_TOKENS.saturating_add(usage.input) >= INPUT_STOP_AFTER
        || PRIOR_OUTPUT_TOKENS.saturating_add(usage.output) >= OUTPUT_STOP_AFTER
        || PRIOR_REASONING_TOKENS.saturating_add(usage.reasoning) >= REASONING_STOP_AFTER
    {
        return Err("a post-session usage stop threshold was reached".to_owned());
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
struct PairedEstimate {
    blocks: usize,
    bhcp_only: usize,
    prose_only: usize,
    risk_difference: f64,
    mcnemar_p: f64,
}

fn paired_estimate(
    results: &[SessionResult],
    task_filter: Option<&str>,
    positive: impl Fn(&SessionResult) -> bool,
) -> PairedEstimate {
    let mut blocks = 0;
    let mut bhcp_only = 0;
    let mut prose_only = 0;
    for task in TASKS {
        if task_filter.is_some_and(|filter| filter != task.id) {
            continue;
        }
        for seed in SEEDS {
            let prose = results.iter().find(|result| {
                result.task == task.id && result.seed == seed && result.arm == ArmKind::Prose
            });
            let bhcp = results.iter().find(|result| {
                result.task == task.id && result.seed == seed && result.arm == ArmKind::Bhcp
            });
            let (Some(prose), Some(bhcp)) = (prose, bhcp) else {
                continue;
            };
            if prose.excluded || bhcp.excluded {
                continue;
            }
            blocks += 1;
            match (positive(prose), positive(bhcp)) {
                (false, true) => bhcp_only += 1,
                (true, false) => prose_only += 1,
                _ => {}
            }
        }
    }
    PairedEstimate {
        blocks,
        bhcp_only,
        prose_only,
        risk_difference: if blocks == 0 {
            0.0
        } else {
            (bhcp_only as f64 - prose_only as f64) / blocks as f64
        },
        mcnemar_p: exact_mcnemar(bhcp_only, prose_only),
    }
}

fn exact_mcnemar(bhcp_only: usize, prose_only: usize) -> f64 {
    let discordant = bhcp_only + prose_only;
    if discordant == 0 {
        return 1.0;
    }
    let tail = bhcp_only.min(prose_only);
    let probability = (0..=tail)
        .map(|successes| combination(discordant, successes) / 2_f64.powi(discordant as i32))
        .sum::<f64>();
    (2.0 * probability).min(1.0)
}

fn combination(total: usize, selected: usize) -> f64 {
    (1..=selected).fold(1.0, |value, index| {
        value * (total + 1 - index) as f64 / index as f64
    })
}

fn write_results(output: &Path, results: &[SessionResult], usage: &Usage) -> Result<(), String> {
    let mut ledger = String::from("version|bhcp-evidence-generalization-comparative-results@0\n");
    for result in results {
        let metrics = result.metrics.as_ref();
        ledger.push_str(&format!(
            "session|{}|{}|position={}|arm={}|accepted={}|claimed={}|calibrated={}|excluded={}|failure={}|input={}|cached={}|output={}|reasoning={}|commands={}|model-ms={}\n",
            result.task,
            result.seed,
            result.position,
            result.arm.id(),
            result.accepted,
            result
                .claimed_success
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_owned()),
            result.calibrated,
            result.excluded,
            result.failure,
            metrics.map_or(0, |value| value.input_tokens),
            metrics.map_or(0, |value| value.cached_input_tokens),
            metrics.map_or(0, |value| value.output_tokens),
            metrics.map_or(0, |value| value.reasoning_tokens),
            metrics.map_or(0, |value| value.completed_commands),
            result.agent_millis,
        ));
    }
    create_file(&output.join("RESULTS.txt"), ledger.as_bytes())?;

    let correctness = paired_estimate(results, None, |result| result.accepted);
    let calibration = paired_estimate(results, None, |result| result.calibrated);
    let exclusions = results.iter().filter(|result| result.excluded).count();
    let mut report = format!(
        "# Paired BHCP-contract versus prose study\n\nThe frozen comparative study retained `{}` session records with `{exclusions}` infrastructure exclusions. No arm was replaced. Paired estimates use only blocks with two completed, non-excluded arms.\n\n- Independent all-judge acceptance: **paired risk difference {:+.4}** (BHCP minus prose; {} included blocks; discordants BHCP-only `{}`, prose-only `{}`; two-sided exact McNemar `p={:.6}`).\n- Exact claim calibration: **paired risk difference {:+.4}** ({} included blocks; discordants BHCP-only `{}`, prose-only `{}`; two-sided exact McNemar `p={:.6}`).\n- Usage: {} input, {} cached input, {} output, {} reasoning tokens; {:.3} model-minutes.\n- Incremental pay-as-you-go spend authority and observed spend: **USD 0**.\n\n| Task | Seed | Position | Arm | Accepted | Claim | Calibrated | Excluded | Failure category |\n| --- | --- | ---: | --- | --- | --- | --- | --- | --- |\n",
        results.len(),
        correctness.risk_difference,
        correctness.blocks,
        correctness.bhcp_only,
        correctness.prose_only,
        correctness.mcnemar_p,
        calibration.risk_difference,
        calibration.blocks,
        calibration.bhcp_only,
        calibration.prose_only,
        calibration.mcnemar_p,
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
            result.position,
            result.arm.id(),
            yes_no(result.accepted),
            result.claimed_success.map(yes_no).unwrap_or("unknown"),
            yes_no(result.calibrated),
            yes_no(result.excluded),
            result.failure,
        ));
    }
    report.push_str("\n## Task-level paired estimates\n\n| Task | Included blocks | Acceptance risk difference | Acceptance discordants (BHCP/prose) | Exact McNemar p | Calibration risk difference | Calibration discordants (BHCP/prose) | Exact McNemar p |\n| --- | ---: | ---: | --- | ---: | ---: | --- | ---: |\n");
    for task in TASKS {
        let accepted = paired_estimate(results, Some(task.id), |result| result.accepted);
        let calibrated = paired_estimate(results, Some(task.id), |result| result.calibrated);
        report.push_str(&format!(
            "| {} | {} | {:+.4} | {}/{} | {:.6} | {:+.4} | {}/{} | {:.6} |\n",
            task.id,
            accepted.blocks,
            accepted.risk_difference,
            accepted.bhcp_only,
            accepted.prose_only,
            accepted.mcnemar_p,
            calibrated.risk_difference,
            calibrated.bhcp_only,
            calibrated.prose_only,
            calibrated.mcnemar_p,
        ));
    }
    report.push_str("\n## Resource distributions by arm\n\nMedians and interquartile ranges use Tukey hinges over non-excluded sessions with closed usage records.\n\n| Arm | Measure | Median | IQR |\n| --- | --- | ---: | --- |\n");
    for arm in [ArmKind::Prose, ArmKind::Bhcp] {
        for (label, values) in [
            (
                "Input tokens",
                included_values(results, arm, |metrics, _| metrics.input_tokens),
            ),
            (
                "Cached-input tokens",
                included_values(results, arm, |metrics, _| metrics.cached_input_tokens),
            ),
            (
                "Output tokens",
                included_values(results, arm, |metrics, _| metrics.output_tokens),
            ),
            (
                "Reasoning tokens",
                included_values(results, arm, |metrics, _| metrics.reasoning_tokens),
            ),
            (
                "Completed commands",
                included_values(results, arm, |metrics, _| metrics.completed_commands),
            ),
            (
                "Model wall milliseconds",
                included_values(results, arm, |_, result| result.agent_millis),
            ),
        ] {
            match median_iqr(&values) {
                Some((median, first, third)) => report.push_str(&format!(
                    "| {} | {label} | {median:.1} | {first:.1}..{third:.1} |\n",
                    arm.id()
                )),
                None => report.push_str(&format!(
                    "| {} | {label} | unavailable | unavailable |\n",
                    arm.id()
                )),
            }
        }
    }
    report.push_str("\nThese are descriptive paired estimates for three frozen repository fixtures under one pinned model. `alpha=descriptive-only`: the exact McNemar values are uncertainty evidence, not a confirmatory threshold. The study cannot establish a population effect, causal language effect, model-wide effect, developer-productivity effect, or general BHCP advantage. Null, unfavorable, incomplete, and excluded outcomes remain visible.\n");
    create_file(&output.join("README.md"), report.as_bytes())
}

fn included_values(
    results: &[SessionResult],
    arm: ArmKind,
    value: impl Fn(&AgentMetrics, &SessionResult) -> u64,
) -> Vec<u64> {
    results
        .iter()
        .filter(|result| result.arm == arm && !result.excluded)
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
    use super::{
        INPUT_STOP_AFTER, PRIOR_INPUT_TOKENS, Usage, exact_mcnemar, median_iqr, prelaunch,
    };

    #[test]
    fn exact_mcnemar_covers_registered_edge_cases() {
        assert_eq!(exact_mcnemar(0, 0), 1.0);
        assert_eq!(exact_mcnemar(1, 0), 1.0);
        assert_eq!(exact_mcnemar(4, 0), 0.125);
        assert_eq!(exact_mcnemar(6, 2), 0.2890625);
        assert_eq!(exact_mcnemar(2, 6), 0.2890625);
    }

    #[test]
    fn tukey_hinges_are_deterministic() {
        assert_eq!(median_iqr(&[]), None);
        assert_eq!(median_iqr(&[1]), Some((1.0, 1.0, 1.0)));
        assert_eq!(median_iqr(&[1, 2, 3, 4]), Some((2.5, 1.5, 3.5)));
        assert_eq!(median_iqr(&[1, 2, 3, 4, 5]), Some((3.0, 2.0, 4.0)));
    }

    #[test]
    fn usage_monitor_carries_forward_the_positive_study() {
        assert!(prelaunch(0, &Usage::default()).is_ok());
        let below = Usage {
            input: INPUT_STOP_AFTER - PRIOR_INPUT_TOKENS - 1,
            ..Usage::default()
        };
        assert!(prelaunch(1, &below).is_ok());
        let reached = Usage {
            input: INPUT_STOP_AFTER - PRIOR_INPUT_TOKENS,
            ..Usage::default()
        };
        assert_eq!(
            prelaunch(1, &reached).unwrap_err(),
            "a post-session usage stop threshold was reached"
        );
    }
}
