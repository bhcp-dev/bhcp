use std::collections::{BTreeMap, BTreeSet};

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::HashAlgorithm;
use crate::model::is_symbol;
use crate::parser::{ParsedProgram, SurfaceExpression, SurfaceLiteral, SurfaceType};
use crate::value::Value;

const INVALID_TYPE: &str = "BHCP4101";
const TYPE_MISMATCH: &str = "BHCP4102";
const REFINEMENT_EVIDENCE_REQUIRED: &str = "BHCP4103";
const UNCHECKED_DYNAMIC: &str = "BHCP4104";
const NUMERIC_OVERFLOW: &str = "BHCP4105";
const NONCANONICAL_TYPE: &str = "BHCP4106";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicBoundary {
    Strict,
    Checked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeTypeCheck {
    pub expected: CheckedType,
    pub failure: RuntimeTypeCheckFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeTypeCheckFailure {
    TypeMismatch,
    Fault,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RefinementEvidence {
    proofs: BTreeSet<(Vec<u8>, Vec<u8>)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedType {
    value: Value,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeRelations {
    direct: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTypeDefinition {
    pub id: String,
    pub symbol: String,
    pub parameters: Vec<CheckedTypeParameter>,
    pub definition: CheckedType,
    pub refines: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTypeParameter {
    pub id: String,
    pub name: String,
    pub bound: CheckedType,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CheckedTypeProgram {
    pub definitions: Vec<CheckedTypeDefinition>,
    pub relations: TypeRelations,
}

impl CheckedType {
    pub fn from_value(value: &Value) -> Result<Self> {
        let value = normalize_type(value)?;
        encode_deterministic(&value).map_err(|_| {
            invalid("checked type contains a value outside the deterministic wire domain")
        })?;
        Ok(Self { value })
    }

    pub fn from_canonical_value(value: &Value) -> Result<Self> {
        Self::from_canonical_value_with_relations(value, &TypeRelations::default())
    }

    pub fn from_canonical_value_with_relations(
        value: &Value,
        relations: &TypeRelations,
    ) -> Result<Self> {
        let checked = Self::from_value(value)?.normalize(relations)?;
        if checked.value != *value {
            return Err(Diagnostic::plain(
                NONCANONICAL_TYPE,
                "type is valid but not in canonical normalized form",
            ));
        }
        Ok(checked)
    }

    pub fn to_value(&self) -> Value {
        self.value.clone()
    }

    pub fn normalize(&self, relations: &TypeRelations) -> Result<Self> {
        Ok(Self {
            value: normalize_relational(&self.value, relations)?,
        })
    }

    pub fn is_subtype_of(&self, other: &Self, relations: &TypeRelations) -> bool {
        subtype(&self.value, &other.value, relations, &mut BTreeSet::new())
    }

    pub fn can_cross_dynamic_boundary(&self, other: &Self, boundary: DynamicBoundary) -> bool {
        self == other
            || (boundary == DynamicBoundary::Checked
                && matches!(
                    dynamic_boundary_shape(&self.value, &other.value),
                    Some(true)
                ))
    }

    pub fn boundary_check_to(
        &self,
        expected: &Self,
        boundary: DynamicBoundary,
        failure: RuntimeTypeCheckFailure,
    ) -> Result<Option<RuntimeTypeCheck>> {
        if self == expected {
            return Ok(None);
        }
        if boundary == DynamicBoundary::Checked
            && matches!(
                dynamic_boundary_shape(&self.value, &expected.value),
                Some(true)
            )
        {
            return Ok(Some(RuntimeTypeCheck {
                expected: expected.clone(),
                failure,
            }));
        }
        Err(Diagnostic::plain(
            UNCHECKED_DYNAMIC,
            "Dynamic may cross a typed boundary only through an explicit runtime check",
        ))
    }

    pub fn validate_value(&self, value: &Value, evidence: &RefinementEvidence) -> Result<()> {
        validate_value_against(value, &self.value, evidence)
    }

    pub fn validate_untyped_value(value: &Value) -> Result<()> {
        validate_exact_value(value)
    }

    pub fn infer_value(value: &Value) -> Result<Self> {
        let inferred = match value {
            Value::Bool(_) => primitive_type("Bool").value,
            Value::Text(_) => primitive_type("Text").value,
            Value::Bytes(_) => primitive_type("Bytes").value,
            Value::Array(parts) if parts == &[text("unit")] => primitive_type("Unit").value,
            Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Integer(_)] if tag == "integer") =>
            {
                validate_exact_value(value)?;
                exact_type("Integer").value
            }
            Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Integer(_), Value::Integer(_)] if tag == "rational") =>
            {
                validate_exact_value(value)?;
                exact_type("Rational").value
            }
            Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Integer(_), Value::Integer(_)] if tag == "decimal") =>
            {
                validate_exact_value(value)?;
                exact_type("Decimal").value
            }
            Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Text(_), Value::Bytes(_)] if tag == "machine-float") =>
            {
                validate_exact_value(value)?;
                array([text("machine-float"), parts[1].clone()])
            }
            Value::Map(entries) => Value::Array(vec![
                text("record"),
                Value::Bool(false),
                Value::Array(
                    entries
                        .iter()
                        .map(|(name, value)| {
                            Ok(Value::Array(vec![
                                text(name),
                                Self::infer_value(value)?.to_value(),
                                Value::Bool(false),
                            ]))
                        })
                        .collect::<Result<Vec<_>>>()?,
                ),
            ]),
            Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Text(_), _] if tag == "variant") =>
            {
                let Value::Text(case) = &parts[1] else {
                    unreachable!()
                };
                Value::Array(vec![
                    text("variant"),
                    Value::Array(vec![Value::Array(vec![
                        text(case),
                        Value::Array(vec![Self::infer_value(&parts[2])?.to_value()]),
                    ])]),
                ])
            }
            Value::Array(values) => Value::Array(vec![
                text("tuple"),
                Value::Array(
                    values
                        .iter()
                        .map(|value| Self::infer_value(value).map(|value| value.to_value()))
                        .collect::<Result<Vec<_>>>()?,
                ),
            ]),
            Value::Null | Value::Integer(_) | Value::Tag(_, _) => {
                return Err(mismatch(
                    "value has no canonical core type without an explicit lowering boundary",
                ));
            }
        };
        Self::from_value(&inferred)
    }
}

impl RefinementEvidence {
    pub fn prove(&mut self, refinement: &CheckedType, value: &Value) -> Result<()> {
        let Value::Array(parts) = &refinement.value else {
            return Err(invalid(
                "refinement evidence requires a checked refinement type",
            ));
        };
        let [
            Value::Text(tag),
            Value::Text(predicate),
            base,
            binding,
            expression,
        ] = parts.as_slice()
        else {
            return Err(invalid(
                "refinement evidence requires a checked refinement type",
            ));
        };
        if tag != "refinement" {
            return Err(invalid(
                "refinement evidence requires a checked refinement type",
            ));
        }
        validate_value_against(value, base, self)?;
        let binding_id = binding
            .get("id")
            .and_then(value_text)
            .ok_or_else(|| invalid("checked refinement binding has no identity"))?;
        if evaluate_refinement_expression(expression, binding_id, value)? != Value::Bool(true) {
            return Err(Diagnostic::plain(
                REFINEMENT_EVIDENCE_REQUIRED,
                format!("value does not satisfy refinement {predicate:?}"),
            ));
        }
        self.proofs.insert((
            encode_deterministic(&refinement.value)?,
            encode_deterministic(value)?,
        ));
        Ok(())
    }

    fn proves(&self, refinement: &Value, value: &Value) -> Result<bool> {
        Ok(self.proofs.contains(&(
            encode_deterministic(refinement)?,
            encode_deterministic(value)?,
        )))
    }
}

impl TypeRelations {
    pub fn add_refinement(&mut self, subtype: &str, supertype: &str) -> Result<()> {
        if !is_symbol(subtype) || !is_symbol(supertype) || subtype == supertype {
            return Err(invalid(
                "refinement endpoints must be distinct exact symbols",
            ));
        }
        if self.reaches(supertype, subtype) {
            return Err(invalid("nominal refinement graph must be acyclic"));
        }
        if !self
            .direct
            .entry(subtype.to_owned())
            .or_default()
            .insert(supertype.to_owned())
        {
            return Err(invalid("nominal refinement edge is duplicated"));
        }
        Ok(())
    }

    pub fn direct_refinements(&self, symbol: &str) -> Vec<String> {
        self.direct
            .get(symbol)
            .into_iter()
            .flatten()
            .cloned()
            .collect()
    }

    fn reaches(&self, from: &str, to: &str) -> bool {
        let mut pending = vec![from];
        let mut seen = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if current == to {
                return true;
            }
            if seen.insert(current)
                && let Some(next) = self.direct.get(current)
            {
                pending.extend(next.iter().map(String::as_str));
            }
        }
        false
    }
}

impl CheckedTypeDefinition {
    pub fn to_value(&self) -> Value {
        let mut entries = vec![
            ("id".to_owned(), Value::Text(self.id.clone())),
            ("symbol".to_owned(), Value::Text(self.symbol.clone())),
            (
                "parameters".to_owned(),
                Value::Array(
                    self.parameters
                        .iter()
                        .map(CheckedTypeParameter::to_value)
                        .collect(),
                ),
            ),
            ("identity".to_owned(), Value::Text("nominal".to_owned())),
            ("definition".to_owned(), self.definition.to_value()),
        ];
        if !self.refines.is_empty() {
            entries.push((
                "refines".to_owned(),
                Value::Array(self.refines.iter().cloned().map(Value::Text).collect()),
            ));
        }
        Value::owned_map(entries)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.id.is_empty() || !is_symbol(&self.symbol) {
            return Err(invalid("type definition identity is invalid"));
        }
        let parameter_ids = self
            .parameters
            .iter()
            .map(|parameter| parameter.id.as_str())
            .collect::<BTreeSet<_>>();
        if parameter_ids.len() != self.parameters.len()
            || self
                .parameters
                .iter()
                .any(|parameter| parameter.id.is_empty() || parameter.name.is_empty())
        {
            return Err(invalid(
                "type parameters must have unique deterministic IDs",
            ));
        }
        for parameter in &self.parameters {
            CheckedType::from_value(&parameter.bound.value)?;
        }
        CheckedType::from_value(&self.definition.value)?;
        if !self.refines.windows(2).all(|pair| pair[0] < pair[1])
            || self.refines.iter().any(|symbol| !is_symbol(symbol))
        {
            return Err(invalid("type refinements must be sorted exact symbols"));
        }
        Ok(())
    }
}

impl CheckedTypeParameter {
    fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::Text(self.id.clone())),
            ("type", self.bound.to_value()),
        ])
    }
}

