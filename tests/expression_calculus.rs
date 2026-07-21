use bhcp::expression::{CheckedExpression, EvaluationContext, ExpressionContext};
use bhcp::schema::parse_diagnostic;
use bhcp::typecheck::CheckedType;
use bhcp::value::Value;

fn expression(id: &str, value_type: &str, form: Value) -> Value {
    Value::map([
        ("id", Value::Text(id.to_owned())),
        ("type", parse_diagnostic(value_type).unwrap()),
        ("form", form),
    ])
}

fn integer(id: &str, value: i128) -> Value {
    expression(
        id,
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("literal".to_owned()),
            Value::Array(vec![
                Value::Text("integer".to_owned()),
                Value::Integer(value),
            ]),
        ]),
    )
}

fn text(id: &str, value: &str) -> Value {
    expression(
        id,
        r#"["primitive", "Text"]"#,
        Value::Array(vec![
            Value::Text("literal".to_owned()),
            Value::Text(value.to_owned()),
        ]),
    )
}

fn boolean(id: &str, value: bool) -> Value {
    expression(
        id,
        r#"["primitive", "Bool"]"#,
        Value::Array(vec![Value::Text("literal".to_owned()), Value::Bool(value)]),
    )
}

fn evaluate(value: Value) -> Value {
    CheckedExpression::check(&value, &ExpressionContext::default())
        .unwrap()
        .evaluate(&EvaluationContext::default())
        .unwrap()
}

fn checked_type(value: &str) -> CheckedType {
    CheckedType::from_value(&parse_diagnostic(value).unwrap()).unwrap()
}

#[test]
fn exp_01_static_finite_quantification_is_checked_and_deterministic() {
    let domain = expression(
        "domain",
        r#"["list", ["exact-number", "Integer"]]"#,
        Value::Array(vec![
            Value::Text("collection".to_owned()),
            Value::Text("list".to_owned()),
            Value::Array(vec![
                integer("one", 1),
                integer("two", 2),
                integer("three", 3),
            ]),
        ]),
    );
    let reference = expression(
        "candidate",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("reference".to_owned()),
            Value::Text("item".to_owned()),
        ]),
    );
    let predicate = expression(
        "positive",
        r#"["primitive", "Bool"]"#,
        Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text(">".to_owned()),
            reference,
            integer("zero", 0),
        ]),
    );
    let quantified = expression(
        "all-positive",
        r#"["primitive", "Bool"]"#,
        Value::Array(vec![
            Value::Text("quantify".to_owned()),
            Value::Text("forall".to_owned()),
            Value::map([
                ("id", Value::Text("item".to_owned())),
                ("name", Value::Text("item".to_owned())),
                (
                    "type",
                    parse_diagnostic(r#"["exact-number", "Integer"]"#).unwrap(),
                ),
            ]),
            domain,
            predicate,
        ]),
    );

    let checked = CheckedExpression::check(&quantified, &ExpressionContext::default()).unwrap();
    assert_eq!(
        checked.evaluate(&EvaluationContext::default()).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(checked.to_value(), quantified);
}

#[test]
fn exp_02_unknown_calls_and_partial_arithmetic_fail_before_evaluation() {
    let call = expression(
        "ambient-call",
        r#"["primitive", "Text"]"#,
        Value::Array(vec![
            Value::Text("call".to_owned()),
            Value::Text("example/read-clock@0".to_owned()),
            Value::Array(vec![]),
        ]),
    );
    assert_eq!(
        CheckedExpression::check(&call, &ExpressionContext::default())
            .unwrap_err()
            .code,
        "BHCP4201"
    );

    let division = expression(
        "division",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text("/".to_owned()),
            integer("one", 1),
            integer("zero", 0),
        ]),
    );
    let checked = CheckedExpression::check(&division, &ExpressionContext::default()).unwrap();
    assert_eq!(
        checked
            .evaluate(&EvaluationContext::default())
            .unwrap_err()
            .code,
        "BHCP4202"
    );
}

