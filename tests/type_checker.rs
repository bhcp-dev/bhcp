use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::hash::HashAlgorithm;
use bhcp::model::ContentReference;
use bhcp::parser::parse_canonical;
use bhcp::pipeline::compile_source;
use bhcp::policy::TypeMode;
use bhcp::schema::parse_diagnostic;
use bhcp::typecheck::{
    CheckedType, DynamicBoundary, RefinementEvidence, RuntimeTypeCheckFailure, TypeRelations,
    check_type_definitions,
};
use bhcp::value::Value;

fn checked(source: &str) -> CheckedType {
    CheckedType::from_value(&parse_diagnostic(source).unwrap()).unwrap()
}

#[test]
fn every_v0_wire_type_has_a_closed_deterministic_checked_model() {
    let cases = [
        r#"["primitive", "Text"]"#,
        r#"["exact-number", "Rational"]"#,
        r#"["machine-integer", "signed", 32]"#,
        r#"["machine-float", "binary64"]"#,
        r#"["record", true, [["a", ["primitive", "Text"], false]]]"#,
        r#"["tuple", [["primitive", "Text"], ["exact-number", "Integer"]]]"#,
        r#"["variant", [["None", []], ["Some", [["primitive", "Text"]]]]]"#,
        r#"["union", [["primitive", "Text"], ["primitive", "Bytes"]]]"#,
        r#"["intersection", [["primitive", "Text"], ["structural", ["primitive", "Text"]]]]"#,
        r#"["list", ["primitive", "Text"]]"#,
        r#"["set", ["primitive", "Text"]]"#,
        r#"["map", ["primitive", "Text"], ["exact-number", "Integer"]]"#,
        r#"["parameter", 0]"#,
        r#"["application", ["nominal", "example/Box@0", []], [["primitive", "Text"]]]"#,
        r#"["nominal", "example/Name@0", []]"#,
        r#"["structural", ["record", false, []]]"#,
        r#"["option", ["primitive", "Text"]]"#,
        r#"["result", ["primitive", "Text"], ["nominal", "example/Error@0", []]]"#,
        r#"["special", "Dynamic"]"#,
        r#"["special", "Never"]"#,
        r#"["goal", ["primitive", "Text"], ["primitive", "Unit"], {"effects": []}, ["evidence", ["static"]]]"#,
        r#"["effect-row-type", {"effects": []}]"#,
        r#"["evidence", ["formal", "static"]]"#,
        r#"["resource", "example/Repository@0", ["primitive", "Text"]]"#,
        r#"["handle", "owned", "write", "affine", "goal", ["resource", "example/Repository@0", ["primitive", "Text"]]]"#,
        r#"["verdict", ["primitive", "Text"]]"#,
        r#"["execution-result", ["primitive", "Text"]]"#,
        r#"["reduction", ["primitive", "Text"]]"#,
        r#"["meta", "derived-form", ["primitive", "Text"], ["primitive", "Unit"]]"#,
    ];

    for source in cases {
        let value = checked(source).to_value();
        let bytes = encode_deterministic(&value).unwrap();
        assert_eq!(decode_deterministic(&bytes).unwrap(), value, "{source}");
    }
}

