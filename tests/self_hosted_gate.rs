use std::fs;
use std::path::PathBuf;

use bhcp::kernel::{
    ArgumentMode, ChildObservation, ExecutionResult, KernelRuntime, OperationalFault, Reason,
    Reduction, Verdict,
};
use bhcp::model::ExpressionForm;
use bhcp::pipeline::compile_source;
use bhcp::value::Value;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures")
        .join(name)
}

fn compile_gate() -> bhcp::pipeline::Compilation {
    let path = fixture("canonical-gate.bhcp");
    let source = fs::read_to_string(&path).unwrap();
    compile_source(&source, path.to_str().unwrap()).unwrap()
}

fn parent(enabled: bool) -> Value {
    Value::map([
        ("enabled", Value::Bool(enabled)),
        ("request", Value::Text("release".to_owned())),
    ])
}

fn variant(tag: &str, payload: Value) -> Value {
    Value::Array(vec![
        Value::Text("variant".to_owned()),
        Value::Text(tag.to_owned()),
        payload,
    ])
}

fn child_satisfied() -> ChildObservation {
    ChildObservation {
        child: "child-1".to_owned(),
        result: ExecutionResult::Completed(Verdict::Satisfied {
            output: Value::map([("approval", Value::Text("approved".to_owned()))]),
            evidence: vec!["evidence-approval".to_owned()],
        }),
    }
}

fn reason(code: &str) -> Reason {
    Reason {
        code: code.to_owned(),
        message: "fixture".to_owned(),
        details: None,
    }
}

#[test]
fn gate_lowers_to_a_unary_typed_variant_with_an_explicit_parent_edge() {
    let compiled = compile_gate();
    let repeated = compile_gate();
    assert_eq!(compiled.ast_bytes, repeated.ast_bytes);
    assert_eq!(compiled.ir_bytes, repeated.ir_bytes);
    assert_eq!(
        compiled.ast_bytes,
        fs::read(fixture("canonical-gate.ast.cbor")).unwrap()
    );
    assert_eq!(
        compiled.ir_bytes,
        fs::read(fixture("canonical-gate.ir.cbor")).unwrap()
    );

    let gate = &compiled.ir.goals[1];
    assert_eq!(
        gate.output.to_value(),
        Value::Array(vec![
            Value::Text("variant".to_owned()),
            Value::Array(vec![
                Value::Array(vec![
                    Value::Text("Excluded".to_owned()),
                    Value::Array(vec![]),
                ]),
                Value::Array(vec![
                    Value::Text("Included".to_owned()),
                    Value::Array(vec![compiled.ir.goals[0].output.to_value()]),
                ]),
            ]),
        ])
    );
    let network = gate.body.as_ref().unwrap();
    assert_eq!(network.children.len(), 1);
    assert!(network.reducer.starts_with("bhcp/prelude.gate-reducer-"));
    assert!(network.to_value().get("kind").is_none());
    assert!(network.to_value().get("condition").is_none());
    let argument = &network.children[0].arguments[0];
    assert_eq!(argument.name, "request");
    assert_eq!(argument.mode, ArgumentMode::Borrow);
    let ExpressionForm::Call(symbol, parameters) = &argument.value.form else {
        panic!("expected an explicit parent-field edge")
    };
    assert_eq!(symbol, "bhcp/kernel.parent-field@0");
    assert!(matches!(
        parameters.as_slice(),
        [parameter] if parameter.form == ExpressionForm::Literal(Value::Text("request".to_owned()))
    ));
    assert!(
        compiled
            .ir
            .features
            .contains(&"bhcp/feature.self-hosted-gate@0".to_owned())
    );
}

#[test]
fn false_gate_concludes_excluded_without_observing_the_child() {
    let compiled = compile_gate();
    let runtime = KernelRuntime::new(&compiled.ir);
    let reduction = runtime.reduce("network-1", parent(false), &[]).unwrap();
    let Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied { output, evidence }),
        derivation,
    } = &reduction
    else {
        panic!("a false gate must conclude satisfied")
    };
    assert_eq!(
        output,
        &variant(
            "Excluded",
            Value::Array(vec![Value::Text("unit".to_owned())])
        )
    );
    assert!(derivation.premises.is_empty());
    assert_eq!(evidence, std::slice::from_ref(&derivation.id));
    runtime
        .verify("network-1", parent(false), &[], &reduction)
        .unwrap();

    let diagnostic = runtime
        .reduce("network-1", parent(false), &[child_satisfied()])
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4101");
    assert_eq!(
        diagnostic.message,
        "a closed gate cannot observe its unselected child"
    );
}

#[test]
fn true_gate_requests_one_child_and_wraps_satisfaction() {
    let compiled = compile_gate();
    let runtime = KernelRuntime::new(&compiled.ir);
    assert_eq!(
        runtime.reduce("network-1", parent(true), &[]).unwrap(),
        Reduction::Pending {
            required: vec!["approval".to_owned()]
        }
    );

    let observation = child_satisfied();
    let reduction = runtime
        .reduce(
            "network-1",
            parent(true),
            std::slice::from_ref(&observation),
        )
        .unwrap();
    assert!(matches!(
        reduction,
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Satisfied { ref output, ref evidence }),
            ..
        } if output == &variant("Included", Value::map([("approval", Value::Text("approved".to_owned()))]))
            && evidence.starts_with(&["evidence-approval".to_owned()])
    ));
    runtime
        .verify("network-1", parent(true), &[observation], &reduction)
        .unwrap();
}

