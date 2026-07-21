//! Checked canonical BHCP definitions used to derive standard behavior.

use std::collections::HashMap;

use crate::diagnostic::{Diagnostic, Result};
use crate::hash::HashAlgorithm;
use crate::kernel::{KernelArgument, RecursionBound};
use crate::model::{BhcpType, ContentReference, Expression, FieldType, VariantCaseType};
use crate::parser::{
    ParsedProgram, SurfaceExpression, SurfaceFunction, SurfaceLiteral, SurfaceType, parse_canonical,
};

pub const ALL_LOWERER: &str = "bhcp/prelude.lower-all@0";
pub const ALL_REDUCER: &str = "bhcp/prelude.all-reducer@0";
pub const ALL_FEATURE: &str = "bhcp/feature.self-hosted-all@0";
pub const ANY_LOWERER: &str = "bhcp/prelude.lower-any@0";
pub const ANY_REDUCER: &str = "bhcp/prelude.any-reducer@0";
pub const ANY_FEATURE: &str = "bhcp/feature.self-hosted-any@0";
pub const NONE_LOWERER: &str = "bhcp/prelude.lower-none@0";
pub const NONE_REDUCER: &str = "bhcp/prelude.none-reducer@0";
pub const NONE_FEATURE: &str = "bhcp/feature.self-hosted-none@0";
pub const CHAIN_LOWERER: &str = "bhcp/prelude.lower-chain@0";
pub const CHAIN_REDUCER: &str = "bhcp/prelude.chain-reducer@0";
pub const CHAIN_FEATURE: &str = "bhcp/feature.self-hosted-chain@0";
pub const GATE_LOWERER: &str = "bhcp/prelude.lower-gate@0";
pub const GATE_REDUCER: &str = "bhcp/prelude.gate-reducer@0";
pub const RECURSIVE_GATE_REDUCER: &str = "bhcp/prelude.recursive-gate-reducer@0";
pub const GATE_FEATURE: &str = "bhcp/feature.self-hosted-gate@0";
pub const RETAIN_LOWERER: &str = "bhcp/prelude.lower-retain@0";
pub const RETAIN_REDUCER: &str = "bhcp/prelude.retain-reducer@0";
pub const RETAIN_FEATURE: &str = "bhcp/feature.persistent-retention-lowering@0";

const SOURCE_NAME: &str = "prelude/v0/standard.bhcp";
const SOURCE: &str = concat!(
    include_str!("../prelude/v0/all.bhcp"),
    "\n",
    include_str!("../prelude/v0/any.bhcp"),
    "\n",
    include_str!("../prelude/v0/none.bhcp"),
    "\n",
    include_str!("../prelude/v0/chain.bhcp"),
    "\n",
    include_str!("../prelude/v0/gate.bhcp"),
    "\n",
    include_str!("../prelude/v0/recursive-gate.bhcp"),
    "\n",
    include_str!("../prelude/v0/retain.bhcp")
);
const INVALID_PRELUDE: &str = "BHCP3001";

#[derive(Clone, Debug)]
pub struct DerivedChild {
    pub tag: String,
    pub goal: String,
    pub output: BhcpType,
    pub arguments: Vec<KernelArgument>,
    pub recursion: Option<RecursionBound>,
}

#[derive(Clone, Debug)]
pub struct DerivedForm {
    pub input: BhcpType,
    pub output: BhcpType,
    pub children: Vec<DerivedChild>,
    pub condition: Option<Expression>,
}

#[derive(Clone, Debug)]
pub struct NetworkShape {
    pub output: BhcpType,
    pub children: Vec<DerivedChild>,
    pub reducer: String,
}

pub struct Prelude {
    functions: HashMap<String, SurfaceFunction>,
}

