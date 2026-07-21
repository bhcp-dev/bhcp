use bhcp::kernel::{
    ChildObservation, ExecutionResult, KernelChild, KernelNetwork, KernelRuntime, Reduction,
    Verdict,
};
use bhcp::model::{
    BhcpType, Binding, Expression, ExpressionForm, FieldType, FunctionDefinition, GoalDefinition,
    SemanticIrDocument,
};
use bhcp::value::Value;

fn expression(id: &str, value_type: BhcpType, form: ExpressionForm) -> Expression {
    Expression {
        id: id.to_owned(),
        value_type,
        form,
    }
}

fn call(id: &str, value_type: BhcpType, symbol: &str, arguments: Vec<Expression>) -> Expression {
    expression(
        id,
        value_type,
        ExpressionForm::Call(symbol.to_owned(), arguments),
    )
}

fn observations_type(child_output: &BhcpType) -> BhcpType {
    BhcpType::Record(vec![FieldType {
        name: "only".to_owned(),
        value_type: BhcpType::Option(Box::new(BhcpType::ExecutionResult(Box::new(
            child_output.clone(),
        )))),
    }])
}

fn ir(
    output: BhcpType,
    child_output: Option<BhcpType>,
    definition: Expression,
) -> SemanticIrDocument {
    let input = BhcpType::Record(vec![]);
    let observations = child_output
        .as_ref()
        .map(observations_type)
        .unwrap_or_else(|| BhcpType::Record(vec![]));
    let reducer = FunctionDefinition {
        id: "function-reducer".to_owned(),
        symbol: "example/reducer@0".to_owned(),
        parameters: vec![
            Binding {
                id: "parameter-parent".to_owned(),
                name: "parent".to_owned(),
                value_type: input.clone(),
            },
            Binding {
                id: "parameter-observations".to_owned(),
                name: "observations".to_owned(),
                value_type: observations,
            },
        ],
        result: BhcpType::Reduction(Box::new(output.clone())),
        definition,
    };
    let mut goals = Vec::new();
    let children = if let Some(child_output) = child_output {
        goals.push(GoalDefinition {
            id: "goal-child".to_owned(),
            symbol: "example/child@0".to_owned(),
            type_mode: bhcp::policy::TypeMode::InferStrict,
            input: input.clone(),
            output: child_output,
            evidence: BhcpType::Evidence(vec!["static".to_owned()]),
            clauses: vec![],
            policy_decision: None,
            body: None,
        });
        vec![KernelChild {
            id: "child-only".to_owned(),
            tag: "only".to_owned(),
            goal: "goal-child".to_owned(),
            arguments: vec![],
        }]
    } else {
        vec![]
    };
    goals.push(GoalDefinition {
        id: "goal-parent".to_owned(),
        symbol: "example/parent@0".to_owned(),
        type_mode: bhcp::policy::TypeMode::InferStrict,
        input,
        output: output.clone(),
        evidence: BhcpType::Evidence(vec!["static".to_owned()]),
        clauses: vec![],
        policy_decision: None,
        body: Some(KernelNetwork {
            id: "network-1".to_owned(),
            output,
            children,
            reducer: "example/reducer@0".to_owned(),
        }),
    });
    SemanticIrDocument {
        features: vec![],
        type_mode: bhcp::policy::TypeMode::InferStrict,
        types: vec![],
        functions: vec![reducer],
        pure_functions: vec![],
        predicates: vec![],
        goals,
        entrypoints: vec!["goal-parent".to_owned()],
        effective_policy: None,
        semantic_id: None,
        artifact_id: None,
    }
}

fn empty_text_list(id: &str) -> Expression {
    expression(
        id,
        BhcpType::List(Box::new(BhcpType::Primitive("Text"))),
        ExpressionForm::Literal(Value::Array(vec![])),
    )
}

fn conclude_satisfied(id: &str, output: BhcpType, value: Expression) -> Expression {
    let execution = BhcpType::ExecutionResult(Box::new(output.clone()));
    let satisfied = call(
        &format!("{id}-satisfied"),
        execution.clone(),
        "bhcp/kernel.satisfied@0",
        vec![value, empty_text_list(&format!("{id}-evidence"))],
    );
    call(
        id,
        BhcpType::Reduction(Box::new(output)),
        "bhcp/kernel.conclude@0",
        vec![satisfied],
    )
}

