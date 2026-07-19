//! Capability-bounded execution of project-registered verifier processes.

use std::collections::HashSet;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use nix::sys::signal::{Signal, killpg};
#[cfg(unix)]
use nix::unistd::Pid;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::cbor::{decode_deterministic, encode_deterministic};
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::HashAlgorithm;
use crate::kernel::Reason;
use crate::manifest::{VerifierAdapterDeclaration, WorkingScope};
use crate::model::{ContentReference, is_symbol};
use crate::value::Value;
use crate::verification::{VerifierConclusion, VerifierEvidence, VerifierExecution};

const INVALID_ADAPTER: &str = "BHCP7001";
const REQUEST_MEDIA_TYPE: &str = "application/vnd.bhcp.verification-request+cbor";
const RESPONSE_MEDIA_TYPE: &str = "application/vnd.bhcp.verifier-result+cbor";
const REGISTRATION_MEDIA_TYPE: &str = "application/vnd.bhcp.verifier-registration+cbor";
const EXECUTABLE_MEDIA_TYPE: &str = "application/vnd.bhcp.executable";
const POLL_INTERVAL: Duration = Duration::from_millis(2);

pub const MAX_ADAPTER_INPUT_BYTES: usize = 1024 * 1024;
pub const MAX_ADAPTER_OUTPUT_BYTES: usize = 1024 * 1024;
pub const MAX_ADAPTER_STDERR_BYTES: usize = 256 * 1024;
pub const MAX_ADAPTER_EXECUTABLE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AdapterRequest<'a> {
    pub verifier: &'a str,
    pub obligations: &'a [String],
    pub payload: &'a [u8],
    pub effect_ceiling: &'a [String],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterExecutionRecord {
    pub declaration: VerifierAdapterDeclaration,
    pub obligations: Vec<String>,
    pub registration_artifact: ContentReference,
    pub executable_artifact: Option<ContentReference>,
    pub request_artifact: ContentReference,
    pub response_artifact: Option<ContentReference>,
    pub request_bytes: Vec<u8>,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterRun {
    pub execution: VerifierExecution,
    pub record: AdapterExecutionRecord,
}

#[derive(Clone, Debug)]
pub struct VerifierProcessRunner {
    project_root: PathBuf,
    sandbox: SandboxBackend,
}

