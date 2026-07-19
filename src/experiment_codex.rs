//! Closed reduction of Codex JSONL events for registered experiments.

use std::io::BufRead;

use serde_json::Value;

const MAX_EVENT_LINE_BYTES: usize = 8 * 1_024 * 1_024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexEventSummary {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub completed_commands: u64,
}

pub fn summarize_events(input: impl BufRead) -> Result<CodexEventSummary, String> {
    let mut usage = None;
    let mut completed_commands = 0_u64;
    for line in input.lines() {
        let line = line.map_err(|error| format!("cannot read Codex event: {error}"))?;
        if line.len() > MAX_EVENT_LINE_BYTES {
            return Err("Codex event exceeds the bounded line size".to_owned());
        }
        let event: Value = serde_json::from_str(&line)
            .map_err(|error| format!("invalid Codex JSON event: {error}"))?;
        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| "Codex event has no string type".to_owned())?;
        if event_type == "item.completed"
            && event.pointer("/item/type").and_then(Value::as_str) == Some("command_execution")
            && event.pointer("/item/status").and_then(Value::as_str) == Some("completed")
        {
            completed_commands = completed_commands
                .checked_add(1)
                .ok_or_else(|| "Codex command count overflowed".to_owned())?;
        }
        if event_type == "turn.completed" {
            if usage.is_some() {
                return Err("Codex emitted more than one completed turn".to_owned());
            }
            let values = event
                .get("usage")
                .and_then(Value::as_object)
                .ok_or_else(|| "completed Codex turn has no usage object".to_owned())?;
            usage = Some(CodexEventSummary {
                input_tokens: metric(values, "input_tokens")?,
                cached_input_tokens: metric(values, "cached_input_tokens")?,
                output_tokens: metric(values, "output_tokens")?,
                reasoning_tokens: metric(values, "reasoning_output_tokens")?,
                completed_commands: 0,
            });
        }
    }
    let mut summary = usage.ok_or_else(|| "Codex turn did not complete".to_owned())?;
    if summary.cached_input_tokens > summary.input_tokens {
        return Err("Codex cached input exceeds total input".to_owned());
    }
    summary.completed_commands = completed_commands;
    Ok(summary)
}

fn metric(values: &serde_json::Map<String, Value>, name: &str) -> Result<u64, String> {
    values
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("completed Codex turn has no valid {name}"))
}
