---
name: interpret-bhcp-contract
description: Interpret and operationalize canonical BHCP contracts without inventing semantics. Use when implementing or reviewing work governed by a .bhcp file, validating a pinned semantic ID, mapping lowered obligations and verifier bindings to code and evidence, or diagnosing unsupported BHCP syntax.
---

# Interpret BHCP Contract

Treat the BHCP compiler output and `SEMANTICS.md` as normative. Use this skill as a
workflow aid only; never create meaning that is absent from the compiled contract.

## 1. Establish authority

1. Locate the canonical `.bhcp` source, adjacent semantic-ID file, task projection,
   and project manifest when present.
2. Use the repository's documented BHCP executable. In this repository, invoke the
   CLI through `mise exec -- cargo run --quiet -- <command> <file>` when no built
   `bhcp` binary is available.
3. Run `parse`, `lower`, `inspect`, and `hash` once. Reuse their output instead of
   repeatedly rereading the source.
4. Compare `hash` output byte-for-byte with the pinned semantic ID. Stop and report
   the mismatch if it differs.
5. Stop on compiler diagnostics, unsupported syntax, unresolved verifier targets,
   or an absent canonical source. Do not guess a fallback interpretation.

## 2. Build the obligation matrix

Read the lowered entrypoint goal and produce this compact working matrix before
editing code:

| Structural ID | Class | Normalized condition | Evidence producer | Implementation response | State |
| --- | --- | --- | --- | --- | --- |

Include every `requires`, `ensures`, `limit`, `allows`, `forbids`, and `prefer`
clause. Attach each lowered `verify` binding to the structural obligation IDs it
targets.

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

Keep the matrix concise. Do not narrate clauses that have no implementation impact.

## 3. Implement against the matrix

1. Read the governed task and code only after the matrix is complete.
2. Map each intended edit to one or more mandatory obligation IDs.
3. Preserve public interfaces and unrelated behavior unless the contract requires a
   change.
4. Reject attractive implementations that satisfy visible tests but violate a
   higher-precedence obligation, effect boundary, or limit.
5. Update the matrix state as evidence becomes available.

## 4. Verify and report

Run the visible project checks and registered verifier adapters. If an adapter is not
available, mark its obligations unresolved; never report them as accepted from
intuition or visible tests alone.

Report:

- the verified semantic ID;
- changed files;
- checks and evidence producers actually run;
- accepted, rejected, unresolved, or faulted obligations by structural ID; and
- any remaining gap that prevents a success claim.

Claim success only when every mandatory obligation is accepted, no forbidden effect
was observed, and all required verifier evidence is present.
