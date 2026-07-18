//! Checked canonical BHCP definitions used to derive standard behavior.

use std::collections::HashMap;

use crate::diagnostic::{Diagnostic, Result};
use crate::hash::HashAlgorithm;
use crate::kernel::KernelArgument;
use crate::model::{BhcpType, ContentReference, FieldType};
use crate::parser::{
    ParsedProgram, SurfaceExpression, SurfaceFunction, SurfaceLiteral, SurfaceType, parse_canonical,
};

pub const ALL_LOWERER: &str = "bhcp/prelude.lower-all@0";
pub const ALL_REDUCER: &str = "bhcp/prelude.all-reducer@0";
pub const ALL_FEATURE: &str = "bhcp/feature.self-hosted-all@0";

const SOURCE_NAME: &str = "prelude/v0/all.bhcp";
const SOURCE: &str = include_str!("../prelude/v0/all.bhcp");
const INVALID_PRELUDE: &str = "BHCP3001";

#[derive(Clone, Debug)]
pub struct DerivedChild {
    pub tag: String,
    pub goal: String,
    pub output: BhcpType,
    pub arguments: Vec<KernelArgument>,
}

#[derive(Clone, Debug)]
pub struct DerivedForm {
    pub input: BhcpType,
    pub children: Vec<DerivedChild>,
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
        Ok(prelude)
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
        let function = self
            .functions
            .get(ALL_LOWERER)
            .ok_or_else(|| invalid("standard prelude is missing lower-all"))?;
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
            return Err(invalid("lower-all has an invalid meta signature"));
        }
        Ok(())
    }

    fn validate_all_reducer(&self) -> Result<()> {
        let function = self
            .functions
            .get(ALL_REDUCER)
            .ok_or_else(|| invalid("standard prelude is missing all-reducer"))?;
        let valid = function.type_parameters == ["I", "O", "Observations"]
            && function.parameters.len() == 2
            && matches!(
                &function.parameters[0].value_type,
                SurfaceType::Parameter(name) if name == "I"
            )
            && matches!(
                &function.parameters[1].value_type,
                SurfaceType::Parameter(name) if name == "Observations"
            )
            && matches!(
                &function.result,
                SurfaceType::Reduction(output)
                    if matches!(output.as_ref(), SurfaceType::Parameter(name) if name == "O")
            );
        if !valid {
            return Err(invalid("all-reducer has an invalid generic signature"));
        }
        Ok(())
    }
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
