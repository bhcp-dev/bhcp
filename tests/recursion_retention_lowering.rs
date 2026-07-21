use bhcp::kernel::{
    ChildObservation, ExecutionResult, KernelRuntime, OperationalFault, Reason, RecursionBound,
    Reduction, Verdict,
};
use bhcp::model::ExpressionForm;
use bhcp::obligation::build_obligation_graph;
use bhcp::pipeline::compile_source;
use bhcp::prelude::RETAIN_FEATURE;
use bhcp::value::Value;

const BOUNDED: &str = r#"
§goal example/Walk@0 {
    §input remaining: Integer;
    §limit "depth": remaining <= 64;
    §none {
        next = example/Walk@0(remaining = remaining);
    };
}
"#;

const WELL_FOUNDED: &str = r#"
§goal example/Walk@0 {
    §input remaining: Integer;
    §requires "positive-domain": 0 < remaining;
    §none {
        next = example/Walk@0(remaining = remaining - 1);
    };
}
"#;

#[test]
fn recursive_children_retain_static_bounds_and_decreasing_measure_evidence() {
    let bounded = compile_source(BOUNDED, "bounded-recursion.bhcp").unwrap();
    let child = &bounded.ir.goals[0].body.as_ref().unwrap().children[0];
    assert_eq!(
        child.recursion,
        Some(RecursionBound::Bounded { maximum: 64 })
    );
    let graph = build_obligation_graph(&bounded).unwrap();
    let repeated_graph = build_obligation_graph(&bounded).unwrap();
    assert_eq!(graph.to_value(), repeated_graph.to_value());
    let Value::Array(nodes) = graph.to_value().get("nodes").unwrap().clone() else {
        panic!("obligation graph nodes must be an array")
    };
    assert!(nodes.iter().any(|node| {
        node.get("recursion").and_then(|value| value.get("maximum")) == Some(&Value::Integer(64))
    }));

    let first = compile_source(WELL_FOUNDED, "well-founded-recursion.bhcp").unwrap();
    let repeated = compile_source(WELL_FOUNDED, "well-founded-recursion.bhcp").unwrap();
    assert_eq!(first.ir_bytes, repeated.ir_bytes);
    let child = &first.ir.goals[0].body.as_ref().unwrap().children[0];
    let Some(RecursionBound::WellFounded { measure }) = &child.recursion else {
        panic!("recursive child must retain its checked decreasing measure")
    };
    assert!(matches!(measure.form, ExpressionForm::Binary(ref operator, _, _) if operator == "-"));
    assert!(child.to_value().get("recursion").is_some());
}

