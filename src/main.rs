use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::hash::format_hash;
use bhcp::inspection::render_artifact;
use bhcp::manifest::ProjectManifest;
use bhcp::pipeline::{compile_source_with_algorithm, parse_source_with_algorithm};
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

fn write_stdout(bytes: &[u8]) -> Result<(), (u8, String)> {
    io::stdout()
        .lock()
        .write_all(bytes)
        .map_err(|error| (1, format!("stdout: {error}")))
}