impl VerifierProcessRunner {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = fs::canonicalize(project_root.as_ref())
            .map_err(|error| invalid(format!("cannot resolve project root: {error}")))?;
        if !project_root.is_dir() {
            return Err(invalid("project root must be a directory"));
        }
        let sandbox = SandboxBackend::discover()?;
        Ok(Self {
            project_root,
            sandbox,
        })
    }

    pub fn run(
        &self,
        declaration: &VerifierAdapterDeclaration,
        request: AdapterRequest<'_>,
        cancellation: &CancellationToken,
    ) -> Result<AdapterRun> {
        validate_request(declaration, request)?;
        let registration_bytes = encode_deterministic(&declaration_value(declaration))?;
        let request_bytes = encode_deterministic(&request_value(request))?;
        let mut record = AdapterExecutionRecord {
            declaration: declaration.clone(),
            obligations: request.obligations.to_vec(),
            registration_artifact: reference(REGISTRATION_MEDIA_TYPE, &registration_bytes),
            executable_artifact: None,
            request_artifact: reference(REQUEST_MEDIA_TYPE, &request_bytes),
            response_artifact: None,
            request_bytes: request_bytes.clone(),
            exit_code: None,
        };

        let executable = match self.resolve_executable(&declaration.executable) {
            Ok(path) => path,
            Err(ResolveError::Missing) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-executable-missing@0",
                    "registered adapter executable does not exist",
                ));
            }
            Err(ResolveError::Escape) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-path-escape@0",
                    "registered adapter executable escapes the project root",
                ));
            }
            Err(ResolveError::Unreadable(message)) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-executable-unreadable@0",
                    message,
                ));
            }
        };
        let executable_metadata = match fs::metadata(&executable) {
            Ok(metadata) if metadata.len() <= MAX_ADAPTER_EXECUTABLE_BYTES => metadata,
            Ok(_) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-executable-limit@0",
                    "registered adapter executable exceeds the artifact limit",
                ));
            }
            Err(error) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-executable-unreadable@0",
                    format!("cannot inspect registered adapter executable: {error}"),
                ));
            }
        };
        let executable_identity = ExecutableIdentity::from_metadata(&executable_metadata);
        let executable_bytes = match fs::read(&executable) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-executable-unreadable@0",
                    format!("cannot read registered adapter executable: {error}"),
                ));
            }
        };
        let captured_identity = match fs::metadata(&executable) {
            Ok(metadata) => ExecutableIdentity::from_metadata(&metadata),
            Err(error) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-executable-changed@0",
                    format!("registered adapter executable changed during capture: {error}"),
                ));
            }
        };
        if captured_identity != executable_identity
            || executable_bytes.len() as u64 != executable_metadata.len()
        {
            return Ok(faulted(
                record,
                "bhcp.fault/adapter-executable-changed@0",
                "registered adapter executable changed while its artifact was captured",
            ));
        }
        record.executable_artifact = Some(reference(EXECUTABLE_MEDIA_TYPE, &executable_bytes));

        if cancellation.is_cancelled() {
            return Ok(unresolved(
                record,
                "bhcp.reason/adapter-cancelled@0",
                "adapter execution was cancelled before process start",
            ));
        }

        let launch_identity = match fs::metadata(&executable) {
            Ok(metadata) => ExecutableIdentity::from_metadata(&metadata),
            Err(error) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-executable-changed@0",
                    format!("registered adapter executable changed before launch: {error}"),
                ));
            }
        };
        if launch_identity != executable_identity {
            return Ok(faulted(
                record,
                "bhcp.fault/adapter-executable-changed@0",
                "registered adapter executable changed before launch",
            ));
        }

        let mut command = self
            .sandbox
            .command(declaration, &executable, &self.project_root);
        command
            .current_dir(&self.project_root)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        command.process_group(0);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-spawn@0",
                    format!("cannot start registered adapter: {error}"),
                ));
            }
        };

        let stdin = child.stdin.take().expect("piped adapter stdin");
        let stdout = child.stdout.take().expect("piped adapter stdout");
        let stderr = child.stderr.take().expect("piped adapter stderr");
        let writer = thread::spawn(move || {
            let mut stdin = stdin;
            stdin.write_all(&request_bytes)
        });
        let output_exceeded = Arc::new(AtomicBool::new(false));
        let stderr_exceeded = Arc::new(AtomicBool::new(false));
        let output_done = Arc::new(AtomicBool::new(false));
        let stderr_done = Arc::new(AtomicBool::new(false));
        let output_reader = bounded_reader(
            stdout,
            MAX_ADAPTER_OUTPUT_BYTES,
            Arc::clone(&output_exceeded),
            Arc::clone(&output_done),
        );
        let stderr_reader = bounded_reader(
            stderr,
            MAX_ADAPTER_STDERR_BYTES,
            Arc::clone(&stderr_exceeded),
            Arc::clone(&stderr_done),
        );

        let deadline = Instant::now() + Duration::from_millis(declaration.timeout_ms);
        let completion = monitor(
            &mut child,
            deadline,
            cancellation,
            &output_exceeded,
            &stderr_exceeded,
            &output_done,
            &stderr_done,
        );
        let input_write = writer
            .join()
            .unwrap_or_else(|_| Err(std::io::Error::other("adapter input writer panicked")));
        let output = output_reader.join().unwrap_or_default();
        let _stderr = stderr_reader.join().unwrap_or_default();

        let completion = if output_exceeded.load(Ordering::Acquire) {
            MonitorResult::OutputLimit
        } else if stderr_exceeded.load(Ordering::Acquire) {
            MonitorResult::StderrLimit
        } else {
            completion
        };

        let status = match completion {
            MonitorResult::Exited(status) => {
                if let Err(error) = input_write {
                    return Ok(faulted(
                        record,
                        "bhcp.fault/adapter-input-write@0",
                        format!("cannot write the complete adapter request: {error}"),
                    ));
                }
                status
            }
            MonitorResult::Cancelled => {
                return Ok(unresolved(
                    record,
                    "bhcp.reason/adapter-cancelled@0",
                    "adapter execution was cancelled",
                ));
            }
            MonitorResult::TimedOut => {
                return Ok(unresolved(
                    record,
                    "bhcp.reason/adapter-timeout@0",
                    "adapter execution exceeded its declared timeout",
                ));
            }
            MonitorResult::OutputLimit => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-output-limit@0",
                    "adapter stdout exceeded the output limit",
                ));
            }
            MonitorResult::StderrLimit => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-stderr-limit@0",
                    "adapter stderr exceeded the diagnostic limit",
                ));
            }
            MonitorResult::WaitFault(message) => {
                return Ok(faulted(record, "bhcp.fault/adapter-wait@0", message));
            }
        };
        record.exit_code = status.code();
        if !status.success() {
            return Ok(faulted(
                record,
                "bhcp.fault/adapter-nonzero-exit@0",
                "adapter process returned a nonzero exit status",
            ));
        }
        record.response_artifact = Some(reference(RESPONSE_MEDIA_TYPE, &output));
        let execution = match parse_response(declaration, &output) {
            Ok(execution) => execution,
            Err(message) => {
                return Ok(faulted(
                    record,
                    "bhcp.fault/adapter-malformed-output@0",
                    message,
                ));
            }
        };
        Ok(AdapterRun { execution, record })
    }

    fn resolve_executable(&self, path: &Path) -> std::result::Result<PathBuf, ResolveError> {
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(ResolveError::Escape);
        }
        let joined = self.project_root.join(path);
        let resolved = fs::canonicalize(joined).map_err(|error| match error.kind() {
            ErrorKind::NotFound => ResolveError::Missing,
            _ => ResolveError::Unreadable(format!(
                "cannot resolve registered adapter executable: {error}"
            )),
        })?;
        if !resolved.starts_with(&self.project_root) {
            return Err(ResolveError::Escape);
        }
        if !resolved.is_file() {
            return Err(ResolveError::Unreadable(
                "registered adapter executable is not a regular file".to_owned(),
            ));
        }
        Ok(resolved)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExecutableIdentity {
    length: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    mode: u32,
    #[cfg(unix)]
    modified_seconds: i64,
    #[cfg(unix)]
    modified_nanoseconds: i64,
}

