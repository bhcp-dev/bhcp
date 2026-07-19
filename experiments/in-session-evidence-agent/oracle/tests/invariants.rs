#[test]
fn withheld_and_policy_readiness_are_complete() {
    assert!(in_session_evidence::oracle_ready());
    assert!(in_session_evidence::policy_ready());
}
