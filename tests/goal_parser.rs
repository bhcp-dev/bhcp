use bhcp::pipeline::{compile_source, parse_source};
use bhcp::schema::validate_root;
use bhcp::value::Value;

fn attribute<'a>(node: &'a bhcp::model::AstNode, name: &str) -> &'a Value {
    &node
        .attributes
        .iter()
        .find(|(candidate, _)| candidate == name)
        .unwrap_or_else(|| panic!("{} omits attribute {name}", node.kind))
        .1
}

#[test]
fn complete_goal_forms_build_a_closed_ordered_schema_valid_ast() {
    let source = include_str!("fixtures/complete-goal.bhcp");
    let ast = parse_source(source, "complete-goal.bhcp").unwrap();
    ast.validate().unwrap();
    validate_root(&ast.to_value(true), "canonical-ast").unwrap();

    let goal = &ast.root.children[0];
    assert_eq!(goal.kind, "goal");
    assert!(matches!(
        attribute(goal, "type_parameters"),
        Value::Array(parameters) if parameters.len() == 1
    ));
    assert_eq!(
        attribute(goal, "refines"),
        &Value::Array(vec![
            Value::Text("goal".to_owned()),
            Value::Array(vec![
                Value::Text("parameter".to_owned()),
                Value::Text("T".to_owned()),
            ]),
            Value::Array(vec![
                Value::Text("primitive".to_owned()),
                Value::Text("Unit".to_owned()),
            ]),
            Value::Null,
            Value::Null,
        ])
    );
    assert_eq!(
        goal.children
            .iter()
            .map(|node| node.kind.as_str())
            .collect::<Vec<_>>(),
        [
            "input",
            "resource",
            "state",
            "output",
            "requires",
            "ensures",
            "invariant",
            "limit",
            "allows",
            "forbids",
            "prefer",
            "verify",
            "case",
            "goal-call",
            "all",
        ]
    );
    assert_eq!(goal.span.start.line, 1);
    assert_eq!(goal.span.end.line, 24);
    assert!(matches!(
        attribute(&goal.children[0], "type"),
        Value::Array(shape) if shape.first() == Some(&Value::Text("parameter".to_owned()))
    ));
    assert!(matches!(
        attribute(&goal.children[1], "type"),
        Value::Array(shape) if shape.first() == Some(&Value::Text("handle".to_owned()))
    ));
    assert!(matches!(
        attribute(&goal.children[11], "verifier_arguments"),
        Value::Array(arguments) if arguments.len() == 1
    ));
    assert!(matches!(
        attribute(&goal.children[4], "condition"),
        Value::Array(shape) if shape.first() == Some(&Value::Text("literal".to_owned()))
    ));
    assert!(matches!(
        attribute(&goal.children[8], "effects"),
        Value::Array(effects) if effects.len() == 1
    ));
    assert!(matches!(
        attribute(&goal.children[10], "objective"),
        Value::Array(shape) if shape.first() == Some(&Value::Text("reference".to_owned()))
    ));
    assert_eq!(
        attribute(&goal.children[10], "priority"),
        &Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(-1),])
    );
    assert_eq!(goal.children[12].children.len(), 2);
    assert!(
        attribute(&goal.children[14], "quantifier")
            .get("domain")
            .is_some()
    );
    assert_eq!(goal.children[14].children[0].children[0].kind, "gate");
}

#[test]
fn every_composition_form_accepts_nested_and_expression_arguments() {
    let source = r#"
§goal example/Compositions@0 {
    §compose using example/Reducer@0 {
        nested = §any exists item in items {
            child = example/Child@0(value = item);
        };
    };
    §none forall item in items {
        child = example/Child@0(value = item);
    };
    §chain {
        child = example/Child@0(value = 1 + 2);
    };
    §gate when true {
        nested = §all {
            child = example/Child@0(value = true);
        };
    };
}
"#;
    let ast = parse_source(source, "compositions.bhcp").unwrap();
    let kinds = ast.root.children[0]
        .children
        .iter()
        .map(|node| node.kind.as_str())
        .collect::<Vec<_>>();
    assert_eq!(kinds, ["compose", "none", "chain", "gate"]);
    validate_root(&ast.to_value(true), "canonical-ast").unwrap();
}