#[test]
fn every_closed_constructor_selection_and_binding_form_evaluates() {
    let record = expression(
        "record",
        r#"["record", false, [["count", ["exact-number", "Integer"], false], ["name", ["primitive", "Text"], false]]]"#,
        Value::Array(vec![
            Value::Text("record".to_owned()),
            Value::map([
                ("count", integer("record-count", 2)),
                ("name", text("record-name", "Ada")),
            ]),
        ]),
    );
    let selected = expression(
        "selected-name",
        r#"["primitive", "Text"]"#,
        Value::Array(vec![
            Value::Text("select".to_owned()),
            record,
            Value::Text("name".to_owned()),
        ]),
    );
    assert_eq!(evaluate(selected), Value::Text("Ada".to_owned()));

    let tuple = expression(
        "tuple",
        r#"["tuple", [["primitive", "Text"], ["exact-number", "Integer"]]]"#,
        Value::Array(vec![
            Value::Text("tuple".to_owned()),
            Value::Array(vec![text("tuple-name", "Ada"), integer("tuple-count", 2)]),
        ]),
    );
    let selected = expression(
        "selected-count",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("select".to_owned()),
            tuple,
            Value::Integer(1),
        ]),
    );
    assert_eq!(
        evaluate(selected),
        Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(2)])
    );

    let binding = Value::map([
        ("id", Value::Text("bound".to_owned())),
        ("name", Value::Text("presentation-name".to_owned())),
        (
            "type",
            parse_diagnostic(r#"["exact-number", "Integer"]"#).unwrap(),
        ),
    ]);
    let reference = expression(
        "bound-reference",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("reference".to_owned()),
            Value::Text("bound".to_owned()),
        ]),
    );
    let let_expression = expression(
        "let",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("let".to_owned()),
            binding,
            integer("initializer", 4),
            expression(
                "sum",
                r#"["exact-number", "Integer"]"#,
                Value::Array(vec![
                    Value::Text("binary".to_owned()),
                    Value::Text("+".to_owned()),
                    reference,
                    integer("increment", 1),
                ]),
            ),
        ]),
    );
    assert_eq!(
        evaluate(let_expression),
        Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(5)])
    );

    let conditional = expression(
        "if",
        r#"["primitive", "Text"]"#,
        Value::Array(vec![
            Value::Text("if".to_owned()),
            boolean("condition", true),
            text("then", "chosen"),
            text("else", "ignored"),
        ]),
    );
    assert_eq!(evaluate(conditional), Value::Text("chosen".to_owned()));
}

#[test]
fn exhaustive_patterns_bind_nested_values_and_guards() {
    let variant_type = r#"["variant", [["None", []], ["Some", [["exact-number", "Integer"]]]]]"#;
    let subject = expression(
        "subject",
        variant_type,
        Value::Array(vec![
            Value::Text("variant".to_owned()),
            Value::Text("Some".to_owned()),
            integer("payload", 3),
        ]),
    );
    let bound = expression(
        "match-reference",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("reference".to_owned()),
            Value::Text("match-value".to_owned()),
        ]),
    );
    let match_expression = expression(
        "match",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("match".to_owned()),
            subject,
            Value::Array(vec![
                Value::Array(vec![
                    Value::Array(vec![
                        Value::Text("variant".to_owned()),
                        Value::Text("Some".to_owned()),
                        Value::Array(vec![Value::Array(vec![
                            Value::Text("bind".to_owned()),
                            Value::map([
                                ("id", Value::Text("match-value".to_owned())),
                                ("name", Value::Text("value".to_owned())),
                                (
                                    "type",
                                    parse_diagnostic(r#"["exact-number", "Integer"]"#).unwrap(),
                                ),
                            ]),
                        ])]),
                    ]),
                    boolean("guard", true),
                    bound,
                ]),
                Value::Array(vec![
                    Value::Array(vec![Value::Text("wildcard".to_owned())]),
                    integer("fallback", 0),
                ]),
            ]),
        ]),
    );
    assert_eq!(
        evaluate(match_expression),
        Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(3)])
    );
}

#[test]
fn tuple_record_and_literal_patterns_are_checked_recursively() {
    let record_type = r#"["record", false, [["count", ["exact-number", "Integer"], false]]]"#;
    let tuple_type = format!(r#"["tuple", [{record_type}, ["primitive", "Bool"]]]"#);
    let subject = expression(
        "pattern-subject",
        &tuple_type,
        Value::Array(vec![
            Value::Text("tuple".to_owned()),
            Value::Array(vec![
                expression(
                    "pattern-record",
                    record_type,
                    Value::Array(vec![
                        Value::Text("record".to_owned()),
                        Value::map([("count", integer("pattern-count", 9))]),
                    ]),
                ),
                boolean("pattern-bool", true),
            ]),
        ]),
    );
    let matched = expression(
        "compound-match",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("match".to_owned()),
            subject,
            Value::Array(vec![
                Value::Array(vec![
                    Value::Array(vec![
                        Value::Text("tuple".to_owned()),
                        Value::Array(vec![
                            Value::Array(vec![
                                Value::Text("record".to_owned()),
                                Value::map([(
                                    "count",
                                    Value::Array(vec![
                                        Value::Text("bind".to_owned()),
                                        Value::map([
                                            ("id", Value::Text("compound-binding".to_owned())),
                                            (
                                                "type",
                                                parse_diagnostic(r#"["exact-number", "Integer"]"#)
                                                    .unwrap(),
                                            ),
                                        ]),
                                    ]),
                                )]),
                            ]),
                            Value::Array(vec![
                                Value::Text("literal".to_owned()),
                                Value::Bool(true),
                            ]),
                        ]),
                    ]),
                    expression(
                        "compound-reference",
                        r#"["exact-number", "Integer"]"#,
                        Value::Array(vec![
                            Value::Text("reference".to_owned()),
                            Value::Text("compound-binding".to_owned()),
                        ]),
                    ),
                ]),
                Value::Array(vec![
                    Value::Array(vec![Value::Text("wildcard".to_owned())]),
                    integer("compound-fallback", 0),
                ]),
            ]),
        ]),
    );
    assert_eq!(
        evaluate(matched),
        Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(9)])
    );
}