impl Prelude {
    pub fn load() -> Result<Self> {
        let algorithm = HashAlgorithm::default();
        let source = ContentReference {
            media_type: "text/bhcp;profile=bhcp%2Fcanonical%400".to_owned(),
            size: SOURCE.len(),
            digests: vec![algorithm.hash(SOURCE.as_bytes())],
        };
        let ParsedProgram {
            functions, goals, ..
        } = parse_canonical(SOURCE, SOURCE_NAME, source)?;
        if !goals.is_empty() {
            return Err(invalid(
                "the standard prelude slice must contain only functions",
            ));
        }
        let mut indexed = HashMap::new();
        for function in functions {
            let symbol = function.symbol.clone();
            if indexed.insert(symbol, function).is_some() {
                return Err(invalid("duplicate standard-prelude function"));
            }
        }
        let prelude = Self { functions: indexed };
        prelude.validate_all_lowerer()?;
        prelude.validate_all_reducer()?;
        prelude.validate_any_lowerer()?;
        prelude.validate_any_reducer()?;
        prelude.validate_none_lowerer()?;
        prelude.validate_none_reducer()?;
        prelude.validate_chain_lowerer()?;
        prelude.validate_chain_reducer()?;
        prelude.validate_gate_lowerer()?;
        prelude.validate_gate_reducer()?;
        prelude.validate_reducer(RECURSIVE_GATE_REDUCER, "recursive-gate-reducer")?;
        prelude.validate_retain_lowerer()?;
        prelude.validate_retain_reducer()?;
        Ok(prelude)
    }

    pub(crate) fn with_project_functions(mut self, program: &ParsedProgram) -> Result<Self> {
        for function in &program.functions {
            if self.functions.contains_key(&function.symbol) {
                return Err(Diagnostic::plain(
                    "BHCP5003",
                    format!(
                        "project definition attempts to override core function {:?}",
                        function.symbol
                    ),
                ));
            }
            self.functions
                .insert(function.symbol.clone(), function.clone());
        }
        Ok(self)
    }

    pub fn lower(&self, symbol: &str, form: DerivedForm) -> Result<NetworkShape> {
        let function = self
            .functions
            .get(symbol)
            .ok_or_else(|| invalid("derived form does not resolve to a prelude lowerer"))?;
        let mut environment =
            HashMap::from([(function.parameters[0].name.clone(), MetaValue::Form(form))]);
        match evaluate_meta(&function.definition, &mut environment)? {
            MetaValue::Shape(shape) => Ok(shape),
            _ => Err(invalid("prelude lowerer did not return a network shape")),
        }
    }

    pub fn lower_retention(&self, form: DerivedForm) -> Result<NetworkShape> {
        let tags = form
            .children
            .iter()
            .map(|child| child.tag.as_str())
            .collect::<Vec<_>>();
        if tags != ["state-read", "candidate", "compare-and-swap"] {
            return Err(invalid(
                "retention lowering requires state-read, candidate, and compare-and-swap children in order",
            ));
        }
        if form.children.iter().any(|child| child.recursion.is_some()) {
            return Err(invalid(
                "retention lowering children cannot carry recursion evidence",
            ));
        }
        let [snapshot, candidate, commit] = form.children.as_slice() else {
            unreachable!("retention child count follows from exact tags")
        };
        let snapshot_resource = retention_argument(snapshot, "resource")?;
        let candidate_prior = retention_argument(candidate, "prior")?;
        let candidate_resource = retention_argument(candidate, "resource")?;
        let commit_resource = retention_argument(commit, "resource")?;
        let expected_version = retention_argument(commit, "expected_version")?;
        let new_value = retention_argument(commit, "new_value")?;
        if snapshot.arguments.len() != 1
            || candidate.arguments.len() != 2
            || commit.arguments.len() != 3
            || parent_edge(snapshot_resource) != Some("resource")
            || parent_edge(candidate_resource) != Some("resource")
            || parent_edge(commit_resource) != Some("resource")
            || predecessor_edge(candidate_prior) != Some("state-read")
            || predecessor_edge(expected_version) != Some("state-read")
            || predecessor_edge(new_value) != Some("candidate")
        {
            return Err(invalid(
                "retention lowering does not preserve exact resource, prior-version, and new-value coordinates",
            ));
        }
        self.lower(RETAIN_LOWERER, form)
    }

    pub fn reducer(&self, symbol: &str) -> Result<&SurfaceFunction> {
        let function = self
            .functions
            .get(symbol)
            .ok_or_else(|| invalid("network reducer does not resolve in the standard prelude"))?;
        if function.parameters.len() != 2 || !matches!(function.result, SurfaceType::Reduction(_)) {
            return Err(invalid(
                "prelude symbol is not an executable network reducer",
            ));
        }
        Ok(function)
    }

