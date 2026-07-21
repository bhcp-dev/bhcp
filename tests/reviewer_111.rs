use bhcp::hash::HashAlgorithm;
use bhcp::kernel::{ChildObservation, ExecutionResult, KernelRuntime, Verdict};
use bhcp::model::ContentReference;
use bhcp::obligation::build_obligation_graph;
use bhcp::pipeline::compile_source;
use bhcp::proof::{ObligationProofRequest, verify_obligation_proof};
use bhcp::value::Value;
use bhcp::verification::{VerificationRequest, VerifierRegistry};

#[test]
fn decreasing_measure_must_prove_the_child_measure_stays_nonnegative() {
    let unsound = r#"
§goal example/Walk@0 {
    §input remaining: Integer;
    §requires "positive-domain": 0 < remaining;
    §none {
        next = example/Walk@0(remaining = remaining - 2);
    };
}
"#;
    let diagnostic = compile_source(unsound, "unsound-measure.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2301");
}

#[test]
fn guarded_nonnegative_reference_measure_is_accepted() {
    let guarded = r#"
§goal example/Walk@0 {
    §input remaining: Integer;
    §requires "nonnegative-domain": 0 <= remaining;
    §gate when 0 < remaining {
        next = example/Walk@0(remaining = remaining - 1);
    };
}
"#;
    let mut compilation = compile_source(guarded, "guarded-measure.bhcp").unwrap();
    let network = compilation.ir.goals[0].body.as_ref().unwrap();
    let runtime = KernelRuntime::new(&compilation.ir);
    let base = runtime
        .reduce(&network.id, Value::map([("remaining", integer(0))]), &[])
        .unwrap();
    assert!(matches!(
        base,
        bhcp::kernel::Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Satisfied {
                output,
                ..
            }),
            ..
        } if output == unit()
    ));
    let diagnostic = runtime
        .reduce(
            &network.id,
            Value::map([("remaining", integer(0))]),
            &[ChildObservation {
                child: network.children[0].id.clone(),
                result: ExecutionResult::Completed(Verdict::Satisfied {
                    output: unit(),
                    evidence: vec!["unselected-recursive-step".to_owned()],
                }),
            }],
        )
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4101");
    assert_eq!(
        diagnostic.message,
        "a closed gate cannot observe its unselected child"
    );
    let recursive = runtime
        .reduce(
            &network.id,
            Value::map([("remaining", integer(1))]),
            &[ChildObservation {
                child: network.children[0].id.clone(),
                result: ExecutionResult::Completed(Verdict::Satisfied {
                    output: unit(),
                    evidence: vec!["recursive-step".to_owned()],
                }),
            }],
        )
        .unwrap();
    assert!(matches!(
        recursive,
        bhcp::kernel::Reduction::Concluded {
            result: ExecutionResult::Completed(Verdict::Satisfied {
                output,
                ..
            }),
            ..
        } if output == unit()
    ));
    let reducer = &mut compilation.ir.functions[0];
    let bhcp::model::ExpressionForm::If(condition, _, _) = &mut reducer.definition.form else {
        panic!("recursive gate reducer must retain its checked guard")
    };
    condition.form = bhcp::model::ExpressionForm::Literal(Value::Bool(false));
    let diagnostic = compilation.ir.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert_eq!(
        diagnostic.message,
        "recursive child measure does not match retained checker evidence"
    );
}

#[test]
fn measure_step_must_fit_the_retained_domain_boundary() {
    let sound = r#"
§goal example/Walk@0 {
    §input remaining: Integer;
    §requires "two-or-more": 2 <= remaining;
    §none {
        next = example/Walk@0(remaining = remaining - 2);
    };
}
"#;
    let mut compilation = compile_source(sound, "sound-measure.bhcp").unwrap();
    let child = &mut compilation.ir.goals[0].body.as_mut().unwrap().children[0];
    let bhcp::kernel::RecursionBound::WellFounded { measure } = child.recursion.as_mut().unwrap()
    else {
        panic!("expected a well-founded measure")
    };
    let bhcp::model::ExpressionForm::Binary(_, _, step) = &mut measure.form else {
        panic!("expected a subtractive measure")
    };
    step.form = bhcp::model::ExpressionForm::Literal(integer(3));
    child.arguments[0].value = measure.clone();
    let diagnostic = compilation.ir.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
}

