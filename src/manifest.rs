use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::diagnostic::{Diagnostic, Result};
use crate::hash::HashAlgorithm;
use crate::model::is_symbol;

pub const FILE_NAME: &str = "bhcp-project.toml";

const ADAPTER_TABLE: &str = "[[verifier_adapter]]";
const MAX_TIMEOUT_MS: u64 = 86_400_000;
const ALLOWED_ADAPTER_EFFECTS: [&str; 4] = [
    "bhcp-effect/clock@0",
    "bhcp-effect/fs.read@0",
    "bhcp-effect/fs.write@0",
    "bhcp-effect/process@0",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkingScope {
    Project,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierAdapterDeclaration {
    pub symbol: String,
    pub executable: PathBuf,
    pub argv: Vec<String>,
    pub working_scope: WorkingScope,
    pub input_media_type: String,
    pub output_media_type: String,
    pub timeout_ms: u64,
    pub allowed_effects: Vec<String>,
    pub evidence_kind: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectManifest {
    pub identity_algorithm: HashAlgorithm,
    pub verifier_adapters: Vec<VerifierAdapterDeclaration>,
}

impl ProjectManifest {
    pub fn parse(source: &str, source_name: &str) -> Result<Self> {
        let mut algorithm = None;
        let mut adapters = Vec::new();
        let mut current: Option<AdapterBuilder> = None;

        for (index, raw_line) in source.lines().enumerate() {
            let line_number = index + 1;
            let line = strip_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }
            if line == ADAPTER_TABLE {
                if let Some(builder) = current.take() {
                    adapters.push(builder.finish(source_name)?);
                }
                current = Some(AdapterBuilder::new(line_number));
                continue;
            }
            if line.starts_with('[') {
                return Err(error(
                    format!("unknown project-manifest table {line:?}"),
                    source_name,
                    line_number,
                ));
            }

            let Some((raw_key, raw_value)) = line.split_once('=') else {
                return Err(error(
                    "manifest entry must use key = value",
                    source_name,
                    line_number,
                ));
            };
            let key = raw_key.trim();
            let value = raw_value.trim();
            if key.is_empty() {
                return Err(error(
                    "manifest key must not be empty",
                    source_name,
                    line_number,
                ));
            }

            if let Some(builder) = current.as_mut() {
                builder.set(key, value, source_name, line_number)?;
            } else if key == "identity_algorithm" {
                if algorithm.is_some() {
                    return Err(error(
                        "duplicate identity_algorithm",
                        source_name,
                        line_number,
                    ));
                }
                let value = parse_string(value, source_name, line_number)?;
                algorithm = Some(HashAlgorithm::from_id(&value).map_err(|diagnostic| {
                    Diagnostic::new(
                        diagnostic.code,
                        diagnostic.message,
                        source_name,
                        line_number,
                        1,
                    )
                })?);
            } else {
                return Err(error(
                    format!("unknown project-manifest key {key:?}"),
                    source_name,
                    line_number,
                ));
            }
        }

        if let Some(builder) = current {
            adapters.push(builder.finish(source_name)?);
        }
        adapters.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        for pair in adapters.windows(2) {
            if pair[0].symbol == pair[1].symbol {
                return Err(error(
                    format!("duplicate verifier symbol {:?}", pair[0].symbol),
                    source_name,
                    1,
                ));
            }
        }

        Ok(Self {
            identity_algorithm: algorithm.unwrap_or_default(),
            verifier_adapters: adapters,
        })
    }

    pub fn adapter(&self, symbol: &str) -> Option<&VerifierAdapterDeclaration> {
        self.verifier_adapters
            .binary_search_by_key(&symbol, |adapter| adapter.symbol.as_str())
            .ok()
            .map(|index| &self.verifier_adapters[index])
    }

    pub fn discover(source_path: &Path) -> Result<Self> {
        Self::discover_with_root(source_path).map(|(manifest, _)| manifest)
    }

    pub fn discover_with_root(source_path: &Path) -> Result<(Self, PathBuf)> {
        let mut directory = source_path.parent();
        while let Some(candidate_directory) = directory {
            let parent = candidate_directory.parent();
            let candidate_directory = if candidate_directory.as_os_str().is_empty() {
                Path::new(".")
            } else {
                candidate_directory
            };
            let candidate = candidate_directory.join(FILE_NAME);
            if candidate.is_file() {
                let source = fs::read_to_string(&candidate).map_err(|read_error| {
                    error(read_error.to_string(), &candidate.display().to_string(), 1)
                })?;
                let manifest = Self::parse(&source, &candidate.display().to_string())?;
                let root = fs::canonicalize(candidate_directory).map_err(|read_error| {
                    error(read_error.to_string(), &candidate.display().to_string(), 1)
                })?;
                return Ok((manifest, root));
            }
            directory = parent;
        }
        let parent = source_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let root = fs::canonicalize(parent).map_err(|read_error| {
            error(
                read_error.to_string(),
                &source_path.display().to_string(),
                1,
            )
        })?;
        Ok((Self::default(), root))
    }
}

#[derive(Default)]
struct AdapterBuilder {
    start_line: usize,
    symbol: Option<String>,
    executable: Option<PathBuf>,
    argv: Option<Vec<String>>,
    working_scope: Option<WorkingScope>,
    input_media_type: Option<String>,
    output_media_type: Option<String>,
    timeout_ms: Option<u64>,
    allowed_effects: Option<Vec<String>>,
    evidence_kind: Option<String>,
}

impl AdapterBuilder {
    fn new(start_line: usize) -> Self {
        Self {
            start_line,
            ..Self::default()
        }
    }

    fn set(&mut self, key: &str, value: &str, source: &str, line: usize) -> Result<()> {
        match key {
            "symbol" => {
                let value = parse_string(value, source, line)?;
                if !is_symbol(&value) {
                    return Err(error("verifier symbol must be a symbol-id", source, line));
                }
                set_once(&mut self.symbol, value, key, source, line)
            }
            "executable" => {
                let value = parse_string(value, source, line)?;
                let executable = validate_executable(&value, source, line)?;
                set_once(&mut self.executable, executable, key, source, line)
            }
            "argv" => {
                let values = parse_string_array(value, source, line)?;
                if values
                    .iter()
                    .any(|argument| argument.contains(['\0', '\n', '\r']))
                {
                    return Err(error(
                        "verifier argv contains a control character",
                        source,
                        line,
                    ));
                }
                set_once(&mut self.argv, values, key, source, line)
            }
            "working_scope" => {
                let value = parse_string(value, source, line)?;
                if value != "project" {
                    return Err(error(
                        "working_scope must be exactly \"project\"",
                        source,
                        line,
                    ));
                }
                set_once(
                    &mut self.working_scope,
                    WorkingScope::Project,
                    key,
                    source,
                    line,
                )
            }
            "input_media_type" => {
                let value = parse_string(value, source, line)?;
                validate_media_type(&value, source, line)?;
                set_once(&mut self.input_media_type, value, key, source, line)
            }
            "output_media_type" => {
                let value = parse_string(value, source, line)?;
                validate_media_type(&value, source, line)?;
                set_once(&mut self.output_media_type, value, key, source, line)
            }
            "timeout_ms" => {
                let timeout = value
                    .parse::<u64>()
                    .map_err(|_| error("timeout_ms must be an unquoted integer", source, line))?;
                if timeout == 0 || timeout > MAX_TIMEOUT_MS {
                    return Err(error(
                        "timeout_ms must be between 1 and 86400000",
                        source,
                        line,
                    ));
                }
                set_once(&mut self.timeout_ms, timeout, key, source, line)
            }
            "allowed_effects" => {
                let mut effects = parse_string_array(value, source, line)?;
                for effect in &effects {
                    if !ALLOWED_ADAPTER_EFFECTS.contains(&effect.as_str()) {
                        return Err(error(
                            format!("adapter effect {effect:?} is not locally permitted"),
                            source,
                            line,
                        ));
                    }
                }
                let unique = effects.iter().collect::<HashSet<_>>();
                if unique.len() != effects.len() {
                    return Err(error("duplicate adapter effect", source, line));
                }
                effects.sort();
                set_once(&mut self.allowed_effects, effects, key, source, line)
            }
            "evidence_kind" => {
                let value = parse_string(value, source, line)?;
                if !is_evidence_kind(&value) {
                    return Err(error(
                        "evidence_kind must be a registered class or symbol-id",
                        source,
                        line,
                    ));
                }
                set_once(&mut self.evidence_kind, value, key, source, line)
            }
            _ => Err(error(
                format!("unknown verifier-adapter key {key:?}"),
                source,
                line,
            )),
        }
    }

    fn finish(self, source: &str) -> Result<VerifierAdapterDeclaration> {
        Ok(VerifierAdapterDeclaration {
            symbol: required(self.symbol, "symbol", source, self.start_line)?,
            executable: required(self.executable, "executable", source, self.start_line)?,
            argv: required(self.argv, "argv", source, self.start_line)?,
            working_scope: required(self.working_scope, "working_scope", source, self.start_line)?,
            input_media_type: required(
                self.input_media_type,
                "input_media_type",
                source,
                self.start_line,
            )?,
            output_media_type: required(
                self.output_media_type,
                "output_media_type",
                source,
                self.start_line,
            )?,
            timeout_ms: required(self.timeout_ms, "timeout_ms", source, self.start_line)?,
            allowed_effects: required(
                self.allowed_effects,
                "allowed_effects",
                source,
                self.start_line,
            )?,
            evidence_kind: required(self.evidence_kind, "evidence_kind", source, self.start_line)?,
        })
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, key: &str, source: &str, line: usize) -> Result<()> {
    if slot.is_some() {
        return Err(error(format!("duplicate {key}"), source, line));
    }
    *slot = Some(value);
    Ok(())
}

fn required<T>(slot: Option<T>, key: &str, source: &str, line: usize) -> Result<T> {
    slot.ok_or_else(|| error(format!("verifier adapter requires {key}"), source, line))
}

fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' && quoted {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if character == '#' && !quoted {
            return &line[..index];
        }
    }
    line
}

