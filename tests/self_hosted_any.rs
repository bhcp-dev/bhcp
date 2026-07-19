use std::fs;
use std::path::PathBuf;

use bhcp::kernel::{
    ChildObservation, ExecutionResult, KernelRuntime, OperationalFault, Reason, Reduction, Verdict,
};
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

fn satisfied(value: &str, evidence: &str) -> ExecutionResult {
    ExecutionResult::Completed(Verdict::Satisfied {
        output: Value::map([("value", Value::Text(value.to_owned()))]),
        evidence: vec![evidence.to_owned()],
    })
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
fn any_and_explicit_compose_lower_to_identical_semantics_and_bytes() {
    let derived = compile("canonical-any.bhcp");
    let explicit = compile("canonical-any-compose.bhcp");
    let repeated = compile("canonical-any.bhcp");

    assert_eq!(derived.ir.semantic_id, explicit.ir.semantic_id);
    assert_eq!(derived.ir.semantic_value(), explicit.ir.semantic_value());
    assert_ne!(derived.ast.artifact_id, explicit.ast.artifact_id);
    assert_eq!(derived.ast_bytes, repeated.ast_bytes);
    assert_eq!(derived.ir_bytes, repeated.ir_bytes);
    assert_eq!(
        derived.ast_bytes,
        fs::read(fixture("canonical-any.ast.cbor")).unwrap()
    );
    assert_eq!(
        derived.ir_bytes,
        fs::read(fixture("canonical-any.ir.cbor")).unwrap()
    );
    assert_eq!(derived.ir.functions.len(), 1);

    let network = derived.ir.goals[2].body.as_ref().unwrap();
    assert!(network.reducer.starts_with("bhcp/prelude.any-reducer-"));
    let encoded = network.to_value();
    assert!(encoded.get("kind").is_none());
    assert!(encoded.get("lowerer").is_none());
}

#[test]
fn any_branch_source_order_is_not_semantic_or_winner_order() {
    let source = fs::read_to_string(fixture("canonical-any.bhcp")).unwrap();
    let reversed = source.replace(
        "        cache = example/Cache@0();\n        source = example/Source@0();",
        "        source = example/Source@0();\n        cache = example/Cache@0();",
    );
    let normal = compile_source(&source, "normal-any.bhcp").unwrap();
    let reversed = compile_source(&reversed, "reversed-any.bhcp").unwrap();
    assert_eq!(normal.ir.semantic_id, reversed.ir.semantic_id);
    assert_ne!(normal.ast.artifact_id, reversed.ast.artifact_id);

    let observations = vec![
        ChildObservation {
            child: "child-2".to_owned(),
            result: satisfied("source", "evidence-source"),
        },
        ChildObservation {
            child: "child-1".to_owned(),
            result: satisfied("cache", "evidence-cache"),
        },
    ];
    for compiled in [&normal, &reversed] {
        let reduction = KernelRuntime::new(&compiled.ir)
            .reduce("network-1", Value::owned_map(vec![]), &observations)
            .unwrap();
        assert!(matches!(
            reduction,
            Reduction::Concluded {
                result: ExecutionResult::Completed(Verdict::Satisfied { output, .. }),
                ..
            } if output == Value::map([
                ("output", Value::map([("value", Value::Text("cache".to_owned()))])),
                ("tag", Value::Text("cache".to_owned())),
            ])
        ));
    }
}

#[test]
fn any_rejects_a_parent_or_child_output_shape_mismatch() {
    let source = fs::read_to_string(fixture("canonical-any.bhcp")).unwrap();
    let wrong_parent = source.replace("§output tag: Text;", "§output tag: Bool;");
    let diagnostic = compile_source(&wrong_parent, "wrong-any-parent.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");
    assert_eq!(
        diagnostic.message,
        "composition output does not match the parent goal output"
    );

    let unlike_children = source.replace(
        "§goal example/Source@0 {\n    §output value: Text;\n}",
        "§goal example/Source@0 {\n    §output value: Bool;\n}",
    );
    let diagnostic = compile_source(&unlike_children, "unlike-any-children.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP3001");
    assert_eq!(
        diagnostic.message,
        "any requires every child to have the same output type in this executable slice"
    );

    let explicit = fs::read_to_string(fixture("canonical-any-compose.bhcp")).unwrap();
    let unlike_explicit = explicit.replace(
        "§goal example/Source@0 {\n    §output value: Text;\n}",
        "§goal example/Source@0 {\n    §output value: Bool;\n}",
    );
    let diagnostic = compile_source(&unlike_explicit, "unlike-explicit-any.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP3001");
    assert!(diagnostic.message.contains("first-satisfied-winner"));
}

#[test]
fn any_requests_all_children_and_satisfaction_wins_despite_faults() {
    let compiled = compile("canonical-any.bhcp");
    let runtime = KernelRuntime::new(&compiled.ir);
    assert_eq!(
        runtime
            .reduce("network-1", Value::owned_map(vec![]), &[])
            .unwrap(),
        Reduction::Pending {
            required: vec!["cache".to_owned(), "source".to_owned()]
        }
    );

    let observations = vec![
        ChildObservation {
            child: "child-1".to_owned(),
            result: faulted(),
        },
        ChildObservation {
            child: "child-2".to_owned(),
            result: satisfied("source", "evidence-source"),
        },
    ];
    let conclusion = runtime
        .reduce("network-1", Value::owned_map(vec![]), &observations)
        .unwrap();
    let Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied { output, evidence }),
        ..
    } = &conclusion
    else {
        panic!("a satisfied child must decide any")
    };
    assert_eq!(
        output,
        &Value::map([
            (
                "output",
                Value::map([("value", Value::Text("source".to_owned()))]),
            ),
            ("tag", Value::Text("source".to_owned())),
        ])
    );
    assert!(evidence.contains(&"evidence-source".to_owned()));
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
fn any_obeys_refutation_missing_fault_and_unresolved_precedence() {
    let compiled = compile("canonical-any.bhcp");
    let runtime = KernelRuntime::new(&compiled.ir);
    let cache_refuted = ChildObservation {
        child: "child-1".to_owned(),
        result: refuted("counter-cache"),
    };
    assert_eq!(
        runtime
            .reduce(
                "network-1",
                Value::owned_map(vec![]),
                std::slice::from_ref(&cache_refuted),
            )
            .unwrap(),
        Reduction::Pending {
            required: vec!["source".to_owned()]
        }
    );

    let source_refuted = ChildObservation {
        child: "child-2".to_owned(),
        result: refuted("counter-source"),
    };
    let rejected = runtime
        .reduce(
            "network-1",
            Value::owned_map(vec![]),
            &[cache_refuted.clone(), source_refuted],
        )
        .unwrap();
    assert!(matches!(
        rejected,
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Refuted { ref counter_evidence }),
            ..
        } if counter_evidence.starts_with(&["counter-cache".to_owned(), "counter-source".to_owned()])
    ));

    let fault = ChildObservation {
        child: "child-2".to_owned(),
        result: faulted(),
    };
    assert!(matches!(
        runtime
            .reduce(
                "network-1",
                Value::owned_map(vec![]),
                &[cache_refuted.clone(), fault],
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
            partial_evidence: vec!["partial-source".to_owned()],
        }),
    };
    assert!(matches!(
        runtime
            .reduce(
                "network-1",
                Value::owned_map(vec![]),
                &[cache_refuted, unresolved],
            )
            .unwrap(),
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Unresolved { .. }),
            ..
        }
    ));
}

#[test]
fn empty_any_is_a_premise_free_refuted_identity() {
    let compiled = compile_source("§goal example/Empty@0 { §any { }; }", "empty-any.bhcp").unwrap();
    let reduction = KernelRuntime::new(&compiled.ir)
        .reduce("network-1", Value::owned_map(vec![]), &[])
        .unwrap();
    let Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Refuted { counter_evidence }),
        derivation,
    } = reduction
    else {
        panic!("empty any must refute")
    };
    assert!(derivation.premises.is_empty());
    assert_eq!(counter_evidence, [derivation.id]);
}

#[test]
fn generic_checker_rejects_a_tampered_any_reduction() {
    let compiled = compile("canonical-any.bhcp");
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
    assert_eq!(
        diagnostic.message,
        "reducer result does not match re-evaluation"
    );
}
