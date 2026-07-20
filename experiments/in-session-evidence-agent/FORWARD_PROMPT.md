Work autonomously on the coding task in ../TASK.md. Do not ask questions. Edit only
src/lib.rs and do not use the network.

Use $interpret-bhcp-contract with ../contract.bhcp and
../contract.semantic-id. After the visible checks pass, invoke the canonical
project registry exactly as follows:

```sh
bhcp verify ../contract.bhcp experiment/InSessionEvidence@0 ../candidate.cbor \
  src/lib.rs 2026-07-19T20:30:00Z > "$CARGO_TARGET_DIR/evidence.cbor"
bhcp inspect "$CARGO_TARGET_DIR/evidence.cbor"
```

The public, oracle, and change-policy adapters are real bounded producers. Treat
their retained evidence as authoritative. Your structured final response must set
`claimed_success=true` only if every mandatory obligation is discharged.
