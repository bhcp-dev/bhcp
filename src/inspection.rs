//! Human inspection of validated canonical CBOR artifacts.

use std::collections::HashMap;
use std::fmt::Write;

use crate::value::Value;

pub fn render_artifact(artifact: &Value, source: Option<&str>) -> String {
    let mut output = String::new();
    if let Some(source) = source {
        writeln!(output, "source {source}").unwrap();
    }
    let kind = text_field(artifact, "kind").unwrap_or("unknown");
    writeln!(output, "artifact {kind}").unwrap();
    for field in ["semantic_id", "artifact_id"] {
        if let Some(identity) = artifact.get(field).and_then(render_hash) {
            writeln!(output, "{field} {identity}").unwrap();
        }
    }
    if kind == "semantic-ir" {
        render_semantic_ir(artifact, &mut output);
    } else if kind == "policy" {
        render_policy(artifact, &mut output);
    } else if kind == "evidence-bundle" {
        render_evidence_bundle(artifact, &mut output);
    } else {
        if let Some(profile) = text_field(artifact, "profile") {
            writeln!(output, "profile {profile}").unwrap();
        }
        if let Some(Value::Array(features)) = artifact.get("features") {
            writeln!(output, "features {}", features.len()).unwrap();
        }
    }
    output
}

fn render_evidence_bundle(artifact: &Value, output: &mut String) {
    if let Some(Value::Array(obligations)) = artifact.get("policy_obligations") {
        for obligation in obligations {
            let id = text_field(obligation, "id").unwrap_or("?");
            let symbol = text_field(obligation, "symbol").unwrap_or("?");
            let minimum = obligation
                .get("minimum")
                .map(render_value)
                .unwrap_or_else(|| "?".to_owned());
            let classes = match obligation.get("classes") {
                Some(Value::Array(classes)) => classes
                    .iter()
                    .filter_map(text_value)
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => "?".to_owned(),
            };
            writeln!(
                output,
                "policy-obligation {id} {symbol} minimum {minimum} classes [{classes}]"
            )
            .unwrap();
            if let Some(Value::Array(sources)) = obligation.get("sources") {
                for source in sources {
                    let layer = text_field(source, "layer").unwrap_or("?");
                    let policy = text_field(source, "policy").unwrap_or("?");
                    let rule = text_field(source, "rule").unwrap_or("?");
                    writeln!(output, "  policy-source {layer} {policy}:{rule}").unwrap();
                }
            }
        }
    }
    if let Some(Value::Map(statuses)) = artifact.get("obligation_status") {
        for (obligation, status) in statuses {
            let Value::Text(status) = status else {
                continue;
            };
            writeln!(output, "obligation {obligation} {status}").unwrap();
        }
    }
    if let Some(Value::Array(claims)) = artifact.get("claims") {
        for claim in claims {
            let id = text_field(claim, "id").unwrap_or("?");
            let obligation = text_field(claim, "obligation").unwrap_or("?");
            let polarity = text_field(claim, "polarity").unwrap_or("?");
            let status = text_field(claim, "status").unwrap_or("?");
            let outcome = if polarity == "refutes" && status == "accepted" {
                "rejected"
            } else {
                status
            };
            let predicate = text_field(claim, "predicate").unwrap_or("?");
            writeln!(
                output,
                "claim {id} {obligation} {outcome} {polarity} {predicate}"
            )
            .unwrap();
        }
    }
    if let Some(Value::Array(items)) = artifact.get("items") {
        for item in items {
            let id = text_field(item, "id").unwrap_or("?");
            let class = text_field(item, "class").unwrap_or("?");
            let verifier = text_field(item, "verifier").unwrap_or("?");
            writeln!(output, "evidence {id} {class} {verifier}").unwrap();
        }
    }
    if let Some(Value::Array(gaps)) = artifact.get("gaps") {
        for gap in gaps {
            let id = text_field(gap, "id").unwrap_or("?");
            let kind = text_field(gap, "kind").unwrap_or("?");
            let reason = gap
                .get("reason")
                .and_then(|reason| text_field(reason, "code"))
                .unwrap_or("?");
            let obligations = match gap.get("obligations") {
                Some(Value::Array(obligations)) => obligations
                    .iter()
                    .filter_map(text_value)
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => "?".to_owned(),
            };
            writeln!(output, "gap {id} {kind} {reason} -> {obligations}").unwrap();
        }
    }
}