    fn validate_all_lowerer(&self) -> Result<()> {
        self.validate_lowerer(ALL_LOWERER, "lower-all")
    }

    fn validate_all_reducer(&self) -> Result<()> {
        self.validate_reducer(ALL_REDUCER, "all-reducer")
    }

    fn validate_any_lowerer(&self) -> Result<()> {
        self.validate_lowerer(ANY_LOWERER, "lower-any")
    }

    fn validate_any_reducer(&self) -> Result<()> {
        self.validate_reducer(ANY_REDUCER, "any-reducer")
    }

    fn validate_none_lowerer(&self) -> Result<()> {
        self.validate_lowerer(NONE_LOWERER, "lower-none")
    }

    fn validate_none_reducer(&self) -> Result<()> {
        self.validate_reducer(NONE_REDUCER, "none-reducer")
    }

    fn validate_chain_lowerer(&self) -> Result<()> {
        self.validate_lowerer(CHAIN_LOWERER, "lower-chain")
    }

    fn validate_chain_reducer(&self) -> Result<()> {
        self.validate_reducer(CHAIN_REDUCER, "chain-reducer")
    }

    fn validate_gate_lowerer(&self) -> Result<()> {
        self.validate_lowerer(GATE_LOWERER, "lower-gate")
    }

    fn validate_gate_reducer(&self) -> Result<()> {
        self.validate_reducer(GATE_REDUCER, "gate-reducer")
    }

    fn validate_retain_lowerer(&self) -> Result<()> {
        self.validate_lowerer(RETAIN_LOWERER, "lower-retain")
    }

    fn validate_retain_reducer(&self) -> Result<()> {
        self.validate_reducer(RETAIN_REDUCER, "retain-reducer")
    }

    fn validate_lowerer(&self, symbol: &str, name: &str) -> Result<()> {
        let function = self
            .functions
            .get(symbol)
            .ok_or_else(|| invalid(format!("standard prelude is missing {name}")))?;
        let valid = function.type_parameters == ["I", "O"]
            && function.parameters.len() == 1
            && matches!(
                &function.parameters[0].value_type,
                SurfaceType::Meta {
                    kind: "derived-form",
                    ..
                }
            )
            && matches!(
                &function.result,
                SurfaceType::Meta {
                    kind: "network-shape",
                    ..
                }
            );
        if !valid {
            return Err(invalid(format!("{name} has an invalid meta signature")));
        }
        Ok(())
    }

    fn validate_reducer(&self, symbol: &str, name: &str) -> Result<()> {
        let function = self
            .functions
            .get(symbol)
            .ok_or_else(|| invalid(format!("standard prelude is missing {name}")))?;
        let valid = function.type_parameters == ["I", "O", "Observations"]
            && function.parameters.len() == 2
            && matches!(
                &function.parameters[0].value_type,
                SurfaceType::Parameter(parameter) if parameter == "I"
            )
            && matches!(
                &function.parameters[1].value_type,
                SurfaceType::Parameter(parameter) if parameter == "Observations"
            )
            && matches!(
                &function.result,
                SurfaceType::Reduction(output)
                    if matches!(output.as_ref(), SurfaceType::Parameter(parameter) if parameter == "O")
            );
        if !valid {
            return Err(invalid(format!("{name} has an invalid generic signature")));
        }
        Ok(())
    }
}

fn retention_argument<'a>(child: &'a DerivedChild, name: &str) -> Result<&'a KernelArgument> {
    child
        .arguments
        .iter()
        .find(|argument| argument.name == name)
        .ok_or_else(|| invalid(format!("retention child {:?} omits {name:?}", child.tag)))
}

fn parent_edge(argument: &KernelArgument) -> Option<&str> {
    edge_coordinate(argument, "bhcp/kernel.parent-field@0")
}

fn predecessor_edge(argument: &KernelArgument) -> Option<&str> {
    edge_coordinate(argument, "bhcp/kernel.observed-output@0")
}