#[test]
fn typed_literals_boolean_operations_and_total_conditionals_execute_without_callbacks() {
    let bool_type = BhcpType::Primitive("Bool");
    let text_type = BhcpType::Primitive("Text");
    let unit_type = BhcpType::Primitive("Unit");
    let condition = expression(
        "condition-and",
        bool_type.clone(),
        ExpressionForm::Binary(
            "&&".to_owned(),
            Box::new(expression(
                "condition-not",
                bool_type.clone(),
                ExpressionForm::Unary(
                    "!".to_owned(),
                    Box::new(expression(
                        "literal-false",
                        bool_type.clone(),
                        ExpressionForm::Literal(Value::Bool(false)),
                    )),
                ),
            )),
            Box::new(expression(
                "condition-equal",
                bool_type.clone(),
                ExpressionForm::Binary(
                    "==".to_owned(),
                    Box::new(expression(
                        "literal-left",
                        text_type.clone(),
                        ExpressionForm::Literal(Value::Text("gate".to_owned())),
                    )),
                    Box::new(expression(
                        "literal-right",
                        text_type,
                        ExpressionForm::Literal(Value::Text("gate".to_owned())),
                    )),
                ),
            )),
        ),
    );
    let unit = call("unit", unit_type.clone(), "bhcp/kernel.unit@0", vec![]);
    let conclusion = conclude_satisfied("conclude-true", unit_type.clone(), unit);
    let alternative = conclude_satisfied(
        "conclude-false",
        unit_type.clone(),
        call(
            "alternative-unit",
            unit_type.clone(),
            "bhcp/kernel.unit@0",
            vec![],
        ),
    );
    let definition = expression(
        "if",
        BhcpType::Reduction(Box::new(unit_type.clone())),
        ExpressionForm::If(
            Box::new(condition),
            Box::new(conclusion),
            Box::new(alternative),
        ),
    );
    let ir = ir(unit_type, None, definition);
    let reduction = KernelRuntime::new(&ir)
        .reduce("network-1", Value::owned_map(vec![]), &[])
        .unwrap();
    assert!(matches!(
        reduction,
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Satisfied { output, .. }),
            ..
        } if output == Value::Array(vec![Value::Text("unit".to_owned())])
    ));
}

#[test]
fn observation_primitives_cover_choice_negation_and_sequential_demand() {
    let text = BhcpType::Primitive("Text");
    let winner = BhcpType::Record(vec![
        FieldType {
            name: "output".to_owned(),
            value_type: text.clone(),
        },
        FieldType {
            name: "tag".to_owned(),
            value_type: text.clone(),
        },
    ]);
    let observation_reference = |id: &str| {
        expression(
            id,
            observations_type(&text),
            ExpressionForm::Reference("parameter-observations".to_owned()),
        )
    };
    let has_satisfied = call(
        "has-satisfied",
        BhcpType::Primitive("Bool"),
        "bhcp/kernel.has-satisfied@0",
        vec![observation_reference("observations-has-satisfied")],
    );
    let winner_value = call(
        "winner",
        winner.clone(),
        "bhcp/kernel.first-satisfied-winner@0",
        vec![observation_reference("observations-winner")],
    );
    let winner_evidence = call(
        "winner-evidence",
        BhcpType::List(Box::new(text.clone())),
        "bhcp/kernel.first-satisfied-evidence@0",
        vec![observation_reference("observations-evidence")],
    );
    let winner_result = call(
        "winner-result",
        BhcpType::ExecutionResult(Box::new(winner.clone())),
        "bhcp/kernel.satisfied@0",
        vec![winner_value, winner_evidence],
    );
    let winner_conclusion = call(
        "winner-conclusion",
        BhcpType::Reduction(Box::new(winner.clone())),
        "bhcp/kernel.conclude@0",
        vec![winner_result],
    );
    let next = call(
        "first-missing",
        BhcpType::List(Box::new(text.clone())),
        "bhcp/kernel.first-missing-tag@0",
        vec![observation_reference("observations-first-missing")],
    );
    let pending = call(
        "pending",
        BhcpType::Reduction(Box::new(winner.clone())),
        "bhcp/kernel.pending@0",
        vec![next],
    );
    let all_refuted = call(
        "all-refuted",
        BhcpType::Primitive("Bool"),
        "bhcp/kernel.all-refuted@0",
        vec![observation_reference("observations-all-refuted")],
    );
    let counter_evidence = call(
        "all-counter-evidence",
        BhcpType::List(Box::new(BhcpType::Primitive("Text"))),
        "bhcp/kernel.all-counter-evidence@0",
        vec![observation_reference("observations-counter-evidence")],
    );
    let refuted = call(
        "refuted",
        BhcpType::ExecutionResult(Box::new(winner.clone())),
        "bhcp/kernel.refuted@0",
        vec![counter_evidence],
    );
    let refuted_conclusion = call(
        "refuted-conclusion",
        BhcpType::Reduction(Box::new(winner.clone())),
        "bhcp/kernel.conclude@0",
        vec![refuted],
    );
    let incomplete = expression(
        "refuted-or-pending",
        BhcpType::Reduction(Box::new(winner.clone())),
        ExpressionForm::If(
            Box::new(all_refuted),
            Box::new(refuted_conclusion),
            Box::new(pending),
        ),
    );
    let definition = expression(
        "choice",
        BhcpType::Reduction(Box::new(winner.clone())),
        ExpressionForm::If(
            Box::new(has_satisfied),
            Box::new(winner_conclusion),
            Box::new(incomplete),
        ),
    );
    let ir = ir(winner, Some(BhcpType::Primitive("Text")), definition);
    let runtime = KernelRuntime::new(&ir);
    assert_eq!(
        runtime
            .reduce("network-1", Value::owned_map(vec![]), &[])
            .unwrap(),
        Reduction::Pending {
            required: vec!["only".to_owned()]
        }
    );
    let observation = ChildObservation {
        child: "child-only".to_owned(),
        result: ExecutionResult::Completed(Verdict::Satisfied {
            output: Value::Text("chosen".to_owned()),
            evidence: vec!["evidence-choice".to_owned()],
        }),
    };
    let reduction = runtime
        .reduce("network-1", Value::owned_map(vec![]), &[observation])
        .unwrap();
    assert!(matches!(
        reduction,
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Satisfied { output, .. }),
            ..
        } if output == Value::map([
            ("output", Value::Text("chosen".to_owned())),
            ("tag", Value::Text("only".to_owned())),
        ])
    ));

    let refuted = ChildObservation {
        child: "child-only".to_owned(),
        result: ExecutionResult::Completed(Verdict::Refuted {
            counter_evidence: vec!["counter-choice".to_owned()],
        }),
    };
    let reduction = runtime
        .reduce("network-1", Value::owned_map(vec![]), &[refuted])
        .unwrap();
    assert!(matches!(
        reduction,
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Refuted { counter_evidence }),
            ..
        } if counter_evidence.contains(&"counter-choice".to_owned())
    ));
}

