use std::fs;
use std::path::PathBuf;

use bhcp::kernel::{
    ChildObservation, ExecutionResult, KernelRuntime, OperationalFault, Reason, Reduction, Verdict,
};
use bhcp::model::BhcpType;
use bhcp::pipeline::compile_source;
use bhcp::value::Value;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance/v0/fixtures")
        .join(name)
}

fn compile(name: &str) -> bhcp::pipeline::Compilation {
    let path = fixture(name);
    let source = fs::read_to_string(&path).unwrap();
    compile_source(&source, path.to_str().unwrap()).unwrap()
}

fn refuted(evidence: &str) -> ExecutionResult {
    ExecutionResult::Completed(Verdict::Refuted {
        counter_evidence: vec![evidence.to_owned()],
    })
}

fn reason(code: &str) -> Reason {
    Reason {
        code: code.to_owned(),
        message: "fixture".to_owned(),
        details: None,
    }
}

fn faulted() -> ExecutionResult {
    ExecutionResult::Faulted(OperationalFault {
        error: reason("bhcp.fault/executor@0"),
        trace: vec![],
    })
}

#[test]
fn none_and_explicit_compose_lower_to_identical_unit_semantics_and_bytes() {
    let derived = compile("canonical-none.bhcp");
    let explicit = compile("canonical-none-compose.bhcp");
    let repeated = compile("canonical-none.bhcp");

    assert_eq!(derived.ir.semantic_id, explicit.ir.semantic_id);
    assert_eq!(derived.ir.semantic_value(), explicit.ir.semantic_value());
    assert_ne!(derived.ast.artifact_id, explicit.ast.artifact_id);
    assert_eq!(derived.ast_bytes, repeated.ast_bytes);
    assert_eq!(derived.ir_bytes, repeated.ir_bytes);
    assert_eq!(
        derived.ast_bytes,
        fs::read(fixture("canonical-none.ast.cbor")).unwrap()
    );
    assert_eq!(
        derived.ir_bytes,
        fs::read(fixture("canonical-none.ir.cbor")).unwrap()
    );
    assert_eq!(derived.ir.goals[2].output, BhcpType::Primitive("Unit"));
    assert_eq!(
        derived.ir.goals[2].body.as_ref().unwrap().output,
        BhcpType::Primitive("Unit")
    );
    assert!(
        derived.ir.goals[2]
            .body
            .as_ref()
            .unwrap()
            .reducer
            .starts_with("bhcp/prelude.none-reducer-")
    );
    let encoded = derived.ir.goals[2].body.as_ref().unwrap().to_value();
    assert!(encoded.get("kind").is_none());
    assert!(encoded.get("lowerer").is_none());
}

#[test]
fn none_branch_source_order_is_not_semantic() {
    let source = fs::read_to_string(fixture("canonical-none.bhcp")).unwrap();
    let reversed = source.replace(
        "        malware = example/Malware@0();\n        violation = example/Violation@0();",
        "        violation = example/Violation@0();\n        malware = example/Malware@0();",
    );
    let normal = compile_source(&source, "normal-none.bhcp").unwrap();
    let reversed = compile_source(&reversed, "reversed-none.bhcp").unwrap();
    assert_eq!(normal.ir.semantic_id, reversed.ir.semantic_id);
    assert_ne!(normal.ast.artifact_id, reversed.ast.artifact_id);
}

#[test]
fn none_requests_all_children_and_all_refutations_prove_unit() {
    let compiled = compile("canonical-none.bhcp");
    let runtime = KernelRuntime::new(&compiled.ir);
    assert_eq!(
        runtime
            .reduce("network-1", Value::owned_map(vec![]), &[])
            .unwrap(),
        Reduction::Pending {
            required: vec!["malware".to_owned(), "violation".to_owned()]
        }
    );
    let observations = vec![
        ChildObservation {
            child: "child-1".to_owned(),
            result: refuted("counter-malware"),
        },
        ChildObservation {
            child: "child-2".to_owned(),
            result: refuted("counter-violation"),
        },
    ];
    let conclusion = runtime
        .reduce("network-1", Value::owned_map(vec![]), &observations)
        .unwrap();
    let Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied { output, evidence }),
        derivation,
    } = &conclusion
    else {
        panic!("all refuted children must satisfy none")
    };
    assert_eq!(output, &Value::Array(vec![Value::Text("unit".to_owned())]));
    assert_eq!(
        derivation.premises,
        ["counter-malware", "counter-violation"]
    );
    assert!(evidence.contains(&derivation.id));
    runtime
        .verify(
            "network-1",
            Value::owned_map(vec![]),
            &observations,
            &conclusion,
        )
        .unwrap();
}

