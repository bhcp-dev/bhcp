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

fn satisfied(output: Value, evidence: &str) -> ExecutionResult {
    ExecutionResult::Completed(Verdict::Satisfied {
        output,
        evidence: vec![evidence.to_owned()],
    })
}

fn reason(code: &str) -> Reason {
    Reason {
        code: code.to_owned(),
        message: "fixture".to_owned(),
        details: None,
    }
}

#[test]
fn all_and_explicit_compose_lower_to_identical_semantics() {
    let derived = compile("canonical-all.bhcp");
    let explicit = compile("canonical-all-compose.bhcp");

    assert_eq!(derived.ir.semantic_id, explicit.ir.semantic_id);
    assert_eq!(derived.ir.semantic_value(), explicit.ir.semantic_value());
    assert_ne!(derived.ast.artifact_id, explicit.ast.artifact_id);
    assert_eq!(derived.ir.functions.len(), 1);
    assert_eq!(
        derived.ast_bytes,
        fs::read(fixture("canonical-all.ast.cbor")).unwrap()
    );
    assert_eq!(
        derived.ir_bytes,
        fs::read(fixture("canonical-all.ir.cbor")).unwrap()
    );

    let release = &derived.ir.goals[2];
    let network = release.body.as_ref().unwrap();
    assert_eq!(network.children.len(), 2);
    assert!(network.reducer.starts_with("bhcp/prelude.all-reducer-"));
    let encoded = network.to_value();
    assert!(encoded.get("kind").is_none());
    assert!(encoded.get("lowerer").is_none());
}

#[test]
fn all_branch_source_order_is_not_semantic() {
    let source = fs::read_to_string(fixture("canonical-all.bhcp")).unwrap();
    let reversed = source.replace(
        "        build = example/Build@0();\n        docs = example/Document@0();",
        "        docs = example/Document@0();\n        build = example/Build@0();",
    );
    let normal = compile_source(&source, "normal.bhcp").unwrap();
    let reversed = compile_source(&reversed, "reversed.bhcp").unwrap();
    assert_eq!(normal.ir.semantic_id, reversed.ir.semantic_id);
    assert_ne!(normal.ast.artifact_id, reversed.ast.artifact_id);
}

#[test]
fn all_rejects_an_output_shape_that_disagrees_with_its_children() {
    let source = fs::read_to_string(fixture("canonical-all.bhcp")).unwrap();
    let changed = source.replace("§output docs: { page: Text };", "§output docs: Text;");
    let diagnostic = compile_source(&changed, "wrong-output.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");
    assert_eq!(
        diagnostic.message,
        "composition output does not match the parent goal output"
    );
}

#[test]
fn all_reducer_requests_tags_and_builds_the_named_product() {
    let compiled = compile("canonical-all.bhcp");
    let runtime = KernelRuntime::new(&compiled.ir);

    let initial = runtime
        .reduce("network-1", Value::owned_map(vec![]), &[])
        .unwrap();
    assert_eq!(
        initial,
        Reduction::Pending {
            required: vec!["build".to_owned(), "docs".to_owned()]
        }
    );

    let build = ChildObservation {
        child: "child-1".to_owned(),
        result: satisfied(
            Value::map([("artifact", Value::Text("app".to_owned()))]),
            "evidence-build",
        ),
    };
    assert_eq!(
        runtime
            .reduce(
                "network-1",
                Value::owned_map(vec![]),
                std::slice::from_ref(&build),
            )
            .unwrap(),
        Reduction::Pending {
            required: vec!["docs".to_owned()]
        }
    );

    let docs = ChildObservation {
        child: "child-2".to_owned(),
        result: satisfied(
            Value::map([("page", Value::Text("guide".to_owned()))]),
            "evidence-docs",
        ),
    };
    let observations = vec![build, docs];
    let conclusion = runtime
        .reduce("network-1", Value::owned_map(vec![]), &observations)
        .unwrap();
    let Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied { output, evidence }),
        derivation,
    } = &conclusion
    else {
        panic!("expected a satisfied conclusion")
    };
    assert_eq!(
        output,
        &Value::map([
            (
                "build",
                Value::map([("artifact", Value::Text("app".to_owned()))]),
            ),
            (
                "docs",
                Value::map([("page", Value::Text("guide".to_owned()))]),
            ),
        ])
    );
    assert_eq!(derivation.premises, ["evidence-build", "evidence-docs"]);
    assert!(derivation.id.starts_with("derivation-"));
    assert_eq!(derivation.id.len(), 75);
    assert!(evidence.contains(&derivation.id));
    runtime
        .verify(
            "network-1",
            Value::owned_map(vec![]),
            &observations,
            &conclusion,
        )
        .unwrap();
    runtime
        .verify("network-1", Value::owned_map(vec![]), &[], &initial)
        .unwrap();
}

