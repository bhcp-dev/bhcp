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

fn integer(value: i128) -> Value {
    Value::Array(vec![
        Value::Text("integer".to_owned()),
        Value::Integer(value),
    ])
}

fn unit() -> Value {
    Value::Array(vec![Value::Text("unit".to_owned())])
}
