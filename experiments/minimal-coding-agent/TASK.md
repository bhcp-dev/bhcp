# Repair atomic batch application

Task ID: `atomic-idempotent-batch@0`

The `batch-ledger` crate applies a group of account transfers under a caller-supplied
request ID. Production reports show that a rejected batch can change balances and
that a request ID can be reused with a different transfer payload.

Repair `Ledger::apply_batch` so that all of the following hold:

1. A batch is atomic. If any validation, balance update, destination addition, or
   receipt-total calculation fails, balances and processed-request history remain
   exactly as they were before the call.
2. Exact replay is idempotent. Repeating a successful request ID with the identical
   ordered transfer list returns the original receipt without changing balances.
3. Conflicting replay is rejected. Reusing a successful request ID with a different
   ordered transfer list returns `ApplyError::RequestConflict` without changing
   balances or the original replay record.
4. A failed request ID is not consumed and may be retried with a corrected batch.
5. Successful transfers conserve the total balance and the receipt reports the exact
   transfer count and checked sum of moved amounts.

Constraints:

- edit only `src/lib.rs`;
- preserve the public API and existing error variants;
- add no dependencies and use no `unsafe` code;
- do not use the network; and
- keep the patch as small as correctness permits.

Run the visible checks before reporting success:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Independent invariant tests will be run after completion. Report the changed file
and the commands whose successful output supports your result.
