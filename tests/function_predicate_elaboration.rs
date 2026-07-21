use bhcp::pipeline::compile_source;
use bhcp::schema::validate_root;
use bhcp::value::Value;

fn definitions<'a>(ir: &'a Value, field: &str) -> &'a [Value] {
    let Some(Value::Array(definitions)) = ir.get(field) else {
        panic!("semantic IR omits {field}");
    };
    definitions
}

fn symbol(definition: &Value) -> &str {
    let Some(Value::Text(symbol)) = definition.get("symbol") else {
        panic!("definition omits its symbol");
    };
    symbol
}

#[test]
fn functions_and_predicates_resolve_forward_and_lower_deterministically() {
    let source = r#"
§predicate example/accepted@0(value: Bool): Bool = example/invert@0(value);
§function example/invert@0(value: Bool): Bool = !value;
§goal example/Check@0 {
    §input candidate: Bool;
    §requires example/accepted@0(candidate);
}
"#;
    let compiled = compile_source(source, "definitions.bhcp").unwrap();
    let ir = compiled.ir.to_value(true);
    validate_root(&ir, "semantic-ir").unwrap();

    let functions = definitions(&ir, "functions");
    assert!(
        functions
            .iter()
            .any(|definition| symbol(definition) == "example/invert@0")
    );
    let predicates = definitions(&ir, "predicates");
    assert_eq!(predicates.len(), 1);
    assert_eq!(symbol(&predicates[0]), "example/accepted@0");

    let reordered = compile_source(
        &source.replacen(
            "§predicate example/accepted@0(value: Bool): Bool = example/invert@0(value);\n§function example/invert@0(value: Bool): Bool = !value;",
            "§function example/invert@0(value: Bool): Bool = !value;\n§predicate example/accepted@0(value: Bool): Bool = example/invert@0(value);",
            1,
        ),
        "definitions-reordered.bhcp",
    )
    .unwrap();
    assert_eq!(compiled.semantic_hash, reordered.semantic_hash);
}

#[test]
fn generic_calls_are_soundly_inferred_and_monomorphized() {
    let source = r#"
§function example/identity@0<T: Dynamic>(value: T): T = value;
§function example/textIdentity@0(value: Text): Text = example/identity@0(value);
§function example/textIdentityAgain@0(value: Text): Text = example/identity@0(value);
"#;
    let compiled = compile_source(source, "generic.bhcp").unwrap();
    let ir = compiled.ir.to_value(true);
    validate_root(&ir, "semantic-ir").unwrap();
    let functions = definitions(&ir, "functions");

    assert!(
        functions
            .iter()
            .any(|definition| symbol(definition) == "example/textIdentity@0")
    );
    let specializations = functions
        .iter()
        .filter(|definition| symbol(definition).starts_with("example/identity-"))
        .collect::<Vec<_>>();
    assert_eq!(specializations.len(), 1);
    assert!(symbol(specializations[0]).ends_with("@0"));
}

#[test]
fn unused_generic_templates_are_still_validated_without_fake_specializations() {
    let source = r#"
§function example/first@0<T: Dynamic>(left: T, right: T): T = left;
§function example/concrete@0(value: Bool): Bool = value;
"#;
    let compiled = compile_source(source, "unused-generic.bhcp").unwrap();
    let ir = compiled.ir.to_value(true);
    let functions = definitions(&ir, "functions");
    assert!(
        functions
            .iter()
            .any(|definition| symbol(definition) == "example/concrete@0")
    );
    assert!(
        !functions
            .iter()
            .any(|definition| symbol(definition).starts_with("example/first-"))
    );
}

#[test]
fn verifier_backed_predicates_retain_the_closed_typed_binding() {
    let source = r#"
§predicate example/nonEmpty@0(value: Text): Bool
    = value != "" with example/static@0(subject = borrow value);
"#;
    let compiled = compile_source(source, "verified-predicate.bhcp").unwrap();
    let ir = compiled.ir.to_value(true);
    validate_root(&ir, "semantic-ir").unwrap();
    let predicates = definitions(&ir, "predicates");
    let verifier = predicates[0]
        .get("verifier")
        .expect("predicate verifier is retained");

    assert_eq!(
        verifier.get("verifier"),
        Some(&Value::Text("example/static@0".to_owned()))
    );
    assert!(matches!(
        verifier.get("input"),
        Some(Value::Array(parts)) if parts.first() == Some(&Value::Text("record".to_owned()))
    ));
    assert_eq!(
        verifier.get("output"),
        Some(&Value::Array(vec![
            Value::Text("evidence".to_owned()),
            Value::Array(vec![]),
        ]))
    );
    assert_eq!(verifier.get("trust"), None);
    assert!(verifier.get("configuration").is_some());
}

