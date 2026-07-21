use bhcp::extensions::ExtensionRegistry;
use bhcp::inspection::render_artifact;
use bhcp::pipeline::{compile_source, compile_source_with_extension_registry};
use bhcp::schema::validate_root;
use bhcp::value::Value;

const DERIVED: &str = r#"
§function example/reviewReducer@0(
    parent: Unit,
    observations: {}
): Reduction<Unit> =
    bhcp/kernel.conclude@0(
        bhcp/kernel.satisfied@0(
            bhcp/kernel.unit@0(),
            bhcp/kernel.satisfied-evidence@0(observations)
        )
    );

§function example/lowerReview@0(
    form: Meta<DerivedForm, Unit, Unit>
): Meta<NetworkShape, Unit, Unit> =
    bhcp/meta.network-shape@0(
        bhcp/meta.unit-type@0(),
        form,
        "example/reviewReducer@0"
    );

§extension example/review@0 derived {
    lowering example/lowerReview@0;
    input Unit;
    output Unit;
    children [];
}
"#;

const NATIVE: &str = r#"
§extension example/native@0 native {
    payload_schema { media_type: "application/cbor", size: 1, digests: [{ algorithm: example/hash@0, digest: h'06' }] };
    type_rule { media_type: "text/plain", size: 1, digests: [{ algorithm: example/hash@0, digest: h'07' }] };
    effect_rule { media_type: "text/plain", size: 1, digests: [{ algorithm: example/hash@0, digest: h'08' }] };
    policy_rule { media_type: "text/plain", size: 1, digests: [{ algorithm: example/hash@0, digest: h'09' }] };
    normalization_rule { media_type: "text/plain", size: 1, digests: [{ algorithm: example/hash@0, digest: h'0a' }] };
    evidence_rule { media_type: "text/plain", size: 1, digests: [{ algorithm: example/hash@0, digest: h'0b' }] };
}
"#;

fn payload_schema(digest: u8) -> Value {
    Value::map([
        ("media_type", Value::Text("application/cbor".to_owned())),
        ("size", Value::Integer(1)),
        (
            "digests",
            Value::Array(vec![Value::map([
                ("algorithm", Value::Text("example/hash@0".to_owned())),
                ("digest", Value::Bytes(vec![digest])),
            ])]),
        ),
    ])
}

