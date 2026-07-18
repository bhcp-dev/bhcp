use std::fs;
use std::path::PathBuf;

use bhcp::pipeline::compile_source;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("conformance/v0/fixtures");
    for name in ["canonical-simple", "canonical-all"] {
        let source_path = directory.join(format!("{name}.bhcp"));
        let source = fs::read_to_string(&source_path)?;
        let compiled = compile_source(&source, source_path.to_str().unwrap())?;
        fs::write(
            directory.join(format!("{name}.ast.cbor")),
            &compiled.ast_bytes,
        )?;
        fs::write(
            directory.join(format!("{name}.ir.cbor")),
            &compiled.ir_bytes,
        )?;
    }
    Ok(())
}
