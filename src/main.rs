use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::hash::{HashAlgorithm, format_hash};
use bhcp::inspection::render_artifact;
use bhcp::manifest::ProjectManifest;
use bhcp::pipeline::{
    compile_source_with_algorithm, parse_policy_source_with_algorithm, parse_source_with_algorithm,
};
use bhcp::policy::{PolicyDocument, SourcePolicyDocument, compose_policies};
use bhcp::schema::validate_root;

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
    if arguments.len() != 2
        || !matches!(
            arguments[0].as_str(),
            "parse" | "lower" | "inspect" | "hash"
        )
    {
        return Err((
            2,
            "usage: bhcp <parse|lower|inspect|hash> <source-or-cbor-file>".to_owned(),
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

    let source = fs::read_to_string(file).map_err(|error| (1, format!("{file}: {error}")))?;
    let manifest =
        ProjectManifest::discover(Path::new(file)).map_err(|error| (1, error.to_string()))?;
    if command == "parse" {
        let ast = parse_source_with_algorithm(&source, file, manifest.identity_algorithm)
            .map_err(|error| (1, error.to_string()))?;
        let value = ast.to_value(true);
        validate_root(&value, "canonical-ast").map_err(|error| (1, error.to_string()))?;
        let bytes = encode_deterministic(&value).map_err(|error| (1, error.to_string()))?;
        return write_stdout(&bytes);
    }

    let compiled = compile_source_with_algorithm(&source, file, manifest.identity_algorithm)
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
        let source = fs::read_to_string(file).map_err(|error| (1, format!("{file}: {error}")))?;
        let manifest =
            ProjectManifest::discover(Path::new(file)).map_err(|error| (1, error.to_string()))?;
        parse_policy_source_with_algorithm(&source, file, manifest.identity_algorithm)
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
    let source = fs::read_to_string(file).map_err(|error| (1, format!("{file}: {error}")))?;
    let manifest =
        ProjectManifest::discover(Path::new(file)).map_err(|error| (1, error.to_string()))?;
    let parsed = parse_policy_source_with_algorithm(&source, file, manifest.identity_algorithm)
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
