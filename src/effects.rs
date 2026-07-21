use std::collections::{BTreeMap, HashMap};

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::model::{
    BhcpType, ClauseKind, Effect, EffectRow, Expression, ExpressionForm, GoalDefinition,
};
use crate::value::Value;

pub(crate) const EFFECT_ANALYSIS_FEATURE: &str = "bhcp/feature.effect-authority-analysis@0";

pub(crate) fn analyze(goals: &mut [GoalDefinition], source_name: &str) -> Result<()> {
    let by_id: HashMap<_, _> = goals
        .iter()
        .enumerate()
        .map(|(index, goal)| (goal.id.clone(), index))
        .collect();
    let authored: Vec<_> = goals.iter().map(authored_authority).collect();

    for (goal, authority) in goals.iter().zip(&authored) {
        validate_limits_and_preferences(goal, source_name)?;
        for effect in &authority.allows {
            if authority
                .forbids
                .iter()
                .any(|forbidden| effect_covers(forbidden, effect))
            {
                return Err(authority_error(
                    goal,
                    effect,
                    "is denied because an authored prohibition overrides its allowance",
                    source_name,
                ));
            }
        }
    }

    let mut rows: Vec<Vec<Effect>> = goals
        .iter()
        .zip(&authored)
        .map(|(goal, authority)| {
            canonicalize(if goal.body.is_none() {
                authority.allows.clone()
            } else {
                Vec::new()
            })
        })
        .collect::<Result<_>>()?;
    loop {
        let previous = rows.clone();
        let mut changed = false;
        for (index, goal) in goals.iter().enumerate() {
            let mut inferred = if goal.body.is_none() {
                authored[index].allows.clone()
            } else {
                Vec::new()
            };
            if let Some(network) = &goal.body {
                for child in &network.children {
                    let child_index = by_id[&child.goal];
                    for effect in &previous[child_index] {
                        inferred.push(project_resource(
                            effect,
                            &goals[child_index],
                            goal,
                            &child.arguments,
                        ));
                    }
                }
            }
            let inferred = canonicalize(inferred)?;
            for effect in &inferred {
                if authored[index]
                    .forbids
                    .iter()
                    .any(|forbidden| effect_covers(forbidden, effect))
                {
                    return Err(authority_error(
                        goal,
                        effect,
                        "is denied by the goal prohibition after child-effect propagation",
                        source_name,
                    ));
                }
                if authored[index].has_ceiling
                    && !authored[index]
                        .allows
                        .iter()
                        .any(|allowed| effect_covers(allowed, effect))
                {
                    return Err(authority_error(
                        goal,
                        effect,
                        "exceeds the goal's authored capability ceiling",
                        source_name,
                    ));
                }
            }
            if inferred != rows[index] {
                rows[index] = inferred;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    for (goal, effects) in goals.iter_mut().zip(rows) {
        if effects.iter().any(requires_evidence_gap) {
            let BhcpType::Evidence(classes) = &mut goal.evidence else {
                return Err(Diagnostic::plain(
                    "BHCP4001",
                    format!(
                        "goal {} with unsafe or foreign effects requires an Evidence type",
                        goal.symbol
                    ),
                ));
            };
            if !classes.iter().any(|class| class == "unresolved") {
                classes.push("unresolved".to_owned());
                classes.sort();
            }
        }
        goal.effects = EffectRow {
            effects,
            row_variable: None,
        };
    }
    Ok(())
}

pub(crate) fn validate_materialized(goals: &[GoalDefinition]) -> Result<()> {
    let mut expected = goals.to_vec();
    analyze(&mut expected, "semantic-ir")?;
    for (actual, expected) in goals.iter().zip(expected) {
        if actual.effects != expected.effects {
            return Err(Diagnostic::plain(
                "BHCP4001",
                format!(
                    "goal {} materialized effect row does not match its authored and propagated effects",
                    actual.symbol
                ),
            ));
        }
        if actual.evidence != expected.evidence {
            return Err(Diagnostic::plain(
                "BHCP4001",
                format!(
                    "goal {} evidence omits a required unsafe or foreign effect gap",
                    actual.symbol
                ),
            ));
        }
    }
    Ok(())
}

struct AuthoredAuthority {
    allows: Vec<Effect>,
    forbids: Vec<Effect>,
    has_ceiling: bool,
}

fn authored_authority(goal: &GoalDefinition) -> AuthoredAuthority {
    let mut allows = Vec::new();
    let mut forbids = Vec::new();
    let mut has_ceiling = false;
    for clause in &goal.clauses {
        match &clause.kind {
            ClauseKind::Authority {
                kind: "allows",
                effects,
            } => {
                has_ceiling = true;
                allows.extend(effects.iter().cloned());
            }
            ClauseKind::Authority {
                kind: "forbids",
                effects,
            } => forbids.extend(effects.iter().cloned()),
            _ => {}
        }
    }
    AuthoredAuthority {
        allows,
        forbids,
        has_ceiling,
    }
}

fn project_resource(
    effect: &Effect,
    child_goal: &GoalDefinition,
    parent_goal: &GoalDefinition,
    arguments: &[crate::kernel::KernelArgument],
) -> Effect {
    let Some(resource) = &effect.resource else {
        return effect.clone();
    };
    let Some(name) = child_goal
        .clauses
        .iter()
        .find_map(|clause| match &clause.kind {
            ClauseKind::Fact {
                kind: "input",
                binding,
            } if binding.id == *resource => Some(binding.name.as_str()),
            _ => None,
        })
    else {
        return effect.clone();
    };
    let Some(argument) = arguments.iter().find(|argument| argument.name == name) else {
        return effect.clone();
    };
    let parent_resource = match &argument.value.form {
        ExpressionForm::Reference(parent_resource) => parent_resource.clone(),
        ExpressionForm::Call(function, arguments)
            if function == "bhcp/kernel.parent-field@0"
                && matches!(
                    arguments.as_slice(),
                    [Expression {
                        form: ExpressionForm::Literal(Value::Text(_)),
                        ..
                    }]
                ) =>
        {
            let [
                Expression {
                    form: ExpressionForm::Literal(Value::Text(parent_name)),
                    ..
                },
            ] = arguments.as_slice()
            else {
                unreachable!()
            };
            let Some(binding) = parent_goal
                .clauses
                .iter()
                .find_map(|clause| match &clause.kind {
                    ClauseKind::Fact {
                        kind: "input",
                        binding,
                    } if binding.name == *parent_name => Some(binding),
                    _ => None,
                })
            else {
                return effect.clone();
            };
            binding.id.clone()
        }
        _ => return effect.clone(),
    };
    Effect {
        id: effect.id.clone(),
        resource: Some(parent_resource),
        parameters: effect.parameters.clone(),
    }
}

fn effect_covers(rule: &Effect, request: &Effect) -> bool {
    rule.id == request.id
        && rule
            .resource
            .as_ref()
            .is_none_or(|resource| request.resource.as_ref() == Some(resource))
        && (rule.parameters.is_empty() || rule.parameters == request.parameters)
}

pub(crate) fn canonicalize(mut effects: Vec<Effect>) -> Result<Vec<Effect>> {
    let mut encoded = effects
        .drain(..)
        .map(|effect| Ok((encode_deterministic(&effect.to_value())?, effect)))
        .collect::<Result<Vec<_>>>()?;
    encoded.sort_by(|left, right| left.0.cmp(&right.0));
    encoded.dedup_by(|left, right| left.0 == right.0);
    Ok(encoded.into_iter().map(|(_, effect)| effect).collect())
}

fn validate_limits_and_preferences(goal: &GoalDefinition, source_name: &str) -> Result<()> {
    let mut preference_types: BTreeMap<i64, &BhcpType> = BTreeMap::new();
    for clause in &goal.clauses {
        match &clause.kind {
            ClauseKind::Contract {
                kind: "limit",
                dimension: Some(dimension),
                condition,
            } if !is_direct_non_negative_exact_upper_bound(condition) => {
                return Err(Diagnostic::new(
                    "BHCP4503",
                    format!(
                        "goal {} limit {} must use a direct non-negative exact upper bound",
                        goal.symbol, dimension
                    ),
                    source_name,
                    1,
                    1,
                ));
            }
            ClauseKind::Preference {
                priority,
                objective,
            } => {
                if let Some(previous) = preference_types.get(priority) {
                    if *previous != &objective.value_type {
                        return Err(Diagnostic::new(
                            "BHCP4504",
                            format!(
                                "goal {} preferences at priority {} have incompatible objective types",
                                goal.symbol, priority
                            ),
                            source_name,
                            1,
                            1,
                        ));
                    }
                } else {
                    preference_types.insert(*priority, &objective.value_type);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_direct_non_negative_exact_upper_bound(expression: &Expression) -> bool {
    let ExpressionForm::Binary(operator, _, right) = &expression.form else {
        return false;
    };
    if operator != "<=" {
        return false;
    }
    let ExpressionForm::Literal(value) = &right.form else {
        return false;
    };
    match value {
        Value::Array(parts) => match parts.as_slice() {
            [Value::Text(kind), Value::Integer(value)] => kind == "integer" && *value >= 0,
            [
                Value::Text(kind),
                Value::Integer(numerator),
                Value::Integer(denominator),
            ] if kind == "rational" => *numerator >= 0 && *denominator > 0,
            [
                Value::Text(kind),
                Value::Integer(coefficient),
                Value::Integer(_),
            ] if kind == "decimal" => *coefficient >= 0,
            _ => false,
        },
        _ => false,
    }
}

fn requires_evidence_gap(effect: &Effect) -> bool {
    let atom = effect
        .id
        .rsplit('/')
        .next()
        .unwrap_or(&effect.id)
        .split('@')
        .next()
        .unwrap_or_default();
    atom == "unsafe" || atom.starts_with("foreign")
}

fn authority_error(
    goal: &GoalDefinition,
    effect: &Effect,
    reason: &str,
    source_name: &str,
) -> Diagnostic {
    Diagnostic::new(
        "BHCP4502",
        format!("goal {} effect {} {}", goal.symbol, effect.id, reason),
        source_name,
        1,
        1,
    )
}