#[test]
fn any_new_goal_form_closes_earlier_clause_and_gate_payloads() {
    let source = r#"
§goal example/Payloads@0 {
    §requires alpha;
    §gate when alpha {
        child = example/Child@0();
    };
    §state cache: Text;
}
"#;
    let ast = parse_source(source, "payloads.bhcp").unwrap();
    let goal = &ast.root.children[0];
    assert!(matches!(
        attribute(&goal.children[0], "condition"),
        Value::Array(shape) if shape.first() == Some(&Value::Text("reference".to_owned()))
    ));
    assert!(matches!(
        attribute(&goal.children[1], "condition"),
        Value::Array(shape) if shape.first() == Some(&Value::Text("reference".to_owned()))
    ));
    validate_root(&ast.to_value(true), "canonical-ast").unwrap();
}

#[test]
fn complete_goal_syntax_stops_before_the_existing_executable_slice() {
    let diagnostic = compile_source(
        include_str!("fixtures/complete-goal.bhcp"),
        "complete-goal.bhcp",
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP1004");
    assert!(diagnostic.message.contains("goal syntax"));
}

#[test]
fn malformed_goal_boundaries_fail_stably_without_an_artifact() {
    let cases = [
        (
            "§goal example/G@0<T, T> { }",
            "BHCP1003",
            "duplicate type parameter",
        ),
        (
            "§goal example/G@0 { §case { expect completed; }; }",
            "BHCP1001",
            "verdict state",
        ),
        (
            "§goal example/G@0 { §case { value = true; value = false; }; }",
            "BHCP1003",
            "duplicate case binding",
        ),
        (
            "§goal example/G@0 { child = example/C@0(value = true, value = false); }",
            "BHCP1003",
            "duplicate argument",
        ),
        (
            "§goal example/G@0 { §input value: Text; §state value: Text; }",
            "BHCP1003",
            "duplicate goal binding",
        ),
        (
            "§goal example/G@0<T> { §input value: Text; §output value: Text; }",
            "BHCP1003",
            "duplicate goal binding",
        ),
        (
            "§goal example/G@0<T> { §requires \"same\": true; §ensures \"same\": true; }",
            "BHCP1003",
            "duplicate goal clause label",
        ),
        (
            "§goal example/G@0<T> { §input true: Text; }",
            "BHCP1001",
            "reserved spelling",
        ),
        (
            "§goal example/G@0 { child = example/C@0(true = false); }",
            "BHCP1001",
            "reserved spelling",
        ),
        (
            "§goal example/G@0 { child = example/C@0(value = true,); }",
            "BHCP1001",
            "cannot end with a comma",
        ),
        (
            "§goal example/G@0 { §all forall item in items { same = example/C@0(); same = example/C@0(); }; }",
            "BHCP1003",
            "duplicate branch tag",
        ),
        (
            "§goal example/G@0 { §all { same = example/C@0(); same = example/C@0(); }; }",
            "BHCP1003",
            "duplicate branch tag",
        ),
        (
            "§goal example/G@0 { §chain { child = example/C@0(value = source, value = source); }; }",
            "BHCP1003",
            "duplicate argument",
        ),
        (
            "§goal example/G@0 { §all forall item in items { nested = §gate when true { first = example/C@0(); second = example/C@0(); }; }; }",
            "BHCP1001",
            "exactly one branch",
        ),
        (
            "§goal example/G@0 { §verify with example/V@0 for \"same\", \"same\"; }",
            "BHCP1003",
            "duplicate verifier obligation label",
        ),
        (
            "§goal example/G@0 { §prefer \"label\": 1: true; }",
            "BHCP1001",
            "priority must precede",
        ),
        (
            "§goal example/G@0 { §all forall item source { child = example/C@0(); }; }",
            "BHCP1001",
            "expected \"in\"",
        ),
        (
            "§goal example/G@0 { §all { nested = §foo { child = example/C@0(); }; }; }",
            "BHCP1004",
            "unsupported nested composition",
        ),
    ];

    for (source, code, message) in cases {
        let diagnostic = parse_source(source, "bad-goal.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, code, "{source}");
        assert!(
            diagnostic.message.contains(message),
            "{source}: {diagnostic}"
        );
    }
}
