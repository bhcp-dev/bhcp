---
name: interpret-bhcp-contract
description: Interpret and operationalize canonical BHCP contracts without inventing semantics. Use when implementing or reviewing work governed by a .bhcp file, validating a pinned semantic ID, mapping the Rust-rendered inspection of CDDL-aligned canonical CBOR to code and evidence, or diagnosing unsupported BHCP syntax.
---

# Interpret BHCP Contract

Treat the BHCP compiler output and `SEMANTICS.md` as normative. Use this skill as a
workflow aid only; never create meaning that is absent from the compiled contract.

## 1. Establish authority

1. Locate the canonical `.bhcp` source, adjacent semantic-ID file, task projection,
   and project manifest when present.
2. Use `bhcp` directly when it is on `PATH`; otherwise use the repository's
   documented wrapper. In the BHCP repository, invoke
   `mise exec -- cargo run --quiet -- <command> <file>` when no built binary exists.
   Do not probe `bhcp --help`; this CLI reports usage with a nonzero exit status.
   For Rust project checks, use `mise exec -- cargo ...` immediately when `cargo` is
   absent and `mise` is available; do not search for alternate toolchain installs.
3. Run `hash` first and compare its output byte-for-byte with the pinned semantic ID.
   Stop and report the mismatch if it differs.
4. Run `inspect` once on the source or a checked-in `.cbor` artifact. The Rust CLI
   validates deterministic CBOR against the existing artifact boundary and renders
   typed goal interfaces, structural clauses, effects, preferences, and expanded
   verifier targets as a compact human outline. Distill it immediately into the
   matrix below and retain that matrix as the working contract view. Do not reread
   the `.bhcp` source to reconstruct information already present in the outline.
5. Do not run `lower` for routine contract interpretation. `parse` and `lower` emit
   canonical CBOR, not a conversational representation. When debugging an exact
   artifact, capture those bytes outside the repository; pass the `.cbor` file back to `bhcp inspect`.
   Never stream raw AST or IR bytes into the conversation.
6. Reuse the hash and inspection output. Do not repeat compiler views to rediscover
   information already present, including during final reporting or adapter search.
7. Stop on compiler diagnostics, unsupported syntax, unresolved verifier targets,
   or an absent canonical source. Do not guess a fallback interpretation.

## 2. Build the obligation matrix

Read the inspection outline and produce this compact working matrix before editing
code:

| Structural ID | Class | Normalized condition | Evidence producer | Implementation response | Implementation state | Evidence state |
| --- | --- | --- | --- | --- | --- | --- |

Include every `requires`, `ensures`, `limit`, `allows`, `forbids`, and `prefer`
clause. Reverse each `verify ... -> target-id` line onto the matching structural
rows. The inspector already expands goal-wide verifier coverage.

Apply these rules:

- Treat `requires` as entry assumptions that must be established.
- Treat `ensures` and `limit` as mandatory acceptance conditions.
- Treat `forbids` as prohibited effects and `allows` as an upper bound, not an
  instruction to exercise every effect.
- Optimize `prefer` only after all mandatory obligations are satisfied.
- Treat labels as navigation aids. Use lowered structural IDs for identity and
  verifier coverage.
- Treat a verifier symbol as a binding to a registered evidence producer, not as
  permission to invent or execute an arbitrary command.
- Treat `inspect` as a human view of validated canonical CBOR, not evidence that any
  verifier ran.
- Apply evidence state to contract-obligation IDs in each verifier's `targets` list.
  A `verify` clause configures coverage; its own clause ID is not an obligation and
  does not receive an evidence state.
- Keep implementation state (`unaddressed`, `implemented`, `observed`) separate from
  evidence state (`accepted`, `rejected`, `unresolved`, `faulted`). A plausible edit
  or visible test may establish implementation progress but cannot mark an
  obligation accepted when its bound evidence producer has not run.

Keep the matrix concise. Do not narrate clauses that have no implementation impact.
Use `not-applicable` evidence state for authority and preference rows; report their
effect compliance or optimization outcome separately.

## 3. Implement against the matrix

1. Read the governed task and code only after the matrix is complete.
2. Map each intended edit to one or more mandatory obligation IDs.
3. Preserve public interfaces and unrelated behavior unless the contract requires a
   change.
4. Reject attractive implementations that satisfy visible tests but violate a
   higher-precedence obligation, effect boundary, or limit.
5. Update implementation and evidence state independently.

## 4. Verify and report

Run the visible project checks and registered verifier adapters. Update evidence
state only from the producer bound to that structural obligation. If an adapter is
not available, mark its obligations unresolved; never report them as accepted from
intuition, source inspection, or visible tests alone.

Report:

- the verified semantic ID;
- changed files;
- checks and evidence producers actually run;
- accepted, rejected, unresolved, or faulted obligations by structural ID; and
- any remaining gap that prevents a success claim.

Claim success only when every mandatory obligation is accepted, no forbidden effect
was observed, and all required verifier evidence is present.