impl CheckedTypeProgram {
    pub fn to_value(&self) -> Value {
        Value::Array(
            self.definitions
                .iter()
                .map(CheckedTypeDefinition::to_value)
                .collect(),
        )
    }
}

pub fn check_type_definitions(program: &ParsedProgram) -> Result<CheckedTypeProgram> {
    let symbols = program
        .types
        .iter()
        .map(|definition| definition.symbol.as_str())
        .collect::<BTreeSet<_>>();
    let mut relations = TypeRelations::default();
    for edge in &program.refinements {
        let (
            SurfaceType::Nominal {
                symbol: subtype, ..
            },
            SurfaceType::Nominal {
                symbol: supertype, ..
            },
        ) = (&edge.subtype, &edge.supertype)
        else {
            return Err(invalid("refinement endpoints must be nominal types"));
        };
        if !symbols.contains(subtype.as_str()) {
            return Err(invalid("refinement subtype does not resolve"));
        }
        relations.add_refinement(subtype, supertype)?;
    }

    let mut source_definitions = program.types.iter().collect::<Vec<_>>();
    source_definitions.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    let mut definitions = Vec::with_capacity(source_definitions.len());
    for (index, definition) in source_definitions.into_iter().enumerate() {
        let parameters = definition
            .type_parameters
            .iter()
            .enumerate()
            .map(|(parameter_index, name)| {
                let bound = definition.type_parameter_bounds[parameter_index]
                    .as_ref()
                    .map(|value| surface_type(value, &definition.type_parameters))
                    .transpose()?
                    .unwrap_or_else(dynamic_type);
                Ok(CheckedTypeParameter {
                    id: format!("type-{}-parameter-{}", index + 1, parameter_index + 1),
                    name: name.clone(),
                    bound,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut refines = relations.direct_refinements(&definition.symbol);
        refines.sort();
        definitions.push(CheckedTypeDefinition {
            id: format!("type-{}", index + 1),
            symbol: definition.symbol.clone(),
            parameters,
            definition: surface_type(&definition.definition, &definition.type_parameters)?
                .normalize(&relations)?,
            refines,
        });
    }
    let checked = CheckedTypeProgram {
        definitions,
        relations,
    };
    validate_generic_applications(&checked)?;
    for definition in &checked.definitions {
        definition.validate()?;
    }
    Ok(checked)
}

fn validate_generic_applications(program: &CheckedTypeProgram) -> Result<()> {
    let definitions = program
        .definitions
        .iter()
        .map(|definition| (definition.symbol.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    for definition in &program.definitions {
        validate_type_applications(
            &definition.definition.value,
            &definitions,
            &program.relations,
        )?;
    }
    Ok(())
}

fn validate_type_applications(
    value: &Value,
    definitions: &BTreeMap<&str, &CheckedTypeDefinition>,
    relations: &TypeRelations,
) -> Result<()> {
    if let Value::Array(parts) = value
        && matches!(parts.first().and_then(value_text), Some("nominal"))
    {
        let (Value::Text(symbol), Value::Array(arguments)) = (&parts[1], &parts[2]) else {
            return Err(invalid("nominal application lost its normalized shape"));
        };
        if let Some(definition) = definitions.get(symbol.as_str()) {
            if arguments.len() != definition.parameters.len() {
                return Err(invalid(format!(
                    "nominal application {symbol:?} has the wrong generic arity"
                )));
            }
            for (argument, parameter) in arguments.iter().zip(&definition.parameters) {
                let argument = CheckedType::from_value(argument)?;
                if !is_special(&parameter.bound.value, "Dynamic")
                    && !argument.is_subtype_of(&parameter.bound, relations)
                {
                    return Err(invalid(format!(
                        "generic argument does not satisfy bound for parameter {:?}",
                        parameter.name
                    )));
                }
            }
        }
    }
    match value {
        Value::Array(values) => {
            for nested in values {
                validate_type_applications(nested, definitions, relations)?;
            }
        }
        Value::Map(entries) => {
            for (_, nested) in entries {
                validate_type_applications(nested, definitions, relations)?;
            }
        }
        Value::Tag(_, nested) => validate_type_applications(nested, definitions, relations)?,
        _ => {}
    }
    Ok(())
}

fn surface_type(value: &SurfaceType, parameters: &[String]) -> Result<CheckedType> {
    let wire = match value {
        SurfaceType::Primitive(name) => array([text("primitive"), text(name)]),
        SurfaceType::Exact(name) => array([text("exact-number"), text(name)]),
        SurfaceType::Parameter(name) => {
            let index = parameters
                .iter()
                .position(|candidate| candidate == name)
                .ok_or_else(|| invalid("type parameter does not resolve"))?;
            array([text("parameter"), Value::Integer(index as i128)])
        }
        SurfaceType::Dynamic => dynamic_type().to_value(),
        SurfaceType::Never => array([text("special"), text("Never")]),
        SurfaceType::Record(fields) => Value::Array(vec![
            text("record"),
            Value::Bool(false),
            Value::Array(
                fields
                    .iter()
                    .map(|field| {
                        Ok(Value::Array(vec![
                            text(&field.name),
                            surface_type(&field.value_type, parameters)?.to_value(),
                            Value::Bool(false),
                        ]))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
        ]),
        SurfaceType::StructuralRecord { fields, open } => Value::Array(vec![
            text("record"),
            Value::Bool(*open),
            Value::Array(
                fields
                    .iter()
                    .map(|field| {
                        Ok(Value::Array(vec![
                            text(&field.name),
                            surface_type(&field.value_type, parameters)?.to_value(),
                            Value::Bool(field.optional),
                        ]))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
        ]),
        SurfaceType::Nominal { symbol, arguments } => Value::Array(vec![
            text("nominal"),
            text(symbol),
            Value::Array(
                arguments
                    .iter()
                    .map(|value| surface_type(value, parameters).map(|value| value.to_value()))
                    .collect::<Result<Vec<_>>>()?,
            ),
        ]),
        SurfaceType::Tuple(members) => tagged_types("tuple", members, parameters)?,
        SurfaceType::List(element) => unary_type("list", element, parameters)?,
        SurfaceType::Set(element) => unary_type("set", element, parameters)?,
        SurfaceType::Map { key, value } => Value::Array(vec![
            text("map"),
            surface_type(key, parameters)?.to_value(),
            surface_type(value, parameters)?.to_value(),
        ]),
        SurfaceType::Option(element) => unary_type("option", element, parameters)?,
        SurfaceType::Result { ok, error } => Value::Array(vec![
            text("result"),
            surface_type(ok, parameters)?.to_value(),
            surface_type(error, parameters)?.to_value(),
        ]),
        SurfaceType::Variant(cases) => Value::Array(vec![
            text("variant"),
            Value::Array(
                cases
                    .iter()
                    .map(|case| {
                        Ok(Value::Array(vec![
                            text(&case.name),
                            Value::Array(
                                case.payload
                                    .iter()
                                    .map(|value| {
                                        surface_type(value, parameters)
                                            .map(|value| value.to_value())
                                    })
                                    .collect::<Result<Vec<_>>>()?,
                            ),
                        ]))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
        ]),
        SurfaceType::Union(members) => tagged_types("union", members, parameters)?,
        SurfaceType::Intersection(members) => tagged_types("intersection", members, parameters)?,
        SurfaceType::Reduction(output) => unary_type("reduction", output, parameters)?,
        SurfaceType::Meta {
            kind,
            input,
            output,
        } => Value::Array(vec![
            text("meta"),
            text(kind),
            surface_type(input, parameters)?.to_value(),
            surface_type(output, parameters)?.to_value(),
        ]),
        SurfaceType::Handle {
            ownership,
            access,
            usage,
            lifetime,
            value_type,
        } => {
            if ownership == "borrowed" && access.is_none() {
                return Err(invalid("borrowed handles must state read or write access"));
            }
            Value::Array(vec![
                text("handle"),
                text(ownership),
                text(access.as_deref().unwrap_or(if ownership == "shared" {
                    "read"
                } else {
                    "write"
                })),
                text(usage.as_deref().unwrap_or("unrestricted")),
                text(lifetime.as_deref().unwrap_or("goal")),
                surface_type(value_type, parameters)?.to_value(),
            ])
        }
        SurfaceType::Goal {
            input,
            output,
            effects,
            evidence,
        } => Value::Array(vec![
            text("goal"),
            surface_type(input, parameters)?.to_value(),
            surface_type(output, parameters)?.to_value(),
            effects.as_ref().map_or_else(empty_effect_row, |row| {
                let mut entries = vec![(
                    "effects".to_owned(),
                    Value::Array(
                        row.effects
                            .iter()
                            .map(|effect| Value::map([("id", text(effect))]))
                            .collect(),
                    ),
                )];
                if let Some(tail) = &row.tail {
                    entries.push(("row_variable".to_owned(), text(tail)));
                }
                Value::owned_map(entries)
            }),
            evidence
                .as_ref()
                .map(|value| surface_type(value, parameters).map(|value| value.to_value()))
                .transpose()?
                .unwrap_or_else(|| array([text("evidence"), Value::Array(vec![])])),
        ]),
        SurfaceType::Refined {
            value_type,
            binder,
            predicate,
        } => {
            let base = surface_type(value_type, parameters)?;
            let predicate_seed = surface_expression_seed(predicate, binder, &base)?;
            let ref_id = format!("refinement-{predicate_seed}");
            let mut next_id = 1;
            let (predicate, predicate_type) =
                lower_refinement_expression(predicate, binder, &base, &ref_id, &mut next_id)?;
            if predicate_type != primitive_type("Bool") {
                return Err(invalid("refinement predicate must have type Bool"));
            }
            Value::Array(vec![
                text("refinement"),
                text(&ref_id),
                base.to_value(),
                Value::map([
                    ("id", text(&format!("{ref_id}-binding"))),
                    ("type", base.to_value()),
                ]),
                predicate,
            ])
        }
    };
    CheckedType::from_value(&wire)
}

fn surface_expression_seed(
    expression: &SurfaceExpression,
    binder: &str,
    binder_type: &CheckedType,
) -> Result<String> {
    let value = Value::Array(vec![
        binder_type.to_value(),
        surface_expression_shape(expression, binder),
    ]);
    let digest = HashAlgorithm::default()
        .hash(&encode_deterministic(&value)?)
        .digest;
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn surface_expression_shape(expression: &SurfaceExpression, binder: &str) -> Value {
    match expression {
        SurfaceExpression::Literal { value, .. } => Value::Array(vec![
            text("literal"),
            match value {
                SurfaceLiteral::Bool(value) => Value::Bool(*value),
                SurfaceLiteral::Integer(value) => Value::Integer(i128::from(*value)),
                SurfaceLiteral::Text(value) => text(value),
            },
        ]),
        SurfaceExpression::Reference { name, .. } => Value::Array(vec![
            text("reference"),
            text(if name == binder { "$bound" } else { name }),
        ]),
        SurfaceExpression::Unary {
            operator, operand, ..
        } => Value::Array(vec![
            text("unary"),
            text(operator),
            surface_expression_shape(operand, binder),
        ]),
        SurfaceExpression::Binary {
            operator,
            left,
            right,
            ..
        } => Value::Array(vec![
            text("binary"),
            text(operator),
            surface_expression_shape(left, binder),
            surface_expression_shape(right, binder),
        ]),
        SurfaceExpression::Call {
            function,
            arguments,
            ..
        } => Value::Array(vec![
            text("call"),
            text(function),
            Value::Array(
                arguments
                    .iter()
                    .map(|argument| surface_expression_shape(argument, binder))
                    .collect(),
            ),
        ]),
        SurfaceExpression::If {
            condition,
            consequent,
            alternative,
            ..
        } => Value::Array(vec![
            text("if"),
            surface_expression_shape(condition, binder),
            surface_expression_shape(consequent, binder),
            surface_expression_shape(alternative, binder),
        ]),
    }
}

fn lower_refinement_expression(
    expression: &SurfaceExpression,
    binder: &str,
    binder_type: &CheckedType,
    ref_id: &str,
    next_id: &mut usize,
) -> Result<(Value, CheckedType)> {
    let id = format!("{ref_id}-expression-{}", *next_id);
    *next_id += 1;
    let (form, value_type) = match expression {
        SurfaceExpression::Literal { value, .. } => match value {
            SurfaceLiteral::Bool(value) => (
                Value::Array(vec![text("literal"), Value::Bool(*value)]),
                primitive_type("Bool"),
            ),
            SurfaceLiteral::Integer(value) => (
                Value::Array(vec![
                    text("literal"),
                    Value::Array(vec![text("integer"), Value::Integer(i128::from(*value))]),
                ]),
                exact_type("Integer"),
            ),
            SurfaceLiteral::Text(value) => (
                Value::Array(vec![text("literal"), text(value)]),
                primitive_type("Text"),
            ),
        },
        SurfaceExpression::Reference { name, .. } if name == binder => (
            Value::Array(vec![text("reference"), text(&format!("{ref_id}-binding"))]),
            binder_type.clone(),
        ),
        SurfaceExpression::Reference { .. } => {
            return Err(invalid(
                "refinement predicate may reference only its bound value",
            ));
        }
        SurfaceExpression::Unary {
            operator, operand, ..
        } => {
            let (operand, operand_type) =
                lower_refinement_expression(operand, binder, binder_type, ref_id, next_id)?;
            let result = match operator.as_str() {
                "!" if operand_type == primitive_type("Bool") => primitive_type("Bool"),
                "-" if operand_type == exact_type("Integer") => exact_type("Integer"),
                _ => {
                    return Err(invalid(
                        "refinement unary operator has invalid operand type",
                    ));
                }
            };
            (
                Value::Array(vec![text("unary"), text(operator), operand]),
                result,
            )
        }
        SurfaceExpression::Binary {
            operator,
            left,
            right,
            ..
        } => {
            let (left, left_type) =
                lower_refinement_expression(left, binder, binder_type, ref_id, next_id)?;
            let (right, right_type) =
                lower_refinement_expression(right, binder, binder_type, ref_id, next_id)?;
            if left_type != right_type {
                return Err(invalid(
                    "refinement binary operands must have the same type",
                ));
            }
            let result = match operator.as_str() {
                "==" | "!=" => primitive_type("Bool"),
                "<" | "<=" | ">" | ">=" if is_ordered_refinement_type(&left_type.to_value()) => {
                    primitive_type("Bool")
                }
                "&&" | "||" if left_type == primitive_type("Bool") => primitive_type("Bool"),
                _ => return Err(invalid("refinement binary operator is not total and typed")),
            };
            (
                Value::Array(vec![text("binary"), text(operator), left, right]),
                result,
            )
        }
        SurfaceExpression::If {
            condition,
            consequent,
            alternative,
            ..
        } => {
            let (condition, condition_type) =
                lower_refinement_expression(condition, binder, binder_type, ref_id, next_id)?;
            let (consequent, consequent_type) =
                lower_refinement_expression(consequent, binder, binder_type, ref_id, next_id)?;
            let (alternative, alternative_type) =
                lower_refinement_expression(alternative, binder, binder_type, ref_id, next_id)?;
            if condition_type != primitive_type("Bool") || consequent_type != alternative_type {
                return Err(invalid("refinement conditional has incompatible types"));
            }
            (
                Value::Array(vec![text("if"), condition, consequent, alternative]),
                consequent_type,
            )
        }
        SurfaceExpression::Call { .. } => {
            return Err(invalid(
                "refinement calls require a resolved total pure predicate",
            ));
        }
    };
    Ok((
        Value::map([
            ("id", text(&id)),
            ("type", value_type.to_value()),
            ("form", form),
        ]),
        value_type,
    ))
}

fn primitive_type(name: &str) -> CheckedType {
    CheckedType {
        value: array([text("primitive"), text(name)]),
    }
}

fn exact_type(name: &str) -> CheckedType {
    CheckedType {
        value: array([text("exact-number"), text(name)]),
    }
}

fn normalize_type(value: &Value) -> Result<Value> {
    let Value::Array(parts) = value else {
        return Err(invalid("type must be a tagged array"));
    };
    let tag = parts
        .first()
        .and_then(value_text)
        .ok_or_else(|| invalid("type tag must be text"))?;
    match tag {
        "primitive" => exact_choice(
            parts,
            &["Bool", "Text", "Bytes", "Unit", "Timestamp", "Duration"],
        ),
        "exact-number" => exact_choice(parts, &["Integer", "Rational", "Decimal"]),
        "machine-integer" => validate_machine_integer(parts),
        "machine-float" if parts.len() == 2 => {
            exact_choice(parts, &["binary16", "binary32", "binary64", "binary128"])
        }
        "record" => normalize_record(parts),
        "tuple" => normalize_type_list(parts, "tuple", 0),
        "variant" => normalize_variant(parts),
        "union" => normalize_set_type(parts, true),
        "intersection" => normalize_set_type(parts, false),
        "list" | "set" | "option" | "structural" | "verdict" | "execution-result" | "reduction" => {
            normalize_unary(parts, tag)
        }
        "map" | "result" => normalize_binary(parts, tag),
        "parameter" => validate_parameter(parts),
        "application" => normalize_application(parts),
        "refinement" => normalize_refinement(parts),
        "nominal" => normalize_nominal(parts),
        "special" => exact_choice(parts, &["Dynamic", "Never"]),
        "goal" => normalize_goal(parts),
        "effect-row-type" => normalize_effect_row_type(parts),
        "evidence" => normalize_evidence(parts),
        "resource" => normalize_resource(parts),
        "handle" => normalize_handle(parts),
        "meta" => normalize_meta(parts),
        _ => Err(invalid(format!("unknown type form {tag:?}"))),
    }
}

fn exact_choice(parts: &[Value], choices: &[&str]) -> Result<Value> {
    if let [Value::Text(tag), Value::Text(choice)] = parts
        && choices.contains(&choice.as_str())
    {
        Ok(array([text(tag), text(choice)]))
    } else {
        Err(invalid("type form has an invalid closed value"))
    }
}

fn validate_machine_integer(parts: &[Value]) -> Result<Value> {
    let [Value::Text(_), Value::Text(sign), Value::Integer(width)] = parts else {
        return Err(invalid("machine integer requires signedness and width"));
    };
    if !matches!(sign.as_str(), "signed" | "unsigned") || *width <= 0 {
        return Err(invalid("machine integer has invalid signedness or width"));
    }
    Ok(Value::Array(parts.to_vec()))
}

fn normalize_record(parts: &[Value]) -> Result<Value> {
    let [_, Value::Bool(open), Value::Array(fields)] = parts else {
        return Err(invalid("record type requires openness and fields"));
    };
    let mut normalized = fields
        .iter()
        .map(|field| {
            let [Value::Text(name), value_type, Value::Bool(optional)] = field.as_array()? else {
                return Err(invalid(
                    "record field requires name, type, and optional flag",
                ));
            };
            if name.is_empty() {
                return Err(invalid("record field name must be non-empty"));
            }
            Ok(Value::Array(vec![
                text(name),
                normalize_type(value_type)?,
                Value::Bool(*optional),
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    normalized.sort_by(|left, right| {
        value_text(&left.as_array().unwrap()[0]).cmp(&value_text(&right.as_array().unwrap()[0]))
    });
    if normalized
        .windows(2)
        .any(|pair| pair[0].as_array().unwrap()[0] == pair[1].as_array().unwrap()[0])
    {
        return Err(invalid("record field names must be unique"));
    }
    Ok(Value::Array(vec![
        text("record"),
        Value::Bool(*open),
        Value::Array(normalized),
    ]))
}

trait ValueArray {
    fn as_array(&self) -> Result<&[Value]>;
}
impl ValueArray for Value {
    fn as_array(&self) -> Result<&[Value]> {
        match self {
            Value::Array(values) => Ok(values),
            _ => Err(invalid("expected array")),
        }
    }
}

fn normalize_type_list(parts: &[Value], tag: &str, minimum: usize) -> Result<Value> {
    let [_, Value::Array(values)] = parts else {
        return Err(invalid("type list has invalid shape"));
    };
    if values.len() < minimum {
        return Err(invalid("type list has too few members"));
    }
    Ok(Value::Array(vec![
        text(tag),
        Value::Array(
            values
                .iter()
                .map(normalize_type)
                .collect::<Result<Vec<_>>>()?,
        ),
    ]))
}

fn normalize_variant(parts: &[Value]) -> Result<Value> {
    let [_, Value::Array(cases)] = parts else {
        return Err(invalid("variant has invalid shape"));
    };
    if cases.is_empty() {
        return Err(invalid("variant requires at least one case"));
    }
    let mut normalized = cases
        .iter()
        .map(|case| {
            let [Value::Text(tag), Value::Array(payload)] = case.as_array()? else {
                return Err(invalid("variant case has invalid shape"));
            };
            if tag.is_empty() {
                return Err(invalid("variant tag must be non-empty"));
            }
            Ok(Value::Array(vec![
                text(tag),
                Value::Array(
                    payload
                        .iter()
                        .map(normalize_type)
                        .collect::<Result<Vec<_>>>()?,
                ),
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    normalized.sort_by(|left, right| {
        value_text(&left.as_array().unwrap()[0]).cmp(&value_text(&right.as_array().unwrap()[0]))
    });
    if normalized
        .windows(2)
        .any(|pair| pair[0].as_array().unwrap()[0] == pair[1].as_array().unwrap()[0])
    {
        return Err(invalid("variant tags must be unique"));
    }
    Ok(Value::Array(vec![
        text("variant"),
        Value::Array(normalized),
    ]))
}

fn normalize_set_type(parts: &[Value], union: bool) -> Result<Value> {
    let tag = if union { "union" } else { "intersection" };
    let [_, Value::Array(values)] = parts else {
        return Err(invalid("set type has invalid shape"));
    };
    if values.len() < 2 {
        return Err(invalid(
            "union and intersection require at least two members",
        ));
    }
    let mut flat = Vec::new();
    for value in values {
        let normalized = normalize_type(value)?;
        if !union && is_special(&normalized, "Never") {
            return Ok(normalized);
        }
        if union && is_special(&normalized, "Never") {
            continue;
        }
        if let Value::Array(nested) = &normalized
            && nested.first() == Some(&text(tag))
            && let Some(Value::Array(members)) = nested.get(1)
        {
            flat.extend(members.clone());
        } else {
            flat.push(normalized);
        }
    }
    let mut keyed = flat
        .into_iter()
        .map(|value| Ok((encode_deterministic(&value)?, value)))
        .collect::<Result<Vec<_>>>()?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.dedup_by(|left, right| left.0 == right.0);
    let mut flat = keyed
        .into_iter()
        .map(|(_, value)| value)
        .collect::<Vec<_>>();
    if flat.is_empty() {
        return Ok(array([text("special"), text("Never")]));
    }
    if flat.len() == 1 {
        return Ok(flat.remove(0));
    }
    Ok(Value::Array(vec![text(tag), Value::Array(flat)]))
}

fn normalize_relational(value: &Value, relations: &TypeRelations) -> Result<Value> {
    let normalized = normalize_relation_children(normalize_type(value)?, relations)?;
    let Value::Array(parts) = &normalized else {
        unreachable!("normalize_type always returns a tagged array")
    };
    let Some(Value::Text(tag)) = parts.first() else {
        unreachable!("normalize_type always returns a text tag")
    };
    if !matches!(tag.as_str(), "union" | "intersection") {
        return Ok(normalized);
    }
    let Value::Array(members) = &parts[1] else {
        unreachable!("normalized set type always has members")
    };
    let mut reduced = Vec::new();
    for (index, member) in members.iter().enumerate() {
        let subsumed = members.iter().enumerate().any(|(other_index, other)| {
            if index == other_index {
                return false;
            }
            let forward = if tag == "union" {
                subtype(member, other, relations, &mut BTreeSet::new())
            } else {
                subtype(other, member, relations, &mut BTreeSet::new())
            };
            let reverse = if tag == "union" {
                subtype(other, member, relations, &mut BTreeSet::new())
            } else {
                subtype(member, other, relations, &mut BTreeSet::new())
            };
            forward && (!reverse || other_index < index)
        });
        if !subsumed {
            reduced.push(member.clone());
        }
    }
    if reduced.len() == 1 {
        return Ok(reduced.remove(0));
    }
    Ok(Value::Array(vec![text(tag), Value::Array(reduced)]))
}

fn normalize_relation_children(mut value: Value, relations: &TypeRelations) -> Result<Value> {
    let Value::Array(parts) = &mut value else {
        return Err(invalid("normalized type must be a tagged array"));
    };
    let Some(tag) = parts.first().and_then(value_text).map(str::to_owned) else {
        return Err(invalid("normalized type must have a text tag"));
    };
    match tag.as_str() {
        "record" => {
            let Value::Array(fields) = &mut parts[2] else {
                unreachable!("normalized record has fields")
            };
            for field in fields {
                let values = field.as_array()?.to_vec();
                *field = Value::Array(vec![
                    values[0].clone(),
                    normalize_relational(&values[1], relations)?,
                    values[2].clone(),
                ]);
            }
        }
        "tuple" | "union" | "intersection" => {
            let Value::Array(members) = &mut parts[1] else {
                unreachable!("normalized aggregate type has members")
            };
            for member in members {
                *member = normalize_relational(member, relations)?;
            }
        }
        "variant" => {
            let Value::Array(cases) = &mut parts[1] else {
                unreachable!("normalized variant has cases")
            };
            for case in cases {
                let values = case.as_array()?.to_vec();
                let Value::Array(payload) = &values[1] else {
                    unreachable!("normalized variant case has payload")
                };
                *case = Value::Array(vec![
                    values[0].clone(),
                    Value::Array(
                        payload
                            .iter()
                            .map(|member| normalize_relational(member, relations))
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ]);
            }
        }
        "list" | "set" | "option" | "structural" | "verdict" | "execution-result" | "reduction" => {
            parts[1] = normalize_relational(&parts[1], relations)?;
        }
        "map" | "result" => {
            parts[1] = normalize_relational(&parts[1], relations)?;
            parts[2] = normalize_relational(&parts[2], relations)?;
        }
        "application" => {
            parts[1] = normalize_relational(&parts[1], relations)?;
            let Value::Array(arguments) = &mut parts[2] else {
                unreachable!("normalized application has arguments")
            };
            for argument in arguments {
                *argument = normalize_relational(argument, relations)?;
            }
        }
        "nominal" => {
            let Value::Array(arguments) = &mut parts[2] else {
                unreachable!("normalized nominal has arguments")
            };
            for argument in arguments {
                *argument = normalize_relational(argument, relations)?;
            }
        }
        "goal" => {
            parts[1] = normalize_relational(&parts[1], relations)?;
            parts[2] = normalize_relational(&parts[2], relations)?;
        }
        "resource" => parts[2] = normalize_relational(&parts[2], relations)?,
        "handle" => parts[5] = normalize_relational(&parts[5], relations)?,
        "meta" => {
            parts[2] = normalize_relational(&parts[2], relations)?;
            parts[3] = normalize_relational(&parts[3], relations)?;
        }
        "primitive" | "exact-number" | "machine-integer" | "machine-float" | "parameter"
        | "special" | "effect-row-type" | "evidence" | "refinement" => {}
        _ => return Err(invalid("unsupported normalized relation child")),
    }
    normalize_type(&value)
}

fn normalize_unary(parts: &[Value], tag: &str) -> Result<Value> {
    let [_, value] = parts else {
        return Err(invalid("unary type has invalid shape"));
    };
    Ok(Value::Array(vec![text(tag), normalize_type(value)?]))
}
fn normalize_binary(parts: &[Value], tag: &str) -> Result<Value> {
    let [_, left, right] = parts else {
        return Err(invalid("binary type has invalid shape"));
    };
    Ok(Value::Array(vec![
        text(tag),
        normalize_type(left)?,
        normalize_type(right)?,
    ]))
}
fn validate_parameter(parts: &[Value]) -> Result<Value> {
    if matches!(parts, [_, Value::Integer(index)] if *index >= 0) {
        Ok(Value::Array(parts.to_vec()))
    } else {
        Err(invalid("parameter index must be non-negative"))
    }
}
fn normalize_application(parts: &[Value]) -> Result<Value> {
    let [_, constructor, Value::Array(arguments)] = parts else {
        return Err(invalid("application type has invalid shape"));
    };
    if arguments.is_empty() {
        return Err(invalid("application requires arguments"));
    }
    let constructor = normalize_type(constructor)?;
    if !matches!(&constructor, Value::Array(parts) if matches!(parts.first().and_then(value_text), Some("nominal" | "parameter")))
    {
        return Err(invalid(
            "application constructor must be nominal or a type parameter",
        ));
    }
    Ok(Value::Array(vec![
        text("application"),
        constructor,
        Value::Array(
            arguments
                .iter()
                .map(normalize_type)
                .collect::<Result<Vec<_>>>()?,
        ),
    ]))
}
fn normalize_refinement(parts: &[Value]) -> Result<Value> {
    if parts.len() != 5 || !matches!(&parts[1], Value::Text(id) if !id.is_empty()) {
        return Err(invalid("refinement type has invalid shape"));
    }
    let base = normalize_type(&parts[2])?;
    let binding = normalize_refinement_binding(&parts[3], &base)?;
    let binding_id = binding
        .get("id")
        .and_then(value_text)
        .expect("normalized refinement binding has an ID");
    let predicate_type = validate_refinement_expression(&parts[4], binding_id, &base)?;
    if predicate_type != array([text("primitive"), text("Bool")]) {
        return Err(invalid("refinement predicate must have type Bool"));
    }
    Ok(Value::Array(vec![
        text("refinement"),
        parts[1].clone(),
        base,
        binding,
        parts[4].clone(),
    ]))
}

fn normalize_refinement_binding(value: &Value, base: &Value) -> Result<Value> {
    let Value::Map(entries) = value else {
        return Err(invalid("refinement binding must be a map"));
    };
    if !map_keys_unique(entries)
        || !matches!(value.get("id"), Some(Value::Text(id)) if !id.is_empty())
        || !matches!(value.get("name"), None | Some(Value::Text(_)))
        || entries
            .iter()
            .any(|(key, _)| !matches!(key.as_str(), "id" | "name" | "type"))
    {
        return Err(invalid("refinement binding has invalid fields"));
    }
    let binding_type = value
        .get("type")
        .ok_or_else(|| invalid("refinement binding requires a type"))?;
    if normalize_type(binding_type)? != *base {
        return Err(invalid("refinement binding type must equal its base type"));
    }
    Ok(Value::map([
        ("id", value.get("id").unwrap().clone()),
        ("type", base.clone()),
    ]))
}

fn validate_refinement_expression(
    value: &Value,
    binding_id: &str,
    binding_type: &Value,
) -> Result<Value> {
    let Value::Map(entries) = value else {
        return Err(invalid("refinement predicate expression must be a map"));
    };
    if !map_keys_unique(entries)
        || entries
            .iter()
            .any(|(key, _)| !matches!(key.as_str(), "id" | "type" | "form"))
        || !matches!(value.get("id"), Some(Value::Text(id)) if !id.is_empty())
    {
        return Err(invalid(
            "refinement predicate expression has invalid fields",
        ));
    }
    let expression_type = normalize_type(
        value
            .get("type")
            .ok_or_else(|| invalid("refinement predicate expression requires a type"))?,
    )?;
    let Value::Array(form) = value
        .get("form")
        .ok_or_else(|| invalid("refinement predicate expression requires a form"))?
    else {
        return Err(invalid(
            "refinement predicate expression form must be an array",
        ));
    };
    let inferred_type = match form.as_slice() {
        [Value::Text(tag), literal] if tag == "literal" => {
            CheckedType::infer_value(literal)?.to_value()
        }
        [Value::Text(tag), Value::Text(reference)]
            if tag == "reference" && reference == binding_id =>
        {
            binding_type.clone()
        }
        [Value::Text(tag), Value::Text(operator), operand]
            if tag == "unary" && matches!(operator.as_str(), "!" | "-") =>
        {
            let operand_type = validate_refinement_expression(operand, binding_id, binding_type)?;
            match operator.as_str() {
                "!" if operand_type == primitive_type("Bool").to_value() => {
                    primitive_type("Bool").to_value()
                }
                "-" if operand_type == exact_type("Integer").to_value() => {
                    exact_type("Integer").to_value()
                }
                _ => return Err(invalid("refinement unary operand type is invalid")),
            }
        }
        [Value::Text(tag), Value::Text(operator), left, right]
            if tag == "binary"
                && matches!(
                    operator.as_str(),
                    "==" | "!=" | "<" | "<=" | ">" | ">=" | "&&" | "||"
                ) =>
        {
            let left_type = validate_refinement_expression(left, binding_id, binding_type)?;
            let right_type = validate_refinement_expression(right, binding_id, binding_type)?;
            if left_type != right_type {
                return Err(invalid("refinement binary operand types differ"));
            }
            match operator.as_str() {
                "==" | "!=" => primitive_type("Bool").to_value(),
                "<" | "<=" | ">" | ">=" if is_ordered_refinement_type(&left_type) => {
                    primitive_type("Bool").to_value()
                }
                "&&" | "||" if left_type == primitive_type("Bool").to_value() => {
                    primitive_type("Bool").to_value()
                }
                _ => return Err(invalid("refinement binary operand type is invalid")),
            }
        }
        [Value::Text(tag), condition, consequent, alternative] if tag == "if" => {
            let condition_type =
                validate_refinement_expression(condition, binding_id, binding_type)?;
            let consequent_type =
                validate_refinement_expression(consequent, binding_id, binding_type)?;
            let alternative_type =
                validate_refinement_expression(alternative, binding_id, binding_type)?;
            if condition_type != primitive_type("Bool").to_value()
                || consequent_type != alternative_type
            {
                return Err(invalid("refinement conditional types are incompatible"));
            }
            consequent_type
        }
        [Value::Text(tag), ..] if tag == "call" => {
            return Err(invalid(
                "refinement calls require a resolved total pure predicate",
            ));
        }
        _ => {
            return Err(invalid(
                "refinement predicate is not a total pure expression",
            ));
        }
    };
    if inferred_type != expression_type {
        return Err(invalid(
            "refinement expression declared type does not match its checked form",
        ));
    }
    Ok(expression_type)
}

fn is_ordered_refinement_type(value_type: &Value) -> bool {
    value_type == &exact_type("Integer").to_value()
        || value_type == &primitive_type("Text").to_value()
}

fn evaluate_refinement_expression(
    expression: &Value,
    binding_id: &str,
    candidate: &Value,
) -> Result<Value> {
    let Value::Array(form) = expression
        .get("form")
        .ok_or_else(|| invalid("checked refinement expression has no form"))?
    else {
        return Err(invalid("checked refinement expression form is invalid"));
    };
    match form.as_slice() {
        [Value::Text(tag), literal] if tag == "literal" => Ok(literal.clone()),
        [Value::Text(tag), Value::Text(reference)]
            if tag == "reference" && reference == binding_id =>
        {
            Ok(candidate.clone())
        }
        [Value::Text(tag), Value::Text(operator), operand] if tag == "unary" => {
            let operand = evaluate_refinement_expression(operand, binding_id, candidate)?;
            match (operator.as_str(), operand) {
                ("!", Value::Bool(value)) => Ok(Value::Bool(!value)),
                ("-", value) => exact_integer(&value)
                    .and_then(|value| {
                        value.checked_neg().ok_or_else(|| {
                            Diagnostic::plain(
                                REFINEMENT_EVIDENCE_REQUIRED,
                                "refinement integer negation exceeds the executable domain",
                            )
                        })
                    })
                    .map(exact_integer_value),
                _ => Err(invalid("checked refinement unary expression is invalid")),
            }
        }
        [Value::Text(tag), Value::Text(operator), left, right] if tag == "binary" => {
            let left = evaluate_refinement_expression(left, binding_id, candidate)?;
            let right = evaluate_refinement_expression(right, binding_id, candidate)?;
            match operator.as_str() {
                "==" => Ok(Value::Bool(left == right)),
                "!=" => Ok(Value::Bool(left != right)),
                "<" | "<=" | ">" | ">=" => {
                    let ordering = refinement_value_cmp(&left, &right)?;
                    Ok(Value::Bool(match operator.as_str() {
                        "<" => ordering.is_lt(),
                        "<=" => ordering.is_le(),
                        ">" => ordering.is_gt(),
                        ">=" => ordering.is_ge(),
                        _ => unreachable!(),
                    }))
                }
                "&&" | "||" => match (left, right) {
                    (Value::Bool(left), Value::Bool(right)) => {
                        Ok(Value::Bool(if operator == "&&" {
                            left && right
                        } else {
                            left || right
                        }))
                    }
                    _ => Err(invalid("checked refinement Boolean expression is invalid")),
                },
                _ => Err(invalid("checked refinement binary expression is invalid")),
            }
        }
        [Value::Text(tag), condition, consequent, alternative] if tag == "if" => {
            match evaluate_refinement_expression(condition, binding_id, candidate)? {
                Value::Bool(true) => {
                    evaluate_refinement_expression(consequent, binding_id, candidate)
                }
                Value::Bool(false) => {
                    evaluate_refinement_expression(alternative, binding_id, candidate)
                }
                _ => Err(invalid("checked refinement condition is not Boolean")),
            }
        }
        _ => Err(invalid("checked refinement expression is not executable")),
    }
}

fn exact_integer(value: &Value) -> Result<i128> {
    match value {
        Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Integer(_)] if tag == "integer") =>
        {
            let Value::Integer(value) = parts[1] else {
                unreachable!()
            };
            Ok(value)
        }
        _ => Err(invalid("checked refinement value is not an exact Integer")),
    }
}

fn exact_integer_value(value: i128) -> Value {
    Value::Array(vec![text("integer"), Value::Integer(value)])
}

fn refinement_value_cmp(left: &Value, right: &Value) -> Result<std::cmp::Ordering> {
    match (left, right) {
        (Value::Text(left), Value::Text(right)) => Ok(left.cmp(right)),
        _ => Ok(exact_integer(left)?.cmp(&exact_integer(right)?)),
    }
}

fn normalize_nominal(parts: &[Value]) -> Result<Value> {
    let [_, Value::Text(symbol), Value::Array(arguments)] = parts else {
        return Err(invalid("nominal type has invalid shape"));
    };
    if !is_symbol(symbol) {
        return Err(invalid("nominal type symbol is invalid"));
    }
    Ok(Value::Array(vec![
        text("nominal"),
        text(symbol),
        Value::Array(
            arguments
                .iter()
                .map(normalize_type)
                .collect::<Result<Vec<_>>>()?,
        ),
    ]))
}
fn normalize_goal(parts: &[Value]) -> Result<Value> {
    let [_, input, output, effects, evidence] = parts else {
        return Err(invalid("goal type has invalid shape"));
    };
    let effects = normalize_effect_row(effects)?;
    let evidence = normalize_type(evidence)?;
    if !matches!(&evidence, Value::Array(values) if values.first() == Some(&text("evidence"))) {
        return Err(invalid("goal evidence must be an evidence type"));
    }
    Ok(Value::Array(vec![
        text("goal"),
        normalize_type(input)?,
        normalize_type(output)?,
        effects,
        evidence,
    ]))
}
fn normalize_effect_row_type(parts: &[Value]) -> Result<Value> {
    let [_, row] = parts else {
        return Err(invalid("effect-row type has invalid shape"));
    };
    Ok(Value::Array(vec![
        text("effect-row-type"),
        normalize_effect_row(row)?,
    ]))
}
fn normalize_effect_row(value: &Value) -> Result<Value> {
    let Value::Map(entries) = value else {
        return Err(invalid("effect row must be a map"));
    };
    if !map_keys_unique(entries)
        || entries
            .iter()
            .any(|(key, _)| !matches!(key.as_str(), "effects" | "row_variable"))
        || value
            .get("row_variable")
            .is_some_and(|row| !matches!(row, Value::Text(name) if !name.is_empty()))
    {
        return Err(invalid("effect row has invalid fields"));
    }
    let Value::Array(effects) = value
        .get("effects")
        .ok_or_else(|| invalid("effect row requires effects"))?
    else {
        return Err(invalid("effect row effects must be an array"));
    };
    let effects = effects.clone();
    for effect in &effects {
        let Value::Map(fields) = effect else {
            return Err(invalid("effect must be a map"));
        };
        if !map_keys_unique(fields)
            || fields
                .iter()
                .any(|(key, _)| !matches!(key.as_str(), "id" | "resource" | "parameters"))
            || !matches!(effect.get("id"), Some(Value::Text(id)) if is_symbol(id))
            || effect.get("resource").is_some_and(
                |resource| !matches!(resource, Value::Text(value) if !value.is_empty()),
            )
            || effect
                .get("parameters")
                .is_some_and(|parameters| !matches!(parameters, Value::Array(_)))
        {
            return Err(invalid("effect has invalid fields"));
        }
    }
    let mut keyed_effects = effects
        .into_iter()
        .map(|effect| Ok((encode_deterministic(&effect)?, effect)))
        .collect::<Result<Vec<_>>>()?;
    keyed_effects.sort_by(|left, right| left.0.cmp(&right.0));
    keyed_effects.dedup_by(|left, right| left.0 == right.0);
    let effects = keyed_effects
        .into_iter()
        .map(|(_, effect)| effect)
        .collect();
    let mut output = vec![("effects".to_owned(), Value::Array(effects))];
    if let Some(row_variable) = value.get("row_variable") {
        output.push(("row_variable".to_owned(), row_variable.clone()));
    }
    Ok(Value::owned_map(output))
}
fn normalize_evidence(parts: &[Value]) -> Result<Value> {
    let [_, Value::Array(classes)] = parts else {
        return Err(invalid("evidence type has invalid shape"));
    };
    let mut values = classes
        .iter()
        .map(|value| match value {
            Value::Text(value)
                if matches!(
                    value.as_str(),
                    "formal"
                        | "static"
                        | "empirical"
                        | "statistical"
                        | "model-judged"
                        | "human-approved"
                        | "unresolved"
                ) || is_symbol(value) =>
            {
                Ok(value.clone())
            }
            _ => Err(invalid("evidence class must be text")),
        })
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(invalid("evidence classes must be unique"));
    }
    Ok(Value::Array(vec![
        text("evidence"),
        Value::Array(values.into_iter().map(Value::Text).collect()),
    ]))
}
fn normalize_resource(parts: &[Value]) -> Result<Value> {
    let [_, Value::Text(symbol), value_type] = parts else {
        return Err(invalid("resource type has invalid shape"));
    };
    if !is_symbol(symbol) {
        return Err(invalid("resource symbol is invalid"));
    }
    Ok(Value::Array(vec![
        text("resource"),
        text(symbol),
        normalize_type(value_type)?,
    ]))
}
fn normalize_handle(parts: &[Value]) -> Result<Value> {
    let [
        _,
        Value::Text(mode),
        Value::Text(access),
        Value::Text(usage),
        Value::Text(lifetime),
        value_type,
    ] = parts
    else {
        return Err(invalid("handle type has invalid shape"));
    };
    if !matches!(mode.as_str(), "owned" | "shared" | "borrowed")
        || !matches!(access.as_str(), "read" | "write")
        || !matches!(usage.as_str(), "unrestricted" | "affine" | "linear")
        || lifetime.is_empty()
        || (mode == "shared" && access == "write")
    {
        return Err(invalid("handle qualifiers are invalid"));
    }
    Ok(Value::Array(vec![
        text("handle"),
        text(mode),
        text(access),
        text(usage),
        text(lifetime),
        normalize_type(value_type)?,
    ]))
}
fn normalize_meta(parts: &[Value]) -> Result<Value> {
    let [_, Value::Text(kind), input, output] = parts else {
        return Err(invalid("meta type has invalid shape"));
    };
    if !matches!(kind.as_str(), "derived-form" | "network-shape") {
        return Err(invalid("meta kind is invalid"));
    }
    let input = normalize_type(input)?;
    let output = normalize_type(output)?;
    if contains_dynamic(&input) || contains_dynamic(&output) {
        return Err(invalid("meta types cannot contain Dynamic"));
    }
    Ok(Value::Array(vec![text("meta"), text(kind), input, output]))
}

fn subtype(
    left: &Value,
    right: &Value,
    relations: &TypeRelations,
    seen: &mut BTreeSet<(Vec<u8>, Vec<u8>)>,
) -> bool {
    if left == right || is_special(left, "Never") {
        return true;
    }
    if is_special(right, "Dynamic") {
        return false;
    }
    let key = (
        encode_deterministic(left).unwrap(),
        encode_deterministic(right).unwrap(),
    );
    if !seen.insert(key) {
        return false;
    }
    match (left, right) {
        (_, Value::Array(r)) if r.first() == Some(&text("refinement")) => false,
        (Value::Array(l), _) if l.first() == Some(&text("refinement")) && l.len() == 5 => {
            subtype(&l[2], right, relations, seen)
        }
        (Value::Array(l), Value::Array(r))
            if l.first() == Some(&text("structural")) && r.first() == Some(&text("structural")) =>
        {
            subtype(&l[1], &r[1], relations, seen)
        }
        (Value::Array(l), Value::Array(r))
            if l.first() == Some(&text("nominal")) && r.first() == Some(&text("nominal")) =>
        {
            matches!((&l[1], &r[1], &l[2], &r[2]), (Value::Text(a), Value::Text(b), left_arguments, right_arguments) if relations.reaches(a, b) && left_arguments == right_arguments)
        }
        (Value::Array(l), Value::Array(r))
            if l.first() == Some(&text("record")) && r.first() == Some(&text("record")) =>
        {
            record_subtype(l, r, relations, seen)
        }
        (_, Value::Array(r)) if r.first() == Some(&text("union")) => {
            matches!(&r[1], Value::Array(members) if members.iter().any(|member| subtype(left, member, relations, seen)))
        }
        (Value::Array(l), _) if l.first() == Some(&text("union")) => {
            matches!(&l[1], Value::Array(members) if members.iter().all(|member| subtype(member, right, relations, seen)))
        }
        (_, Value::Array(r)) if r.first() == Some(&text("intersection")) => {
            matches!(&r[1], Value::Array(members) if members.iter().all(|member| subtype(left, member, relations, seen)))
        }
        (Value::Array(l), _) if l.first() == Some(&text("intersection")) => {
            matches!(&l[1], Value::Array(members) if members.iter().any(|member| subtype(member, right, relations, seen)))
        }
        (Value::Array(l), Value::Array(r))
            if l.first() == Some(&text("goal")) && r.first() == Some(&text("goal")) =>
        {
            subtype(&r[1], &l[1], relations, seen)
                && subtype(&l[2], &r[2], relations, seen)
                && l[3] == r[3]
                && l[4] == r[4]
        }
        (Value::Array(l), Value::Array(r))
            if matches!(
                l.first().and_then(value_text),
                Some("option" | "list" | "set")
            ) && l.first() == r.first() =>
        {
            subtype(&l[1], &r[1], relations, seen)
        }
        (Value::Array(l), Value::Array(r))
            if l.first() == Some(&text("result")) && r.first() == Some(&text("result")) =>
        {
            subtype(&l[1], &r[1], relations, seen) && subtype(&l[2], &r[2], relations, seen)
        }
        _ => false,
    }
}

fn record_subtype(
    left: &[Value],
    right: &[Value],
    relations: &TypeRelations,
    seen: &mut BTreeSet<(Vec<u8>, Vec<u8>)>,
) -> bool {
    let (
        Value::Bool(left_open),
        Value::Bool(right_open),
        Value::Array(left_fields),
        Value::Array(right_fields),
    ) = (&left[1], &right[1], &left[2], &right[2])
    else {
        return false;
    };
    if !right_open && (*left_open || left_fields.len() != right_fields.len()) {
        return false;
    }
    right_fields.iter().all(|required| {
        let Ok([Value::Text(name), expected, Value::Bool(optional)]) = <&[Value; 3]>::try_from(required.as_array().unwrap_or(&[])) else { return false; };
        left_fields.iter().find(|field| matches!(field.as_array(), Ok([Value::Text(candidate), _, _]) if candidate == name)).is_some_and(|field| {
            let values = field.as_array().unwrap();
            matches!(&values[2], Value::Bool(actual_optional) if !actual_optional || *optional) && subtype(&values[1], expected, relations, seen)
        }) || *optional
    })
}

fn dynamic_boundary_shape(actual: &Value, expected: &Value) -> Option<bool> {
    if actual == expected {
        return Some(false);
    }
    if is_special(actual, "Dynamic") || is_special(expected, "Dynamic") {
        return Some(true);
    }
    let (Value::Array(actual), Value::Array(expected)) = (actual, expected) else {
        return None;
    };
    if actual.first() != expected.first() {
        return None;
    }
    let tag = actual.first().and_then(value_text)?;
    match tag {
        "record" => {
            if actual.get(1) != expected.get(1) {
                return None;
            }
            let (Value::Array(actual_fields), Value::Array(expected_fields)) =
                (actual.get(2)?, expected.get(2)?)
            else {
                return None;
            };
            if actual_fields.len() != expected_fields.len() {
                return None;
            }
            combine_dynamic(
                actual_fields
                    .iter()
                    .zip(expected_fields)
                    .map(|(actual, expected)| {
                        let (
                            Ok([actual_name, actual_type, actual_optional]),
                            Ok([expected_name, expected_type, expected_optional]),
                        ) = (actual.as_array(), expected.as_array())
                        else {
                            return None;
                        };
                        if actual_name != expected_name || actual_optional != expected_optional {
                            None
                        } else {
                            dynamic_boundary_shape(actual_type, expected_type)
                        }
                    }),
            )
        }
        "tuple" => dynamic_array_shape(actual.get(1)?, expected.get(1)?),
        "union" | "intersection" => dynamic_set_shape(actual.get(1)?, expected.get(1)?),
        "variant" => {
            let (Value::Array(actual_cases), Value::Array(expected_cases)) =
                (actual.get(1)?, expected.get(1)?)
            else {
                return None;
            };
            if actual_cases.len() != expected_cases.len() {
                return None;
            }
            combine_dynamic(actual_cases.iter().zip(expected_cases).map(
                |(actual_case, expected_case)| {
                    let (Ok([actual_name, actual_payload]), Ok([expected_name, expected_payload])) =
                        (actual_case.as_array(), expected_case.as_array())
                    else {
                        return None;
                    };
                    if actual_name != expected_name {
                        None
                    } else {
                        dynamic_array_shape(actual_payload, expected_payload)
                    }
                },
            ))
        }
        "list" | "set" | "option" | "structural" | "verdict" | "execution-result" | "reduction" => {
            dynamic_boundary_shape(actual.get(1)?, expected.get(1)?)
        }
        "map" | "result" => combine_dynamic([
            dynamic_boundary_shape(actual.get(1)?, expected.get(1)?),
            dynamic_boundary_shape(actual.get(2)?, expected.get(2)?),
        ]),
        "application" => combine_dynamic([
            dynamic_boundary_shape(actual.get(1)?, expected.get(1)?),
            dynamic_array_shape(actual.get(2)?, expected.get(2)?),
        ]),
        "nominal" => {
            if actual.get(1) != expected.get(1) {
                None
            } else {
                dynamic_array_shape(actual.get(2)?, expected.get(2)?)
            }
        }
        "goal" => {
            if actual.get(3) != expected.get(3) || actual.get(4) != expected.get(4) {
                None
            } else {
                combine_dynamic([
                    dynamic_boundary_shape(actual.get(1)?, expected.get(1)?),
                    dynamic_boundary_shape(actual.get(2)?, expected.get(2)?),
                ])
            }
        }
        "resource" => {
            if actual.get(1) != expected.get(1) {
                None
            } else {
                dynamic_boundary_shape(actual.get(2)?, expected.get(2)?)
            }
        }
        "handle" => {
            if actual.get(1..5) != expected.get(1..5) {
                None
            } else {
                dynamic_boundary_shape(actual.get(5)?, expected.get(5)?)
            }
        }
        _ => None,
    }
}

fn dynamic_array_shape(actual: &Value, expected: &Value) -> Option<bool> {
    let (Value::Array(actual), Value::Array(expected)) = (actual, expected) else {
        return None;
    };
    if actual.len() != expected.len() {
        return None;
    }
    combine_dynamic(
        actual
            .iter()
            .zip(expected)
            .map(|(actual, expected)| dynamic_boundary_shape(actual, expected)),
    )
}

fn dynamic_set_shape(actual: &Value, expected: &Value) -> Option<bool> {
    let (Value::Array(actual), Value::Array(expected)) = (actual, expected) else {
        return None;
    };
    if actual.len() != expected.len() {
        return None;
    }
    match_dynamic_members(actual, expected, &mut vec![false; expected.len()], 0, false)
}

fn match_dynamic_members(
    actual: &[Value],
    expected: &[Value],
    used: &mut [bool],
    index: usize,
    found_dynamic: bool,
) -> Option<bool> {
    if index == actual.len() {
        return Some(found_dynamic);
    }
    let mut candidates = expected
        .iter()
        .enumerate()
        .filter(|(candidate, _)| !used[*candidate])
        .filter_map(|(candidate, expected)| {
            dynamic_boundary_shape(&actual[index], expected).map(|dynamic| (candidate, dynamic))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, dynamic)| *dynamic);
    for (candidate, dynamic) in candidates {
        used[candidate] = true;
        if let Some(result) =
            match_dynamic_members(actual, expected, used, index + 1, found_dynamic || dynamic)
        {
            return Some(result);
        }
        used[candidate] = false;
    }
    None
}

fn combine_dynamic(values: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    values
        .into_iter()
        .try_fold(false, |found, value| value.map(|value| found || value))
}

fn validate_value_against(
    value: &Value,
    value_type: &Value,
    evidence: &RefinementEvidence,
) -> Result<()> {
    let Value::Array(parts) = value_type else {
        return Err(invalid("checked type lost its normalized representation"));
    };
    let Some(Value::Text(tag)) = parts.first() else {
        return Err(invalid("checked type lost its normalized tag"));
    };
    match tag.as_str() {
        "primitive" => validate_primitive_value(value, value_text(&parts[1]).unwrap()),
        "exact-number" => validate_exact_number_value(value, value_text(&parts[1]).unwrap()),
        "machine-integer" => validate_machine_integer_value(value, parts),
        "machine-float" => {
            validate_exact_value(value)?;
            if matches!(value, Value::Array(actual) if actual.get(1) == parts.get(1)) {
                Ok(())
            } else {
                Err(mismatch(
                    "machine float value format does not match its type",
                ))
            }
        }
        "record" => validate_record_value(value, parts, evidence),
        "tuple" => validate_sequence_value(value, &parts[1], evidence, false),
        "variant" => validate_variant_value(value, &parts[1], evidence),
        "union" => validate_any_member(value, &parts[1], evidence),
        "intersection" => validate_all_members(value, &parts[1], evidence),
        "list" => validate_homogeneous_sequence(value, &parts[1], evidence, false),
        "set" => validate_homogeneous_sequence(value, &parts[1], evidence, true),
        "map" => validate_map_value(value, &parts[1], &parts[2], evidence),
        "refinement" => {
            let predicate = value_text(&parts[1]).unwrap();
            if !evidence.proves(value_type, value)? {
                return Err(Diagnostic::plain(
                    REFINEMENT_EVIDENCE_REQUIRED,
                    format!("refinement {predicate:?} requires explicit predicate evidence"),
                ));
            }
            validate_value_against(value, &parts[2], evidence)
        }
        "structural" => validate_value_against(value, &parts[1], evidence),
        "option" => validate_option_value(value, &parts[1], evidence),
        "result" => validate_result_value(value, &parts[1], &parts[2], evidence),
        "special" if value_text(&parts[1]) == Some("Dynamic") => validate_exact_value(value),
        "special" => Err(mismatch("Never has no values")),
        "evidence" => validate_evidence_value(value, &parts[1]),
        "verdict" | "execution-result" | "reduction" => {
            validate_tagged_wrapper(value, &parts[1], evidence)
        }
        "resource" | "handle" => validate_reference_value(value),
        "parameter" | "application" | "nominal" => Err(mismatch(
            "generic and nominal values require a resolved definition environment",
        )),
        "goal" | "effect-row-type" => Err(mismatch(
            "goal and effect-row types do not admit ordinary runtime values",
        )),
        "meta" => Err(mismatch(
            "Meta types cannot occur in executable runtime values",
        )),
        _ => Err(invalid("unsupported normalized type form")),
    }
}

fn validate_primitive_value(value: &Value, name: &str) -> Result<()> {
    let valid = match name {
        "Bool" => matches!(value, Value::Bool(_)),
        "Text" | "Timestamp" | "Duration" => matches!(value, Value::Text(_)),
        "Bytes" => matches!(value, Value::Bytes(_)),
        "Unit" => matches!(value, Value::Array(parts) if parts == &[text("unit")]),
        _ => false,
    };
    valid
        .then_some(())
        .ok_or_else(|| mismatch(format!("value does not inhabit primitive {name}")))
}

fn validate_reference_value(value: &Value) -> Result<()> {
    match value {
        Value::Map(entries)
            if entries.len() == 1
                && matches!(entries.as_slice(), [(key, Value::Text(reference))] if key == "ref" && !reference.is_empty() && reference.len() <= 128) =>
        {
            Ok(())
        }
        _ => Err(mismatch(
            "resource and handle values must contain exactly one text ref of 1 to 128 bytes",
        )),
    }
}

fn validate_exact_number_value(value: &Value, name: &str) -> Result<()> {
    validate_exact_value(value)?;
    let expected_tag = match name {
        "Integer" => "integer",
        "Rational" => "rational",
        "Decimal" => "decimal",
        _ => return Err(invalid("unknown exact number type")),
    };
    matches!(value, Value::Array(parts) if parts.first().and_then(value_text) == Some(expected_tag))
        .then_some(())
        .ok_or_else(|| mismatch(format!("value does not inhabit exact {name}")))
}

fn validate_machine_integer_value(value: &Value, value_type: &[Value]) -> Result<()> {
    validate_exact_value(value)?;
    let [_, Value::Text(sign), Value::Integer(width)] = value_type else {
        return Err(invalid("machine integer type lost its normalized shape"));
    };
    let Value::Array(value_parts) = value else {
        return Err(mismatch(
            "machine integer value must use exact integer encoding",
        ));
    };
    let [Value::Text(tag), Value::Integer(integer)] = value_parts.as_slice() else {
        return Err(mismatch(
            "machine integer value must use exact integer encoding",
        ));
    };
    if tag != "integer" {
        return Err(mismatch(
            "machine integer value must use exact integer encoding",
        ));
    }
    let fits = if sign == "signed" {
        if *width >= 65 {
            true
        } else {
            let bound = 1_i128 << (*width as u32 - 1);
            *integer >= -bound && *integer < bound
        }
    } else if *integer < 0 {
        false
    } else if *width >= 64 {
        true
    } else {
        *integer < (1_i128 << *width as u32)
    };
    if fits {
        Ok(())
    } else {
        Err(Diagnostic::plain(
            NUMERIC_OVERFLOW,
            format!("integer value overflows {sign} {width}-bit type"),
        ))
    }
}

fn validate_record_value(
    value: &Value,
    value_type: &[Value],
    evidence: &RefinementEvidence,
) -> Result<()> {
    let Value::Map(entries) = value else {
        return Err(mismatch("record value must be a map"));
    };
    let (Value::Bool(open), Value::Array(fields)) = (&value_type[1], &value_type[2]) else {
        return Err(invalid("record type lost its normalized shape"));
    };
    for field in fields {
        let [Value::Text(name), field_type, Value::Bool(optional)] = field.as_array()? else {
            return Err(invalid("record field lost its normalized shape"));
        };
        match entries.iter().find(|(candidate, _)| candidate == name) {
            Some((_, field_value)) => validate_value_against(field_value, field_type, evidence)?,
            None if *optional => {}
            None => return Err(mismatch(format!("record value is missing field {name:?}"))),
        }
    }
    if !open
        && entries.iter().any(|(name, _)| {
            !fields.iter().any(
                |field| matches!(field.as_array(), Ok([Value::Text(candidate), _, _]) if candidate == name),
            )
        })
    {
        return Err(mismatch("closed record value contains an undeclared field"));
    }
    Ok(())
}

fn validate_sequence_value(
    value: &Value,
    member_types: &Value,
    evidence: &RefinementEvidence,
    unique: bool,
) -> Result<()> {
    let (Value::Array(values), Value::Array(types)) = (value, member_types) else {
        return Err(mismatch("tuple value must be an array"));
    };
    if values.len() != types.len() {
        return Err(mismatch("tuple value has the wrong arity"));
    }
    for (member, member_type) in values.iter().zip(types) {
        validate_value_against(member, member_type, evidence)?;
    }
    if unique {
        validate_sorted_unique(values)?;
    }
    Ok(())
}

fn validate_homogeneous_sequence(
    value: &Value,
    member_type: &Value,
    evidence: &RefinementEvidence,
    unique: bool,
) -> Result<()> {
    let Value::Array(values) = value else {
        return Err(mismatch("collection value must be an array"));
    };
    for member in values {
        validate_value_against(member, member_type, evidence)?;
    }
    if unique {
        validate_sorted_unique(values)?;
    }
    Ok(())
}

fn validate_sorted_unique(values: &[Value]) -> Result<()> {
    let encodings = values
        .iter()
        .map(encode_deterministic)
        .collect::<Result<Vec<_>>>()?;
    if encodings.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        Err(Diagnostic::plain(
            NONCANONICAL_TYPE,
            "set values must be sorted and unique by deterministic encoding",
        ))
    }
}

fn validate_map_value(
    value: &Value,
    key_type: &Value,
    element_type: &Value,
    evidence: &RefinementEvidence,
) -> Result<()> {
    // Native CBOR maps provide the unique canonical encoding for exact Text keys.
    // Every other K uses a key-encoding-sorted array of [K, V] pairs so map<K, V>
    // remains generic within the schema's recursively defined value domain.
    let text_keys = matches!(key_type, Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Text(name)] if tag == "primitive" && name == "Text"));
    match value {
        Value::Map(entries) if text_keys => {
            for (key, element) in entries {
                validate_value_against(&text(key), key_type, evidence)?;
                validate_value_against(element, element_type, evidence)?;
            }
        }
        Value::Array(entries) if !text_keys => {
            let mut previous = None;
            for entry in entries {
                let Value::Array(pair) = entry else {
                    return Err(mismatch(
                        "generic map entries must be canonical key-value pairs",
                    ));
                };
                let [key, element] = pair.as_slice() else {
                    return Err(mismatch(
                        "generic map entries must be canonical key-value pairs",
                    ));
                };
                validate_value_against(key, key_type, evidence)?;
                validate_value_against(element, element_type, evidence)?;
                let encoding = encode_deterministic(key)?;
                if previous
                    .as_ref()
                    .is_some_and(|previous| previous >= &encoding)
                {
                    return Err(Diagnostic::plain(
                        NONCANONICAL_TYPE,
                        "generic map keys must be sorted and unique by deterministic encoding",
                    ));
                }
                previous = Some(encoding);
            }
        }
        Value::Map(_) => {
            return Err(mismatch(
                "generic-key map values must use canonical key-value pairs",
            ));
        }
        Value::Array(_) => {
            return Err(mismatch("Text-keyed map values must use a CBOR map"));
        }
        _ => return Err(mismatch("map value has the wrong shape")),
    }
    Ok(())
}

fn validate_variant_value(
    value: &Value,
    cases: &Value,
    evidence: &RefinementEvidence,
) -> Result<()> {
    let (Value::Array(parts), Value::Array(cases)) = (value, cases) else {
        return Err(mismatch("variant value must use tagged variant encoding"));
    };
    let [Value::Text(tag), Value::Text(case_name), payload] = parts.as_slice() else {
        return Err(mismatch("variant value must use tagged variant encoding"));
    };
    if tag != "variant" {
        return Err(mismatch("variant value must use tagged variant encoding"));
    }
    let case = cases
        .iter()
        .find(|case| matches!(case.as_array(), Ok([Value::Text(candidate), _]) if candidate == case_name))
        .ok_or_else(|| mismatch(format!("unknown variant tag {case_name:?}")))?;
    let [_, Value::Array(payload_types)] = case.as_array()? else {
        return Err(invalid("variant case lost its normalized shape"));
    };
    match payload_types.as_slice() {
        [] => validate_primitive_value(payload, "Unit"),
        [payload_type] => validate_value_against(payload, payload_type, evidence),
        _ => validate_sequence_value(
            payload,
            &Value::Array(payload_types.clone()),
            evidence,
            false,
        ),
    }
}

fn validate_any_member(
    value: &Value,
    members: &Value,
    evidence: &RefinementEvidence,
) -> Result<()> {
    let Value::Array(members) = members else {
        return Err(invalid("union type lost its normalized members"));
    };
    members
        .iter()
        .any(|member| validate_value_against(value, member, evidence).is_ok())
        .then_some(())
        .ok_or_else(|| mismatch("value does not inhabit any union member"))
}

fn validate_all_members(
    value: &Value,
    members: &Value,
    evidence: &RefinementEvidence,
) -> Result<()> {
    let Value::Array(members) = members else {
        return Err(invalid("intersection type lost its normalized members"));
    };
    for member in members {
        validate_value_against(value, member, evidence)?;
    }
    Ok(())
}

fn validate_option_value(
    value: &Value,
    some_type: &Value,
    evidence: &RefinementEvidence,
) -> Result<()> {
    let Value::Array(parts) = value else {
        return Err(mismatch(
            "Option value must be explicitly tagged None or Some",
        ));
    };
    match parts.as_slice() {
        [Value::Text(tag), Value::Text(case), payload] if tag == "variant" && case == "None" => {
            validate_primitive_value(payload, "Unit")
        }
        [Value::Text(tag), Value::Text(case), payload] if tag == "variant" && case == "Some" => {
            validate_value_against(payload, some_type, evidence)
        }
        _ => Err(mismatch(
            "Option value must be explicitly tagged None or Some",
        )),
    }
}

fn validate_result_value(
    value: &Value,
    ok_type: &Value,
    error_type: &Value,
    evidence: &RefinementEvidence,
) -> Result<()> {
    let Value::Array(parts) = value else {
        return Err(mismatch("Result value must be explicitly tagged Ok or Err"));
    };
    match parts.as_slice() {
        [Value::Text(tag), Value::Text(case), payload] if tag == "variant" && case == "Ok" => {
            validate_value_against(payload, ok_type, evidence)
        }
        [Value::Text(tag), Value::Text(case), payload] if tag == "variant" && case == "Err" => {
            validate_value_against(payload, error_type, evidence)
        }
        _ => Err(mismatch("Result value must be explicitly tagged Ok or Err")),
    }
}

fn validate_evidence_value(value: &Value, classes: &Value) -> Result<()> {
    let (Value::Array(actual), Value::Array(required)) = (value, classes) else {
        return Err(mismatch("evidence value must be an array of classes"));
    };
    for required_class in required {
        if !actual.contains(required_class) {
            return Err(mismatch("evidence value omits a required class"));
        }
    }
    Ok(())
}

fn validate_tagged_wrapper(
    value: &Value,
    payload_type: &Value,
    evidence: &RefinementEvidence,
) -> Result<()> {
    let Value::Array(parts) = value else {
        return Err(mismatch("wrapped value must use an explicit variant tag"));
    };
    let [Value::Text(tag), Value::Text(_), payload] = parts.as_slice() else {
        return Err(mismatch("wrapped value must use an explicit variant tag"));
    };
    if tag != "variant" {
        return Err(mismatch("wrapped value must use an explicit variant tag"));
    }
    validate_value_against(payload, payload_type, evidence)
}

fn validate_exact_value(value: &Value) -> Result<()> {
    match value {
        Value::Null => {
            return Err(mismatch(
                "ambient null is not a core BHCP value; lower it to Option or a tagged absence",
            ));
        }
        Value::Integer(_) => {
            return Err(mismatch(
                "core integer values require explicit exact integer encoding",
            ));
        }
        Value::Array(parts) if matches!(parts.first(), Some(Value::Text(tag)) if tag == "unit") => {
            if !matches!(parts.as_slice(), [Value::Text(tag)] if tag == "unit") {
                return Err(invalid("unit value has invalid shape"));
            }
        }
        Value::Array(parts) if matches!(parts.first(), Some(Value::Text(tag)) if tag == "integer") =>
        {
            let [Value::Text(tag), Value::Integer(integer)] = parts.as_slice() else {
                return Err(invalid("integer value has invalid shape"));
            };
            if tag != "integer" || !is_cbor_integer(*integer) {
                return Err(invalid(
                    "integer value is outside the deterministic CBOR int domain",
                ));
            }
        }
        Value::Array(parts) if matches!(parts.first(), Some(Value::Text(tag)) if tag == "rational") =>
        {
            let [_, Value::Integer(numerator), Value::Integer(denominator)] = parts.as_slice()
            else {
                return Err(invalid("rational components are invalid"));
            };
            if *denominator <= 0 {
                return Err(invalid("rational denominator must be positive"));
            }
            if !is_cbor_integer(*numerator) || !is_cbor_integer(*denominator) {
                return Err(invalid(
                    "rational component is outside the deterministic CBOR int domain",
                ));
            }
            if greatest_common_divisor(numerator.unsigned_abs(), *denominator as u128) != 1 {
                return Err(Diagnostic::plain(
                    NONCANONICAL_TYPE,
                    "rational value must be reduced to coprime components",
                ));
            }
        }
        Value::Array(parts) if matches!(parts.first(), Some(Value::Text(tag)) if tag == "decimal") =>
        {
            let [_, Value::Integer(coefficient), Value::Integer(exponent)] = parts.as_slice()
            else {
                return Err(invalid("decimal components are invalid"));
            };
            if !is_cbor_integer(*coefficient) || !is_cbor_integer(*exponent) {
                return Err(invalid(
                    "decimal component is outside the deterministic CBOR int domain",
                ));
            }
        }
        Value::Array(parts) if matches!(parts.first(), Some(Value::Text(tag)) if tag == "machine-float") =>
        {
            let [_, Value::Text(format), Value::Bytes(bits)] = parts.as_slice() else {
                return Err(invalid("machine float value is invalid"));
            };
            let expected = match format.as_str() {
                "binary16" => 2,
                "binary32" => 4,
                "binary64" => 8,
                "binary128" => 16,
                _ => return Err(invalid("machine float format is invalid")),
            };
            if bits.len() != expected {
                return Err(invalid("machine float bit width is invalid"));
            }
        }
        Value::Array(parts) if matches!(parts.first(), Some(Value::Text(tag)) if tag == "variant") =>
        {
            let [_, Value::Text(case), _] = parts.as_slice() else {
                return Err(invalid("variant value has invalid shape"));
            };
            if case.is_empty() {
                return Err(invalid("variant value has an empty case tag"));
            }
        }
        _ => {}
    }
    match value {
        Value::Array(values)
            if matches!(
                values.first().and_then(value_text),
                Some("unit" | "integer" | "rational" | "decimal" | "machine-float")
            ) => {}
        Value::Array(values)
            if values.first().and_then(value_text) == Some("variant") && values.len() == 3 =>
        {
            validate_exact_value(&values[2])?;
        }
        Value::Array(values) => {
            for nested in values {
                validate_exact_value(nested)?;
            }
        }
        Value::Map(entries) => {
            if !map_keys_unique(entries) {
                return Err(invalid("core value maps must have unique keys"));
            }
            for (_, nested) in entries {
                validate_exact_value(nested)?;
            }
        }
        Value::Tag(_, _) => return Err(mismatch("CBOR tags are not core BHCP values")),
        _ => {}
    }
    Ok(())
}

fn is_cbor_integer(value: i128) -> bool {
    (-1 - i128::from(u64::MAX)..=i128::from(u64::MAX)).contains(&value)
}

fn greatest_common_divisor(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn tagged_types(tag: &str, members: &[SurfaceType], parameters: &[String]) -> Result<Value> {
    Ok(Value::Array(vec![
        text(tag),
        Value::Array(
            members
                .iter()
                .map(|value| surface_type(value, parameters).map(|value| value.to_value()))
                .collect::<Result<Vec<_>>>()?,
        ),
    ]))
}
fn unary_type(tag: &str, value: &SurfaceType, parameters: &[String]) -> Result<Value> {
    Ok(Value::Array(vec![
        text(tag),
        surface_type(value, parameters)?.to_value(),
    ]))
}
fn dynamic_type() -> CheckedType {
    CheckedType {
        value: array([text("special"), text("Dynamic")]),
    }
}
fn empty_effect_row() -> Value {
    Value::map([("effects", Value::Array(vec![]))])
}
fn contains_dynamic(value: &Value) -> bool {
    is_special(value, "Dynamic")
        || match value {
            Value::Array(values) => values.iter().any(contains_dynamic),
            Value::Map(entries) => entries.iter().any(|(_, value)| contains_dynamic(value)),
            Value::Tag(_, value) => contains_dynamic(value),
            _ => false,
        }
}
fn is_special(value: &Value, name: &str) -> bool {
    matches!(value, Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(tag), Value::Text(actual)] if tag == "special" && actual == name))
}
fn value_text(value: &Value) -> Option<&str> {
    match value {
        Value::Text(value) => Some(value),
        _ => None,
    }
}
fn map_keys_unique(entries: &[(String, Value)]) -> bool {
    entries
        .iter()
        .map(|(key, _)| key)
        .collect::<BTreeSet<_>>()
        .len()
        == entries.len()
}
fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}
fn array<const N: usize>(values: [Value; N]) -> Value {
    Value::Array(values.into())
}
fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_TYPE, message)
}

fn mismatch(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(TYPE_MISMATCH, message)
}