fn parse_string(value: &str, source: &str, line: usize) -> Result<String> {
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(error(
            "manifest value must be a quoted string",
            source,
            line,
        ));
    }
    let mut result = String::new();
    let mut characters = value[1..value.len() - 1].chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            if character == '"' || character.is_control() {
                return Err(error(
                    "manifest string contains an unescaped quote or control character",
                    source,
                    line,
                ));
            }
            result.push(character);
            continue;
        }
        let Some(escaped) = characters.next() else {
            return Err(error("unterminated manifest escape", source, line));
        };
        match escaped {
            '"' | '\\' => result.push(escaped),
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            _ => return Err(error("unsupported manifest escape", source, line)),
        }
    }
    Ok(result)
}

fn parse_string_array(value: &str, source: &str, line: usize) -> Result<Vec<String>> {
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(error("manifest value must be a string array", source, line));
    }
    let inner = value[1..value.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    for (index, character) in inner.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' && quoted {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if character == ',' && !quoted {
            entries.push(parse_string(inner[start..index].trim(), source, line)?);
            start = index + 1;
        }
    }
    if quoted || escaped {
        return Err(error("unterminated manifest string array", source, line));
    }
    entries.push(parse_string(inner[start..].trim(), source, line)?);
    Ok(entries)
}

