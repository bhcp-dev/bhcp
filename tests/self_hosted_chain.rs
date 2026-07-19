use std::fs;
use std::path::PathBuf;

use bhcp::kernel::{
    ArgumentMode, ChildObservation, ExecutionResult, KernelRuntime, OperationalFault, Reason,
    Reduction, Verdict,
};
use bhcp::model::{BhcpType, ExpressionForm};
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
fn chain_and_explicit_compose_have_identical_typed_edges_and_semantics() {
    let derived = compile("canonical-chain.bhcp");
    let explicit = compile("canonical-chain-compose.bhcp");
    assert_eq!(derived.ir.semantic_id, explicit.ir.semantic_id);
    assert_eq!(derived.ir.semantic_value(), explicit.ir.semantic_value());
    assert_ne!(derived.ast.artifact_id, explicit.ast.artifact_id);

    let network = derived.ir.goals[3].body.as_ref().unwrap();
    assert_eq!(network.children.iter().map(|child| child.tag.as_str()).collect::<Vec<_>>(), ["patch", "checked", "saved"]);
    assert!(network.reducer.starts_with("bhcp/prelude.chain-reducer-"));
    assert!(network.to_value().get("kind").is_none());
    assert!(network.to_value().get("order").is_none());
    assert!(network.to_value().get("dependency").is_none());
    assert!(network.children[0].arguments.is_empty());
    assert_eq!(network.children[1].arguments[0].name, "patch");
    assert_eq!(network.children[1].arguments[0].mode, ArgumentMode::Borrow);
    assert_eq!(network.children[2].arguments[0].mode, ArgumentMode::Move);
    for (argument, predecessor) in [
        (&network.children[1].arguments[0], "patch"),
        (&network.children[2].arguments[0], "checked"),
    ] {
        let ExpressionForm::Call(symbol, parameters) = &argument.value.form else {
            panic!("expected an explicit observed-output edge")
        };
        assert_eq!(symbol, "bhcp/kernel.observed-output@0");
        assert!(matches!(
            parameters.as_slice(),
            [parameter] if parameter.form == ExpressionForm::Literal(Value::Text(predecessor.to_owned()))
        ));
    }
}

#[test]
fn chain_requests_only_the_next_child_and_returns_the_last_output() {
    let compiled = compile("canonical-chain.bhcp");
    let runtime = KernelRuntime::new(&compiled.ir);
    assert_eq!(runtime.reduce("network-1", Value::owned_map(vec![]), &[]).unwrap(), Reduction::Pending { required: vec!["patch".to_owned()] });
    let patch = ChildObservation { child: "child-1".to_owned(), result: satisfied(Value::map([("value", Value::Text("diff".to_owned()))]), "evidence-edit") };
    assert_eq!(runtime.reduce("network-1", Value::owned_map(vec![]), std::slice::from_ref(&patch)).unwrap(), Reduction::Pending { required: vec!["checked".to_owned()] });
    let checked = ChildObservation { child: "child-2".to_owned(), result: satisfied(Value::map([("passed", Value::Bool(true))]), "evidence-check") };
    assert_eq!(runtime.reduce("network-1", Value::owned_map(vec![]), &[patch.clone(), checked.clone()]).unwrap(), Reduction::Pending { required: vec!["saved".to_owned()] });
    let saved = ChildObservation { child: "child-3".to_owned(), result: satisfied(Value::map([("receipt", Value::Text("stored".to_owned()))]), "evidence-save") };
    let observations = vec![patch, checked, saved];
    let result = runtime.reduce("network-1", Value::owned_map(vec![]), &observations).unwrap();
    assert!(matches!(result, Reduction::Concluded { result: ExecutionResult::Completed(Verdict::Satisfied { ref output, ref evidence }), .. } if output == &Value::map([("receipt", Value::Text("stored".to_owned()))]) && evidence.starts_with(&["evidence-edit".to_owned(), "evidence-check".to_owned(), "evidence-save".to_owned()])));
    runtime.verify("network-1", Value::owned_map(vec![]), &observations, &result).unwrap();
}

#[test]
fn refutation_unresolved_and_fault_stop_the_chain() {
    let compiled = compile("canonical-chain.bhcp");
    let runtime = KernelRuntime::new(&compiled.ir);
    let cases = [
        ExecutionResult::Completed(Verdict::Refuted { counter_evidence: vec!["counter-edit".to_owned()] }),
        ExecutionResult::Completed(Verdict::Unresolved { reason: reason("bhcp.reason/evidence-missing@0"), partial_evidence: vec!["partial-edit".to_owned()] }),
        ExecutionResult::Faulted(OperationalFault { error: reason("bhcp.fault/executor@0"), trace: vec![] }),
    ];
    for result in cases {
        let observation = ChildObservation { child: "child-1".to_owned(), result };
        assert!(matches!(runtime.reduce("network-1", Value::owned_map(vec![]), &[observation]).unwrap(), Reduction::Concluded { .. }));
    }
}

#[test]
fn chain_order_is_semantic_and_edge_types_fail_closed() {
    let source = fs::read_to_string(fixture("canonical-chain.bhcp")).unwrap();
    let reordered = source.replace(
        "        patch = example/Edit@0();\n        checked = example/Check@0(patch = borrow patch);",
        "        checked = example/Check@0(patch = borrow patch);\n        patch = example/Edit@0();",
    );
    let diagnostic = compile_source(&reordered, "forward-chain-edge.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2001");

    let wrong_type = source.replace("§input patch: { value: Text };", "§input patch: Text;");
    let diagnostic = compile_source(&wrong_type, "wrong-chain-edge.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");
}

#[test]
fn forged_observation_edge_is_rejected_before_reduction() {
    let mut compiled = compile("canonical-chain.bhcp");
    let argument = &mut compiled.ir.goals[3].body.as_mut().unwrap().children[1].arguments[0];
    let ExpressionForm::Call(_, parameters) = &mut argument.value.form else { unreachable!() };
    parameters[0].form = ExpressionForm::Literal(Value::Text("saved".to_owned()));
    let diagnostic = KernelRuntime::new(&compiled.ir).reduce("network-1", Value::owned_map(vec![]), &[]).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4001");
}

#[test]
fn empty_chain_is_a_premise_free_satisfied_unit_identity() {
    let compiled = compile_source("§goal example/Empty@0 { §chain { }; }", "empty-chain.bhcp").unwrap();
    assert_eq!(compiled.ir.goals[0].output, BhcpType::Primitive("Unit"));
    let result = KernelRuntime::new(&compiled.ir).reduce("network-1", Value::owned_map(vec![]), &[]).unwrap();
    assert!(matches!(result, Reduction::Concluded { result: ExecutionResult::Completed(Verdict::Satisfied { output, .. }), ref derivation } if output == Value::Array(vec![Value::Text("unit".to_owned())]) && derivation.premises.is_empty()));
}
