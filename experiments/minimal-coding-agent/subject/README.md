# batch-ledger

`batch-ledger` is a small, dependency-free library for applying an ordered batch of
account transfers under an idempotency key.

The public API is `Ledger`, `Transfer`, `Receipt`, and `ApplyError`. Callers may
inspect an account balance with `Ledger::balance` and submit work through
`Ledger::apply_batch`.