#[test]
fn normalization_and_subtyping_obey_the_v0_identity_rules() {
    let text = checked(r#"["primitive", "Text"]"#);
    let never = checked(r#"["special", "Never"]"#);
    let dynamic = checked(r#"["special", "Dynamic"]"#);
    let open = checked(r#"["record", true, [["name", ["primitive", "Text"], false]]]"#);
    let wider = checked(
        r#"["record", false, [["name", ["primitive", "Text"], false], ["note", ["primitive", "Text"], false]]]"#,
    );
    let relations = TypeRelations::default();

    assert!(never.is_subtype_of(&text, &relations));
    assert!(wider.is_subtype_of(&open, &relations));
    assert!(!text.is_subtype_of(&dynamic, &relations));
    assert!(!text.can_cross_dynamic_boundary(&dynamic, DynamicBoundary::Strict));
    assert!(text.can_cross_dynamic_boundary(&dynamic, DynamicBoundary::Checked));

    let union = checked(
        r#"["union", [["primitive", "Text"], ["special", "Never"], ["primitive", "Text"]]]"#,
    );
    assert_eq!(union.normalize(&relations).unwrap(), text);

    let mut nominal = TypeRelations::default();
    nominal
        .add_refinement("example/Child@0", "example/Parent@0")
        .unwrap();
    assert!(
        checked(r#"["nominal", "example/Child@0", []]"#)
            .is_subtype_of(&checked(r#"["nominal", "example/Parent@0", []]"#), &nominal,)
    );
}

#[test]
fn exact_numbers_and_machine_float_bits_validate_without_host_float_conversion() {
    for source in [
        r#"["rational", -1, 3]"#,
        r#"["decimal", -10, -2]"#,
        r#"["machine-float", "binary16", h'8000']"#,
        r#"["machine-float", "binary32", h'7fc00001']"#,
        r#"["machine-float", "binary64", h'7ff0000000000000']"#,
        r#"["machine-float", "binary128", h'7fff8000000000000000000000000001']"#,
    ] {
        let value = parse_diagnostic(source).unwrap();
        CheckedType::validate_untyped_value(&value).unwrap();
        assert_eq!(
            decode_deterministic(&encode_deterministic(&value).unwrap()).unwrap(),
            value
        );
    }
    assert!(
        CheckedType::validate_untyped_value(&parse_diagnostic(r#"["rational", 1, 0]"#).unwrap())
            .is_err()
    );
}

#[test]
fn parsed_type_definitions_materialize_with_resolved_parameters_and_refinements() {
    let source = r#"
§type example/Box@0<T: Dynamic> = { value: T, ... };
§type example/Child@0 = example/Box@0<Text>;
§refines example/Child@0 example/Box@0;
"#;
    let source_ref =
        ContentReference::from_bytes("text/bhcp", source.as_bytes(), HashAlgorithm::default());
    let parsed = parse_canonical(source, "types.bhcp", source_ref).unwrap();
    let checked = check_type_definitions(&parsed).unwrap();
    assert_eq!(checked.definitions.len(), 2);
    assert_eq!(
        checked.relations.direct_refinements("example/Child@0"),
        ["example/Box@0"]
    );
    assert_eq!(
        checked.to_value(),
        check_type_definitions(&parsed).unwrap().to_value()
    );
}

#[test]
fn generic_applications_enforce_local_arity_and_bounds() {
    for source in [
        "§type example/Box@0<T: Text> = { value: T }; §type example/Bad@0 = example/Box@0<Integer>;",
        "§type example/Box@0<T: Dynamic> = { value: T }; §type example/Bad@0 = example/Box@0;",
    ] {
        let source_ref =
            ContentReference::from_bytes("text/bhcp", source.as_bytes(), HashAlgorithm::default());
        let parsed = parse_canonical(source, "generic-invalid.bhcp", source_ref).unwrap();
        assert_eq!(
            check_type_definitions(&parsed).unwrap_err().code,
            "BHCP4101"
        );
    }

    let left = compile_source(
        "§type example/Box@0<T: Dynamic> = { value: T };",
        "generic-alpha-a.bhcp",
    )
    .unwrap();
    let right = compile_source(
        "§type example/Box@0<U: Dynamic> = { value: U };",
        "generic-alpha-b.bhcp",
    )
    .unwrap();
    assert_eq!(left.semantic_hash, right.semantic_hash);
}

#[test]
fn invalid_type_boundaries_fail_closed() {
    for source in [
        r#"["machine-integer", "signed", 0]"#,
        r#"["machine-float", "binary80"]"#,
        r#"["record", false, [["a", ["primitive", "Text"], false], ["a", ["primitive", "Text"], false]]]"#,
        r#"["variant", []]"#,
        r#"["union", [["primitive", "Text"]]]"#,
        r#"["application", ["primitive", "Text"], [["primitive", "Text"]]]"#,
        r#"["evidence", ["static", "static"]]"#,
        r#"["evidence", ["unknown"]]"#,
        r#"["handle", "shared", "write", "unrestricted", "goal", ["primitive", "Text"]]"#,
        r#"["meta", "derived-form", ["special", "Dynamic"], ["primitive", "Unit"]]"#,
        r#"["refinement", "r", ["exact-number", "Integer"], {"id": "b", "name": "x", "type": ["exact-number", "Integer"]}, {"id": "e", "type": ["primitive", "Bool"], "form": ["literal", ["integer", 1]]}]"#,
    ] {
        assert!(
            CheckedType::from_value(&parse_diagnostic(source).unwrap()).is_err(),
            "{source}"
        );
    }

    let source = "§type example/BadBorrow@0 = borrowed example/Resource@0;";
    let source_ref =
        ContentReference::from_bytes("text/bhcp", source.as_bytes(), HashAlgorithm::default());
    let parsed = parse_canonical(source, "bad-borrow.bhcp", source_ref).unwrap();
    assert_eq!(
        check_type_definitions(&parsed).unwrap_err().code,
        "BHCP4101"
    );
}

#[test]
fn typ_01_infer_strict_materializes_types_without_implicit_dynamic() {
    let compiled = compile_source("§type example/Name@0 = Text;", "typ-01.bhcp").unwrap();
    assert_eq!(compiled.ir.type_mode, TypeMode::InferStrict);
    let materialized = compiled.ir.types[0].definition.to_value();
    assert_eq!(materialized, checked(r#"["primitive", "Text"]"#).to_value());
    assert!(
        !encode_deterministic(&materialized)
            .unwrap()
            .windows("Dynamic".len())
            .any(|window| window == b"Dynamic")
    );

    let local = CheckedType::infer_value(
        &parse_diagnostic(r#"{"count": ["integer", 2], "name": "Ada"}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(
        local,
        checked(
            r#"["record", false, [["count", ["exact-number", "Integer"], false], ["name", ["primitive", "Text"], false]]]"#,
        )
    );
    assert!(
        !encode_deterministic(&local.to_value())
            .unwrap()
            .windows("Dynamic".len())
            .any(|window| window == b"Dynamic")
    );
}

#[test]
fn typ_02_03_dynamic_boundaries_require_and_materialize_runtime_checks() {
    let dynamic = checked(r#"["special", "Dynamic"]"#);
    let text = checked(r#"["primitive", "Text"]"#);
    let strict = dynamic.boundary_check_to(
        &text,
        DynamicBoundary::Strict,
        RuntimeTypeCheckFailure::TypeMismatch,
    );
    assert_eq!(strict.unwrap_err().code, "BHCP4104");

    let gradual = dynamic
        .boundary_check_to(
            &text,
            DynamicBoundary::Checked,
            RuntimeTypeCheckFailure::Fault,
        )
        .unwrap()
        .unwrap();
    assert_eq!(gradual.expected, text);
    assert_eq!(gradual.failure, RuntimeTypeCheckFailure::Fault);

    let dynamic_field = checked(r#"["record", false, [["value", ["special", "Dynamic"], false]]]"#);
    let text_field = checked(r#"["record", false, [["value", ["primitive", "Text"], false]]]"#);
    assert!(
        dynamic_field
            .boundary_check_to(
                &text_field,
                DynamicBoundary::Checked,
                RuntimeTypeCheckFailure::TypeMismatch,
            )
            .unwrap()
            .is_some()
    );
    let different_field =
        checked(r#"["record", false, [["other", ["primitive", "Text"], false]]]"#);
    assert_eq!(
        dynamic_field
            .boundary_check_to(
                &different_field,
                DynamicBoundary::Checked,
                RuntimeTypeCheckFailure::TypeMismatch,
            )
            .unwrap_err()
            .code,
        "BHCP4104"
    );

    for (actual, expected) in [
        (
            r#"["tuple", [["special", "Dynamic"], ["primitive", "Text"]]]"#,
            r#"["tuple", [["primitive", "Bytes"], ["primitive", "Text"]]]"#,
        ),
        (
            r#"["variant", [["Some", [["special", "Dynamic"]]]]]"#,
            r#"["variant", [["Some", [["primitive", "Text"]]]]]"#,
        ),
        (
            r#"["list", ["special", "Dynamic"]]"#,
            r#"["list", ["primitive", "Text"]]"#,
        ),
        (
            r#"["map", ["primitive", "Text"], ["special", "Dynamic"]]"#,
            r#"["map", ["primitive", "Text"], ["exact-number", "Integer"]]"#,
        ),
        (
            r#"["option", ["special", "Dynamic"]]"#,
            r#"["option", ["primitive", "Text"]]"#,
        ),
        (
            r#"["result", ["special", "Dynamic"], ["primitive", "Text"]]"#,
            r#"["result", ["primitive", "Bytes"], ["primitive", "Text"]]"#,
        ),
        (
            r#"["union", [["special", "Dynamic"], ["primitive", "Text"]]]"#,
            r#"["union", [["primitive", "Bytes"], ["primitive", "Text"]]]"#,
        ),
        (
            r#"["resource", "example/Repository@0", ["special", "Dynamic"]]"#,
            r#"["resource", "example/Repository@0", ["primitive", "Text"]]"#,
        ),
        (
            r#"["handle", "owned", "write", "affine", "goal", ["special", "Dynamic"]]"#,
            r#"["handle", "owned", "write", "affine", "goal", ["primitive", "Text"]]"#,
        ),
    ] {
        let actual = checked(actual);
        let expected = checked(expected);
        assert!(actual.can_cross_dynamic_boundary(&expected, DynamicBoundary::Checked));
        assert!(
            actual
                .boundary_check_to(
                    &expected,
                    DynamicBoundary::Checked,
                    RuntimeTypeCheckFailure::TypeMismatch,
                )
                .unwrap()
                .is_some()
        );
    }
}

#[test]
fn typ_04_05_nominal_identity_and_structural_width_are_distinct() {
    let relations = TypeRelations::default();
    let left = checked(r#"["nominal", "example/Left@0", []]"#);
    let right = checked(r#"["nominal", "example/Right@0", []]"#);
    assert!(!left.is_subtype_of(&right, &relations));

    let wider = checked(
        r#"["record", false, [["name", ["primitive", "Text"], false], ["note", ["primitive", "Text"], false]]]"#,
    );
    let open = checked(r#"["record", true, [["name", ["primitive", "Text"], false]]]"#);
    let closed = checked(r#"["record", false, [["name", ["primitive", "Text"], false]]]"#);
    assert!(wider.is_subtype_of(&open, &relations));
    assert!(!wider.is_subtype_of(&closed, &relations));

    let union = CheckedType::from_value(&Value::Array(vec![
        Value::Text("union".to_owned()),
        Value::Array(vec![wider.to_value(), open.to_value()]),
    ]))
    .unwrap();
    assert_eq!(union.normalize(&relations).unwrap(), open);

    let nested = CheckedType::from_value(&Value::Array(vec![
        Value::Text("list".to_owned()),
        union.to_value(),
    ]))
    .unwrap();
    assert_eq!(
        nested.normalize(&relations).unwrap(),
        CheckedType::from_value(&Value::Array(vec![
            Value::Text("list".to_owned()),
            open.to_value(),
        ]))
        .unwrap()
    );
}

#[test]
fn typ_06_refinement_introduction_requires_predicate_evidence() {
    let source = "§type example/Positive@0 = Integer where value => value > 0;";
    let source_ref =
        ContentReference::from_bytes("text/bhcp", source.as_bytes(), HashAlgorithm::default());
    let parsed = parse_canonical(source, "typ-06.bhcp", source_ref).unwrap();
    let program = check_type_definitions(&parsed).unwrap();
    let refined = &program.definitions[0].definition;
    let value = parse_diagnostic(r#"["integer", 1]"#).unwrap();

    assert_eq!(
        refined
            .validate_value(&value, &RefinementEvidence::default())
            .unwrap_err()
            .code,
        "BHCP4103"
    );
    let mut evidence = RefinementEvidence::default();
    evidence.prove(refined, &value).unwrap();
    refined.validate_value(&value, &evidence).unwrap();

    let negative = parse_diagnostic(r#"["integer", -1]"#).unwrap();
    assert_eq!(
        evidence.prove(refined, &negative).unwrap_err().code,
        "BHCP4103"
    );
    assert_eq!(
        refined
            .validate_value(&negative, &evidence)
            .unwrap_err()
            .code,
        "BHCP4103"
    );

    let opposite_source = "§type example/Negative@0 = Integer where value => value < 0;";
    let opposite_ref = ContentReference::from_bytes(
        "text/bhcp",
        opposite_source.as_bytes(),
        HashAlgorithm::default(),
    );
    let opposite_program = check_type_definitions(
        &parse_canonical(opposite_source, "typ-06-opposite.bhcp", opposite_ref).unwrap(),
    )
    .unwrap();
    let mut opposite_evidence = RefinementEvidence::default();
    opposite_evidence
        .prove(&opposite_program.definitions[0].definition, &negative)
        .unwrap();
    assert_eq!(
        refined
            .validate_value(&negative, &opposite_evidence)
            .unwrap_err()
            .code,
        "BHCP4103"
    );

    let renamed = "§type example/Positive@0 = Integer where candidate => candidate > 0;";
    assert_eq!(
        compile_source(source, "typ-06-a.bhcp")
            .unwrap()
            .semantic_hash,
        compile_source(renamed, "typ-06-b.bhcp")
            .unwrap()
            .semantic_hash
    );

    let partial = "§type example/Unsafe@0 = Integer where value => value / 0 > 1;";
    let partial_ref =
        ContentReference::from_bytes("text/bhcp", partial.as_bytes(), HashAlgorithm::default());
    let parsed = parse_canonical(partial, "typ-06-partial.bhcp", partial_ref).unwrap();
    assert_eq!(
        check_type_definitions(&parsed).unwrap_err().code,
        "BHCP4101"
    );
}

#[test]
fn refinement_binding_names_are_alpha_metadata_not_semantic_identity() {
    let with_name = parse_diagnostic(
        r#"["refinement", "positive", ["exact-number", "Integer"], {"id": "bound", "name": "value", "type": ["exact-number", "Integer"]}, {"id": "predicate", "type": ["primitive", "Bool"], "form": ["binary", ">", {"id": "left", "type": ["exact-number", "Integer"], "form": ["reference", "bound"]}, {"id": "right", "type": ["exact-number", "Integer"], "form": ["literal", ["integer", 0]]}]}]"#,
    )
    .unwrap();
    let renamed = parse_diagnostic(
        r#"["refinement", "positive", ["exact-number", "Integer"], {"id": "bound", "name": "candidate", "type": ["exact-number", "Integer"]}, {"id": "predicate", "type": ["primitive", "Bool"], "form": ["binary", ">", {"id": "left", "type": ["exact-number", "Integer"], "form": ["reference", "bound"]}, {"id": "right", "type": ["exact-number", "Integer"], "form": ["literal", ["integer", 0]]}]}]"#,
    )
    .unwrap();
    assert_eq!(
        CheckedType::from_value(&with_name).unwrap(),
        CheckedType::from_value(&renamed).unwrap()
    );
}

#[test]
fn resource_and_handle_references_have_one_exact_text_field() {
    let evidence = RefinementEvidence::default();
    let resource = checked(r#"["resource", "example/Repository@0", ["primitive", "Text"]]"#);
    resource
        .validate_value(
            &parse_diagnostic(r#"{"ref": "repository-1"}"#).unwrap(),
            &evidence,
        )
        .unwrap();
    for invalid in [
        r#"{"ref": true}"#,
        r#"{"ref": "repository-1", "extra": ["unit"]}"#,
        r#"{"ref": ""}"#,
    ] {
        assert_eq!(
            resource
                .validate_value(&parse_diagnostic(invalid).unwrap(), &evidence)
                .unwrap_err()
                .code,
            "BHCP4102"
        );
    }
    assert_eq!(
        resource
            .validate_value(
                &Value::map([("ref", Value::Text("x".repeat(129)))]),
                &evidence,
            )
            .unwrap_err()
            .code,
        "BHCP4102"
    );
}

#[test]
fn typ_07_08_option_and_result_preserve_explicit_tags_and_payloads() {
    let no_evidence = RefinementEvidence::default();
    let option = checked(r#"["option", ["primitive", "Text"]]"#);
    option
        .validate_value(
            &parse_diagnostic(r#"["variant", "None", ["unit"]]"#).unwrap(),
            &no_evidence,
        )
        .unwrap();
    option
        .validate_value(
            &parse_diagnostic(r#"["variant", "Some", "present"]"#).unwrap(),
            &no_evidence,
        )
        .unwrap();
    assert_eq!(
        option
            .validate_value(&Value::Null, &no_evidence)
            .unwrap_err()
            .code,
        "BHCP4102"
    );

    let result = checked(r#"["result", ["primitive", "Text"], ["exact-number", "Integer"]]"#);
    for source in [
        r#"["variant", "Ok", "done"]"#,
        r#"["variant", "Err", ["integer", 7]]"#,
    ] {
        result
            .validate_value(&parse_diagnostic(source).unwrap(), &no_evidence)
            .unwrap();
    }
    assert!(
        result
            .validate_value(
                &parse_diagnostic(r#"["variant", "Err", "wrong"]"#).unwrap(),
                &no_evidence,
            )
            .is_err()
    );
}

#[test]
fn goal_variance_is_input_contravariant_output_covariant_and_rows_invariant() {
    let relations = TypeRelations::default();
    let broad_input = r#"["record", true, [["name", ["primitive", "Text"], false]]]"#;
    let narrow_input = r#"["record", false, [["name", ["primitive", "Text"], false], ["note", ["primitive", "Text"], false]]]"#;
    let goal = |input: &str, output: &str, evidence: &str| {
        checked(&format!(
            r#"["goal", {input}, {output}, {{"effects": []}}, ["evidence", [{evidence}]]]"#
        ))
    };
    let accepts_more = goal(broad_input, r#"["primitive", "Text"]"#, r#""static""#);
    let accepts_less = goal(narrow_input, r#"["special", "Dynamic"]"#, r#""static""#);
    assert!(!accepts_more.is_subtype_of(&accepts_less, &relations));

    let same_output_more_input = goal(broad_input, r#"["primitive", "Text"]"#, r#""static""#);
    let same_output_less_input = goal(narrow_input, r#"["primitive", "Text"]"#, r#""static""#);
    assert!(same_output_more_input.is_subtype_of(&same_output_less_input, &relations));

    let different_evidence = goal(narrow_input, r#"["primitive", "Text"]"#, r#""formal""#);
    assert!(!same_output_more_input.is_subtype_of(&different_evidence, &relations));
}

#[test]
fn num_01_preserves_bits_rejects_overflow_and_requires_canonical_rationals() {
    let evidence = RefinementEvidence::default();
    checked(r#"["machine-float", "binary32"]"#)
        .validate_value(
            &parse_diagnostic(r#"["machine-float", "binary32", h'80000000']"#).unwrap(),
            &evidence,
        )
        .unwrap();
    assert_eq!(
        checked(r#"["machine-integer", "signed", 8]"#)
            .validate_value(&parse_diagnostic(r#"["integer", 128]"#).unwrap(), &evidence,)
            .unwrap_err()
            .code,
        "BHCP4105"
    );
    assert_eq!(
        CheckedType::validate_untyped_value(&parse_diagnostic(r#"["rational", 2, 4]"#).unwrap())
            .unwrap_err()
            .code,
        "BHCP4106"
    );
}

#[test]
fn canonical_type_validation_rejects_normalizable_but_noncanonical_input() {
    let value = parse_diagnostic(
        r#"["record", false, [["z", ["primitive", "Text"], false], ["a", ["primitive", "Text"], false]]]"#,
    )
    .unwrap();
    assert_eq!(
        CheckedType::from_canonical_value(&value).unwrap_err().code,
        "BHCP4106"
    );
}

#[test]
fn canonical_validation_rejects_relationally_subsumed_union_members() {
    let value = parse_diagnostic(
        r#"["union", [["record", true, [["name", ["primitive", "Text"], false]]], ["record", false, [["name", ["primitive", "Text"], false], ["note", ["primitive", "Text"], false]]]]]"#,
    )
    .unwrap();
    assert_eq!(
        CheckedType::from_canonical_value(&value).unwrap_err().code,
        "BHCP4106"
    );

    let mut relations = TypeRelations::default();
    relations
        .add_refinement("example/Child@0", "example/Parent@0")
        .unwrap();
    let nominal = parse_diagnostic(
        r#"["union", [["nominal", "example/Child@0", []], ["nominal", "example/Parent@0", []]]]"#,
    )
    .unwrap();
    assert_eq!(
        CheckedType::from_canonical_value_with_relations(&nominal, &relations)
            .unwrap_err()
            .code,
        "BHCP4106"
    );
}

#[test]
fn exact_integer_above_i64_max_round_trips_and_type_checks() {
    let value = parse_diagnostic(r#"["integer", 9223372036854775808]"#).unwrap();
    assert_eq!(
        decode_deterministic(&encode_deterministic(&value).unwrap()).unwrap(),
        value
    );
    checked(r#"["exact-number", "Integer"]"#)
        .validate_value(&value, &RefinementEvidence::default())
        .unwrap();
    assert_eq!(
        checked(r#"["machine-integer", "signed", 64]"#)
            .validate_value(&value, &RefinementEvidence::default())
            .unwrap_err()
            .code,
        "BHCP4105"
    );
    assert_eq!(
        checked(r#"["machine-integer", "unsigned", 63]"#)
            .validate_value(&value, &RefinementEvidence::default())
            .unwrap_err()
            .code,
        "BHCP4105"
    );
    checked(r#"["machine-integer", "unsigned", 64]"#)
        .validate_value(&value, &RefinementEvidence::default())
        .unwrap();
}
