use bhcp::experiment_codex::summarize_events;

#[test]
fn closed_codex_event_summary_counts_only_completed_commands_and_final_usage() {
    let events = concat!(
        "{\"type\":\"thread.started\",\"thread_id\":\"t\"}\n",
        "{\"type\":\"item.completed\",\"item\":{\"type\":\"command_execution\",\"status\":\"completed\"}}\n",
        "{\"type\":\"item.completed\",\"item\":{\"type\":\"reasoning\",\"text\":\"private\"}}\n",
        "{\"type\":\"item.completed\",\"item\":{\"type\":\"command_execution\",\"status\":\"failed\"}}\n",
        "{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":120,\"cached_input_tokens\":80,\"output_tokens\":30,\"reasoning_output_tokens\":12}}\n",
    );

    let summary = summarize_events(events.as_bytes()).unwrap();
    assert_eq!(summary.input_tokens, 120);
    assert_eq!(summary.cached_input_tokens, 80);
    assert_eq!(summary.output_tokens, 30);
    assert_eq!(summary.reasoning_tokens, 12);
    assert_eq!(summary.completed_commands, 1);
}

#[test]
fn codex_event_summary_fails_closed_on_unknown_or_incomplete_usage() {
    assert!(summarize_events(b"not-json\n".as_slice()).is_err());
    assert!(
        summarize_events(
            b"{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1}}\n".as_slice()
        )
        .is_err()
    );
    assert!(summarize_events(b"{\"type\":\"turn.started\"}\n".as_slice()).is_err());
}