impl ExecutableIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            length: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(unix)]
            mode: metadata.mode(),
            #[cfg(unix)]
            modified_seconds: metadata.mtime(),
            #[cfg(unix)]
            modified_nanoseconds: metadata.mtime_nsec(),
        }
    }
}

#[derive(Clone, Debug)]
enum SandboxBackend {
    #[cfg(target_os = "macos")]
    MacOs(PathBuf),
    #[cfg(target_os = "linux")]
    LinuxHelper(PathBuf),
}

impl SandboxBackend {
    fn discover() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            let sandbox = Path::new("/usr/bin/sandbox-exec");
            if !sandbox.is_file() {
                return Err(invalid("the required macOS adapter sandbox is unavailable"));
            }
            return Ok(Self::MacOs(packaged_sandbox_helper()?));
        }
        #[cfg(target_os = "linux")]
        {
            return Ok(Self::LinuxHelper(packaged_sandbox_helper()?));
        }
        #[allow(unreachable_code)]
        Err(invalid("this platform has no supported adapter sandbox"))
    }

    fn command(
        &self,
        declaration: &VerifierAdapterDeclaration,
        executable: &Path,
        project_root: &Path,
    ) -> Command {
        #[cfg(target_os = "macos")]
        {
            let Self::MacOs(helper) = self;
            let readable = declaration
                .allowed_effects
                .iter()
                .any(|effect| effect == "bhcp-effect/fs.read@0");
            let writable = declaration
                .allowed_effects
                .iter()
                .any(|effect| effect == "bhcp-effect/fs.write@0");
            let mut profile = String::from(
                "(version 1)\n(allow default)\n(deny network*)\n(deny file-write*)\n\
                 (deny process-exec)\n\
                 (allow process-exec (literal (param \"BHCP_EXECUTABLE\")))\n\
                 (deny file-read* (subpath \"/Users\") (subpath \"/home\") (subpath \"/Volumes\") \
                 (subpath \"/tmp\") (subpath \"/private/tmp\") (subpath \"/private/var/folders\") \
                 (subpath \"/etc\") (subpath \"/private/etc\"))\n\
                 (allow file-read* (literal (param \"BHCP_EXECUTABLE\")))\n",
            );
            if readable {
                profile.push_str("(allow file-read* (subpath (param \"BHCP_PROJECT_ROOT\")))\n");
            }
            if writable {
                profile.push_str("(allow file-write* (subpath (param \"BHCP_PROJECT_ROOT\")))\n");
            }
            let mut command = Command::new(helper);
            command
                .arg("--project-root")
                .arg(project_root)
                .arg("--profile")
                .arg(profile)
                .arg("--")
                .arg(executable)
                .args(&declaration.argv);
            return command;
        }
        #[cfg(target_os = "linux")]
        {
            let Self::LinuxHelper(helper) = self;
            let mut command = Command::new(helper);
            command
                .arg("--project-root")
                .arg(project_root)
                .arg("--effects")
                .arg(declaration.allowed_effects.join(","))
                .arg("--")
                .arg(executable)
                .args(&declaration.argv);
            return command;
        }
        #[allow(unreachable_code)]
        Command::new(executable)
    }
}

