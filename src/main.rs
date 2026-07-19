use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use bhcp::adapter::{CancellationToken, VerifierProcessRunner};
use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::formatting::format_source_bytes_with_profile_registry_and_algorithm;
use bhcp::hash::{HashAlgorithm, format_hash};
use bhcp::inspection::render_artifact;
use bhcp::manifest::ProjectManifest;
use bhcp::model::{ClauseKind, ContentReference, SemanticIrDocument};
use bhcp::pipeline::{
    compile_source_bytes_with_algorithm, parse_policy_source_bytes_with_algorithm,
    parse_source_bytes_with_algorithm,
};
use bhcp::policy::{PolicyDocument, SourcePolicyDocument, compose_policies};
use bhcp::profile::{PresentationDocument, ProfileRegistry};
use bhcp::schema::validate_root;
use bhcp::value::Value;
use bhcp::verification::{
    VerificationDecision, VerificationRequest, VerificationState, VerifierRegistry,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn run() -> Result<(), (u8, String)> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|argument| argument == "policy")
    {
        return run_policy(&arguments[1..]);
    }
    if arguments
        .first()
        .is_some_and(|argument| argument == "format")
    {
        return format_source_file(&arguments[1..]);
    }
    if arguments
        .first()
        .is_some_and(|argument| argument == "verify")
    {
        return verify_candidate(&arguments[1..]);
    }
    if arguments.len() != 2
        || !matches!(
            arguments[0].as_str(),
            "parse" | "lower" | "inspect" | "hash"
        )
    {
        return Err((
            2,
            "usage: bhcp <parse|lower|inspect|hash> <source-or-cbor-file> | bhcp format <source-file> [syntax-profile-or-policy-cbor]... | bhcp verify <source-file> <goal> <candidate-cbor> <subject-file> <produced-at>".to_owned(),
        ));
    }
    let command = &arguments[0];
    let file = &arguments[1];

    if command == "inspect"
        && Path::new(file)
            .extension()
            .is_some_and(|value| value == "cbor")
    {
        let bytes = fs::read(file).map_err(|error| (1, format!("{file}: {error}")))?;
        let artifact = decode_deterministic(&bytes).map_err(|error| (1, error.to_string()))?;
        let kind = artifact
            .kind()
            .ok_or_else(|| (1, "BHCP5002: artifact has no root kind".to_owned()))?;
        validate_root(&artifact, kind).map_err(|error| (1, error.to_string()))?;
        print!("{}", render_artifact(&artifact, Some(file)));
        return Ok(());
    }

    let source = fs::read(file).map_err(|error| (1, format!("{file}: {error}")))?;
    let manifest =
        ProjectManifest::discover(Path::new(file)).map_err(|error| (1, error.to_string()))?;
    if command == "parse" {
        let ast = parse_source_bytes_with_algorithm(&source, file, manifest.identity_algorithm)
            .map_err(|error| (1, error.to_string()))?;
        let value = ast.to_value(true);
        validate_root(&value, "canonical-ast").map_err(|error| (1, error.to_string()))?;
        let bytes = encode_deterministic(&value).map_err(|error| (1, error.to_string()))?;
        return write_stdout(&bytes);
    }

    let compiled = compile_source_bytes_with_algorithm(&source, file, manifest.identity_algorithm)
        .map_err(|error| (1, error.to_string()))?;
    if command == "lower" {
        validate_root(&compiled.ir.to_value(true), "semantic-ir")
            .map_err(|error| (1, error.to_string()))?;
        write_stdout(&compiled.ir_bytes)
    } else if command == "hash" {
        println!("{}", format_hash(&compiled.semantic_hash));
        Ok(())
    } else {
        print!(
            "{}",
            render_artifact(&compiled.ir.to_value(true), Some(file))
        );
        Ok(())
    }
}

