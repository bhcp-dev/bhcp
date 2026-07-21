use std::collections::HashSet;

use bhcp::kernel::{
    ChildObservation, Derivation, ExecutionResult, KernelChild, KernelNetwork, OperationalFault,
    Reason, Reduction, Verdict,
};
use bhcp::model::{
    BhcpType, Binding, Expression, ExpressionForm, FieldType, FunctionDefinition, GoalDefinition,
    SemanticIrDocument,
};
use bhcp::value::Value;

fn reason(code: &str) -> Reason {
    Reason {
        code: code.to_owned(),
        message: "fixture".to_owned(),
        details: None,
    }
}

fn network() -> KernelNetwork {
    KernelNetwork {
        id: "network-1".to_owned(),
        output: BhcpType::Primitive("Text"),
        children: vec![
            KernelChild {
                id: "child-a".to_owned(),
                tag: "a".to_owned(),
                goal: "goal-a".to_owned(),
                arguments: vec![],
            },
            KernelChild {
                id: "child-b".to_owned(),
                tag: "b".to_owned(),
                goal: "goal-b".to_owned(),
                arguments: vec![],
            },
        ],
        reducer: "bhcp/prelude.all-reducer@0".to_owned(),
    }
}

#[test]
fn execution_result_factors_operational_and_semantic_states() {
    let completed = ExecutionResult::Completed(Verdict::Unresolved {
        reason: reason("bhcp.reason/evidence-missing@0"),
        partial_evidence: vec!["evidence-1".to_owned()],
    });
    let value = completed.to_value();
    assert_eq!(
        value.get("state"),
        Some(&Value::Text("completed".to_owned()))
    );
    assert_eq!(
        value
            .get("verdict")
            .and_then(|verdict| verdict.get("state")),
        Some(&Value::Text("unresolved".to_owned()))
    );
    completed.validate().unwrap();

    let faulted = ExecutionResult::Faulted(OperationalFault {
        error: reason("bhcp.fault/executor-crashed@0"),
        trace: vec![],
    });
    let value = faulted.to_value();
    assert_eq!(value.get("state"), Some(&Value::Text("faulted".to_owned())));
    assert!(value.get("verdict").is_none());
    faulted.validate().unwrap();
}

#[test]
fn reduction_uses_consistent_adjectival_states() {
    let network = network();
    network.validate().unwrap();
    let observed = HashSet::from(["a".to_owned()]);
    let pending = Reduction::Pending {
        required: vec!["b".to_owned()],
    };
    assert_eq!(
        pending.to_value().get("state"),
        Some(&Value::Text("pending".to_owned()))
    );
    pending.validate(&network, &observed).unwrap();

    let concluded = Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied {
            output: Value::Text("done".to_owned()),
            evidence: vec!["evidence-1".to_owned(), "derivation-1".to_owned()],
        }),
        derivation: Derivation {
            id: "derivation-1".to_owned(),
            premises: vec!["evidence-1".to_owned()],
        },
    };
    assert_eq!(
        concluded.to_value().get("state"),
        Some(&Value::Text("concluded".to_owned()))
    );
    assert!(
        concluded
            .to_value()
            .get("derivation")
            .and_then(|derivation| derivation.get("rule"))
            .is_none()
    );
    concluded.validate(&network, &observed).unwrap();
}

#[test]
fn concluded_truth_claims_reference_their_checked_derivation() {
    let network = KernelNetwork {
        children: vec![],
        ..network()
    };
    let observed = HashSet::new();
    let mut concluded = Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied {
            output: Value::owned_map(vec![]),
            evidence: vec!["derivation-empty-all".to_owned()],
        }),
        derivation: Derivation {
            id: "derivation-empty-all".to_owned(),
            premises: vec![],
        },
    };
    concluded.validate(&network, &observed).unwrap();

    let Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied { evidence, .. }),
        ..
    } = &mut concluded
    else {
        unreachable!();
    };
    evidence.clear();
    assert_eq!(
        concluded.validate(&network, &observed).unwrap_err().code,
        "BHCP4101"
    );
}

#[test]
fn pending_reduction_requires_unobserved_known_child_tags() {
    let network = network();
    let observed = HashSet::from(["a".to_owned()]);

    for required in [vec![], vec!["missing".to_owned()], vec!["a".to_owned()]] {
        let diagnostic = Reduction::Pending { required }
            .validate(&network, &observed)
            .unwrap_err();
        assert_eq!(diagnostic.code, "BHCP4101");
    }
}

#[test]
fn child_observations_are_sealed_to_declared_network_children() {
    let network = network();
    let mut observation = ChildObservation {
        child: "child-a".to_owned(),
        result: ExecutionResult::Completed(Verdict::Refuted {
            counter_evidence: vec!["evidence-1".to_owned()],
        }),
    };
    observation.validate(&network).unwrap();
    assert_eq!(
        observation.to_value().get("child"),
        Some(&Value::Text("child-a".to_owned()))
    );

    observation.child = "missing".to_owned();
    assert_eq!(observation.validate(&network).unwrap_err().code, "BHCP4101");
}

