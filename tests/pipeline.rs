use std::fs;
use std::path::PathBuf;

use bhcp::hash::{artifact_hash, semantic_hash};
use bhcp::pipeline::{compile_source, parse_source};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures")
        .join(name)
}

#[test]
fn canonical_source_produces_validated_ast_ir_and_stable_artifacts() {
    let path = fixture("canonical-simple.bhcp");
    let source = fs::read_to_string(&path).unwrap();
    let ast = parse_source(&source, path.to_str().unwrap()).unwrap();
    let compiled = compile_source(&source, path.to_str().unwrap()).unwrap();

    ast.validate().unwrap();
    compiled.ir.validate().unwrap();
    assert_eq!(compiled.ir.goals[0].symbol, "example/Greet@0");
    assert_eq!(compiled.ir.entrypoints, ["goal-1"]);
    assert_eq!(
        semantic_hash(&compiled.ir).unwrap(),
        compiled.ir.semantic_id.clone().unwrap()
    );
    assert_eq!(
        artifact_hash(&compiled.ir.to_value(false)).unwrap(),
        compiled.ir.artifact_id.clone().unwrap()
    );
    assert_eq!(
        compiled.ast_bytes,
        fs::read(fixture("canonical-simple.ast.cbor")).unwrap()
    );
    assert_eq!(
        compiled.ir_bytes,
        fs::read(fixture("canonical-simple.ir.cbor")).unwrap()
    );
}

#[test]
fn presentation_and_labels_do_not_change_semantic_id() {
    let left = fs::read_to_string(fixture("canonical-simple.bhcp")).unwrap();
    let right = fs::read_to_string(fixture("canonical-simple-presentation.bhcp")).unwrap();
    let a = compile_source(&left, "left.bhcp").unwrap();
    let b = compile_source(&right, "right.bhcp").unwrap();
    assert_eq!(a.ir.semantic_id, b.ir.semantic_id);
    assert_ne!(a.ast.artifact_id, b.ast.artifact_id);
}

#[test]
fn semantic_changes_change_identity() {
    let source = fs::read_to_string(fixture("canonical-simple.bhcp")).unwrap();
    let baseline = compile_source(&source, "base.bhcp").unwrap().ir.semantic_id;
    for changed in [
        source.replace("greeting", "salutation"),
        source.replace("name != \"\"", "name == \"\""),
        source.replace("fs-read@0", "fs-write@0"),
        source.replace("§prefer 1:", "§prefer 2:"),
        source.replace("network@0", "process-run@0"),
        source.replace("attempts <= 3", "attempts <= 4"),
    ] {
        assert_ne!(
            compile_source(&changed, "changed.bhcp")
                .unwrap()
                .ir
                .semantic_id,
            baseline
        );
    }
}

#[test]
fn unsupported_and_unresolved_syntax_have_stable_codes() {
    let unsupported =
        compile_source("§goal example/G@0 { §state cache: Text; }", "bad.bhcp").unwrap_err();
    let unresolved =
        compile_source("§goal example/G@0 { §requires missing == 1; }", "bad.bhcp").unwrap_err();
    assert_eq!(unsupported.code, "BHCP1004");
    assert_eq!(unresolved.code, "BHCP2001");
}