#[test]
fn a_satisfied_child_refutes_none_despite_an_unrelated_fault() {
    let compiled = compile("canonical-none.bhcp");
    let observations = vec![
        ChildObservation {
            child: "child-1".to_owned(),
            result: ExecutionResult::Completed(Verdict::Satisfied {
                output: Value::map([("finding", Value::Text("present".to_owned()))]),
                evidence: vec!["evidence-malware".to_owned()],
            }),
        },
        ChildObservation {
            child: "child-2".to_owned(),
            result: faulted(),
        },
    ];
    let reduction = KernelRuntime::new(&compiled.ir)
        .reduce("network-1", Value::owned_map(vec![]), &observations)
        .unwrap();
    assert!(matches!(
        reduction,
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Refuted { ref counter_evidence }),
            ..
        } if counter_evidence.starts_with(&["evidence-malware".to_owned()])
    ));
}

#[test]
fn none_obeys_missing_fault_and_unresolved_precedence() {
    let compiled = compile("canonical-none.bhcp");
    let runtime = KernelRuntime::new(&compiled.ir);
    let malware_refuted = ChildObservation {
        child: "child-1".to_owned(),
        result: refuted("counter-malware"),
    };
    assert_eq!(
        runtime
            .reduce(
                "network-1",
                Value::owned_map(vec![]),
                std::slice::from_ref(&malware_refuted),
            )
            .unwrap(),
        Reduction::Pending {
            required: vec!["violation".to_owned()]
        }
    );

    let fault = ChildObservation {
        child: "child-2".to_owned(),
        result: faulted(),
    };
    assert!(matches!(
        runtime
            .reduce(
                "network-1",
                Value::owned_map(vec![]),
                &[malware_refuted.clone(), fault],
            )
            .unwrap(),
        Reduction::Concluded {
            result: ExecutionResult::Faulted(_),
            ..
        }
    ));

    let unresolved = ChildObservation {
        child: "child-2".to_owned(),
        result: ExecutionResult::Completed(Verdict::Unresolved {
            reason: reason("bhcp.reason/evidence-missing@0"),
            partial_evidence: vec!["partial-violation".to_owned()],
        }),
    };
    let reduction = runtime
        .reduce(
            "network-1",
            Value::owned_map(vec![]),
            &[malware_refuted, unresolved],
        )
        .unwrap();
    assert!(matches!(
        reduction,
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Unresolved { ref partial_evidence, .. }),
            ..
        } if partial_evidence == &["counter-malware".to_owned(), "partial-violation".to_owned()]
    ));
}

#[test]
fn empty_none_is_a_premise_free_satisfied_unit_identity() {
    let compiled =
        compile_source("§goal example/Empty@0 { §none { }; }", "empty-none.bhcp").unwrap();
    let reduction = KernelRuntime::new(&compiled.ir)
        .reduce("network-1", Value::owned_map(vec![]), &[])
        .unwrap();
    let Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied { output, evidence }),
        derivation,
    } = reduction
    else {
        panic!("empty none must satisfy")
    };
    assert_eq!(compiled.ir.goals[0].output, BhcpType::Primitive("Unit"));
    assert_eq!(output, Value::Array(vec![Value::Text("unit".to_owned())]));
    assert!(derivation.premises.is_empty());
    assert_eq!(evidence, [derivation.id]);
}

#[test]
fn none_rejects_an_explicit_output_shape() {
    let source = fs::read_to_string(fixture("canonical-none.bhcp")).unwrap();
    let changed = source.replace(
        "§goal example/Clean@0 {",
        "§goal example/Clean@0 {\n    §output value: Unit;",
    );
    let diagnostic = compile_source(&changed, "wrong-none-output.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");
    assert_eq!(
        diagnostic.message,
        "composition output does not match the parent goal output"
    );
}

#[test]
fn unsupported_nested_none_fails_with_a_stable_diagnostic() {
    let source = "§goal example/Leaf@0 { }\n\
                  §goal example/Parent@0 {\n\
                      §none { nested = §none { leaf = example/Leaf@0(); }; };\n\
                  }";
    let diagnostic = compile_source(source, "nested-none.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP1004");
    assert_eq!(
        diagnostic.message,
        "nested composition is outside the implemented vertical slice"
    );
}

#[test]
fn generic_checker_rejects_a_tampered_none_reduction() {
    let compiled = compile("canonical-none.bhcp");
    let runtime = KernelRuntime::new(&compiled.ir);
    let mut reduction = runtime
        .reduce("network-1", Value::owned_map(vec![]), &[])
        .unwrap();
    let Reduction::Pending { required } = &mut reduction else {
        unreachable!()
    };
    required.reverse();
    let diagnostic = runtime
        .verify("network-1", Value::owned_map(vec![]), &[], &reduction)
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4102");
}