#[test]
fn derived_extension_executes_to_core_and_removes_its_meta_definition() {
    let compiled = compile_source(DERIVED, "derived-extension.bhcp").unwrap();
    compiled.ir.validate().unwrap();
    validate_root(&compiled.ir.to_value(true), "semantic-ir").unwrap();

    assert!(compiled.ir.extensions.is_empty());
    let goal = compiled
        .ir
        .goals
        .iter()
        .find(|goal| goal.symbol == "example/review@0")
        .expect("derived extension must become checked core goal IR");
    let body = goal
        .body
        .as_ref()
        .expect("derived lowerer must emit a network");
    assert!(body.children.is_empty());
    assert!(body.reducer.starts_with("example/reviewReducer-"));
    let Value::Array(functions) = compiled
        .ir
        .to_value(true)
        .get("functions")
        .expect("semantic IR functions")
        .clone()
    else {
        panic!("semantic IR functions are not an array");
    };
    let symbols = functions
        .iter()
        .filter_map(|function| match function.get("symbol") {
            Some(Value::Text(symbol)) => Some(symbol.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!symbols.contains(&"example/lowerReview@0"));
    assert!(symbols.contains(&body.reducer.as_str()));
}

#[test]
fn derived_lowerer_identity_is_unobservable_after_equivalent_execution() {
    let renamed = DERIVED.replace("example/lowerReview@0", "example/alternateLowerer@0");
    let baseline = compile_source(DERIVED, "derived-a.bhcp").unwrap();
    let alternate = compile_source(&renamed, "derived-b.bhcp").unwrap();
    assert_eq!(baseline.ir.semantic_value(), alternate.ir.semantic_value());
    assert_eq!(baseline.semantic_hash, alternate.semantic_hash);
}

#[test]
fn missing_or_core_overriding_derived_extensions_fail_before_ir() {
    let missing = DERIVED.replace(
        "lowering example/lowerReview@0;",
        "lowering example/missing@0;",
    );
    let diagnostic = compile_source(&missing, "missing-lowerer.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP5003");
    assert!(diagnostic.message.contains("lowering"));

    let core_override = DERIVED.replace("example/review@0", "bhcp/prelude.all@0");
    let diagnostic = compile_source(&core_override, "core-override.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP5003");
    assert!(diagnostic.message.contains("core"));
}

#[test]
fn derived_extension_reducer_declaration_must_match_the_specialization() {
    for (name, changed) in [
        ("parent", DERIVED.replace("parent: Unit", "parent: Text")),
        (
            "observations",
            DERIVED.replace("observations: {}", "observations: { forged: Text }"),
        ),
        (
            "result",
            DERIVED.replace("): Reduction<Unit> =", "): Reduction<Text> ="),
        ),
    ] {
        let diagnostic = compile_source(&changed, &format!("wrong-{name}.bhcp")).unwrap_err();
        assert_eq!(diagnostic.code, "BHCP5003", "{name}: {diagnostic}");
        assert!(diagnostic.message.contains("reducer signature"));
    }
}

#[test]
fn supported_native_extension_retains_exact_schema_payload_and_identity() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register_native(
            "example/native@0",
            payload_schema(0x06),
            Value::map([("enabled", Value::Bool(true))]),
        )
        .unwrap();
    let compiled =
        compile_source_with_extension_registry(NATIVE, "native.bhcp", &registry).unwrap();
    compiled.ir.validate().unwrap();
    validate_root(&compiled.ir.to_value(true), "semantic-ir").unwrap();
    assert_eq!(compiled.ir.extensions.len(), 1);
    let node = &compiled.ir.extensions[0];
    assert_eq!(node.extension, "example/native@0");
    assert!(node.must_understand);
    assert_eq!(
        node.payload.get("value"),
        Some(&Value::map([("enabled", Value::Bool(true))]))
    );
    assert!(node.payload.get("descriptor").is_some());

    let mut changed_registry = ExtensionRegistry::new();
    changed_registry
        .register_native(
            "example/native@0",
            payload_schema(0x06),
            Value::map([("enabled", Value::Bool(false))]),
        )
        .unwrap();
    let changed =
        compile_source_with_extension_registry(NATIVE, "native.bhcp", &changed_registry).unwrap();
    assert_ne!(compiled.semantic_hash, changed.semantic_hash);
}

#[test]
fn unsupported_or_schema_mismatched_native_extensions_fail_closed() {
    let unsupported = compile_source(NATIVE, "unsupported-native.bhcp").unwrap_err();
    assert_eq!(unsupported.code, "BHCP5003");
    assert!(unsupported.message.contains("unsupported native"));

    let mut registry = ExtensionRegistry::new();
    registry
        .register_native(
            "example/native@0",
            payload_schema(0xff),
            Value::map([("enabled", Value::Bool(true))]),
        )
        .unwrap();
    let mismatch =
        compile_source_with_extension_registry(NATIVE, "mismatched-native.bhcp", &registry)
            .unwrap_err();
    assert_eq!(mismatch.code, "BHCP5003");
    assert!(mismatch.message.contains("payload schema"));
}

#[test]
fn registry_rejects_duplicate_or_invalid_native_registrations() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register_native(
            "example/native@0",
            payload_schema(0x06),
            Value::map([("enabled", Value::Bool(true))]),
        )
        .unwrap();
    assert!(
        registry
            .register_native(
                "example/native@0",
                payload_schema(0x06),
                Value::map([("enabled", Value::Bool(false))]),
            )
            .is_err()
    );
    assert!(
        ExtensionRegistry::new()
            .register_native("not a symbol", payload_schema(0x06), Value::Bool(true))
            .is_err()
    );
}

#[test]
fn checked_in_reference_extension_is_executable_and_fully_lowered() {
    let source = include_str!("../conformance/v0/reference-program/extension.bhcp");
    let compiled = compile_source(source, "reference-program/extension.bhcp").unwrap();
    assert!(compiled.ir.extensions.is_empty());
    assert!(
        compiled
            .ir
            .goals
            .iter()
            .any(|goal| goal.symbol == "bhcp.reference/review@0" && goal.body.is_some())
    );
}

