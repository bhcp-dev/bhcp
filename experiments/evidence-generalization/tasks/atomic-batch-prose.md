# Prose treatment: atomic batch ledger

Repair `Ledger::apply_batch` in the dependency-free `batch-ledger` crate. Edit only
`src/lib.rs`, preserve the public API and existing errors, add no dependency or
unsafe code, and do not use the network.

The implementation must satisfy this complete ordered obligation inventory:

- `atomic-rollback`: validation, source subtraction, destination addition, and
  receipt-total overflow must leave balances and request history exactly unchanged.
- `exact-idempotent-replay`: an identical ordered payload under a successful request
  ID returns the original receipt without another state change.
- `conflicting-replay`: a different ordered payload under a successful request ID
  returns `ApplyError::RequestConflict` and preserves the original state and record.
- `failed-id-retry`: a failed request does not consume its ID and a corrected payload
  can retry against the original state.
- `conservation-and-checked-receipt`: a success conserves total balance and reports
  the exact transfer count and checked sum of moved amounts.

Run Rustfmt, Clippy with warnings denied, and all visible tests. Report success only
when those checks pass; an independent withheld oracle and one-file policy judge run
after the session.