#[test]
fn kernel_network_contains_structure_but_no_privileged_behavior_kind() {
    let network = network();
    let value = network.to_value();
    assert_eq!(
        value.get("reducer"),
        Some(&Value::Text("bhcp/prelude.all-reducer@0".to_owned()))
    );
    assert!(value.get("kind").is_none());
    assert!(value.get("dependencies").is_none());
    assert!(value.get("guards").is_none());
    assert!(value.get("quantified").is_none());
    assert!(value.get("recursion").is_none());
    assert!(value.get("parallel_eligible").is_none());
    assert!(value.get("parallel_reasons").is_none());
}

#[test]
fn kernel_types_preserve_the_factored_categories() {
    let text = BhcpType::Primitive("Text");
    assert_eq!(
        BhcpType::Verdict(Box::new(text.clone())).to_value(),
        Value::Array(vec![Value::Text("verdict".to_owned()), text.to_value()])
    );
    assert_eq!(
        BhcpType::ExecutionResult(Box::new(text.clone())).to_value(),
        Value::Array(vec![
            Value::Text("execution-result".to_owned()),
            text.to_value(),
        ])
    );
    assert_eq!(
        BhcpType::Reduction(Box::new(text.clone())).to_value(),
        Value::Array(vec![Value::Text("reduction".to_owned()), text.to_value()])
    );
    assert_eq!(
        BhcpType::Option(Box::new(text.clone())).to_value(),
        Value::Array(vec![Value::Text("option".to_owned()), text.to_value()])
    );
}

#[test]
fn semantic_ir_resolves_network_reducers_as_bhcp_functions() {
    let text = BhcpType::Primitive("Text");
    let parent_input = BhcpType::Record(vec![]);
    let observations = BhcpType::Record(vec![FieldType {
        name: "a".to_owned(),
        value_type: BhcpType::Option(Box::new(BhcpType::ExecutionResult(Box::new(text.clone())))),
    }]);
    let reducer_symbol = "bhcp/prelude.all-reducer@0";
    let reducer = FunctionDefinition {
        id: "function-1".to_owned(),
        symbol: reducer_symbol.to_owned(),
        parameters: vec![
            Binding {
                id: "parameter-parent".to_owned(),
                name: "parent".to_owned(),
                value_type: parent_input.clone(),
            },
            Binding {
                id: "parameter-observations".to_owned(),
                name: "observations".to_owned(),
                value_type: observations,
            },
        ],
        result: BhcpType::Reduction(Box::new(text.clone())),
        definition: Expression {
            id: "expression-1".to_owned(),
            value_type: BhcpType::Reduction(Box::new(text.clone())),
            form: ExpressionForm::Literal(Value::map([
                ("state", Value::Text("pending".to_owned())),
                (
                    "required",
                    Value::Array(vec![Value::Text("child-a".to_owned())]),
                ),
            ])),
        },
    };
    let child = GoalDefinition {
        id: "goal-a".to_owned(),
        symbol: "example/child@0".to_owned(),
        type_mode: bhcp::policy::TypeMode::InferStrict,
        input: parent_input,
        output: text.clone(),
        evidence: BhcpType::Evidence(vec!["static".to_owned()]),
        clauses: vec![],
        policy_decision: None,
        body: None,
    };
    let parent = GoalDefinition {
        id: "goal-parent".to_owned(),
        symbol: "example/parent@0".to_owned(),
        type_mode: bhcp::policy::TypeMode::InferStrict,
        input: BhcpType::Record(vec![]),
        output: text.clone(),
        evidence: BhcpType::Evidence(vec!["static".to_owned()]),
        clauses: vec![],
        policy_decision: None,
        body: Some(KernelNetwork {
            id: "network-1".to_owned(),
            output: text,
            children: vec![KernelChild {
                id: "child-a".to_owned(),
                tag: "a".to_owned(),
                goal: "goal-a".to_owned(),
                arguments: vec![],
            }],
            reducer: reducer_symbol.to_owned(),
        }),
    };
    let mut ir = SemanticIrDocument {
        features: vec![],
        type_mode: bhcp::policy::TypeMode::InferStrict,
        types: vec![],
        functions: vec![reducer],
        pure_functions: vec![],
        predicates: vec![],
        goals: vec![child, parent],
        entrypoints: vec!["goal-parent".to_owned()],
        effective_policy: None,
        semantic_id: None,
        artifact_id: None,
    };
    ir.validate().unwrap();

    ir.functions[0].parameters.pop();
    assert_eq!(ir.validate().unwrap_err().code, "BHCP4001");

    ir.functions.clear();
    let diagnostic = ir.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert_eq!(
        diagnostic.message,
        "kernel reducer does not resolve to a function"
    );
}
