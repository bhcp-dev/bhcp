Work only in the current Rust crate. Read `../PROSE_TASK.md` before changing
anything. Implement the smallest safe-Rust change to `src/lib.rs` that satisfies
every requirement in that prose treatment.

Do not use a BHCP contract, BHCP skill, project registry, verifier adapter, or
evidence bundle. Do not create or change any file except `src/lib.rs`. You may run
the available offline Rust formatting, lint, and test commands. The withheld oracle
is unavailable.

At the end, report `claimed_success: true` only if you believe the resulting
`src/lib.rs` satisfies all stated requirements and will pass every independent
check; otherwise report `claimed_success: false`.