#[test]
fn dynamic_casts_are_explicit_and_failed_casts_fault() {
    let dynamic = expression(
        "dynamic",
        r#"["special", "Dynamic"]"#,
        Value::Array(vec![
            Value::Text("literal".to_owned()),
            Value::Text("value".to_owned()),
        ]),
    );
    let cast = expression(
        "cast",
        r#"["primitive", "Text"]"#,
        Value::Array(vec![
            Value::Text("cast-dynamic".to_owned()),
            dynamic.clone(),
            parse_diagnostic(r#"["primitive", "Text"]"#).unwrap(),
        ]),
    );
    assert_eq!(evaluate(cast), Value::Text("value".to_owned()));

    let failed = expression(
        "failed-cast",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("cast-dynamic".to_owned()),
            dynamic,
            parse_diagnostic(r#"["exact-number", "Integer"]"#).unwrap(),
        ]),
    );
    let checked = CheckedExpression::check(&failed, &ExpressionContext::default()).unwrap();
    assert_eq!(
        checked
            .evaluate(&EvaluationContext::default())
            .unwrap_err()
            .code,
        "BHCP4202"
    );
}

#[test]
fn collections_maps_unary_and_all_exact_number_families_are_deterministic() {
    let set = expression(
        "set",
        r#"["set", ["primitive", "Text"]]"#,
        Value::Array(vec![
            Value::Text("collection".to_owned()),
            Value::Text("set".to_owned()),
            Value::Array(vec![text("z", "z"), text("a", "a")]),
        ]),
    );
    assert_eq!(
        evaluate(set),
        Value::Array(vec![
            Value::Text("a".to_owned()),
            Value::Text("z".to_owned())
        ])
    );

    let map = expression(
        "map",
        r#"["map", ["primitive", "Text"], ["exact-number", "Integer"]]"#,
        Value::Array(vec![
            Value::Text("map".to_owned()),
            Value::Array(vec![
                Value::Array(vec![text("map-b", "b"), integer("map-two", 2)]),
                Value::Array(vec![text("map-a", "a"), integer("map-one", 1)]),
            ]),
        ]),
    );
    assert_eq!(
        evaluate(map),
        Value::map([
            (
                "a",
                Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(1)]),
            ),
            (
                "b",
                Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(2)]),
            ),
        ])
    );

    let negated = expression(
        "negated",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("unary".to_owned()),
            Value::Text("-".to_owned()),
            integer("positive", 7),
        ]),
    );
    assert_eq!(
        evaluate(negated),
        Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(-7)])
    );

    let rational = |id: &str, numerator: i128, denominator: i128| {
        expression(
            id,
            r#"["exact-number", "Rational"]"#,
            Value::Array(vec![
                Value::Text("literal".to_owned()),
                Value::Array(vec![
                    Value::Text("rational".to_owned()),
                    Value::Integer(numerator),
                    Value::Integer(denominator),
                ]),
            ]),
        )
    };
    let sum = expression(
        "rational-sum",
        r#"["exact-number", "Rational"]"#,
        Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text("+".to_owned()),
            rational("half", 1, 2),
            rational("third", 1, 3),
        ]),
    );
    assert_eq!(
        evaluate(sum),
        Value::Array(vec![
            Value::Text("rational".to_owned()),
            Value::Integer(5),
            Value::Integer(6),
        ])
    );

    let decimal = |id: &str, coefficient: i128| {
        expression(
            id,
            r#"["exact-number", "Decimal"]"#,
            Value::Array(vec![
                Value::Text("literal".to_owned()),
                Value::Array(vec![
                    Value::Text("decimal".to_owned()),
                    Value::Integer(coefficient),
                    Value::Integer(-1),
                ]),
            ]),
        )
    };
    let sum = expression(
        "decimal-sum",
        r#"["exact-number", "Decimal"]"#,
        Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text("+".to_owned()),
            decimal("one-point-five", 15),
            decimal("point-five", 5),
        ]),
    );
    assert_eq!(
        evaluate(sum),
        Value::Array(vec![
            Value::Text("decimal".to_owned()),
            Value::Integer(2),
            Value::Integer(0),
        ])
    );
}

