//! Reproducible, fail-closed coding-agent experiment orchestration.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::diagnostic::{Diagnostic, Result};
use crate::hash::{HashAlgorithm, format_hash, hash_value};
use crate::value::Value;

#[cfg(unix)]
use nix::sys::signal::{Signal, killpg};
#[cfg(unix)]
use nix::unistd::Pid;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

const INVALID_PLAN: &str = "BHCP7501";
const CONTROLLER_FAILURE: &str = "BHCP7502";
const POLL_INTERVAL: Duration = Duration::from_millis(2);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentPins {
    pub model: String,
    pub reasoning: String,
    pub sandbox: String,
    pub toolchain: String,
}

impl ExperimentPins {
    fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("model", &self.model),
            ("reasoning", &self.reasoning),
            ("sandbox", &self.sandbox),
            ("toolchain", &self.toolchain),
        ] {
            if value.trim().is_empty() {
                return Err(invalid(format!("experiment {name} pin must not be empty")));
            }
        }
        if self.sandbox != "workspace-write/no-network" {
            return Err(invalid(
                "experiment sandbox pin must be workspace-write/no-network",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentLimits {
    pub timeout_millis: u64,
    pub max_agent_output_bytes: usize,
    pub max_judge_output_bytes: usize,
}

impl Default for ExperimentLimits {
    fn default() -> Self {
        Self {
            timeout_millis: 30 * 60 * 1_000,
            max_agent_output_bytes: 256 * 1_024,
            max_judge_output_bytes: 1_024 * 1_024,
        }
    }
}

impl ExperimentLimits {
    fn validate(&self) -> Result<()> {
        if self.timeout_millis == 0 || self.timeout_millis > 24 * 60 * 60 * 1_000 {
            return Err(invalid(
                "experiment timeout must be between 1 millisecond and 24 hours",
            ));
        }
        const MAX_OUTPUT: usize = 64 * 1_024 * 1_024;
        if self.max_agent_output_bytes == 0
            || self.max_judge_output_bytes == 0
            || self.max_agent_output_bytes > MAX_OUTPUT
            || self.max_judge_output_bytes > MAX_OUTPUT
        {
            return Err(invalid(
                "experiment output limits must be between 1 byte and 64 MiB",
            ));
        }
        Ok(())
    }
}

struct BoundedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
    overflowed: bool,
    orphaned: bool,
}

fn run_bounded(command: &mut Command, timeout: Duration, limit: usize) -> Result<BoundedOutput> {
    #[cfg(unix)]
    command.process_group(0);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| failure(format!("cannot launch experiment process: {error}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| failure("experiment process stdout was not captured"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| failure("experiment process stderr was not captured"))?;
    let bytes_seen = Arc::new(AtomicUsize::new(0));
    let overflow_signal = Arc::new(AtomicBool::new(false));
    let stdout_reader = {
        let bytes_seen = Arc::clone(&bytes_seen);
        let overflow_signal = Arc::clone(&overflow_signal);
        thread::spawn(move || read_bounded(stdout, limit, bytes_seen, overflow_signal))
    };
    let stderr_reader = {
        let bytes_seen = Arc::clone(&bytes_seen);
        let overflow_signal = Arc::clone(&overflow_signal);
        thread::spawn(move || read_bounded(stderr, limit, bytes_seen, overflow_signal))
    };
    let started = Instant::now();
    let mut timed_out = false;
    let mut orphaned = false;
    let status = loop {
        if overflow_signal.load(Ordering::Acquire) {
            #[cfg(unix)]
            if killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL).is_err() {
                child.kill().map_err(|error| {
                    failure(format!("cannot stop output-flooding process: {error}"))
                })?;
            }
            #[cfg(not(unix))]
            child.kill().map_err(|error| {
                failure(format!("cannot stop output-flooding process: {error}"))
            })?;
            break child.wait().map_err(|error| {
                failure(format!("cannot reap output-flooding process: {error}"))
            })?;
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| failure(format!("cannot poll experiment process: {error}")))?
        {
            #[cfg(unix)]
            if killpg(Pid::from_raw(child.id() as i32), None).is_ok() {
                orphaned = true;
                killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL).map_err(|error| {
                    failure(format!(
                        "cannot stop orphaned experiment process group: {error}"
                    ))
                })?;
            }
            break status;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            #[cfg(unix)]
            if killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL).is_err() {
                child
                    .kill()
                    .map_err(|error| failure(format!("cannot stop timed-out process: {error}")))?;
            }
            #[cfg(not(unix))]
            child
                .kill()
                .map_err(|error| failure(format!("cannot stop timed-out process: {error}")))?;
            break child
                .wait()
                .map_err(|error| failure(format!("cannot reap timed-out process: {error}")))?;
        }
        thread::sleep(POLL_INTERVAL);
    };
    let (stdout, stdout_overflow) = stdout_reader
        .join()
        .map_err(|_| failure("experiment stdout reader panicked"))?
        .map_err(|error| failure(format!("cannot read experiment stdout: {error}")))?;
    let (stderr, stderr_overflow) = stderr_reader
        .join()
        .map_err(|_| failure("experiment stderr reader panicked"))?
        .map_err(|error| failure(format!("cannot read experiment stderr: {error}")))?;
    let overflowed = overflow_signal.load(Ordering::Acquire)
        || stdout_overflow
        || stderr_overflow
        || stdout.len().saturating_add(stderr.len()) > limit;
    Ok(BoundedOutput {
        status,
        stdout,
        stderr,
        timed_out,
        overflowed,
        orphaned,
    })
}

fn read_bounded(
    mut reader: impl Read,
    limit: usize,
    bytes_seen: Arc<AtomicUsize>,
    overflow_signal: Arc<AtomicBool>,
) -> std::io::Result<(Vec<u8>, bool)> {
    let mut retained = Vec::with_capacity(limit.min(8 * 1_024));
    let mut buffer = [0_u8; 8 * 1_024];
    let mut overflowed = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let previous = bytes_seen.fetch_add(read, Ordering::AcqRel);
        let remaining = limit.saturating_sub(previous);
        retained.extend_from_slice(&buffer[..read.min(remaining)]);
        if read > remaining {
            overflowed = true;
            overflow_signal.store(true, Ordering::Release);
        }
    }
    Ok((retained, overflowed))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentArm {
    pub id: String,
    pub prompt_file: PathBuf,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub contract_files: Vec<PathBuf>,
}

impl ExperimentArm {
    pub fn new(
        id: impl Into<String>,
        prompt_file: impl Into<PathBuf>,
        executable: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id: id.into(),
            prompt_file: prompt_file.into(),
            executable: executable.into(),
            arguments: Vec::new(),
            contract_files: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JudgeCommand {
    pub name: String,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub uses_oracle: bool,
}

impl JudgeCommand {
    pub fn new<I, S>(name: impl Into<String>, executable: impl Into<PathBuf>, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            name: name.into(),
            executable: executable.into(),
            arguments: arguments.into_iter().map(Into::into).collect(),
            uses_oracle: false,
        }
    }

    pub fn with_oracle(mut self) -> Self {
        self.uses_oracle = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentPlan {
    pub id: String,
    pub fixture_root: PathBuf,
    pub scratch_root: PathBuf,
    pub pins: ExperimentPins,
    pub limits: ExperimentLimits,
    pub arms: Vec<ExperimentArm>,
    pub judges: Vec<JudgeCommand>,
    pub oracle_source: Option<PathBuf>,
    pub allowed_changes: Vec<PathBuf>,
}

impl ExperimentPlan {
    pub fn new(
        id: impl Into<String>,
        fixture_root: impl Into<PathBuf>,
        scratch_root: impl Into<PathBuf>,
        pins: ExperimentPins,
    ) -> Self {
        Self {
            id: id.into(),
            fixture_root: fixture_root.into(),
            scratch_root: scratch_root.into(),
            pins,
            limits: ExperimentLimits::default(),
            arms: Vec::new(),
            judges: Vec::new(),
            oracle_source: None,
            allowed_changes: vec![PathBuf::from("subject/src/lib.rs")],
        }
    }

    pub fn validate(&self) -> Result<()> {
        validate_component("experiment id", &self.id)?;
        self.pins.validate()?;
        self.limits.validate()?;
        if self.arms.is_empty() {
            return Err(invalid("experiment plan requires at least one arm"));
        }
        let mut ids = BTreeSet::new();
        for arm in &self.arms {
            validate_component("arm id", &arm.id)?;
            if !ids.insert(&arm.id) {
                return Err(invalid(format!("duplicate arm id {:?}", arm.id)));
            }
        }
        for arm in &self.arms {
            validate_relative_file("prompt file", &arm.prompt_file)?;
            if !arm.executable.is_absolute() || !arm.executable.is_file() {
                return Err(invalid(format!(
                    "arm {:?} executable must be an absolute existing file",
                    arm.id
                )));
            }
            path_text(&arm.executable)?;
            let mut visible_files = BTreeSet::from([&arm.prompt_file]);
            for file in &arm.contract_files {
                validate_relative_file("contract file", file)?;
                if !visible_files.insert(file) {
                    return Err(invalid(format!(
                        "arm {:?} repeats visible input {}",
                        arm.id,
                        file.display()
                    )));
                }
            }
        }
        for path in &self.allowed_changes {
            validate_relative_file("allowed change", path)?;
        }
        let mut judges = BTreeSet::new();
        for judge in &self.judges {
            validate_component("judge name", &judge.name)?;
            if !judges.insert(&judge.name) {
                return Err(invalid(format!("duplicate judge name {:?}", judge.name)));
            }
            if !judge.executable.is_absolute() || !judge.executable.is_file() {
                return Err(invalid(format!(
                    "judge {:?} executable must be an absolute existing file",
                    judge.name
                )));
            }
            path_text(&judge.executable)?;
        }
        Ok(())
    }

    pub fn freeze(&self) -> Result<FrozenExperimentPlan> {
        self.validate()?;
        let fixture = fs::canonicalize(&self.fixture_root)
            .map_err(|error| failure(format!("cannot resolve experiment fixture root: {error}")))?;
        if !fixture.join("subject").is_dir() {
            return Err(failure("experiment fixture has no subject directory"));
        }
        if let Some(oracle) = &self.oracle_source {
            let oracle = fs::canonicalize(oracle)
                .map_err(|error| failure(format!("cannot resolve oracle source: {error}")))?;
            let fixture_oracle = fs::canonicalize(fixture.join("oracle")).map_err(|error| {
                failure(format!("cannot resolve fixture oracle directory: {error}"))
            })?;
            if oracle != fixture_oracle {
                return Err(failure(
                    "withheld oracle must be the fixture's exact oracle directory",
                ));
            }
        }
        let fixture_digest = digest_tree(&fixture)?;
        Ok(FrozenExperimentPlan {
            plan_digest: plan_digest(self, &fixture_digest)?,
            fixture_digest,
            run_order: self.arms.iter().map(|arm| arm.id.clone()).collect(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenExperimentPlan {
    pub plan_digest: String,
    pub fixture_digest: String,
    pub run_order: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionStatus {
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectionReason {
    Interrupted,
    Contaminated,
    AdaptiveOracle,
    Incomplete,
    VerificationFailed,
}

impl RejectionReason {
    fn name(self) -> &'static str {
        match self {
            Self::Interrupted => "interrupted",
            Self::Contaminated => "contaminated",
            Self::AdaptiveOracle => "adaptive-oracle",
            Self::Incomplete => "incomplete",
            Self::VerificationFailed => "verification-failed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentMetrics {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub completed_commands: u64,
    pub claimed_success: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JudgeResult {
    pub name: String,
    pub command: Vec<String>,
    pub accepted: bool,
    pub exit_code: Option<i32>,
    pub elapsed_millis: u64,
    pub stdout_digest: String,
    pub stderr_digest: String,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArmOutcome {
    pub arm: String,
    pub status: SessionStatus,
    pub rejection: Option<RejectionReason>,
    pub detail: String,
    pub elapsed_millis: u64,
    pub metrics: Option<AgentMetrics>,
    pub agent_command: Vec<String>,
    pub agent_elapsed_millis: u64,
    pub input_digests: BTreeMap<PathBuf, String>,
    pub agent_executable_digest: String,
    pub agent_stdout_digest: String,
    pub agent_stderr_digest: String,
    pub subject_digest_before: String,
    pub subject_digest_after: String,
    pub judges: Vec<JudgeResult>,
}

impl ArmOutcome {
    pub fn rejected(
        arm: impl Into<String>,
        reason: RejectionReason,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            arm: arm.into(),
            status: SessionStatus::Rejected,
            rejection: Some(reason),
            detail: detail.into(),
            elapsed_millis: 0,
            metrics: None,
            agent_command: Vec::new(),
            agent_elapsed_millis: 0,
            input_digests: BTreeMap::new(),
            agent_executable_digest: String::new(),
            agent_stdout_digest: String::new(),
            agent_stderr_digest: String::new(),
            subject_digest_before: String::new(),
            subject_digest_after: String::new(),
            judges: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentReport {
    pub experiment: String,
    pub plan_digest: String,
    pub fixture_digest: String,
    pub pins: ExperimentPins,
    pub run_order: Vec<String>,
    pub arms: Vec<ArmOutcome>,
}

impl ExperimentReport {
    pub fn to_markdown(&self) -> String {
        let mut output = format!(
            "# Experiment {}\n\n- Plan: `{}`\n- Fixture: `{}`\n- Model: `{}`\n- Reasoning: `{}`\n- Sandbox: `{}`\n- Toolchain: `{}`\n- Run order: {}\n\n| Arm | Status | Claimed | Input tokens | Commands | Elapsed ms |\n| --- | --- | --- | ---: | ---: | ---: |\n",
            self.experiment,
            self.plan_digest,
            self.fixture_digest,
            self.pins.model,
            self.pins.reasoning,
            self.pins.sandbox,
            self.pins.toolchain,
            self.run_order.join(" → ")
        );
        for arm in &self.arms {
            let claimed = arm
                .metrics
                .as_ref()
                .map(|metrics| if metrics.claimed_success { "yes" } else { "no" })
                .unwrap_or("unknown");
            let input = arm
                .metrics
                .as_ref()
                .map(|metrics| metrics.input_tokens.to_string())
                .unwrap_or_else(|| "-".to_owned());
            let commands = arm
                .metrics
                .as_ref()
                .map(|metrics| metrics.completed_commands.to_string())
                .unwrap_or_else(|| "-".to_owned());
            let status = match arm.status {
                SessionStatus::Accepted => "accepted".to_owned(),
                SessionStatus::Rejected => format!(
                    "rejected ({})",
                    arm.rejection
                        .map(RejectionReason::name)
                        .unwrap_or("unknown")
                ),
            };
            output.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                arm.arm, status, claimed, input, commands, arm.elapsed_millis
            ));
        }
        for arm in &self.arms {
            output.push_str(&format!(
                "\n## Arm {}\n\n- Result: {}\n- Total elapsed: {} ms\n- Agent elapsed: {} ms\n- Agent command: {}\n- Subject before: `{}`\n- Subject after: `{}`\n- Agent executable: `{}`\n- Agent stdout: `{}`\n- Agent stderr: `{}`\n- Frozen inputs: {}\n",
                arm.arm,
                arm.detail,
                arm.elapsed_millis,
                arm.agent_elapsed_millis,
                markdown_command(&arm.agent_command),
                arm.subject_digest_before,
                arm.subject_digest_after,
                arm.agent_executable_digest,
                arm.agent_stdout_digest,
                arm.agent_stderr_digest,
                arm.input_digests.len()
            ));
            if let Some(metrics) = &arm.metrics {
                output.push_str(&format!(
                    "- Tokens: input {}, cached {}, output {}, reasoning {}\n- Completed commands: {}\n",
                    metrics.input_tokens,
                    metrics.cached_input_tokens,
                    metrics.output_tokens,
                    metrics.reasoning_tokens,
                    metrics.completed_commands
                ));
            }
            for (path, digest) in &arm.input_digests {
                output.push_str(&format!(
                    "- Input `{}`: `{digest}`\n",
                    markdown_inline(&path.to_string_lossy())
                ));
            }
            for judge in &arm.judges {
                output.push_str(&format!(
                    "- Judge `{}`: {} (exit {:?}, {} ms); command {}; stdout `{}`; stderr `{}`\n",
                    judge.name,
                    if judge.accepted {
                        "accepted"
                    } else {
                        "rejected"
                    },
                    judge.exit_code,
                    judge.elapsed_millis,
                    markdown_command(&judge.command),
                    judge.stdout_digest,
                    judge.stderr_digest
                ));
            }
        }
        output
    }

    pub fn write_markdown(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if path.extension() != Some(OsStr::new("md")) {
            return Err(failure("experiment repository output must be Markdown"));
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| failure(format!("cannot create experiment summary: {error}")))?;
        output
            .write_all(self.to_markdown().as_bytes())
            .map_err(|error| failure(format!("cannot write experiment summary: {error}")))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ExperimentController;

struct RunRoots {
    workspaces: PathBuf,
    agent_targets: PathBuf,
    judge_views: PathBuf,
    judge_targets: PathBuf,
}

impl ExperimentController {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, plan: &ExperimentPlan) -> Result<ExperimentReport> {
        let frozen = plan.freeze()?;
        let fixture = fs::canonicalize(&plan.fixture_root)
            .map_err(|error| failure(format!("cannot resolve experiment fixture root: {error}")))?;
        let subject = fixture.join("subject");
        if !subject.is_dir() {
            return Err(failure("experiment fixture has no subject directory"));
        }
        let oracle = plan
            .oracle_source
            .as_ref()
            .map(|path| {
                fs::canonicalize(path)
                    .map_err(|error| failure(format!("cannot resolve oracle source: {error}")))
            })
            .transpose()?;
        fs::create_dir_all(&plan.scratch_root)
            .map_err(|error| failure(format!("cannot create scratch root: {error}")))?;
        let scratch = fs::canonicalize(&plan.scratch_root)
            .map_err(|error| failure(format!("cannot resolve scratch root: {error}")))?;
        if scratch.starts_with(&fixture) || fixture.starts_with(&scratch) {
            return Err(failure(
                "scratch root and fixture root must be disjoint trees",
            ));
        }
        if oracle
            .as_ref()
            .is_some_and(|path| path.starts_with(&subject))
        {
            return Err(failure("oracle source must not be inside the subject tree"));
        }
        let plan_root = create_exclusive_directory(&scratch, &plan.id, "experiment plan")?;
        let roots = RunRoots {
            workspaces: create_exclusive_directory(&plan_root, "workspaces", "workspace root")?,
            agent_targets: create_exclusive_directory(
                &plan_root,
                "agent-targets",
                "agent target root",
            )?,
            judge_views: create_exclusive_directory(&plan_root, "judge-views", "judge view root")?,
            judge_targets: create_exclusive_directory(
                &plan_root,
                "judge-targets",
                "judge target root",
            )?,
        };

        let mut outcomes = Vec::with_capacity(plan.arms.len());
        for arm in &plan.arms {
            if plan.freeze()? != frozen {
                return Err(failure(
                    "experiment plan or frozen inputs changed after freeze",
                ));
            }
            outcomes.push(self.run_arm(
                plan,
                arm,
                &fixture,
                &subject,
                &roots,
                oracle.as_deref(),
            )?);
        }
        if plan.freeze()? != frozen {
            return Err(failure(
                "experiment plan or frozen inputs changed during the run",
            ));
        }
        Ok(ExperimentReport {
            experiment: plan.id.clone(),
            plan_digest: frozen.plan_digest,
            fixture_digest: frozen.fixture_digest,
            pins: plan.pins.clone(),
            run_order: frozen.run_order,
            arms: outcomes,
        })
    }

    fn run_arm(
        &self,
        plan: &ExperimentPlan,
        arm: &ExperimentArm,
        fixture: &Path,
        subject: &Path,
        roots: &RunRoots,
        oracle: Option<&Path>,
    ) -> Result<ArmOutcome> {
        let workspace = create_exclusive_directory(&roots.workspaces, &arm.id, "arm workspace")?;
        copy_tree(subject, &workspace.join("subject"))?;
        copy_fixture_file(fixture, &arm.prompt_file, &workspace)?;
        for file in &arm.contract_files {
            copy_fixture_file(fixture, file, &workspace)?;
        }
        let input_digests = snapshot(&workspace, &[])?;
        let immutable_before = snapshot(&workspace, &plan.allowed_changes)?;
        let subject_before = digest_tree(&workspace.join("subject"))?;
        let agent_executable_digest = digest_file(&arm.executable)?;

        let agent_command = command_record(&arm.executable, &arm.arguments)?;
        let agent_target =
            create_exclusive_directory(&roots.agent_targets, &arm.id, "agent target directory")?;
        let started = Instant::now();
        let mut command = Command::new(&arm.executable);
        command
            .args(&arm.arguments)
            .current_dir(&workspace)
            .env_clear()
            .env("BHCP_EXPERIMENT_MODEL", &plan.pins.model)
            .env("BHCP_EXPERIMENT_REASONING", &plan.pins.reasoning)
            .env("BHCP_EXPERIMENT_SANDBOX", &plan.pins.sandbox)
            .env("BHCP_EXPERIMENT_TOOLCHAIN", &plan.pins.toolchain)
            .env("BHCP_EXPERIMENT_PROMPT", &arm.prompt_file)
            .env("CARGO_TARGET_DIR", &agent_target);
        let output = run_bounded(
            &mut command,
            Duration::from_millis(plan.limits.timeout_millis),
            plan.limits.max_agent_output_bytes,
        )?;
        let agent_elapsed_millis = elapsed_millis(started.elapsed());
        let stdout_digest = digest_bytes(&output.stdout);
        let stderr_digest = digest_bytes(&output.stderr);
        let reject_with_subject = |reason, detail: String, subject_after: String| {
            let mut outcome = rejected_after_run(
                arm,
                reason,
                detail,
                agent_elapsed_millis,
                subject_before.clone(),
                subject_after,
            );
            outcome.agent_command = agent_command.clone();
            outcome.agent_elapsed_millis = agent_elapsed_millis;
            outcome.input_digests = input_digests.clone();
            outcome.agent_executable_digest = agent_executable_digest.clone();
            outcome.agent_stdout_digest = stdout_digest.clone();
            outcome.agent_stderr_digest = stderr_digest.clone();
            outcome
        };
        let subject_after = match digest_tree(&workspace.join("subject")) {
            Ok(digest) => digest,
            Err(error) => {
                return Ok(reject_with_subject(
                    RejectionReason::Contaminated,
                    format!("agent left an invalid subject tree: {}", error.message),
                    "unavailable".to_owned(),
                ));
            }
        };
        let reject =
            |reason, detail: String| reject_with_subject(reason, detail, subject_after.clone());

        if workspace.join("oracle").exists() {
            return Ok(reject(
                RejectionReason::AdaptiveOracle,
                "oracle appeared before the agent stopped".to_owned(),
            ));
        }
        let immutable_after = match snapshot(&workspace, &plan.allowed_changes) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return Ok(reject(
                    RejectionReason::Contaminated,
                    format!("agent left an invalid workspace: {}", error.message),
                ));
            }
        };
        if immutable_after != immutable_before {
            return Ok(reject(
                RejectionReason::Contaminated,
                "agent changed or introduced content outside the allowed paths".to_owned(),
            ));
        }
        if output.timed_out {
            return Ok(reject(
                RejectionReason::Interrupted,
                "agent exceeded the pinned timeout".to_owned(),
            ));
        }
        if output.overflowed {
            return Ok(reject(
                RejectionReason::Incomplete,
                "agent output exceeded the pinned byte limit".to_owned(),
            ));
        }
        if output.orphaned {
            return Ok(reject(
                RejectionReason::Incomplete,
                "agent process group remained active after the driver exited".to_owned(),
            ));
        }
        if !output.status.success() {
            return Ok(reject(
                RejectionReason::Interrupted,
                format!("agent exited with {:?}", output.status.code()),
            ));
        }
        let (reported_pins, metrics) = match parse_agent_result(&output.stdout) {
            Ok(result) => result,
            Err(detail) => {
                return Ok(reject(RejectionReason::Incomplete, detail));
            }
        };
        if reported_pins != plan.pins {
            let mut outcome = reject(
                RejectionReason::Contaminated,
                "agent result does not match the controller pins".to_owned(),
            );
            outcome.metrics = Some(metrics);
            return Ok(outcome);
        }

        let mut withheld_oracle_digest = None;
        if let Some(oracle) = oracle {
            withheld_oracle_digest = Some(digest_tree(oracle)?);
            if !plan.judges.iter().any(|judge| judge.uses_oracle) {
                let mut outcome = reject(
                    RejectionReason::Incomplete,
                    "oracle source is configured without an oracle judge".to_owned(),
                );
                outcome.metrics = Some(metrics);
                return Ok(outcome);
            }
        }

        let mut judges = Vec::with_capacity(plan.judges.len());
        let judge_views =
            create_exclusive_directory(&roots.judge_views, &arm.id, "arm judge view root")?;
        let judge_targets =
            create_exclusive_directory(&roots.judge_targets, &arm.id, "arm judge target root")?;
        let mut judge_contaminated = false;
        for judge in &plan.judges {
            let judge_workspace = judge_views.join(&judge.name);
            copy_tree(&workspace, &judge_workspace)?;
            if judge.uses_oracle {
                let oracle = oracle.ok_or_else(|| {
                    failure(format!(
                        "judge {:?} requires an oracle but none is configured",
                        judge.name
                    ))
                })?;
                copy_tree(oracle, &judge_workspace.join("oracle"))?;
            }
            let judge_target =
                create_exclusive_directory(&judge_targets, &judge.name, "judge target directory")?;
            let judge_executable_directory = judge
                .executable
                .parent()
                .ok_or_else(|| failure("judge executable has no parent directory"))?;
            let judge_path = format!("{}:/usr/bin:/bin", path_text(judge_executable_directory)?);
            let mut command = Command::new(&judge.executable);
            command
                .args(&judge.arguments)
                .current_dir(&judge_workspace)
                .env_clear()
                .env("PATH", judge_path)
                .env("CARGO_NET_OFFLINE", "true")
                .env("CARGO_TARGET_DIR", &judge_target);
            let judge_started = Instant::now();
            let result = run_bounded(
                &mut command,
                Duration::from_millis(plan.limits.timeout_millis),
                plan.limits.max_judge_output_bytes,
            )?;
            let mut captured = String::from_utf8_lossy(&result.stdout).into_owned();
            captured.push_str(&String::from_utf8_lossy(&result.stderr));
            judges.push(JudgeResult {
                name: judge.name.clone(),
                command: command_record(&judge.executable, &judge.arguments)?,
                accepted: result.status.success()
                    && !result.timed_out
                    && !result.overflowed
                    && !result.orphaned,
                exit_code: result.status.code(),
                elapsed_millis: elapsed_millis(judge_started.elapsed()),
                stdout_digest: digest_bytes(&result.stdout),
                stderr_digest: digest_bytes(&result.stderr),
                output: captured,
            });
            let subject_changed = match digest_tree(&judge_workspace.join("subject")) {
                Ok(actual) => actual != subject_after,
                Err(_) => true,
            };
            let oracle_changed = if judge.uses_oracle {
                match &withheld_oracle_digest {
                    Some(expected) => match digest_tree(&judge_workspace.join("oracle")) {
                        Ok(actual) => actual != *expected,
                        Err(_) => true,
                    },
                    None => true,
                }
            } else {
                judge_workspace.join("oracle").exists()
            };
            judge_contaminated |= subject_changed || oracle_changed;
            fs::remove_dir_all(&judge_workspace).map_err(|error| {
                failure(format!("cannot remove ephemeral judge workspace: {error}"))
            })?;
            fs::remove_dir_all(&judge_target).map_err(|error| {
                failure(format!("cannot remove ephemeral judge target: {error}"))
            })?;
        }
        let total_elapsed_millis = elapsed_millis(started.elapsed());
        if judge_contaminated {
            let mut outcome = reject(
                RejectionReason::Contaminated,
                "a judge changed its frozen candidate or oracle view".to_owned(),
            );
            outcome.elapsed_millis = total_elapsed_millis;
            outcome.metrics = Some(metrics);
            outcome.judges = judges;
            return Ok(outcome);
        }
        let accepted = !judges.is_empty() && judges.iter().all(|judge| judge.accepted);
        Ok(ArmOutcome {
            arm: arm.id.clone(),
            status: if accepted {
                SessionStatus::Accepted
            } else {
                SessionStatus::Rejected
            },
            rejection: (!accepted).then_some(RejectionReason::VerificationFailed),
            detail: if accepted {
                "every configured judge accepted the candidate".to_owned()
            } else {
                "one or more configured judges rejected the candidate".to_owned()
            },
            elapsed_millis: total_elapsed_millis,
            metrics: Some(metrics),
            agent_command,
            agent_elapsed_millis,
            input_digests,
            agent_executable_digest,
            agent_stdout_digest: stdout_digest,
            agent_stderr_digest: stderr_digest,
            subject_digest_before: subject_before,
            subject_digest_after: subject_after,
            judges,
        })
    }
}

fn parse_agent_result(bytes: &[u8]) -> std::result::Result<(ExperimentPins, AgentMetrics), String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "agent result is not UTF-8".to_owned())?;
    let mut lines = text.lines();
    if lines.next() != Some("bhcp-agent-result@0") {
        return Err("agent result header is missing".to_owned());
    }
    let mut fields = BTreeMap::new();
    for line in lines {
        let Some((name, value)) = line.split_once('=') else {
            return Err(format!("malformed agent result line {line:?}"));
        };
        if fields.insert(name, value).is_some() {
            return Err(format!("duplicate agent result field {name:?}"));
        }
    }
    if fields.remove("status") != Some("completed")
        || !fields.is_empty() && fields.contains_key("status")
    {
        return Err("agent result is not complete".to_owned());
    }
    let claimed_success = match fields.remove("claimed_success") {
        Some("true") => true,
        Some("false") => false,
        _ => return Err("agent result has no valid final claim".to_owned()),
    };
    let pins = ExperimentPins {
        model: text_field(&mut fields, "model")?,
        reasoning: text_field(&mut fields, "reasoning")?,
        sandbox: text_field(&mut fields, "sandbox")?,
        toolchain: text_field(&mut fields, "toolchain")?,
    };
    let metrics = AgentMetrics {
        input_tokens: metric(&mut fields, "input_tokens")?,
        cached_input_tokens: metric(&mut fields, "cached_input_tokens")?,
        output_tokens: metric(&mut fields, "output_tokens")?,
        reasoning_tokens: metric(&mut fields, "reasoning_tokens")?,
        completed_commands: metric(&mut fields, "completed_commands")?,
        claimed_success,
    };
    if metrics.cached_input_tokens > metrics.input_tokens {
        return Err("cached input tokens exceed total input tokens".to_owned());
    }
    if let Some(field) = fields.keys().next() {
        return Err(format!("unknown agent result field {field:?}"));
    }
    Ok((pins, metrics))
}

fn text_field(
    fields: &mut BTreeMap<&str, &str>,
    name: &str,
) -> std::result::Result<String, String> {
    let value = fields
        .remove(name)
        .ok_or_else(|| format!("agent result is missing {name}"))?;
    if value.is_empty() || value.contains(['\r', '\n']) {
        return Err(format!("agent result has invalid {name}"));
    }
    Ok(value.to_owned())
}

fn metric(fields: &mut BTreeMap<&str, &str>, name: &str) -> std::result::Result<u64, String> {
    fields
        .remove(name)
        .ok_or_else(|| format!("agent result is missing {name}"))?
        .parse()
        .map_err(|_| format!("agent result has invalid {name}"))
}

fn rejected_after_run(
    arm: &ExperimentArm,
    reason: RejectionReason,
    detail: impl Into<String>,
    elapsed_millis: u64,
    before: String,
    after: String,
) -> ArmOutcome {
    let mut outcome = ArmOutcome::rejected(&arm.id, reason, detail);
    outcome.elapsed_millis = elapsed_millis;
    outcome.subject_digest_before = before;
    outcome.subject_digest_after = after;
    outcome
}

fn validate_component(name: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(invalid(format!(
            "{name} must be a safe non-empty identifier"
        )));
    }
    Ok(())
}

fn validate_relative_file(name: &str, path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir
                    | Component::ParentDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err(invalid(format!("{name} must be a project-relative path")));
    }
    Ok(())
}

fn create_exclusive_directory(parent: &Path, name: &str, purpose: &str) -> Result<PathBuf> {
    validate_component(purpose, name)?;
    let path = parent.join(name);
    fs::create_dir(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            failure(format!("{purpose} already exists: {}", path.display()))
        } else {
            failure(format!("cannot create {purpose}: {error}"))
        }
    })?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| failure(format!("cannot inspect created {purpose}: {error}")))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(failure(format!(
            "created {purpose} is not a real directory"
        )));
    }
    let canonical = fs::canonicalize(&path)
        .map_err(|error| failure(format!("cannot resolve created {purpose}: {error}")))?;
    if canonical.parent() != Some(parent) {
        return Err(failure(format!("created {purpose} escaped its parent")));
    }
    Ok(canonical)
}

fn create_relative_parents(root: &Path, relative: &Path) -> Result<PathBuf> {
    let mut current = root.to_owned();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(failure("fixture destination path is not relative"));
        };
        let next = current.join(name);
        match fs::create_dir(&next) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&next).map_err(|inspect_error| {
                    failure(format!("cannot inspect input directory: {inspect_error}"))
                })?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(failure(
                        "fixture destination parent is not a real directory",
                    ));
                }
            }
            Err(error) => {
                return Err(failure(format!("cannot create input directory: {error}")));
            }
        }
        current = next;
    }
    Ok(current)
}