fn packaged_sandbox_helper() -> Result<PathBuf> {
    let current = std::env::current_exe()
        .map_err(|error| invalid(format!("cannot locate adapter sandbox helper: {error}")))?;
    let directory = current
        .parent()
        .ok_or_else(|| invalid("cannot locate adapter sandbox helper directory"))?;
    let mut candidates = vec![directory.join("bhcp-adapter-sandbox")];
    if directory.file_name().is_some_and(|name| name == "deps")
        && let Some(parent) = directory.parent()
    {
        candidates.push(parent.join("bhcp-adapter-sandbox"));
    }
    let helper = candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| invalid("the required adapter sandbox helper is unavailable"))?;
    fs::canonicalize(helper)
        .map_err(|error| invalid(format!("cannot resolve adapter sandbox helper: {error}")))
}

enum ResolveError {
    Missing,
    Escape,
    Unreadable(String),
}

enum MonitorResult {
    Exited(ExitStatus),
    Cancelled,
    TimedOut,
    OutputLimit,
    StderrLimit,
    WaitFault(String),
}

fn monitor(
    child: &mut Child,
    deadline: Instant,
    cancellation: &CancellationToken,
    output_exceeded: &AtomicBool,
    stderr_exceeded: &AtomicBool,
    output_done: &AtomicBool,
    stderr_done: &AtomicBool,
) -> MonitorResult {
    let mut exited = None;
    loop {
        let termination = if cancellation.is_cancelled() {
            Some(MonitorResult::Cancelled)
        } else if output_exceeded.load(Ordering::Acquire) {
            Some(MonitorResult::OutputLimit)
        } else if stderr_exceeded.load(Ordering::Acquire) {
            Some(MonitorResult::StderrLimit)
        } else if Instant::now() >= deadline {
            Some(MonitorResult::TimedOut)
        } else {
            None
        };
        if let Some(termination) = termination {
            terminate_process_group(child);
            return termination;
        }
        if exited.is_none() {
            match child.try_wait() {
                Ok(Some(status)) => exited = Some(status),
                Ok(None) => {}
                Err(error) => {
                    terminate_process_group(child);
                    return MonitorResult::WaitFault(format!(
                        "cannot wait for registered adapter: {error}"
                    ));
                }
            }
        }
        if output_done.load(Ordering::Acquire)
            && stderr_done.load(Ordering::Acquire)
            && let Some(status) = exited
        {
            return MonitorResult::Exited(status);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn terminate_process_group(child: &mut Child) {
    #[cfg(unix)]
    {
        let _ = killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn bounded_reader<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
    exceeded: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => {
                    done.store(true, Ordering::Release);
                    return bytes;
                }
                Ok(count) if bytes.len() + count > limit => {
                    exceeded.store(true, Ordering::Release);
                    done.store(true, Ordering::Release);
                    return bytes;
                }
                Ok(count) => bytes.extend_from_slice(&buffer[..count]),
            }
        }
    })
}

