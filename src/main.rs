use std::env;
use std::fs;
use std::process::ExitCode;

use bhcp::hash::format_hash;
use bhcp::manifest::ProjectManifest;
use bhcp::pipeline::{compile_source_with_algorithm, parse_source_with_algorithm};

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
            "usage: bhcp <parse|lower|inspect|hash> <file>".to_owned(),
        ));
    }
    let command = &arguments[0];
    let file = &arguments[1];
    let source = fs::read_to_string(file).map_err(|error| (1, format!("{file}: {error}")))?;
    let manifest = ProjectManifest::discover(std::path::Path::new(file))
        .map_err(|error| (1, error.to_string()))?;
    if command == "parse" {
        let ast = parse_source_with_algorithm(&source, file, manifest.identity_algorithm)
            .map_err(|error| (1, error.to_string()))?;
        println!("{}", ast.to_value(true).to_json_pretty());
        return Ok(());
    }
    let compiled = compile_source_with_algorithm(&source, file, manifest.identity_algorithm)
        .map_err(|error| (1, error.to_string()))?;
    if command == "lower" {
        println!("{}", compiled.ir.to_value(true).to_json_pretty());
    } else if command == "hash" {
        println!("{}", format_hash(&compiled.semantic_hash));
    } else {
        println!("{{");
        println!("  \"source\": {:?},", file);
        println!("  \"profile\": \"bhcp/canonical@0\",");
        println!("  \"goals\": {},", compiled.ir.goals.len());
        println!("  \"type_mode\": \"infer-strict\",");
        println!(
            "  \"identity_algorithm\": {:?},",
            manifest.identity_algorithm.id()
        );
        println!(
            "  \"semantic_id\": {:?},",
            format_hash(&compiled.semantic_hash)
        );
        println!("  \"artifact_id\": {:?},", format_hash(&compiled.ir_hash));
        println!("  \"canonical_ast_bytes\": {},", compiled.ast_bytes.len());
        println!("  \"semantic_ir_bytes\": {}", compiled.ir_bytes.len());
        println!("}}");
    }
    Ok(())
}