fn copy_regular_file(source: &Path, destination: &Path) -> Result<()> {
    let mut input = fs::File::open(source)
        .map_err(|error| failure(format!("cannot open fixture file: {error}")))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| failure(format!("cannot create fixture file: {error}")))?;
    std::io::copy(&mut input, &mut output)
        .map_err(|error| failure(format!("cannot copy fixture file: {error}")))?;
    Ok(())
}

fn copy_fixture_file(fixture: &Path, relative: &Path, workspace: &Path) -> Result<()> {
    let source = fixture.join(relative);
    let metadata = fs::symlink_metadata(&source).map_err(|error| {
        failure(format!(
            "cannot inspect fixture input {}: {error}",
            source.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(failure(format!(
            "fixture input is not a regular file: {}",
            source.display()
        )));
    }
    let destination = workspace.join(relative);
    let parent = destination
        .parent()
        .ok_or_else(|| failure("fixture input destination has no parent directory"))?;
    let relative_parent = parent
        .strip_prefix(workspace)
        .map_err(|_| failure("fixture input destination escaped its workspace"))?;
    create_relative_parents(workspace, relative_parent)?;
    copy_regular_file(&source, &destination)?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| failure(format!("cannot inspect fixture tree: {error}")))?;
    if metadata.file_type().is_symlink() {
        return Err(failure("fixture trees must not contain symbolic links"));
    }
    if metadata.is_file() {
        let parent = destination
            .parent()
            .ok_or_else(|| failure("fixture destination has no parent directory"))?;
        let parent_metadata = fs::symlink_metadata(parent)
            .map_err(|error| failure(format!("cannot inspect fixture destination: {error}")))?;
        if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
            return Err(failure(
                "fixture destination parent is not a real directory",
            ));
        }
        copy_regular_file(source, destination)?;
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(failure("fixture tree contains an unsupported file type"));
    }
    fs::create_dir(destination)
        .map_err(|error| failure(format!("cannot create fixture directory: {error}")))?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| failure(format!("cannot read fixture directory: {error}")))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| failure(format!("cannot read fixture entry: {error}")))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn snapshot(root: &Path, allowed_changes: &[PathBuf]) -> Result<BTreeMap<PathBuf, String>> {
    let mut snapshot = BTreeMap::new();
    snapshot_entries(root, root, allowed_changes, &mut snapshot)?;
    Ok(snapshot)
}

