use batch_ledger::{ApplyError, Ledger, Transfer};

fn standard_ledger() -> Ledger {
    Ledger::new([("alice", 100), ("bob", 20), ("carol", 5)])
}

fn balances(ledger: &Ledger, accounts: &[&str]) -> Vec<Option<u64>> {
    accounts
        .iter()
        .map(|account| ledger.balance(account))
        .collect()
}

#[test]
fn later_failure_must_not_commit_earlier_transfers() {
    let mut ledger = standard_ledger();
    let before = balances(&ledger, &["alice", "bob", "carol"]);
    let transfers = [
        Transfer::new("alice", "bob", 10),
        Transfer::new("carol", "alice", 6),
    ];

    assert!(matches!(
        ledger.apply_batch("late-failure", &transfers),
        Err(ApplyError::InsufficientFunds { index: 1, .. })
    ));
    assert_eq!(balances(&ledger, &["alice", "bob", "carol"]), before);
}

#[test]
fn destination_overflow_must_not_debit_the_source() {
    let mut ledger = Ledger::new([("source", 10), ("full", u64::MAX)]);
    let before = balances(&ledger, &["source", "full"]);

    assert_eq!(
        ledger.apply_batch(
            "destination-overflow",
            &[Transfer::new("source", "full", 1)],
        ),
        Err(ApplyError::BalanceOverflow {
            index: 0,
            account: "full".to_owned(),
        })
    );
    assert_eq!(balances(&ledger, &["source", "full"]), before);
}

#[test]
fn aggregate_overflow_must_roll_back_the_entire_batch() {
    let mut ledger = Ledger::new([("alice", u64::MAX), ("bob", 0)]);
    let before = balances(&ledger, &["alice", "bob"]);
    let transfers = [
        Transfer::new("alice", "bob", u64::MAX),
        Transfer::new("bob", "alice", 1),
    ];

    assert_eq!(
        ledger.apply_batch("aggregate-overflow", &transfers),
        Err(ApplyError::TotalOverflow)
    );
    assert_eq!(balances(&ledger, &["alice", "bob"]), before);
}

#[test]
fn a_request_id_reused_with_a_different_payload_must_conflict() {
    let mut ledger = standard_ledger();
    let original = [Transfer::new("alice", "bob", 10)];
    ledger.apply_batch("same-id", &original).unwrap();
    let before = balances(&ledger, &["alice", "bob", "carol"]);
    let conflicting = [Transfer::new("alice", "carol", 10)];

    assert_eq!(
        ledger.apply_batch("same-id", &conflicting),
        Err(ApplyError::RequestConflict {
            request_id: "same-id".to_owned(),
        })
    );
    assert_eq!(balances(&ledger, &["alice", "bob", "carol"]), before);
    assert!(ledger.apply_batch("same-id", &original).is_ok());
}

#[test]
fn a_failed_request_id_can_retry_against_the_original_state() {
    let mut ledger = standard_ledger();
    let failing = [
        Transfer::new("alice", "bob", 20),
        Transfer::new("missing", "alice", 1),
    ];
    assert!(ledger.apply_batch("retryable", &failing).is_err());

    let corrected = [Transfer::new("alice", "bob", 100)];
    assert!(ledger.apply_batch("retryable", &corrected).is_ok());
    assert_eq!(ledger.balance("alice"), Some(0));
    assert_eq!(ledger.balance("bob"), Some(120));
}