#[test]
fn calls_are_limited_to_retained_acyclic_total_pure_definitions() {
    let parameter_type = checked_type(r#"["exact-number", "Integer"]"#);
    let reference = expression(
        "definition-reference",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("reference".to_owned()),
            Value::Text("parameter".to_owned()),
        ]),
    );
    let definition = expression(
        "definition",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text("+".to_owned()),
            reference,
            integer("definition-one", 1),
        ]),
    );
    let context = ExpressionContext::default()
        .define(
            "example/increment@0",
            vec![("parameter".to_owned(), parameter_type.clone())],
            parameter_type.clone(),
            &definition,
        )
        .unwrap();
    let call = expression(
        "call",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("call".to_owned()),
            Value::Text("example/increment@0".to_owned()),
            Value::Array(vec![integer("argument", 4)]),
        ]),
    );
    assert_eq!(
        CheckedExpression::check(&call, &context)
            .unwrap()
            .evaluate(&EvaluationContext::default())
            .unwrap(),
        Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(5)])
    );

    let recursive = expression(
        "recursive-definition",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("call".to_owned()),
            Value::Text("example/recurse@0".to_owned()),
            Value::Array(vec![expression(
                "recursive-reference",
                r#"["exact-number", "Integer"]"#,
                Value::Array(vec![
                    Value::Text("reference".to_owned()),
                    Value::Text("parameter".to_owned()),
                ]),
            )]),
        ]),
    );
    assert_eq!(
        ExpressionContext::default()
            .define(
                "example/recurse@0",
                vec![("parameter".to_owned(), parameter_type.clone())],
                parameter_type,
                &recursive,
            )
            .unwrap_err()
            .code,
        "BHCP4201"
    );
}

#[test]
fn exp_01_verifier_backed_quantification_requires_the_exact_finite_domain_witness() {
    let domain = expression(
        "witnessed-domain",
        r#"["list", ["exact-number", "Integer"]]"#,
        Value::Array(vec![
            Value::Text("collection".to_owned()),
            Value::Text("list".to_owned()),
            Value::Array(vec![
                integer("witnessed-one", 1),
                integer("witnessed-two", 2),
            ]),
        ]),
    );
    let predicate = expression(
        "witnessed-positive",
        r#"["primitive", "Bool"]"#,
        Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text(">".to_owned()),
            expression(
                "witnessed-reference",
                r#"["exact-number", "Integer"]"#,
                Value::Array(vec![
                    Value::Text("reference".to_owned()),
                    Value::Text("witnessed-item".to_owned()),
                ]),
            ),
            integer("witnessed-zero", 0),
        ]),
    );
    let quantified = expression(
        "witnessed-quantifier",
        r#"["primitive", "Bool"]"#,
        Value::Array(vec![
            Value::Text("quantify".to_owned()),
            Value::Text("forall".to_owned()),
            Value::map([
                ("id", Value::Text("witnessed-item".to_owned())),
                (
                    "type",
                    parse_diagnostic(r#"["exact-number", "Integer"]"#).unwrap(),
                ),
            ]),
            domain,
            predicate,
            Value::map([
                (
                    "verifier",
                    Value::Text("example/finite-domain@0".to_owned()),
                ),
                (
                    "input",
                    parse_diagnostic(r#"["list", ["exact-number", "Integer"]]"#).unwrap(),
                ),
                (
                    "output",
                    parse_diagnostic(r#"["evidence", ["static"]]"#).unwrap(),
                ),
            ]),
        ]),
    );
    let checked = CheckedExpression::check(&quantified, &ExpressionContext::default()).unwrap();
    assert_eq!(
        checked
            .evaluate(&EvaluationContext::default())
            .unwrap_err()
            .code,
        "BHCP4202"
    );
    let witness = EvaluationContext::default()
        .witness_quantifier_domain(
            "witnessed-quantifier",
            vec![
                Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(1)]),
                Value::Array(vec![Value::Text("integer".to_owned()), Value::Integer(2)]),
            ],
        )
        .unwrap();
    assert_eq!(checked.evaluate(&witness).unwrap(), Value::Bool(true));
}