fn validate_request(
    declaration: &VerifierAdapterDeclaration,
    request: AdapterRequest<'_>,
) -> Result<()> {
    if request.payload.len() > MAX_ADAPTER_INPUT_BYTES {
        return Err(invalid("adapter payload exceeds the input limit"));
    }
    if request.verifier != declaration.symbol {
        return Err(invalid(
            "adapter request does not match the exact registration symbol",
        ));
    }
    if declaration.input_media_type != REQUEST_MEDIA_TYPE
        || declaration.output_media_type != RESPONSE_MEDIA_TYPE
    {
        return Err(invalid(
            "adapter registration uses an unsupported process protocol media type",
        ));
    }
    if declaration.working_scope != WorkingScope::Project {
        return Err(invalid("adapter working scope is not project-local"));
    }
    if declaration.timeout_ms == 0 || declaration.timeout_ms > 86_400_000 {
        return Err(invalid(
            "adapter timeout must be between 1 and 86400000 milliseconds",
        ));
    }
    if !is_symbol(&declaration.symbol) || !is_evidence_kind(&declaration.evidence_kind) {
        return Err(invalid(
            "adapter registration contains an invalid symbol or evidence kind",
        ));
    }
    if request
        .obligations
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(invalid(
            "adapter obligation targets must be unique and normalized",
        ));
    }
    if request.obligations.iter().any(|item| item.is_empty()) {
        return Err(invalid("adapter obligation targets must not be empty"));
    }
    let ceiling = request.effect_ceiling.iter().collect::<HashSet<_>>();
    const SUPPORTED_EFFECTS: [&str; 4] = [
        "bhcp-effect/clock@0",
        "bhcp-effect/fs.read@0",
        "bhcp-effect/fs.write@0",
        "bhcp-effect/process@0",
    ];
    if declaration
        .allowed_effects
        .iter()
        .any(|effect| !SUPPORTED_EFFECTS.contains(&effect.as_str()))
    {
        return Err(invalid(
            "adapter registration contains an unsupported effect",
        ));
    }
    if declaration
        .allowed_effects
        .iter()
        .any(|effect| !ceiling.contains(effect))
    {
        return Err(invalid(
            "adapter registration exceeds the request effect ceiling",
        ));
    }
    if declaration
        .allowed_effects
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(invalid(
            "adapter allowed effects must be unique and normalized",
        ));
    }
    if declaration
        .argv
        .iter()
        .any(|argument| argument.contains(['\0', '\n', '\r']))
    {
        return Err(invalid("adapter argv contains a control character"));
    }
    Ok(())
}

fn request_value(request: AdapterRequest<'_>) -> Value {
    Value::map([
        ("version", Value::Text("bhcp/adapter-request@0".to_owned())),
        ("verifier", Value::Text(request.verifier.to_owned())),
        (
            "obligations",
            Value::Array(
                request
                    .obligations
                    .iter()
                    .cloned()
                    .map(Value::Text)
                    .collect(),
            ),
        ),
        ("payload", Value::Bytes(request.payload.to_vec())),
    ])
}

fn declaration_value(declaration: &VerifierAdapterDeclaration) -> Value {
    Value::map([
        (
            "version",
            Value::Text("bhcp/adapter-registration@0".to_owned()),
        ),
        ("symbol", Value::Text(declaration.symbol.clone())),
        (
            "executable",
            Value::Text(declaration.executable.to_string_lossy().into_owned()),
        ),
        (
            "argv",
            Value::Array(declaration.argv.iter().cloned().map(Value::Text).collect()),
        ),
        ("working_scope", Value::Text("project".to_owned())),
        (
            "input_media_type",
            Value::Text(declaration.input_media_type.clone()),
        ),
        (
            "output_media_type",
            Value::Text(declaration.output_media_type.clone()),
        ),
        ("timeout_ms", Value::Integer(declaration.timeout_ms as i64)),
        (
            "allowed_effects",
            Value::Array(
                declaration
                    .allowed_effects
                    .iter()
                    .cloned()
                    .map(Value::Text)
                    .collect(),
            ),
        ),
        (
            "evidence_kind",
            Value::Text(declaration.evidence_kind.clone()),
        ),
    ])
}