#[test]
fn open_gate_propagates_refutation_unresolution_and_fault() {
    let compiled = compile_gate();
    let runtime = KernelRuntime::new(&compiled.ir);
    let cases = [
        ExecutionResult::Completed(Verdict::Refuted {
            counter_evidence: vec!["counter-approval".to_owned()],
        }),
        ExecutionResult::Completed(Verdict::Unresolved {
            reason: reason("bhcp.reason/evidence-missing@0"),
            partial_evidence: vec!["partial-approval".to_owned()],
        }),
        ExecutionResult::Faulted(OperationalFault {
            error: reason("bhcp.fault/executor@0"),
            trace: vec![],
        }),
    ];
    for result in cases {
        let reduction = runtime
            .reduce(
                "network-1",
                parent(true),
                &[ChildObservation {
                    child: "child-1".to_owned(),
                    result: result.clone(),
                }],
            )
            .unwrap();
        match result {
            ExecutionResult::Completed(Verdict::Refuted { .. }) => assert!(matches!(
                reduction,
                Reduction::Concluded {
                    result: ExecutionResult::Completed(Verdict::Refuted { .. }),
                    ..
                }
            )),
            ExecutionResult::Completed(Verdict::Unresolved { .. }) => assert!(matches!(
                reduction,
                Reduction::Concluded {
                    result: ExecutionResult::Completed(Verdict::Unresolved { .. }),
                    ..
                }
            )),
            ExecutionResult::Faulted(_) => assert!(matches!(
                reduction,
                Reduction::Concluded {
                    result: ExecutionResult::Faulted(_),
                    ..
                }
            )),
            _ => unreachable!(),
        }
    }
}

#[test]
fn gate_rejects_non_bool_non_unary_and_mistyped_child_input() {
    let source = fs::read_to_string(fixture("canonical-gate.bhcp")).unwrap();
    let non_bool = source.replace("§gate when enabled", "§gate when request");
    let diagnostic = compile_source(&non_bool, "non-bool-gate.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");

    let empty = source.replace(
        "        approval = example/Approve@0(request = borrow request);",
        "",
    );
    let diagnostic = compile_source(&empty, "empty-gate.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");

    let multiple = source.replace(
        "        approval = example/Approve@0(request = borrow request);",
        "        approval = example/Approve@0(request = borrow request);\n        backup = example/Approve@0(request = borrow request);",
    );
    let diagnostic = compile_source(&multiple, "multiple-gate.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");

    let missing = source.replace("request = borrow request", "request = borrow enabled");
    let diagnostic = compile_source(&missing, "mistyped-gate-edge.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");

    let explicit_output = source.replace(
        "    §input request: Text;",
        "    §input request: Text;\n    §output result: Text;",
    );
    let diagnostic = compile_source(&explicit_output, "explicit-gate-output.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");
}

#[test]
fn condition_and_child_edge_tampering_fail_closed() {
    let mut compiled = compile_gate();
    let function = &mut compiled.ir.functions[0];
    let ExpressionForm::If(condition, _, _) = &mut function.definition.form else {
        panic!("gate reducer must retain its condition")
    };
    condition.form = ExpressionForm::Literal(Value::Text("not-bool".to_owned()));
    let diagnostic = KernelRuntime::new(&compiled.ir)
        .reduce("network-1", parent(true), &[])
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4101");

    let mut compiled = compile_gate();
    let argument = &mut compiled.ir.goals[1].body.as_mut().unwrap().children[0].arguments[0];
    let ExpressionForm::Call(_, fields) = &mut argument.value.form else {
        unreachable!()
    };
    fields[0].form = ExpressionForm::Literal(Value::Text("enabled".to_owned()));
    let diagnostic = KernelRuntime::new(&compiled.ir)
        .reduce("network-1", parent(true), &[])
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
}

#[test]
fn gate_condition_changes_semantics_but_presentation_does_not() {
    let source = fs::read_to_string(fixture("canonical-gate.bhcp")).unwrap();
    let normal = compile_source(&source, "normal-gate.bhcp").unwrap();
    let presented = compile_source(
        &source.replace("§gate when enabled {", "§gate  when  enabled  {"),
        "presented-gate.bhcp",
    )
    .unwrap();
    assert_eq!(normal.ir.semantic_id, presented.ir.semantic_id);
    assert_ne!(normal.ast.artifact_id, presented.ast.artifact_id);

    let negated = compile_source(
        &source.replace("§gate when enabled", "§gate when !enabled"),
        "negated-gate.bhcp",
    )
    .unwrap();
    assert_ne!(normal.ir.semantic_id, negated.ir.semantic_id);
}

#[test]
fn same_signature_gates_retain_distinct_condition_specializations() {
    let source = "§goal example/Approve@0 { §output approval: Text; }\n\
                  §goal example/Open@0 { §input enabled: Bool; §gate when enabled {\n\
                    approval = example/Approve@0();\n\
                  }; }\n\
                  §goal example/Inverse@0 { §input enabled: Bool; §gate when !enabled {\n\
                    approval = example/Approve@0();\n\
                  }; }";
    let compiled = compile_source(source, "two-gates.bhcp").unwrap();
    let open = compiled.ir.goals[1].body.as_ref().unwrap();
    let inverse = compiled.ir.goals[2].body.as_ref().unwrap();
    assert_ne!(open.reducer, inverse.reducer);
    assert_eq!(compiled.ir.functions.len(), 2);

    let runtime = KernelRuntime::new(&compiled.ir);
    let disabled = Value::map([("enabled", Value::Bool(false))]);
    assert!(matches!(
        runtime.reduce(&open.id, disabled.clone(), &[]).unwrap(),
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Satisfied { ref output, .. }),
            ..
        } if output == &variant("Excluded", Value::Array(vec![Value::Text("unit".to_owned())]))
    ));
    assert_eq!(
        runtime.reduce(&inverse.id, disabled, &[]).unwrap(),
        Reduction::Pending {
            required: vec!["approval".to_owned()]
        }
    );
}