fn verify_candidate(arguments: &[String]) -> Result<(), (u8, String)> {
    let [source_file, goal, candidate_file, subject_file, produced_at] = arguments else {
        return Err((
            2,
            "usage: bhcp verify <source-file> <goal> <candidate-cbor> <subject-file> <produced-at>"
                .to_owned(),
        ));
    };
    let source = fs::read(source_file).map_err(|error| (1, format!("{source_file}: {error}")))?;
    let (manifest, project_root) = ProjectManifest::discover_with_root(Path::new(source_file))
        .map_err(|error| (1, error.to_string()))?;
    let compilation =
        compile_source_bytes_with_algorithm(&source, source_file, manifest.identity_algorithm)
            .map_err(|error| (1, error.to_string()))?;
    let candidate_bytes =
        fs::read(candidate_file).map_err(|error| (1, format!("{candidate_file}: {error}")))?;
    let candidate =
        decode_deterministic(&candidate_bytes).map_err(|error| (1, error.to_string()))?;
    let Value::Map(candidate_fields) = &candidate else {
        return Err((
            1,
            "BHCP7001: verification candidate must be a map".to_owned(),
        ));
    };
    if candidate_fields.len() != 2
        || candidate_fields
            .iter()
            .any(|(key, _)| !matches!(key.as_str(), "input" | "output"))
    {
        return Err((
            1,
            "BHCP7001: verification candidate requires only input and output".to_owned(),
        ));
    }
    let input = candidate
        .get("input")
        .ok_or_else(|| (1, "BHCP7001: verification candidate omits input".to_owned()))?;
    let output = candidate.get("output").ok_or_else(|| {
        (
            1,
            "BHCP7001: verification candidate omits output".to_owned(),
        )
    })?;
    let effect_ceiling = adapter_effect_ceiling(&compilation.ir, goal)?;
    let subject_bytes =
        fs::read(subject_file).map_err(|error| (1, format!("{subject_file}: {error}")))?;
    let subject = ContentReference::from_bytes(
        "application/vnd.bhcp.subject-source",
        &subject_bytes,
        manifest.identity_algorithm,
    );
    let execution_graph_bytes = encode_deterministic(&Value::map([
        ("goal", Value::Text(goal.clone())),
        ("subject", subject.to_value()),
    ]))
    .map_err(|error| (1, error.to_string()))?;
    let execution_graph = ContentReference::from_bytes(
        "application/vnd.bhcp.execution-graph+cbor",
        &execution_graph_bytes,
        manifest.identity_algorithm,
    );

    let mut registry = VerifierRegistry::new();
    for declaration in &manifest.verifier_adapters {
        registry
            .register_adapter(
                VerifierProcessRunner::new(&project_root)
                    .map_err(|error| (1, error.to_string()))?,
                declaration.clone(),
                effect_ceiling.clone(),
                CancellationToken::new(),
            )
            .map_err(|error| (1, error.to_string()))?;
    }
    let report = registry
        .verify(VerificationRequest {
            compilation: &compilation,
            goal,
            input,
            output,
            subject,
            execution_graph,
            produced_at,
        })
        .map_err(|error| (1, error.to_string()))?;
    write_stdout(&report.bundle_bytes)?;
    match report.state {
        VerificationState::Completed(VerificationDecision::Accepted) => Ok(()),
        VerificationState::Completed(VerificationDecision::Rejected) => Err((
            3,
            "BHCP7001: verification rejected the candidate".to_owned(),
        )),
        VerificationState::Completed(VerificationDecision::Unresolved) => Err((
            4,
            "BHCP7001: verification left required evidence unresolved".to_owned(),
        )),
        VerificationState::Faulted(_) => Err((
            5,
            "BHCP7001: verification encountered a verifier fault".to_owned(),
        )),
    }
}

fn adapter_effect_ceiling(
    ir: &SemanticIrDocument,
    goal_selector: &str,
) -> Result<Vec<String>, (u8, String)> {
    let goal = ir
        .goals
        .iter()
        .find(|goal| goal.symbol == goal_selector || goal.id == goal_selector)
        .ok_or_else(|| {
            (
                1,
                format!("BHCP7001: verification goal {goal_selector:?} does not exist"),
            )
        })?;
    let mut allowed = BTreeSet::new();
    let mut forbidden = BTreeSet::new();
    for clause in &goal.clauses {
        let ClauseKind::Authority { kind, effects } = &clause.kind else {
            continue;
        };
        for effect in effects {
            let Some(local) = local_adapter_effect(&effect.id) else {
                continue;
            };
            if *kind == "forbids" {
                forbidden.insert(local);
            } else if *kind == "allows"
                && effect.resource.is_none()
                && effect.parameters.is_empty()
            {
                allowed.insert(local);
            }
        }
    }
    allowed.retain(|effect| !forbidden.contains(effect));
    Ok(allowed.into_iter().map(str::to_owned).collect())
}

