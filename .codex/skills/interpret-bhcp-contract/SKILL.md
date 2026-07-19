---
name: interpret-bhcp-contract
description: Interpret and operationalize canonical BHCP contracts without inventing semantics. Use for work governed by a .bhcp file, validating a pinned semantic ID, mapping obligations to implementation and evidence, or diagnosing unsupported BHCP syntax.
---

# Interpret BHCP Contract

Treat compiler output as authoritative. Never infer meaning absent from the compiled
contract or reread unrelated repository material.

## Establish authority

1. Locate the `.bhcp` source, pinned semantic-ID file, and task.
2. Require `bhcp` on `PATH`. If `command -v bhcp` fails, stop and ask the user to
   install it or make it available on `PATH`. Do not search for repository binaries,
   wrappers, or toolchain fallbacks.
3. Run `hash` once and compare it byte-for-byte with the pin. Stop on mismatch.
4. Run `inspect` once on source or canonical `.cbor`. The Rust CLI validates the
   existing artifact boundary and renders structural clauses and verifier targets.
   Do not open the contract source when `inspect` succeeds. Stop on diagnostics,
   unsupported syntax, or unresolved targets.
5. Do not run `lower` for routine contract interpretation. For exact artifact
   debugging, capture its canonical CBOR outside the repository and pass the `.cbor`
   file back to `bhcp inspect`. Never stream raw AST or IR bytes into conversation.

## Build a compact working view

Keep the checklist internal as `ID · requirement/effect · verifier · action`. Do not
print it unless the user asks.

- Include only clauses that affect implementation or acceptance.
- Apply each `verify -> target` binding to its target rows. Never create a checklist
  row for a `verify` clause; its own clause ID is not an obligation.
- Treat each inspected `policy-obligation` as mandatory; retain its structural ID,
  accepted classes, minimum, and layer/policy/rule provenance.
- Treat `requires` as assumptions to establish; `ensures` and `limit` as mandatory;
  `forbids` as hard boundaries; `allows` as an upper bound; and `prefer` only after
  mandatory obligations hold.
- Use structural IDs for identity. Labels are navigation only.
- Treat a verifier symbol as a registered producer, not an arbitrary command.
- Treat `inspect` as a human view of canonical CBOR, not evidence that any verifier
  ran.

After authority is established, read the task, relevant source, and visible tests
once in one batch. Map each edit to mandatory IDs, preserve public interfaces and
unrelated behavior, and reject visible-test-passing changes that violate a
higher-precedence obligation.

## Verify and report

Run visible checks and available registered verifier adapters. Keep implementation
state separate from evidence state. Update evidence only from the producer bound to
that obligation; if unavailable, mark its obligations unresolved. Never add tests or
extra edits as a substitute for an unavailable adapter. A visible check is not
adapter evidence unless it is the registered producer.

For policy obligations, count distinct bound producers. Missing mappings or
registrations are unresolved; refutation and faults retain their states.

Report the semantic ID, changed files, checks and adapters run, obligation status
grouped by ID or range, and remaining gaps. Claim success only when every mandatory
obligation is accepted, no forbidden effect occurred, and all required evidence is
present. Report from the retained working view; do not reread code solely to prepare
the final response.