#[test]
fn proof_reconstruction_supports_the_same_closed_child_expression_subset() {
    let source = r#"
§goal example/Child@0 {
    §input and_value: Bool;
    §input comparison: Bool;
    §input conditional: Integer;
    §input difference: Integer;
    §input equality: Bool;
    §input inverted: Bool;
    §input negated: Integer;
    §input or_value: Bool;
    §input product: Integer;
    §input quotient: Integer;
    §input remainder: Integer;
    §input sum: Integer;
    §output value: Bool;
}
§goal example/Parent@0 {
    §input count: Integer;
    §input enabled: Bool;
    §input flag: Bool;
    §input threshold: Integer;
    §output child: { value: Bool };
    §requires "ready": true;
    §all {
        child = example/Child@0(
            and_value = enabled && flag,
            comparison = count >= threshold,
            conditional = if enabled then count else threshold,
            difference = count - threshold,
            equality = enabled == flag,
            inverted = !enabled,
            negated = -count,
            or_value = enabled || flag,
            product = count * threshold,
            quotient = count / threshold,
            remainder = count % threshold,
            sum = count + threshold
        );
    };
}
"#;
    let compilation = compile_source(source, "computed-child-edge.bhcp").unwrap();
    let graph = build_obligation_graph(&compilation).unwrap();
    let parent = Value::map([
        ("count", integer(6)),
        ("enabled", Value::Bool(true)),
        ("flag", Value::Bool(false)),
        ("threshold", integer(3)),
    ]);
    let child_output = Value::map([("value", Value::Bool(true))]);
    let parent_output = Value::map([("child", child_output.clone())]);
    let network = compilation.ir.goals[1].body.as_ref().unwrap();
    let observations = [ChildObservation {
        child: network.children[0].id.clone(),
        result: ExecutionResult::Completed(Verdict::Satisfied {
            output: child_output.clone(),
            evidence: vec!["derivation-child-proof".to_owned()],
        }),
    }];
    let claimed = KernelRuntime::new(&compilation.ir)
        .reduce(&network.id, parent.clone(), &observations)
        .unwrap();
    let registry = VerifierRegistry::new();
    let candidate = ContentReference::from_bytes(
        "application/vnd.bhcp.candidate",
        b"candidate-v1",
        HashAlgorithm::default(),
    );
    let execution_graph = ContentReference::from_bytes(
        "application/cbor",
        b"execution-graph",
        HashAlgorithm::default(),
    );
    let verification = registry
        .verify(VerificationRequest {
            compilation: &compilation,
            goal: "example/Parent@0",
            execution_instance: Some(&network.id),
            input: &parent,
            output: &parent_output,
            subject: candidate.clone(),
            subject_bytes: b"candidate-v1",
            execution_graph,
            produced_at: "2026-07-21T09:30:00Z",
        })
        .unwrap();
    verify_obligation_proof(ObligationProofRequest {
        compilation: &compilation,
        obligation_graph: &graph,
        network: &network.id,
        parent: &parent,
        observations: &observations,
        claimed: &claimed,
        evidence: &verification.bundle,
        payloads: &verification.payloads,
        evaluation_contexts: &[],
        verifier_registry: &registry,
        candidate: &candidate,
        candidate_bytes: b"candidate-v1",
        produced_at: "2026-07-21T09:30:00Z",
    })
    .unwrap();
}

#[test]
fn retention_predecessor_outputs_cannot_bypass_recursive_ownership_checks() {
    for (name, handle) in [
        ("owned", "owned write affine 'retain example/File@0"),
        ("borrowed", "borrowed read 'request example/File@0"),
        ("shared", "shared read 'release example/File@0"),
    ] {
        let source = handle_retention_source(handle, "", "", "");
        let diagnostic = compile_source(&source, &format!("{name}-retention.bhcp")).unwrap_err();
        assert_eq!(diagnostic.code, "BHCP4401", "{name}: {diagnostic}");
    }

    for mode in ["move ", "borrow ", "share "] {
        let source = handle_retention_source(
            "owned write affine 'retain example/File@0",
            mode,
            mode,
            mode,
        );
        let diagnostic = compile_source(&source, "explicit-handle-retention.bhcp").unwrap_err();
        assert_eq!(diagnostic.code, "BHCP4401", "{mode}: {diagnostic}");
    }
}

#[test]
fn received_ir_rejects_handle_bearing_retention_predecessors() {
    let source = handle_retention_source("Text", "", "", "");
    let mut compilation = compile_source(&source, "text-retention.bhcp").unwrap();
    let handle = bhcp::model::BhcpType::Handle(Box::new(bhcp::model::HandleType {
        ownership: "owned".to_owned(),
        access: "write".to_owned(),
        usage: "affine".to_owned(),
        lifetime: "retain".to_owned(),
        value_type: bhcp::model::BhcpType::Nominal("example/File@0".to_owned(), vec![]),
    }));
    let state = bhcp::model::BhcpType::Record(vec![bhcp::model::FieldType {
        name: "state".to_owned(),
        value_type: handle,
    }]);
    let read = compilation
        .ir
        .goals
        .iter_mut()
        .find(|goal| goal.symbol == "example/StateRead@0")
        .unwrap();
    read.output = state;
    let diagnostic = compilation.ir.validate().unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
    assert_eq!(
        diagnostic.message,
        "retention predecessor outputs containing resource handles are not executable in this slice"
    );
}

fn handle_retention_source(
    handle: &str,
    prior_mode: &str,
    expected_mode: &str,
    new_mode: &str,
) -> String {
    format!(
        r#"
§goal example/StateRead@0 {{
    §input resource: Text;
    §output state: {handle};
}}
§goal example/Candidate@0 {{
    §input prior: {{ state: {handle} }};
    §input resource: Text;
    §output state: {handle};
}}
§goal example/CompareAndSwap@0 {{
    §input expected_version: {{ state: {handle} }};
    §input new_value: {{ state: {handle} }};
    §input resource: Text;
    §output committed: Text;
}}
§goal example/Retain@0 {{
    §input resource: Text;
    §output committed: Text;
    §compose using bhcp/prelude.retain-reducer@0 {{
        state-read = example/StateRead@0(resource = resource);
        candidate = example/Candidate@0(prior = {prior_mode}state-read, resource = resource);
        compare-and-swap = example/CompareAndSwap@0(
            expected_version = {expected_mode}state-read,
            new_value = {new_mode}candidate,
            resource = resource
        );
    }};
}}
"#
    )
}

fn integer(value: i128) -> Value {
    Value::Array(vec![
        Value::Text("integer".to_owned()),
        Value::Integer(value),
    ])
}

fn unit() -> Value {
    Value::Array(vec![Value::Text("unit".to_owned())])
}