fn local_adapter_effect(effect: &str) -> Option<&'static str> {
    match effect {
        "bhcp-effect/clock@0" => Some("bhcp-effect/clock@0"),
        "bhcp-effect/fs-read@0" => Some("bhcp-effect/fs.read@0"),
        "bhcp-effect/fs-write@0" => Some("bhcp-effect/fs.write@0"),
        "bhcp-effect/process@0" => Some("bhcp-effect/process@0"),
        _ => None,
    }
}

fn format_source_file(arguments: &[String]) -> Result<(), (u8, String)> {
    let Some((source_file, registry_files)) = arguments.split_first() else {
        return Err((
            2,
            "usage: bhcp format <source-file> [syntax-profile-or-policy-cbor]...".to_owned(),
        ));
    };
    let mut registry = ProfileRegistry::new();
    for file in registry_files {
        let bytes = fs::read(file).map_err(|error| (1, format!("{file}: {error}")))?;
        let value = decode_deterministic(&bytes).map_err(|error| (1, error.to_string()))?;
        match value.kind() {
            Some("syntax" | "profile") => {
                match PresentationDocument::from_value(&value)
                    .map_err(|error| (1, error.to_string()))?
                {
                    PresentationDocument::Syntax(document) => registry
                        .register_syntax(document)
                        .map_err(|error| (1, error.to_string()))?,
                    PresentationDocument::Profile(document) => registry
                        .register_profile(document)
                        .map_err(|error| (1, error.to_string()))?,
                }
            }
            Some("policy") => {
                match PolicyDocument::from_value(&value).map_err(|error| (1, error.to_string()))? {
                    PolicyDocument::Source(document) => registry
                        .register_policy(document)
                        .map_err(|error| (1, error.to_string()))?,
                    PolicyDocument::Effective(_) => {
                        return Err((
                            1,
                            "BHCP9004: formatter registry accepts source policy artifacts only"
                                .to_owned(),
                        ));
                    }
                }
            }
            kind => {
                return Err((
                    1,
                    format!(
                        "BHCP9004: formatter registry input {file:?} has unsupported root kind {kind:?}"
                    ),
                ));
            }
        }
    }
    let source = fs::read(source_file).map_err(|error| (1, format!("{source_file}: {error}")))?;
    let manifest = ProjectManifest::discover(Path::new(source_file))
        .map_err(|error| (1, error.to_string()))?;
    let formatted = format_source_bytes_with_profile_registry_and_algorithm(
        &source,
        source_file,
        &registry,
        manifest.identity_algorithm,
    )
    .map_err(|error| (1, error.to_string()))?;
    write_stdout(formatted.as_bytes())
}

fn run_policy(arguments: &[String]) -> Result<(), (u8, String)> {
    match arguments {
        [command, files @ ..] if command == "compose" && !files.is_empty() => {
            compose_policy_files(files)
        }
        [command, file] if command == "inspect" => inspect_policy_file(file),
        _ => Err((
            2,
            "usage: bhcp policy <compose <ordered-policy-file>...|inspect <policy-file>>"
                .to_owned(),
        )),
    }
}

fn compose_policy_files(files: &[String]) -> Result<(), (u8, String)> {
    let mut sources = Vec::new();
    let mut algorithm = None;
    for file in files {
        let (mut documents, input_algorithm) = load_source_policies(file)?;
        if algorithm
            .replace(input_algorithm)
            .is_some_and(|selected| selected != input_algorithm)
        {
            return Err((
                1,
                "BHCP8110: policy inputs select different identity algorithms".to_owned(),
            ));
        }
        sources.append(&mut documents);
    }
    reject_unsupported_features(&sources)?;
    if sources.windows(2).any(|pair| pair[0].layer > pair[1].layer) {
        return Err((
            1,
            "BHCP8110: policy inputs must follow organization-to-user order".to_owned(),
        ));
    }
    let effective = compose_policies(&sources, algorithm.unwrap_or_default())
        .map_err(|error| (1, error.to_string()))?;
    let document = PolicyDocument::Effective(effective);
    let value = document.to_value(true);
    validate_root(&value, "policy").map_err(|error| (1, error.to_string()))?;
    let bytes = document
        .to_cbor(true)
        .map_err(|error| (1, error.to_string()))?;
    write_stdout(&bytes)
}