#[test]
fn totality_attacks_are_rejected_or_fault_with_stable_categories() {
    let duplicate_ids = expression(
        "duplicate-root",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text("+".to_owned()),
            integer("duplicate-child", 1),
            integer("duplicate-child", 2),
        ]),
    );
    assert_eq!(
        CheckedExpression::check(&duplicate_ids, &ExpressionContext::default())
            .unwrap_err()
            .code,
        "BHCP4201"
    );

    let hidden_call = expression(
        "hidden-call-if",
        r#"["primitive", "Text"]"#,
        Value::Array(vec![
            Value::Text("if".to_owned()),
            boolean("hidden-condition", true),
            text("hidden-selected", "safe"),
            expression(
                "hidden-callback",
                r#"["primitive", "Text"]"#,
                Value::Array(vec![
                    Value::Text("call".to_owned()),
                    Value::Text("example/ambient-io@0".to_owned()),
                    Value::Array(vec![]),
                ]),
            ),
        ]),
    );
    assert_eq!(
        CheckedExpression::check(&hidden_call, &ExpressionContext::default())
            .unwrap_err()
            .code,
        "BHCP4201"
    );

    let non_exhaustive = expression(
        "non-exhaustive",
        r#"["primitive", "Text"]"#,
        Value::Array(vec![
            Value::Text("match".to_owned()),
            boolean("matched-bool", true),
            Value::Array(vec![Value::Array(vec![
                Value::Array(vec![Value::Text("literal".to_owned()), Value::Bool(true)]),
                text("only-arm", "true"),
            ])]),
        ]),
    );
    assert_eq!(
        CheckedExpression::check(&non_exhaustive, &ExpressionContext::default())
            .unwrap_err()
            .code,
        "BHCP4201"
    );

    let payload_non_exhaustive = expression(
        "payload-non-exhaustive",
        r#"["primitive", "Text"]"#,
        Value::Array(vec![
            Value::Text("match".to_owned()),
            expression(
                "payload-subject",
                r#"["variant", [["A", [["primitive", "Bool"]]], ["B", []]]]"#,
                Value::Array(vec![
                    Value::Text("variant".to_owned()),
                    Value::Text("A".to_owned()),
                    boolean("payload-true", true),
                ]),
            ),
            Value::Array(vec![
                Value::Array(vec![
                    Value::Array(vec![
                        Value::Text("variant".to_owned()),
                        Value::Text("A".to_owned()),
                        Value::Array(vec![Value::Array(vec![
                            Value::Text("literal".to_owned()),
                            Value::Bool(true),
                        ])]),
                    ]),
                    text("payload-a", "a"),
                ]),
                Value::Array(vec![
                    Value::Array(vec![
                        Value::Text("variant".to_owned()),
                        Value::Text("B".to_owned()),
                        Value::Array(vec![]),
                    ]),
                    text("payload-b", "b"),
                ]),
            ]),
        ]),
    );
    assert_eq!(
        CheckedExpression::check(&payload_non_exhaustive, &ExpressionContext::default())
            .unwrap_err()
            .code,
        "BHCP4201"
    );

    let selected = expression(
        "out-of-bounds",
        r#"["exact-number", "Integer"]"#,
        Value::Array(vec![
            Value::Text("select".to_owned()),
            expression(
                "short-list",
                r#"["list", ["exact-number", "Integer"]]"#,
                Value::Array(vec![
                    Value::Text("collection".to_owned()),
                    Value::Text("list".to_owned()),
                    Value::Array(vec![integer("only-item", 1)]),
                ]),
            ),
            Value::Integer(4),
        ]),
    );
    assert_eq!(
        CheckedExpression::check(&selected, &ExpressionContext::default())
            .unwrap()
            .evaluate(&EvaluationContext::default())
            .unwrap_err()
            .code,
        "BHCP4202"
    );

    let machine_type = r#"["machine-integer", "signed", 8]"#;
    let machine = |id: &str, value: i128| {
        expression(
            id,
            machine_type,
            Value::Array(vec![
                Value::Text("literal".to_owned()),
                Value::Array(vec![
                    Value::Text("integer".to_owned()),
                    Value::Integer(value),
                ]),
            ]),
        )
    };
    let overflow = expression(
        "machine-overflow",
        machine_type,
        Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text("+".to_owned()),
            machine("machine-max", 127),
            machine("machine-one", 1),
        ]),
    );
    assert_eq!(
        CheckedExpression::check(&overflow, &ExpressionContext::default())
            .unwrap()
            .evaluate(&EvaluationContext::default())
            .unwrap_err()
            .code,
        "BHCP4202"
    );
}