fn snapshot_entries(
    root: &Path,
    current: &Path,
    allowed_changes: &[PathBuf],
    output: &mut BTreeMap<PathBuf, String>,
) -> Result<()> {
    let mut entries = fs::read_dir(current)
        .map_err(|error| failure(format!("cannot inspect experiment workspace: {error}")))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| failure(format!("cannot inspect experiment entry: {error}")))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|_| failure("workspace entry escaped its root"))?
            .to_owned();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| failure(format!("cannot inspect experiment file: {error}")))?;
        if metadata.file_type().is_symlink() {
            return Err(failure("experiment workspace contains a symbolic link"));
        }
        if metadata.is_dir() {
            output.insert(relative, "directory".to_owned());
            snapshot_entries(root, &path, allowed_changes, output)?;
        } else if metadata.is_file() {
            if !allowed_changes.iter().any(|allowed| allowed == &relative) {
                output.insert(relative, digest_file(&path)?);
            }
        } else {
            return Err(failure(
                "experiment workspace contains an unsupported file type",
            ));
        }
    }
    Ok(())
}

fn digest_tree(root: &Path) -> Result<String> {
    let mut entries = Vec::new();
    collect_tree_entries(root, root, &mut entries)?;
    let entries = entries
        .into_iter()
        .map(|(path, contents)| {
            Ok(Value::map([
                (
                    "kind",
                    Value::Text(
                        if contents.is_some() {
                            "file"
                        } else {
                            "directory"
                        }
                        .to_owned(),
                    ),
                ),
                ("path", Value::Text(path_text(&path)?)),
                ("contents", Value::Bytes(contents.unwrap_or_default())),
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(format_hash(&hash_value(
        &Value::Array(entries),
        HashAlgorithm::default(),
    )?))
}

fn plan_digest(plan: &ExperimentPlan, fixture_digest: &str) -> Result<String> {
    let arms = plan
        .arms
        .iter()
        .map(|arm| {
            Ok(Value::map([
                ("id", Value::Text(arm.id.clone())),
                ("prompt", Value::Text(path_text(&arm.prompt_file)?)),
                (
                    "executable_digest",
                    Value::Text(digest_file(&arm.executable)?),
                ),
                (
                    "arguments",
                    Value::Array(arm.arguments.iter().cloned().map(Value::Text).collect()),
                ),
                (
                    "contracts",
                    Value::Array(
                        arm.contract_files
                            .iter()
                            .map(|path| path_text(path).map(Value::Text))
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ),
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    let judges = plan
        .judges
        .iter()
        .map(|judge| {
            Ok(Value::map([
                ("name", Value::Text(judge.name.clone())),
                ("executable", Value::Text(path_text(&judge.executable)?)),
                (
                    "executable_digest",
                    Value::Text(digest_file(&judge.executable)?),
                ),
                (
                    "arguments",
                    Value::Array(judge.arguments.iter().cloned().map(Value::Text).collect()),
                ),
                ("uses_oracle", Value::Bool(judge.uses_oracle)),
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    let value = Value::map([
        ("kind", Value::Text("experiment-plan@0".to_owned())),
        ("id", Value::Text(plan.id.clone())),
        ("fixture_digest", Value::Text(fixture_digest.to_owned())),
        ("model", Value::Text(plan.pins.model.clone())),
        ("reasoning", Value::Text(plan.pins.reasoning.clone())),
        ("sandbox", Value::Text(plan.pins.sandbox.clone())),
        ("toolchain", Value::Text(plan.pins.toolchain.clone())),
        (
            "timeout_millis",
            Value::Integer(
                i64::try_from(plan.limits.timeout_millis)
                    .map_err(|_| invalid("experiment timeout does not fit the canonical form"))?,
            ),
        ),
        (
            "max_agent_output_bytes",
            Value::Integer(
                i64::try_from(plan.limits.max_agent_output_bytes)
                    .map_err(|_| invalid("agent output limit does not fit the canonical form"))?,
            ),
        ),
        (
            "max_judge_output_bytes",
            Value::Integer(
                i64::try_from(plan.limits.max_judge_output_bytes)
                    .map_err(|_| invalid("judge output limit does not fit the canonical form"))?,
            ),
        ),
        ("arms", Value::Array(arms)),
        ("judges", Value::Array(judges)),
        (
            "allowed_changes",
            Value::Array(
                plan.allowed_changes
                    .iter()
                    .map(|path| path_text(path).map(Value::Text))
                    .collect::<Result<Vec<_>>>()?,
            ),
        ),
        ("withheld_oracle", Value::Bool(plan.oracle_source.is_some())),
    ]);
    Ok(format_hash(&hash_value(&value, HashAlgorithm::default())?))
}

fn path_text(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid("experiment paths must be valid UTF-8"))
}

fn digest_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .map_err(|error| failure(format!("cannot read experiment input: {error}")))?;
    Ok(format_hash(&HashAlgorithm::default().hash(&bytes)))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format_hash(&HashAlgorithm::default().hash(bytes))
}

fn command_record(executable: &Path, arguments: &[String]) -> Result<Vec<String>> {
    let mut command = Vec::with_capacity(arguments.len() + 1);
    command.push(path_text(executable)?);
    command.extend(arguments.iter().cloned());
    Ok(command)
}

fn elapsed_millis(elapsed: Duration) -> u64 {
    elapsed.as_millis().min(u128::from(u64::MAX)) as u64
}

fn markdown_inline(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace(['\r', '\n'], " ")
}

fn markdown_command(command: &[String]) -> String {
    command
        .iter()
        .map(|part| format!("`{}`", markdown_inline(part)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_tree_entries(
    root: &Path,
    current: &Path,
    output: &mut Vec<(PathBuf, Option<Vec<u8>>)>,
) -> Result<()> {
    let mut entries = fs::read_dir(current)
        .map_err(|error| failure(format!("cannot inspect experiment workspace: {error}")))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| failure(format!("cannot inspect experiment entry: {error}")))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| failure(format!("cannot inspect experiment file: {error}")))?;
        if metadata.file_type().is_symlink() {
            return Err(failure("experiment workspace contains a symbolic link"));
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| failure("experiment entry escaped its root"))?
            .to_owned();
        if metadata.is_dir() {
            output.push((relative, None));
            collect_tree_entries(root, &entry.path(), output)?;
        } else if metadata.is_file() {
            let contents = fs::read(entry.path())
                .map_err(|error| failure(format!("cannot read experiment file: {error}")))?;
            output.push((relative, Some(contents)));
        } else {
            return Err(failure(
                "experiment workspace contains an unsupported file type",
            ));
        }
    }
    output.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(())
}

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_PLAN, message)
}

fn failure(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(CONTROLLER_FAILURE, message)
}