#[test]
fn native_nodes_are_source_order_independent_inspectable_and_fail_closed_when_tampered() {
    let other = NATIVE
        .replace("example/native@0", "example/other@0")
        .replace("h'06'", "h'16'");
    let mut registry = ExtensionRegistry::new();
    registry
        .register_native(
            "example/native@0",
            payload_schema(0x06),
            Value::map([("enabled", Value::Bool(true))]),
        )
        .unwrap();
    registry
        .register_native(
            "example/other@0",
            payload_schema(0x16),
            Value::map([(
                "sequence",
                Value::Array(vec![Value::Integer(2), Value::Integer(1)]),
            )]),
        )
        .unwrap();

    let forward = compile_source_with_extension_registry(
        &format!("{NATIVE}\n{other}"),
        "native-forward.bhcp",
        &registry,
    )
    .unwrap();
    let reverse = compile_source_with_extension_registry(
        &format!("{other}\n{NATIVE}"),
        "native-reverse.bhcp",
        &registry,
    )
    .unwrap();
    assert_eq!(forward.semantic_hash, reverse.semantic_hash);
    let inspection = render_artifact(&forward.ir.to_value(true), None);
    assert!(inspection.contains("example/native@0 must-understand"));
    assert!(inspection.contains("example/other@0 must-understand"));

    let mut reordered = forward.ir.clone();
    reordered.extensions.swap(0, 1);
    assert_eq!(reordered.validate().unwrap_err().code, "BHCP4001");

    let mut duplicate = forward.ir.clone();
    duplicate.extensions[1].extension = duplicate.extensions[0].extension.clone();
    assert_eq!(duplicate.validate().unwrap_err().code, "BHCP4001");

    let mut optional = forward.ir.clone();
    optional.extensions[0].must_understand = false;
    assert_eq!(optional.validate().unwrap_err().code, "BHCP4001");

    let mut forged = forward.ir.clone();
    forged.extensions[0].payload = Value::Text("forged".to_owned());
    assert_eq!(forged.validate().unwrap_err().code, "BHCP4001");

    let mut wrong_symbol = forward.ir.clone();
    let Value::Map(envelope) = &mut wrong_symbol.extensions[0].payload else {
        unreachable!()
    };
    let Value::Map(descriptor) = &mut envelope
        .iter_mut()
        .find(|(key, _)| key == "descriptor")
        .unwrap()
        .1
    else {
        unreachable!()
    };
    descriptor
        .iter_mut()
        .find(|(key, _)| key == "symbol")
        .unwrap()
        .1 = Value::Text("example/forged@0".to_owned());
    assert_eq!(wrong_symbol.validate().unwrap_err().code, "BHCP4001");

    let mut extra_envelope_field = forward.ir.clone();
    let Value::Map(envelope) = &mut extra_envelope_field.extensions[0].payload else {
        unreachable!()
    };
    envelope.push(("forged".to_owned(), Value::Bool(true)));
    assert_eq!(
        extra_envelope_field.validate().unwrap_err().code,
        "BHCP4001"
    );
}

#[test]
fn mixed_modes_policy_override_fields_and_non_record_calls_are_rejected_before_graph_ir() {
    let duplicate = format!(
        "{DERIVED}\n{}",
        NATIVE.replace("example/native@0", "example/review@0")
    );
    let diagnostic = compile_source(&duplicate, "mixed-extension.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP1003");
    assert!(diagnostic.message.contains("duplicate definition symbol"));

    let policy_override = DERIVED.replace(
        "children [];",
        "children [];\n    policy_override example/policy@0;",
    );
    let diagnostic = compile_source(&policy_override, "policy-override.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP1004");
    assert!(diagnostic.message.contains("policy_override"));

    let invoked = format!(
        "{DERIVED}\n§goal example/workflow@0 {{\n    §chain {{\n        review = example/review@0();\n    }};\n}}"
    );
    let diagnostic = compile_source(&invoked, "non-record-extension-call.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP2003");
    assert!(diagnostic.message.contains("record inputs"));
}