fn render_policy(artifact: &Value, output: &mut String) {
    match text_field(artifact, "form") {
        Some("source") => {
            let symbol = text_field(artifact, "symbol").unwrap_or("?");
            let layer = text_field(artifact, "layer").unwrap_or("?");
            writeln!(output, "policy source {symbol} layer {layer}").unwrap();
            if let Some(parent) = text_field(artifact, "extends") {
                writeln!(output, "extends {parent}").unwrap();
            }
            if let Some(Value::Array(rules)) = artifact.get("rules") {
                for rule in rules {
                    let id = text_field(rule, "id").unwrap_or("?");
                    let category = text_field(rule, "category").unwrap_or("?");
                    let operation = text_field(rule, "operation").unwrap_or("?");
                    let governance = render_policy_governance(rule);
                    let value = rule
                        .get("value")
                        .map(render_value)
                        .unwrap_or_else(|| "?".to_owned());
                    writeln!(
                        output,
                        "  [{id}] {category} {operation} {governance}: {value}"
                    )
                    .unwrap();
                }
            }
        }
        Some("effective") => {
            writeln!(output, "policy effective").unwrap();
            if let Some(Value::Array(layers)) = artifact.get("source_layers") {
                for layer in layers {
                    let name = text_field(layer, "layer").unwrap_or("?");
                    let policies = match layer.get("policies") {
                        Some(Value::Array(policies)) => policies.as_slice(),
                        _ => &[],
                    };
                    let count = policies.len();
                    writeln!(output, "source-layer {name} {count}").unwrap();
                    for policy in policies {
                        let symbol = text_field(policy, "symbol").unwrap_or("?");
                        writeln!(output, "  source {symbol}").unwrap();
                    }
                }
            }
            if let Some(effective) = artifact.get("effective") {
                for category in [
                    "requirements",
                    "evidence",
                    "prohibitions",
                    "capabilities",
                    "limits",
                ] {
                    let rules = match effective.get(category) {
                        Some(Value::Array(rules)) => rules.as_slice(),
                        _ => &[],
                    };
                    writeln!(output, "{category} {}", rules.len()).unwrap();
                    for (index, rule) in rules.iter().enumerate() {
                        let governance = render_policy_governance(rule);
                        let value = rule.get("value").unwrap_or(rule);
                        writeln!(
                            output,
                            "effective {category}[{index}] {governance}: {}",
                            render_value(value)
                        )
                        .unwrap();
                    }
                }
                let mode = effective
                    .get("type_mode")
                    .and_then(|rule| text_field(rule, "value"))
                    .unwrap_or("?");
                let governance = effective
                    .get("type_mode")
                    .map(render_policy_governance)
                    .unwrap_or_else(|| "?".to_owned());
                writeln!(output, "type-mode {mode}").unwrap();
                writeln!(output, "effective type-mode {mode} {governance}").unwrap();
            }
            let provenance = match artifact.get("rule_provenance") {
                Some(Value::Array(entries)) => entries.as_slice(),
                _ => &[],
            };
            writeln!(output, "rule-provenance {}", provenance.len()).unwrap();
            for entry in provenance {
                let category = text_field(entry, "category").unwrap_or("?");
                let index = integer_field(entry, "effective_rule").unwrap_or(-1);
                let sources = match entry.get("sources") {
                    Some(Value::Array(sources)) => sources
                        .iter()
                        .filter_map(|source| match source {
                            Value::Array(parts) if parts.len() == 2 => Some(format!(
                                "{}#{}",
                                text_value(&parts[0])?,
                                text_value(&parts[1])?
                            )),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => "?".to_owned(),
                };
                writeln!(output, "provenance {category}[{index}] <- {sources}").unwrap();
            }
        }
        _ => writeln!(output, "policy form unknown").unwrap(),
    }
}

fn render_policy_governance(rule: &Value) -> String {
    if !matches!(rule.get("waivable"), Some(Value::Bool(true))) {
        return "nonwaivable".to_owned();
    }
    match rule.get("authorized_issuers") {
        Some(issuers) => format!("waivable by {}", render_value(issuers)),
        None => "waivable".to_owned(),
    }
}

fn render_semantic_ir(artifact: &Value, output: &mut String) {
    if let Some(policy) = artifact.get("effective_policy") {
        for field in ["semantic_id", "artifact_id"] {
            if let Some(identity) = policy.get(field).and_then(render_hash) {
                writeln!(output, "effective-policy {field} {identity}").unwrap();
            }
        }
    }
    let Some(Value::Array(goals)) = artifact.get("goals") else {
        return;
    };
    for goal in goals {
        let id = text_field(goal, "id").unwrap_or("?");
        let symbol = text_field(goal, "symbol").unwrap_or("?");
        writeln!(output, "goal {id} {symbol}").unwrap();
        if let Some(decision) = goal.get("policy_decision") {
            let mode = text_field(decision, "type_mode").unwrap_or("?");
            writeln!(output, "  policy type-mode {mode}").unwrap();
            for category in [
                "requirements",
                "evidence",
                "prohibitions",
                "capabilities",
                "limits",
            ] {
                let indices = match decision.get(category) {
                    Some(Value::Array(values)) => values
                        .iter()
                        .map(render_value)
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => "?".to_owned(),
                };
                writeln!(output, "  policy {category} [{indices}]").unwrap();
            }
        }
        if let Some(input) = goal.get("input") {
            writeln!(output, "  input  {}", render_type(input)).unwrap();
        }
        if let Some(goal_output) = goal.get("output") {
            writeln!(output, "  output {}", render_type(goal_output)).unwrap();
        }
        let Some(Value::Array(clauses)) = goal.get("clauses") else {
            continue;
        };
        let names: HashMap<_, _> = clauses
            .iter()
            .filter_map(|clause| {
                (text_field(clause, "kind") == Some("input")
                    || text_field(clause, "kind") == Some("output"))
                .then(|| clause.get("binding"))
                .flatten()
                .and_then(|binding| {
                    Some((
                        text_field(binding, "id")?.to_owned(),
                        text_field(binding, "name")?.to_owned(),
                    ))
                })
            })
            .collect();
        let mut obligations: Vec<_> = clauses
            .iter()
            .filter(|clause| {
                matches!(
                    text_field(clause, "kind"),
                    Some("requires" | "ensures" | "invariant" | "limit")
                )
            })
            .filter_map(|clause| text_field(clause, "id").map(str::to_owned))
            .collect();
        obligations.sort();
        for clause in clauses {
            render_clause(clause, &names, &obligations, output);
        }
    }
}

fn render_clause(
    clause: &Value,
    names: &HashMap<String, String>,
    goal_obligations: &[String],
    output: &mut String,
) {
    let Some(kind) = text_field(clause, "kind") else {
        return;
    };
    if matches!(kind, "input" | "output") {
        return;
    }
    let id = text_field(clause, "id").unwrap_or("?");
    let label = text_field(clause, "label")
        .map(|value| format!(" {value:?}"))
        .unwrap_or_default();
    match kind {
        "requires" | "ensures" | "invariant" | "limit" => {
            let condition = clause
                .get("condition")
                .map(|value| render_expression(value, names))
                .unwrap_or_else(|| "?".to_owned());
            writeln!(output, "  [{id}] {kind}{label}: {condition}").unwrap();
        }
        "allows" | "forbids" => {
            let effects = match clause.get("effects") {
                Some(Value::Array(effects)) => effects
                    .iter()
                    .map(render_effect)
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => "?".to_owned(),
            };
            writeln!(output, "  [{id}] {kind}{label}: {effects}").unwrap();
        }
        "prefer" => {
            let priority = integer_field(clause, "priority").unwrap_or(0);
            let objective = clause
                .get("objective")
                .map(|value| render_expression(value, names))
                .unwrap_or_else(|| "?".to_owned());
            writeln!(output, "  [{id}] prefer {priority}{label}: {objective}").unwrap();
        }
        "verify" => {
            let verifier = clause
                .get("binding")
                .and_then(|binding| text_field(binding, "verifier"))
                .unwrap_or("?");
            let targets = match clause.get("obligations") {
                Some(Value::Array(values)) if !values.is_empty() => values
                    .iter()
                    .filter_map(text_value)
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                _ => goal_obligations.to_vec(),
            };
            writeln!(
                output,
                "  [{id}] verify{label}: {verifier} -> {}",
                targets.join(", ")
            )
            .unwrap();
        }
        _ => writeln!(output, "  [{id}] {kind}{label}").unwrap(),
    }
}

fn render_type(value: &Value) -> String {
    let Value::Array(parts) = value else {
        return render_value(value);
    };
    match parts.as_slice() {
        [Value::Text(kind), Value::Text(name)] if kind == "primitive" || kind == "exact-number" => {
            name.clone()
        }
        [Value::Text(kind), _, Value::Array(fields)] if kind == "record" => format!(
            "{{{}}}",
            fields
                .iter()
                .filter_map(|field| match field {
                    Value::Array(parts) if parts.len() >= 2 => {
                        Some(format!(
                            "{}: {}",
                            text_value(&parts[0])?,
                            render_type(&parts[1])
                        ))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        [Value::Text(kind), Value::Array(cases)] if kind == "variant" => cases
            .iter()
            .filter_map(|case| match case {
                Value::Array(parts) if parts.len() == 2 => {
                    let tag = text_value(&parts[0])?;
                    let Value::Array(payload) = &parts[1] else {
                        return None;
                    };
                    Some(if payload.is_empty() {
                        tag.to_owned()
                    } else {
                        format!(
                            "{tag}<{}>",
                            payload
                                .iter()
                                .map(render_type)
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" | "),
        [Value::Text(kind), element] if kind == "list" || kind == "option" => {
            format!("{kind}<{}>", render_type(element))
        }
        [Value::Text(kind), Value::Array(classes)] if kind == "evidence" => format!(
            "Evidence<{}>",
            classes
                .iter()
                .filter_map(text_value)
                .collect::<Vec<_>>()
                .join("|")
        ),
        [Value::Text(kind), output]
            if matches!(kind.as_str(), "verdict" | "execution-result" | "reduction") =>
        {
            format!("{kind}<{}>", render_type(output))
        }
        [
            Value::Text(kind),
            Value::Text(symbol),
            Value::Array(arguments),
        ] if kind == "nominal" => {
            let rendered = arguments
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ");
            if rendered.is_empty() {
                symbol.clone()
            } else {
                format!("{symbol}<{rendered}>")
            }
        }
        _ => render_value(value),
    }
}

fn render_expression(expression: &Value, names: &HashMap<String, String>) -> String {
    let Some(Value::Array(form)) = expression.get("form") else {
        return render_value(expression);
    };
    match form.as_slice() {
        [Value::Text(kind), value] if kind == "literal" => render_value(value),
        [Value::Text(kind), Value::Text(reference)] if kind == "reference" => names
            .get(reference)
            .cloned()
            .unwrap_or_else(|| reference.clone()),
        [Value::Text(kind), Value::Text(operator), operand] if kind == "unary" => {
            format!("({operator}{})", render_expression(operand, names))
        }
        [Value::Text(kind), Value::Text(operator), left, right] if kind == "binary" => format!(
            "({} {operator} {})",
            render_expression(left, names),
            render_expression(right, names)
        ),
        [Value::Text(kind), condition, consequent, alternative] if kind == "if" => format!(
            "if {} then {} else {}",
            render_expression(condition, names),
            render_expression(consequent, names),
            render_expression(alternative, names)
        ),
        [
            Value::Text(kind),
            Value::Text(function),
            Value::Array(arguments),
        ] if kind == "call" => {
            format!(
                "{function}({})",
                arguments
                    .iter()
                    .map(|argument| render_expression(argument, names))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        _ => render_value(expression),
    }
}

fn render_effect(effect: &Value) -> String {
    let id = text_field(effect, "id").unwrap_or("?");
    match text_field(effect, "resource") {
        Some(resource) => format!("{id}({resource:?})"),
        None => id.to_owned(),
    }
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Text(value) => format!("{value:?}"),
        Value::Bytes(bytes) => format!(
            "h'{}'",
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        Value::Array(values) => {
            if let [Value::Text(kind), Value::Integer(value)] = values.as_slice()
                && kind == "integer"
            {
                return value.to_string();
            }
            format!(
                "[{}]",
                values
                    .iter()
                    .map(render_value)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Value::Map(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|(key, value)| format!("{key}: {}", render_value(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Tag(tag, value) => format!("{tag}({})", render_value(value)),
    }
}

fn render_hash(value: &Value) -> Option<String> {
    let algorithm = text_field(value, "algorithm")?;
    let Value::Bytes(digest) = value.get("digest")? else {
        return None;
    };
    Some(format!(
        "{algorithm}:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

fn text_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(text_value)
}

fn integer_field(value: &Value, field: &str) -> Option<i64> {
    match value.get(field) {
        Some(Value::Integer(value)) => Some(*value),
        _ => None,
    }
}

fn text_value(value: &Value) -> Option<&str> {
    match value {
        Value::Text(value) => Some(value),
        _ => None,
    }
}