#[test]
fn unbounded_recursion_is_rejected_before_semantic_ir() {
    let diagnostic = compile_source(
        r#"
§goal example/Walk@0 {
    §input remaining: Integer;
    §none { next = example/Walk@0(remaining = remaining); };
}
"#,
        "unbounded-recursion.bhcp",
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2301");
    assert_eq!(
        diagnostic.message,
        "recursive child requires a static bound or a checked decreasing measure"
    );
}

#[test]
fn retained_ir_revalidates_recursion_metadata_against_the_recursive_edge() {
    let mut compiled = compile_source(WELL_FOUNDED, "well-founded-recursion.bhcp").unwrap();
    compiled.ir.goals[0].body.as_mut().unwrap().children[0].recursion = None;
    let diagnostic = compiled.ir.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert_eq!(
        diagnostic.message,
        "recursive kernel child omits checked termination evidence"
    );

    let mut compiled = compile_source(BOUNDED, "bounded-recursion.bhcp").unwrap();
    compiled.ir.goals[0].body.as_mut().unwrap().children[0].recursion =
        Some(RecursionBound::Bounded { maximum: 65 });
    let diagnostic = compiled.ir.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert_eq!(
        diagnostic.message,
        "recursive child bound does not match a retained static limit"
    );

    let mut compiled = compile_source(WELL_FOUNDED, "well-founded-recursion.bhcp").unwrap();
    let child = &mut compiled.ir.goals[0].body.as_mut().unwrap().children[0];
    let RecursionBound::WellFounded { measure } = child.recursion.as_mut().unwrap() else {
        unreachable!()
    };
    let ExpressionForm::Binary(operator, _, _) = &mut measure.form else {
        unreachable!()
    };
    *operator = "+".to_owned();
    let diagnostic = compiled.ir.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert_eq!(
        diagnostic.message,
        "recursive child measure does not match retained checker evidence"
    );
}

#[test]
fn recursion_rejects_invalid_bounds_and_non_decreasing_or_wrong_coordinate_measures() {
    for (name, source, code) in [
        ("zero", BOUNDED.replace("64", "0"), "BHCP2301"),
        ("negative", BOUNDED.replace("64", "-1"), "BHCP2301"),
        ("non-integer", BOUNDED.replace("64", "\"many\""), "BHCP2003"),
        (
            "unchanged",
            WELL_FOUNDED.replace("remaining - 1", "remaining"),
            "BHCP2301",
        ),
        (
            "increasing",
            WELL_FOUNDED.replace("remaining - 1", "remaining + 1"),
            "BHCP2301",
        ),
        (
            "non-strict-domain",
            WELL_FOUNDED.replace("0 < remaining", "0 <= remaining"),
            "BHCP2301",
        ),
    ] {
        let diagnostic = compile_source(&source, &format!("{name}-recursion.bhcp")).unwrap_err();
        assert_eq!(diagnostic.code, code, "{name}: {diagnostic}");
    }

    let wrong_variable = r#"
§goal example/Walk@0 {
    §input other: Integer;
    §input remaining: Integer;
    §requires "other-positive": 0 < other;
    §none {
        next = example/Walk@0(other = other, remaining = remaining - 1);
    };
}
"#;
    assert_eq!(
        compile_source(wrong_variable, "wrong-variable-recursion.bhcp")
            .unwrap_err()
            .code,
        "BHCP2301"
    );

    let crossed_coordinates = r#"
§goal example/Walk@0 {
    §input other: Integer;
    §input remaining: Integer;
    §limit "depth": remaining <= 64;
    §none {
        next = example/Walk@0(other = remaining, remaining = other);
    };
}
"#;
    assert_eq!(
        compile_source(crossed_coordinates, "crossed-recursion.bhcp")
            .unwrap_err()
            .code,
        "BHCP2301"
    );
}

#[test]
fn recursion_structural_ids_ignore_labels_but_artifact_identity_retains_them() {
    let first = compile_source(BOUNDED, "bounded-recursion.bhcp").unwrap();
    let renamed_source = BOUNDED.replace("\"depth\"", "\"recursion-depth\"");
    let renamed = compile_source(&renamed_source, "bounded-recursion.bhcp").unwrap();
    assert_eq!(first.semantic_hash, renamed.semantic_hash);
    assert_ne!(first.ir_hash, renamed.ir_hash);

    let recursion_id = |compilation| {
        let graph = build_obligation_graph(compilation).unwrap().to_value();
        let Value::Array(nodes) = graph.get("nodes").unwrap() else {
            panic!("obligation nodes must be an array")
        };
        nodes
            .iter()
            .find(|node| node.get("recursion").is_some())
            .and_then(|node| node.get("id"))
            .cloned()
            .unwrap()
    };
    assert_eq!(recursion_id(&first), recursion_id(&renamed));
}

const RETENTION: &str = r#"
§goal example/StateRead@0 {
    §input resource: Text;
    §output state: Text;
}
§goal example/Candidate@0 {
    §input prior: { state: Text };
    §input resource: Text;
    §output state: Text;
}
§goal example/CompareAndSwap@0 {
    §input expected_version: { state: Text };
    §input new_value: { state: Text };
    §input resource: Text;
    §output committed: Text;
}
§goal example/Retain@0 {
    §input other: Text;
    §input resource: Text;
    §output committed: Text;
    §compose using bhcp/prelude.retain-reducer@0 {
        state-read = example/StateRead@0(resource = resource);
        candidate = example/Candidate@0(prior = state-read, resource = resource);
        compare-and-swap = example/CompareAndSwap@0(
            expected_version = state-read,
            new_value = candidate,
            resource = resource
        );
    };
}
"#;

fn evidence_output(field: &str, value: &str) -> ExecutionResult {
    ExecutionResult::Completed(Verdict::Satisfied {
        output: Value::map([(field, Value::Text(value.to_owned()))]),
        evidence: vec![format!("evidence-{field}")],
    })
}

fn unresolved(code: &str) -> ExecutionResult {
    ExecutionResult::Completed(Verdict::Unresolved {
        reason: Reason {
            code: code.to_owned(),
            message: "fixture".to_owned(),
            details: None,
        },
        partial_evidence: vec![],
    })
}

fn retention_parent() -> Value {
    Value::map([
        ("other", Value::Text("other-cell".to_owned())),
        ("resource", Value::Text("cell".to_owned())),
    ])
}

#[test]
fn retention_is_a_versioned_prelude_chain_over_ordinary_child_goals() {
    let compiled = compile_source(RETENTION, "retention.bhcp").unwrap();
    assert!(compiled.ir.features.contains(&RETAIN_FEATURE.to_owned()));
    let network = compiled.ir.goals[3].body.as_ref().unwrap();
    assert_eq!(
        network
            .children
            .iter()
            .map(|child| child.tag.as_str())
            .collect::<Vec<_>>(),
        ["state-read", "candidate", "compare-and-swap"]
    );
    assert!(network.reducer.starts_with("bhcp/prelude.retain-reducer-"));
    assert!(
        network
            .children
            .iter()
            .all(|child| child.recursion.is_none())
    );
    assert!(network.to_value().get("retention").is_none());
    assert!(network.to_value().get("state").is_none());
    assert!(network.to_value().get("retry").is_none());
    assert!(
        compiled
            .ir
            .goals
            .iter()
            .all(|goal| goal.effects.effects.is_empty())
    );

    let diagnostic = compile_source(
        &RETENTION.replace("state-read", "snapshot"),
        "invalid-retention-shape.bhcp",
    )
    .unwrap_err();
    assert_eq!(
        diagnostic.message,
        "retention lowering requires state-read, candidate, and compare-and-swap children in order"
    );
}

#[test]
fn retention_coordinates_are_revalidated_and_create_no_implicit_authority() {
    let source_substitution = RETENTION.replacen(
        "expected_version = state-read",
        "expected_version = candidate",
        1,
    );
    let diagnostic =
        compile_source(&source_substitution, "substituted-retention.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP3001");
    assert_eq!(
        diagnostic.message,
        "retention lowering does not preserve exact resource, prior-version, and new-value coordinates"
    );

    let mut compiled = compile_source(RETENTION, "retention.bhcp").unwrap();
    let commit = &mut compiled.ir.goals[3].body.as_mut().unwrap().children[2];
    let expected = commit
        .arguments
        .iter_mut()
        .find(|argument| argument.name == "expected_version")
        .unwrap();
    let ExpressionForm::Call(_, parameters) = &mut expected.value.form else {
        unreachable!()
    };
    parameters[0].form = ExpressionForm::Literal(Value::Text("candidate".to_owned()));
    let diagnostic = compiled.ir.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert_eq!(
        diagnostic.message,
        "retention network does not preserve exact resource, expected-version, and new-value coordinates"
    );

    let mut compiled = compile_source(RETENTION, "retention.bhcp").unwrap();
    let commit = &mut compiled.ir.goals[3].body.as_mut().unwrap().children[2];
    let resource = commit
        .arguments
        .iter_mut()
        .find(|argument| argument.name == "resource")
        .unwrap();
    let ExpressionForm::Call(_, parameters) = &mut resource.value.form else {
        unreachable!()
    };
    parameters[0].form = ExpressionForm::Literal(Value::Text("other".to_owned()));
    let diagnostic = compiled.ir.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert_eq!(
        diagnostic.message,
        "retention network does not preserve exact resource, expected-version, and new-value coordinates"
    );
}

#[test]
fn only_a_satisfied_candidate_reaches_compare_and_swap() {
    let compiled = compile_source(RETENTION, "retention.bhcp").unwrap();
    let network = compiled.ir.goals[3].body.as_ref().unwrap();
    let runtime = KernelRuntime::new(&compiled.ir);
    let child = |tag: &str, result| ChildObservation {
        child: network
            .children
            .iter()
            .find(|child| child.tag == tag)
            .unwrap()
            .id
            .clone(),
        result,
    };

    assert!(matches!(
        runtime.reduce(&network.id, retention_parent(), &[]).unwrap(),
        Reduction::Pending { required } if required == ["state-read"]
    ));

    let read = child("state-read", evidence_output("state", "v1"));
    let rejected = child(
        "candidate",
        ExecutionResult::Completed(Verdict::Refuted {
            counter_evidence: vec!["evidence-rejected".to_owned()],
        }),
    );
    assert!(matches!(
        runtime
            .reduce(&network.id, retention_parent(), &[read.clone(), rejected])
            .unwrap(),
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Refuted { .. }),
            ..
        }
    ));

    let faulted_candidate = child(
        "candidate",
        ExecutionResult::Faulted(OperationalFault {
            error: Reason {
                code: "bhcp.fault/policy-required-stale-evidence@0".to_owned(),
                message: "fixture".to_owned(),
                details: None,
            },
            trace: vec![],
        }),
    );
    assert!(matches!(
        runtime
            .reduce(
                &network.id,
                retention_parent(),
                &[read.clone(), faulted_candidate]
            )
            .unwrap(),
        Reduction::Concluded {
            result: ExecutionResult::Faulted(_),
            ..
        }
    ));

    let stale = child("state-read", unresolved("bhcp.reason/stale-evidence@0"));
    assert!(matches!(
        runtime
            .reduce(&network.id, retention_parent(), &[stale])
            .unwrap(),
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Unresolved { .. }),
            ..
        }
    ));

    let candidate = child("candidate", evidence_output("state", "v2"));
    assert!(matches!(
        runtime
            .reduce(
                &network.id,
                retention_parent(),
                &[read.clone(), candidate.clone()],
            )
            .unwrap(),
        Reduction::Pending { required } if required == ["compare-and-swap"]
    ));

    let conflict = child(
        "compare-and-swap",
        unresolved("bhcp.reason/compare-and-swap-conflict@0"),
    );
    assert!(matches!(
        runtime
            .reduce(
                &network.id,
                retention_parent(),
                &[read, candidate, conflict],
            )
            .unwrap(),
        Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Unresolved { .. }),
            ..
        }
    ));
}