fn edge_coordinate<'a>(argument: &'a KernelArgument, expected: &str) -> Option<&'a str> {
    let crate::model::ExpressionForm::Call(symbol, parameters) = &argument.value.form else {
        return None;
    };
    let [parameter] = parameters.as_slice() else {
        return None;
    };
    let crate::model::ExpressionForm::Literal(crate::value::Value::Text(coordinate)) =
        &parameter.form
    else {
        return None;
    };
    (symbol == expected).then_some(coordinate.as_str())
}

#[derive(Clone, Debug)]
enum MetaValue {
    Form(DerivedForm),
    Type(BhcpType),
    Text(String),
    Shape(NetworkShape),
}

fn evaluate_meta(
    expression: &SurfaceExpression,
    environment: &mut HashMap<String, MetaValue>,
) -> Result<MetaValue> {
    match expression {
        SurfaceExpression::Reference { name, .. } => environment
            .get(name)
            .cloned()
            .ok_or_else(|| invalid(format!("unknown meta binding {name:?}"))),
        SurfaceExpression::Literal {
            value: SurfaceLiteral::Text(value),
            ..
        } => Ok(MetaValue::Text(value.clone())),
        SurfaceExpression::Call {
            function,
            arguments,
            ..
        } => {
            let values = arguments
                .iter()
                .map(|argument| evaluate_meta(argument, environment))
                .collect::<Result<Vec<_>>>()?;
            match (function.as_str(), values.as_slice()) {
                ("bhcp/meta.unit-type@0", []) => Ok(MetaValue::Type(BhcpType::Primitive("Unit"))),
                ("bhcp/meta.last-child-output-or-unit@0", [MetaValue::Form(form)]) => {
                    Ok(MetaValue::Type(
                        form.children
                            .last()
                            .map(|child| child.output.clone())
                            .unwrap_or(BhcpType::Primitive("Unit")),
                    ))
                }
                ("bhcp/meta.child-output-record@0", [MetaValue::Form(form)]) => {
                    let mut fields: Vec<_> = form
                        .children
                        .iter()
                        .map(|child| FieldType {
                            name: child.tag.clone(),
                            value_type: child.output.clone(),
                        })
                        .collect();
                    fields.sort_by(|left, right| left.name.cmp(&right.name));
                    Ok(MetaValue::Type(BhcpType::Record(fields)))
                }
                ("bhcp/meta.child-output-winner@0", [MetaValue::Form(form)]) => {
                    let Some(first) = form.children.first() else {
                        return Ok(MetaValue::Type(form.output.clone()));
                    };
                    if form
                        .children
                        .iter()
                        .any(|child| child.output != first.output)
                    {
                        return Err(invalid(
                            "any requires every child to have the same output type in this executable slice",
                        ));
                    }
                    Ok(MetaValue::Type(BhcpType::Record(vec![
                        FieldType {
                            name: "output".to_owned(),
                            value_type: first.output.clone(),
                        },
                        FieldType {
                            name: "tag".to_owned(),
                            value_type: BhcpType::Primitive("Text"),
                        },
                    ])))
                }
                ("bhcp/meta.gate-output@0", [MetaValue::Form(form)]) => {
                    let [child] = form.children.as_slice() else {
                        return Err(invalid("gate requires exactly one child"));
                    };
                    if form
                        .condition
                        .as_ref()
                        .is_none_or(|condition| condition.value_type != BhcpType::Primitive("Bool"))
                    {
                        return Err(invalid("gate requires one total pure Bool condition"));
                    }
                    Ok(MetaValue::Type(BhcpType::Variant(vec![
                        VariantCaseType {
                            tag: "Excluded".to_owned(),
                            payload: vec![],
                        },
                        VariantCaseType {
                            tag: "Included".to_owned(),
                            payload: vec![child.output.clone()],
                        },
                    ])))
                }
                (
                    "bhcp/meta.network-shape@0",
                    [
                        MetaValue::Type(output),
                        MetaValue::Form(form),
                        MetaValue::Text(reducer),
                    ],
                ) => Ok(MetaValue::Shape(NetworkShape {
                    output: output.clone(),
                    children: form.children.clone(),
                    reducer: reducer.clone(),
                })),
                _ => Err(invalid(format!(
                    "unsupported or ill-typed meta call {function}"
                ))),
            }
        }
        _ => Err(invalid(
            "expression is outside the total pure meta-expression slice",
        )),
    }
}

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_PRELUDE, message)
}