#[test]
fn empty_all_is_a_premise_free_satisfied_identity() {
    let source = "§goal example/Empty@0 { §all { }; }";
    let compiled = compile_source(source, "empty-all.bhcp").unwrap();
    let runtime = KernelRuntime::new(&compiled.ir);
    let result = runtime
        .reduce("network-1", Value::owned_map(vec![]), &[])
        .unwrap();
    let Reduction::Concluded {
        result: ExecutionResult::Completed(Verdict::Satisfied { output, evidence }),
        derivation,
    } = result
    else {
        panic!("empty all must satisfy")
    };
    assert_eq!(output, Value::owned_map(vec![]));
    assert!(derivation.premises.is_empty());
    assert_eq!(evidence, [derivation.id]);
}

#[test]
fn unsupported_composition_features_have_stable_diagnostics() {
    let unknown = "§goal example/G@0 { §compose using example/reducer@0 { }; }";
    let diagnostic = compile_source(unknown, "unknown-reducer.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2004");

    let arguments = "§goal example/Child@0 { §input value: Text; }\n\
                     §goal example/Parent@0 { §all { child = example/Child@0(value = \"x\"); }; }";
    let diagnostic = compile_source(arguments, "arguments.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP1004");
}

#[test]
fn all_reducer_obeys_refutation_fault_and_unresolved_precedence() {
    let compiled = compile("canonical-all.bhcp");
    let runtime = KernelRuntime::new(&compiled.ir);
    let refuted = ChildObservation {
        child: "child-1".to_owned(),
        result: ExecutionResult::Completed(Verdict::Refuted {
            counter_evidence: vec!["counter-build".to_owned()],
        }),
    };
    let result = runtime
        .reduce(
            "network-1",
            Value::owned_map(vec![]),
            std::slice::from_ref(&refuted),
        )
        .unwrap();
    assert!(matches!(
        result,
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Refuted { .. }),
            ..
        }
    ));

    let faulted = ChildObservation {
        child: "child-2".to_owned(),
        result: ExecutionResult::Faulted(OperationalFault {
            error: reason("bhcp.fault/executor@0"),
            trace: vec![],
        }),
    };
    let decisive = runtime
        .reduce(
            "network-1",
            Value::owned_map(vec![]),
            &[refuted, faulted.clone()],
        )
        .unwrap();
    assert!(matches!(
        decisive,
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Refuted { .. }),
            ..
        }
    ));

    let pending = runtime
        .reduce(
            "network-1",
            Value::owned_map(vec![]),
            std::slice::from_ref(&faulted),
        )
        .unwrap();
    assert_eq!(
        pending,
        Reduction::Pending {
            required: vec!["build".to_owned()]
        }
    );

    let unresolved = ChildObservation {
        child: "child-1".to_owned(),
        result: ExecutionResult::Completed(Verdict::Unresolved {
            reason: reason("bhcp.reason/evidence-missing@0"),
            partial_evidence: vec!["partial-build".to_owned()],
        }),
    };
    let terminal = runtime
        .reduce(
            "network-1",
            Value::owned_map(vec![]),
            &[unresolved, faulted],
        )
        .unwrap();
    assert!(matches!(
        terminal,
        Reduction::Concluded {
            result: ExecutionResult::Faulted(_),
            ..
        }
    ));
}

#[test]
fn generic_checker_rejects_a_tampered_reduction() {
    let compiled = compile("canonical-all.bhcp");
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