#[test]
fn unreachable_unsupported_calls_and_runtime_output_mismatch_fail_closed() {
    let unit = BhcpType::Primitive("Unit");
    let valid = conclude_satisfied(
        "valid",
        unit.clone(),
        call("valid-unit", unit.clone(), "bhcp/kernel.unit@0", vec![]),
    );
    let unsupported = call(
        "unsupported",
        BhcpType::Reduction(Box::new(unit.clone())),
        "example/ambient-callback@0",
        vec![],
    );
    let definition = expression(
        "if-unreachable",
        BhcpType::Reduction(Box::new(unit.clone())),
        ExpressionForm::If(
            Box::new(expression(
                "true",
                BhcpType::Primitive("Bool"),
                ExpressionForm::Literal(Value::Bool(true)),
            )),
            Box::new(valid),
            Box::new(unsupported),
        ),
    );
    let invalid_ir = ir(unit.clone(), None, definition);
    let error = KernelRuntime::new(&invalid_ir)
        .reduce("network-1", Value::owned_map(vec![]), &[])
        .unwrap_err();
    assert_eq!(error.code, "BHCP4001");
    assert_eq!(
        error.message,
        "IR function call does not resolve to a retained definition or closed kernel primitive"
    );

    let observations = expression(
        "mismatch-observations",
        observations_type(&BhcpType::Primitive("Text")),
        ExpressionForm::Reference("parameter-observations".to_owned()),
    );
    let wrong_output = call(
        "wrong-output",
        unit.clone(),
        "bhcp/kernel.first-satisfied-output@0",
        vec![observations],
    );
    let mismatch = conclude_satisfied("mismatch", unit.clone(), wrong_output);
    let mismatch_ir = ir(unit, Some(BhcpType::Primitive("Text")), mismatch);
    let observation = ChildObservation {
        child: "child-only".to_owned(),
        result: ExecutionResult::Completed(Verdict::Satisfied {
            output: Value::Text("wrong shape".to_owned()),
            evidence: vec!["evidence".to_owned()],
        }),
    };
    let error = KernelRuntime::new(&mismatch_ir)
        .reduce("network-1", Value::owned_map(vec![]), &[observation])
        .unwrap_err();
    assert_eq!(error.code, "BHCP4101");
    assert_eq!(
        error.message,
        "reducer primitive result does not match its declared expression type"
    );
}