fn validate_executable(value: &str, source: &str, line: usize) -> Result<PathBuf> {
    if value.is_empty()
        || value.contains([
            '\\', ':', '\0', '\n', '\r', ' ', '\t', '|', '&', ';', '$', '`', '>', '<',
        ])
    {
        return Err(error(
            "executable must be one project-relative path, not a shell string",
            source,
            line,
        ));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(error(
            "executable must stay within the project root",
            source,
            line,
        ));
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let shell_name = file_name.strip_suffix(".exe").unwrap_or(&file_name);
    if matches!(
        shell_name,
        "sh" | "bash" | "zsh" | "fish" | "nu" | "cmd" | "powershell" | "pwsh"
    ) {
        return Err(error(
            "verifier adapter must not invoke a shell",
            source,
            line,
        ));
    }
    Ok(path.to_owned())
}

fn validate_media_type(value: &str, source: &str, line: usize) -> Result<()> {
    let Some((kind, subtype)) = value.split_once('/') else {
        return Err(error("invalid adapter media type", source, line));
    };
    if !is_media_type_token(kind) || !is_media_type_token(subtype) {
        return Err(error("invalid adapter media type", source, line));
    }
    Ok(())
}

fn is_media_type_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
                )
        })
}

fn is_evidence_kind(value: &str) -> bool {
    matches!(
        value,
        "formal"
            | "static"
            | "empirical"
            | "statistical"
            | "model-judged"
            | "human-approved"
            | "unresolved"
    ) || is_symbol(value)
}

fn error(message: impl Into<String>, source: &str, line: usize) -> Diagnostic {
    Diagnostic::new("BHCP6002", message, source, line, 1)
}

pub fn manifest_path(root: &Path) -> PathBuf {
    root.join(FILE_NAME)
}
