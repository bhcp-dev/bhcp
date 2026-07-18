use batch_ledger::{ApplyError, Ledger, Receipt, Transfer};

fn ledger() -> Ledger {
    Ledger::new([("alice", 100), ("bob", 20), ("carol", 5)])
}

#[test]
fn successful_batch_updates_balances_and_receipt() {
    let mut ledger = ledger();
    let transfers = [
        Transfer::new("alice", "bob", 25),
        Transfer::new("bob", "carol", 10),
    ];

    let receipt = ledger.apply_batch("request-1", &transfers).unwrap();

    assert_eq!(
        receipt,
        Receipt {
            request_id: "request-1".to_owned(),
            transfer_count: 2,
            total_moved: 35,
        }
    );
    assert_eq!(ledger.balance("alice"), Some(75));
    assert_eq!(ledger.balance("bob"), Some(35));
    assert_eq!(ledger.balance("carol"), Some(15));
}

#[test]
fn exact_replay_returns_the_original_receipt_without_moving_money_twice() {
    let mut ledger = ledger();
    let transfers = [Transfer::new("alice", "bob", 25)];

    let first = ledger.apply_batch("request-1", &transfers).unwrap();
    let replay = ledger.apply_batch("request-1", &transfers).unwrap();

    assert_eq!(replay, first);
    assert_eq!(ledger.balance("alice"), Some(75));
    assert_eq!(ledger.balance("bob"), Some(45));
}

#[test]
fn an_invalid_first_transfer_does_not_change_balances() {
    let mut ledger = ledger();
    let transfers = [Transfer::new("carol", "alice", 6)];

    let result = ledger.apply_batch("request-1", &transfers);

    assert_eq!(
        result,
        Err(ApplyError::InsufficientFunds {
            index: 0,
            account: "carol".to_owned(),
        })
    );
    assert_eq!(ledger.balance("alice"), Some(100));
    assert_eq!(ledger.balance("bob"), Some(20));
    assert_eq!(ledger.balance("carol"), Some(5));
}

#[test]
fn a_missing_source_is_reported_before_any_mutation() {
    let mut ledger = ledger();
    let transfers = [Transfer::new("nobody", "alice", 1)];

    let result = ledger.apply_batch("request-1", &transfers);

    assert_eq!(
        result,
        Err(ApplyError::MissingAccount {
            index: 0,
            account: "nobody".to_owned(),
        })
    );
    assert_eq!(ledger.balance("alice"), Some(100));
}
