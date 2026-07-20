use std::fs;
use std::path::PathBuf;

use bhcp::hash::{artifact_hash, semantic_hash};
use bhcp::pipeline::{compile_source, parse_source};
use bhcp::schema::validate_root;
use bhcp::value::Value;

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

    let division = compile_source(
        "§goal example/G@0 { §input n: Integer; §requires n / n == 1; }",
        "bad.bhcp",
    )
    .unwrap_err();
    assert_eq!(division.code, "BHCP2004");
}

fn attribute<'a>(node: &'a bhcp::model::AstNode, name: &str) -> &'a Value {
    &node
        .attributes
        .iter()
        .find(|(candidate, _)| candidate == name)
        .unwrap_or_else(|| panic!("{} omits attribute {name}", node.kind))
        .1
}

#[test]
fn complete_definition_forms_build_a_closed_schema_valid_ast() {
    let source = include_str!("fixtures/definition-forms.bhcp");
    let ast = parse_source(source, "definitions.bhcp").unwrap();
    ast.validate().unwrap();
    validate_root(&ast.to_value(true), "canonical-ast").unwrap();

    assert_eq!(
        ast.root
            .children
            .iter()
            .map(|node| node.kind.as_str())
            .collect::<Vec<_>>(),
        [
            "type",
            "type",
            "type",
            "type",
            "type",
            "function",
            "predicate",
            "refines",
        ]
    );
    assert_eq!(
        attribute(&ast.root.children[0], "symbol"),
        &Value::Text("example/Identifier@0".to_owned())
    );
    assert!(matches!(
        attribute(&ast.root.children[0], "type_parameters"),
        Value::Array(parameters) if parameters.len() == 1
    ));
    assert!(matches!(
        attribute(&ast.root.children[0], "definition"),
        Value::Array(shape) if shape.first() == Some(&Value::Text("record".to_owned()))
    ));
    assert_eq!(
        attribute(&ast.root.children[6], "verifier"),
        &Value::Text("example/static@0".to_owned())
    );
    assert!(matches!(
        attribute(&ast.root.children[5], "result"),
        Value::Array(shape)
            if shape.first() == Some(&Value::Text("parameter".to_owned()))
                && shape.get(1) == Some(&Value::Text("T".to_owned()))
    ));
    let Value::Array(union) = attribute(&ast.root.children[3], "definition") else {
        panic!("combined type is not structured as a union");
    };
    assert_eq!(union[0], Value::Text("union".to_owned()));
    let Value::Array(members) = &union[1] else {
        panic!("union members are not retained");
    };
    assert!(matches!(
        &members[1],
        Value::Array(intersection)
            if intersection.first() == Some(&Value::Text("intersection".to_owned()))
    ));
    assert_eq!(ast.root.children[0].span.start.line, 1);
    assert_eq!(ast.root.children[7].span.end.line, 8);
}

#[test]
fn complete_type_grammar_and_verifier_arguments_retain_structure() {
    let source = include_str!("fixtures/definition-types.bhcp");
    let ast = parse_source(source, "complete-types.bhcp").unwrap();
    ast.validate().unwrap();
    validate_root(&ast.to_value(true), "canonical-ast").unwrap();

    assert!(matches!(
        attribute(&ast.root.children[0], "definition"),
        Value::Array(shape) if shape.first() == Some(&Value::Text("tuple".to_owned()))
    ));
    assert!(matches!(
        attribute(&ast.root.children[1], "definition"),
        Value::Array(shape) if shape.first() == Some(&Value::Text("refinement".to_owned()))
    ));
    assert!(matches!(
        attribute(&ast.root.children[2], "definition"),
        Value::Array(shape)
            if shape.first() == Some(&Value::Text("goal".to_owned()))
                && !matches!(shape.get(3), Some(Value::Null))
                && !matches!(shape.get(4), Some(Value::Null))
    ));
    assert!(matches!(
        attribute(&ast.root.children[5], "verifier_arguments"),
        Value::Array(arguments) if arguments.len() == 1
    ));
    let Value::Array(nominal) = attribute(&ast.root.children[3], "definition") else {
        panic!("namespaced type is not nominal");
    };
    assert_eq!(nominal[1], Value::Text("example/Container@0".to_owned()));
    let Value::Array(arguments) = attribute(&ast.root.children[5], "verifier_arguments") else {
        unreachable!();
    };
    assert_eq!(
        arguments[0].get("mode"),
        Some(&Value::Text("borrow".to_owned()))
    );
}

#[test]
fn parsed_definition_forms_stop_before_executable_ir_emission() {
    for source in [
        "§type example/Only@0 = Text;",
        "§predicate example/only@0(): Bool;",
        "§refines example/Child@0 example/Parent@0;",
    ] {
        let diagnostic = compile_source(source, "definition-only.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, "BHCP2004", "{source}");
    }
}

#[test]
fn definition_parser_rejects_duplicates_and_malformed_boundaries_stably() {
    let cases = [
        (
            "§type example/Duplicate@0 = Text;\n§function example/Duplicate@0(): Text = \"x\";",
            "BHCP1003",
            "duplicate definition symbol",
        ),
        (
            "§type example/Duplicate@0<T, T> = T;",
            "BHCP1003",
            "duplicate type parameter",
        ),
        (
            "§type example/Captured@0<Text> = Text;",
            "BHCP1001",
            "reserved spelling \"Text\" cannot be used as type parameter",
        ),
        (
            "§function example/captured@0(true: Bool): Bool = true;",
            "BHCP1001",
            "reserved spelling \"true\" cannot be used as parameter name",
        ),
        (
            "§function example/duplicate@0(value: Text, value: Text): Text = value;",
            "BHCP1003",
            "duplicate parameter",
        ),
        (
            "§type example/Duplicate@0 = { value: Text, value: Text };",
            "BHCP1003",
            "duplicate record field",
        ),
        (
            "§type example/Duplicate@0 = variant { Same, Same(Text) };",
            "BHCP1003",
            "duplicate variant tag",
        ),
        (
            "§predicate example/notBool@0(value: Text): Text = true;",
            "BHCP1001",
            "predicate result must be Bool",
        ),
        (
            "§refines example/Child@0 example/Parent@0;\n§refines example/Child@0 example/Parent@0;",
            "BHCP1003",
            "duplicate refines edge",
        ),
        (
            "§type example/Unresolved@0 = Missing;",
            "BHCP1002",
            "semantic names",
        ),
        (
            "§type example/Empty@0 = variant {};",
            "BHCP1001",
            "at least one case",
        ),
        (
            "§type example/Effects@0 = Goal<Text, Text, !{example/read@0, example/read@0}>;",
            "BHCP1003",
            "duplicate effect-row member",
        ),
        (
            "§predicate example/args@0(value: Text): Bool with example/check@0(subject = value, subject = value);",
            "BHCP1003",
            "duplicate verifier argument",
        ),
    ];

    for (source, code, message) in cases {
        let diagnostic = parse_source(source, "invalid-definition.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, code, "{source}");
        assert!(
            diagnostic.message.contains(message),
            "{source}: {diagnostic}"
        );
    }
}
