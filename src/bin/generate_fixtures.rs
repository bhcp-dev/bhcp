use std::fs;
use std::path::PathBuf;

use bhcp::hash::format_hash;
use bhcp::pipeline::compile_source;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("conformance/v0/fixtures");
    let source_path = directory.join("canonical-simple.bhcp");
    let source = fs::read_to_string(&source_path)?;
    let compiled = compile_source(&source, source_path.to_str().unwrap())?;
    fs::write(
        directory.join("canonical-simple.ast.cbor"),
        &compiled.ast_bytes,
    )?;
    fs::write(
        directory.join("canonical-simple.ir.cbor"),
        &compiled.ir_bytes,
    )?;
    fs::write(
        directory.join("canonical-simple.expected.json"),
        format!(
            "{{\n  \"semantic_id\": {:?},\n  \"ast_bytes\": {},\n  \"ir_bytes\": {}\n}}\n",
            format_hash(&compiled.semantic_hash),
            compiled.ast_bytes.len(),
            compiled.ir_bytes.len(),
        ),
    )?;
    Ok(())
}