fn parse_response(
    declaration: &VerifierAdapterDeclaration,
    bytes: &[u8],
) -> std::result::Result<VerifierExecution, String> {
    let value = decode_deterministic(bytes).map_err(|error| error.message)?;
    let Value::Map(entries) = &value else {
        return Err("adapter response must be a deterministic CBOR map".to_owned());
    };
    let version = text_field(&value, "version")?;
    if version != "bhcp/adapter-result@0" {
        return Err("adapter response has an unsupported version".to_owned());
    }
    let state = text_field(&value, "state")?;
    let expected_fields: &[&str] = match state {
        "accepted" | "rejected" => &["media_type", "payload", "state", "trust", "version"],
        "unresolved" | "faulted" => &["reason", "state", "version"],
        _ => return Err("adapter response state is not registered".to_owned()),
    };
    if entries.len() != expected_fields.len()
        || entries
            .iter()
            .any(|(key, _)| !expected_fields.contains(&key.as_str()))
    {
        return Err("adapter response contains missing or unknown fields".to_owned());
    }
    match state {
        "accepted" | "rejected" => {
            let media_type = text_field(&value, "media_type")?.to_owned();
            if media_type.is_empty() {
                return Err("adapter evidence media type must not be empty".to_owned());
            }
            let Value::Bytes(payload) = value
                .get("payload")
                .ok_or("adapter response omits payload")?
            else {
                return Err("adapter evidence payload must be bytes".to_owned());
            };
            let Value::Array(trust_values) =
                value.get("trust").ok_or("adapter response omits trust")?
            else {
                return Err("adapter evidence trust must be an array".to_owned());
            };
            let mut trust = Vec::with_capacity(trust_values.len());
            for item in trust_values {
                let Value::Text(symbol) = item else {
                    return Err("adapter evidence trust entries must be symbols".to_owned());
                };
                if !is_symbol(symbol) {
                    return Err("adapter evidence trust entry is not a symbol-id".to_owned());
                }
                trust.push(symbol.clone());
            }
            let evidence = VerifierEvidence::new(
                declaration.evidence_kind.clone(),
                declaration.symbol.clone(),
                media_type,
                payload.clone(),
                trust,
            );
            if state == "accepted" {
                Ok(VerifierExecution::Completed(VerifierConclusion::Accepted(
                    evidence,
                )))
            } else {
                Ok(VerifierExecution::Completed(VerifierConclusion::Rejected(
                    evidence,
                )))
            }
        }
        "unresolved" => Ok(VerifierExecution::Completed(
            VerifierConclusion::Unresolved {
                reason: reason_field(&value)?,
                evidence: None,
            },
        )),
        "faulted" => Ok(VerifierExecution::Faulted(reason_field(&value)?)),
        _ => unreachable!(),
    }
}

fn text_field<'a>(value: &'a Value, field: &str) -> std::result::Result<&'a str, String> {
    match value.get(field) {
        Some(Value::Text(text)) => Ok(text),
        _ => Err(format!("adapter response field {field:?} must be text")),
    }
}

fn reason_field(value: &Value) -> std::result::Result<Reason, String> {
    let reason = value.get("reason").ok_or("adapter response omits reason")?;
    let Value::Map(entries) = reason else {
        return Err("adapter reason must be a map".to_owned());
    };
    if entries.len() != 2
        || entries
            .iter()
            .any(|(key, _)| key != "code" && key != "message")
    {
        return Err("adapter reason contains missing or unknown fields".to_owned());
    }
    let code = text_field(reason, "code")?;
    if !is_symbol(code) {
        return Err("adapter reason code is not a symbol-id".to_owned());
    }
    Ok(Reason {
        code: code.to_owned(),
        message: text_field(reason, "message")?.to_owned(),
        details: None,
    })
}

fn reference(media_type: &str, bytes: &[u8]) -> ContentReference {
    ContentReference::from_bytes(media_type, bytes, HashAlgorithm::default())
}

fn faulted(record: AdapterExecutionRecord, code: &str, message: impl Into<String>) -> AdapterRun {
    AdapterRun {
        execution: VerifierExecution::Faulted(Reason {
            code: code.to_owned(),
            message: message.into(),
            details: None,
        }),
        record,
    }
}

fn unresolved(
    record: AdapterExecutionRecord,
    code: &str,
    message: impl Into<String>,
) -> AdapterRun {
    AdapterRun {
        execution: VerifierExecution::Completed(VerifierConclusion::Unresolved {
            reason: Reason {
                code: code.to_owned(),
                message: message.into(),
                details: None,
            },
            evidence: None,
        }),
        record,
    }
}

fn is_evidence_kind(value: &str) -> bool {
    matches!(value, "dynamic" | "static" | "manual" | "attestation") || is_symbol(value)
}

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_ADAPTER, message)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::ExecutableIdentity;

    static NEXT_IDENTITY_TEST: AtomicUsize = AtomicUsize::new(1);

    #[test]
    fn executable_identity_detects_same_length_replacement() {
        let directory = std::env::temp_dir().join(format!(
            "bhcp-executable-identity-{}-{}",
            std::process::id(),
            NEXT_IDENTITY_TEST.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let executable = directory.join("adapter");
        let replacement = directory.join("replacement");
        fs::write(&executable, b"first").unwrap();
        fs::write(&replacement, b"other").unwrap();
        let before = ExecutableIdentity::from_metadata(&fs::metadata(&executable).unwrap());
        fs::rename(&replacement, &executable).unwrap();
        let after = ExecutableIdentity::from_metadata(&fs::metadata(&executable).unwrap());
        assert_ne!(before, after);
        fs::remove_dir_all(directory).unwrap();
    }
}