#[test]
fn verifier_arguments_are_canonical_and_semantically_bound() {
    let source = r#"
§predicate example/checked@0(value: Text, flag: Bool): Bool
    = flag && value != "" with example/static@0(subject = borrow value, enabled = share flag);
"#;
    let reordered = source.replace(
        "subject = borrow value, enabled = share flag",
        "enabled = share flag, subject = borrow value",
    );
    let changed_mode = source.replace("subject = borrow value", "subject = share value");
    let baseline = compile_source(source, "verifier-order.bhcp").unwrap();
    let reordered = compile_source(&reordered, "verifier-reordered.bhcp").unwrap();
    let changed_mode = compile_source(&changed_mode, "verifier-mode.bhcp").unwrap();
    assert_eq!(baseline.semantic_hash, reordered.semantic_hash);
    assert_ne!(baseline.semantic_hash, changed_mode.semantic_hash);

    let verifier_only = compile_source(
        "§predicate example/external@0(value: Text): Bool with example/static@0(subject = value);",
        "verifier-only.bhcp",
    )
    .unwrap();
    validate_root(&verifier_only.ir.to_value(true), "semantic-ir").unwrap();
}

#[test]
fn refinement_types_and_edges_survive_definition_elaboration() {
    let source = r#"
§type example/Entity@0 = Text;
§type example/Identifier@0 = Text where value => value != "";
§refines example/Identifier@0 example/Entity@0;
§predicate example/nonEmpty@0(value: Text): Bool = value != "";
"#;
    let compiled = compile_source(source, "refined-definitions.bhcp").unwrap();
    let ir = compiled.ir.to_value(true);
    validate_root(&ir, "semantic-ir").unwrap();
    let Some(Value::Array(types)) = ir.get("types") else {
        panic!("semantic IR omits checked types");
    };
    let identifier = types
        .iter()
        .find(|definition| symbol(definition) == "example/Identifier@0")
        .unwrap();
    assert!(matches!(
        identifier.get("definition"),
        Some(Value::Array(parts)) if parts.first() == Some(&Value::Text("refinement".to_owned()))
    ));
    assert_eq!(
        identifier.get("refines"),
        Some(&Value::Array(vec![Value::Text(
            "example/Entity@0".to_owned()
        )]))
    );
}

#[test]
fn cycles_unknown_calls_mismatches_and_incomplete_predicates_fail_before_ir() {
    let cases = [
        (
            "§function example/a@0(value: Bool): Bool = example/b@0(value);\n§function example/b@0(value: Bool): Bool = example/a@0(value);",
            "recursive function cycle",
        ),
        (
            "§function example/unknown@0(value: Bool): Bool = example/ambient@0(value);",
            "unresolved pure definition",
        ),
        (
            "§function example/wrong@0(value: Text): Bool = value;",
            "function result",
        ),
        (
            "§predicate example/incomplete@0(value: Text): Bool;",
            "requires a definition or verifier binding",
        ),
        (
            "§function example/pair@0<T: Dynamic>(left: T, right: T): T = left;\n§function example/bad@0(text: Text, flag: Bool): Text = example/pair@0(text, flag);",
            "generic parameter inference is inconsistent",
        ),
        (
            "§function example/onlyText@0<T: Text>(value: T): T = value;\n§function example/bad@0(flag: Bool): Bool = example/onlyText@0(flag);",
            "does not satisfy bound",
        ),
        (
            "§function example/make@0<T: Dynamic>(): T = true;\n§function example/bad@0(): Bool = example/make@0();",
            "cannot be soundly inferred",
        ),
        (
            "§function example/hidden@0<T: Dynamic>(value: T): T = example/ambient@0(value);",
            "unresolved pure definition",
        ),
        (
            "§function example/hidden@0<T: Dynamic>(value: T): Bool = value;",
            "function result",
        ),
    ];
    for (source, message) in cases {
        let diagnostic = compile_source(source, "invalid-definition.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, "BHCP4301", "{source}: {diagnostic}");
        assert!(
            diagnostic.message.contains(message),
            "{source}: unexpected diagnostic {diagnostic}"
        );
    }
}

#[test]
fn an_unknown_call_in_an_unselected_branch_never_dispatches() {
    let diagnostic = compile_source(
        "§function example/closed@0(value: Bool): Bool = if value then true else example/ambient@0();",
        "unknown-branch.bhcp",
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4301");
    assert!(diagnostic.message.contains("unresolved pure definition"));
}