fn inspect_policy_file(file: &str) -> Result<(), (u8, String)> {
    let documents = if is_cbor(file) {
        let bytes = fs::read(file).map_err(|error| (1, format!("{file}: {error}")))?;
        vec![PolicyDocument::from_cbor(&bytes).map_err(|error| (1, error.to_string()))?]
    } else {
        let source = fs::read(file).map_err(|error| (1, format!("{file}: {error}")))?;
        let manifest =
            ProjectManifest::discover(Path::new(file)).map_err(|error| (1, error.to_string()))?;
        parse_policy_source_bytes_with_algorithm(&source, file, manifest.identity_algorithm)
            .map_err(|error| (1, error.to_string()))?
            .documents
            .into_iter()
            .map(PolicyDocument::Source)
            .collect()
    };
    let mut output = String::new();
    for document in &documents {
        reject_document_features(document)?;
        document
            .validate()
            .map_err(|error| (1, error.to_string()))?;
        let value = document.to_value(true);
        validate_root(&value, "policy").map_err(|error| (1, error.to_string()))?;
        output.push_str(&render_artifact(&value, Some(file)));
    }
    write_stdout(output.as_bytes())
}

fn load_source_policies(
    file: &str,
) -> Result<(Vec<SourcePolicyDocument>, HashAlgorithm), (u8, String)> {
    if is_cbor(file) {
        let bytes = fs::read(file).map_err(|error| (1, format!("{file}: {error}")))?;
        let document = PolicyDocument::from_cbor(&bytes).map_err(|error| (1, error.to_string()))?;
        return match document {
            PolicyDocument::Source(source) => {
                let algorithm = source
                    .header
                    .semantic_id
                    .as_ref()
                    .or(source.header.artifact_id.as_ref())
                    .map(|identity| HashAlgorithm::from_id(&identity.algorithm))
                    .transpose()
                    .map_err(|error| (1, error.to_string()))?
                    .unwrap_or_default();
                Ok((vec![source], algorithm))
            }
            PolicyDocument::Effective(_) => Err((
                1,
                "BHCP8001: policy compose accepts source policy artifacts only".to_owned(),
            )),
        };
    }
    let source = fs::read(file).map_err(|error| (1, format!("{file}: {error}")))?;
    let manifest =
        ProjectManifest::discover(Path::new(file)).map_err(|error| (1, error.to_string()))?;
    let parsed =
        parse_policy_source_bytes_with_algorithm(&source, file, manifest.identity_algorithm)
            .map_err(|error| (1, error.to_string()))?;
    Ok((parsed.documents, manifest.identity_algorithm))
}

fn reject_unsupported_features(sources: &[SourcePolicyDocument]) -> Result<(), (u8, String)> {
    for source in sources {
        if let Some(feature) = source.header.features.first() {
            return Err((
                1,
                format!("BHCP8001: unsupported policy feature {feature:?}"),
            ));
        }
    }
    Ok(())
}

fn reject_document_features(document: &PolicyDocument) -> Result<(), (u8, String)> {
    let features = match document {
        PolicyDocument::Source(document) => &document.header.features,
        PolicyDocument::Effective(document) => &document.header.features,
    };
    if let Some(feature) = features.first() {
        return Err((
            1,
            format!("BHCP8001: unsupported policy feature {feature:?}"),
        ));
    }
    Ok(())
}

fn is_cbor(file: &str) -> bool {
    Path::new(file)
        .extension()
        .is_some_and(|extension| extension == "cbor")
}

fn write_stdout(bytes: &[u8]) -> Result<(), (u8, String)> {
    io::stdout()
        .lock()
        .write_all(bytes)
        .map_err(|error| (1, format!("stdout: {error}")))
}
