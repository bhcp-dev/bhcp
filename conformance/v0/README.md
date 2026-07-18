# BHCP v0 conformance scenarios

This catalog is normative. A complete v0 implementation MUST provide executable
fixtures for every scenario ID and publish its result in a feature manifest and
evidence bundle. Until the complete parser, checker, planner, and runtime exist,
unimplemented cases remain the stable acceptance contract rather than executable
tests in this repository.

Each positive case must check canonical AST, semantic IR, graph, outcome, and
deterministic bytes where those stages apply. Each negative case must check a stable
diagnostic code and must not emit a misleading later-stage artifact.

The first executable slice covers the source-to-IR portions of SYN-02, ID-01,
ID-02, and deterministic emission for CBOR-01 using
`fixtures/canonical-simple.bhcp`. The adjacent presentation fixture proves that
comments, formatting, and diagnostic labels do not affect semantic identity. The
checked-in `.ast.cbor` and `.ir.cbor` files are compiler output and are validated by
the same Rust harness as the 17 root diagnostic fixtures. Unlisted
scenarios remain normative acceptance requirements, not claimed implementation
support.

## Syntax, identity, and encoding

| ID | Scenario | Expected result |
| --- | --- | --- |
| SYN-01 | Canonical source and a remapped keyword/sigil/delimiter profile lower to equivalent meaning. | Different AST presentation metadata; byte-identical normalized semantic IR and semantic ID. |
| SYN-02 | Source omits the profile preamble. | Exact profile `bhcp/canonical@0` is selected. |
| SYN-03 | Custom source has a malformed or non-first preamble. | Parse rejection before profile rules run. |
| ID-01 | Only whitespace, comments, labels, source spans, or formatting change. | Semantic ID unchanged; artifact ID changes when complete artifact metadata changes. |
| ID-02 | An observable output field, branch tag, effect, preference, policy, or native extension changes. | Semantic ID changes. |
| ID-03 | Alpha-equivalent local binders and permuted unobservable `all` branches normalize. | Identical canonical semantic bytes. |
| ID-04 | `chain` branches are permuted. | Different canonical bytes and semantic ID. |
| CBOR-01 | Each root diagnostic fixture is encoded, decoded, and encoded again. | Deterministic bytes are identical and validate against `root-document`. |
| CBOR-02 | A content reference has `bhcp.hash/sha3-512@0` and another registered digest. | Both tags survive; understood digests verify; the default digest is 64 bytes. |
| CBOR-03 | A map uses duplicate keys, an indefinite length, or non-shortest integer encoding. | Canonical-wire rejection. |

## Types, expressions, ownership, and effects

| ID | Scenario | Expected result |
| --- | --- | --- |
| TYP-01 | Inferred locals elaborate under `infer-strict`. | Types are materialized; no implicit `Dynamic` remains. |
| TYP-02 | A strict goal consumes an unchecked `Dynamic`. | Static rejection unless an explicit checked boundary is present. |
| TYP-03 | Gradual and dynamic goals cross a typed boundary. | Runtime check is explicit; failure is typed or faulted as declared. |
| TYP-04 | Nominally distinct equal-shaped types are mixed. | Rejected without a `refines` relation. |
| TYP-05 | Open structural records use safe width subtyping. | Accepted; observable field names remain semantic. |
| TYP-06 | A value enters a refinement. | Accepted only with evidence for the total pure predicate. |
| TYP-07 | Foreign null/missing values enter core. | Lowered to `Option<T>` or an explicit tagged absence variant. |
| TYP-08 | Expected failure and success use `Result<T,E>`. | Both variant tags and payload types are preserved. |
| NUM-01 | Rational, decimal, signed zero, infinity, and NaN payload values round-trip. | Exact components/float bits are unchanged. |
| EXP-01 | A finite quantifier and verifier-backed finite quantifier are checked. | Both accepted with a witnessed finite domain. |
| EXP-02 | Expression recursion, I/O, nondeterminism, or an effectful call is attempted. | Rejected as non-total or impure. |
| OWN-01 | Read borrows overlap. | Accepted when lifetimes fit. |
| OWN-02 | A write borrow overlaps any other borrow. | Borrow-conflict rejection. |
| OWN-03 | An owned affine value is moved then reused. | Use-after-move rejection on every branch. |
| OWN-04 | A latch attempts to retain an expiring borrow. | Rejected; ownership or approved persistent sharing is required. |
| EFF-01 | A pure goal calls an effectful child. | Effect propagates or the call is rejected; it is never hidden. |
| EFF-02 | Unsafe/foreign execution is allowed by policy. | Capability is visible and an evidence gap is emitted. |
| EFF-03 | A child allowance exceeds a parent prohibition. | Denied; parent ceiling and deny-wins are preserved. |

## Goal algebra, state, and planning

For each of `all`, `any`, `none`, `chain`, `gate`, and `latch`, scenarios `ALG-x-S`,
`ALG-x-R`, `ALG-x-I`, and `ALG-x-F` cover satisfied, refuted, indeterminate, and
faulted children according to the S8 propagation tables.

| ID | Scenario | Expected result |
| --- | --- | --- |
| ALG-ALL | Products, empty identity, fault-vs-indeterminate precedence, and refutation despite unrelated fault. | Named product and evidence from every success; decisive refutation wins. |
| ALG-ANY | Tagged winner, empty identity, precedence, and success despite unrelated fault. | Stable winning branch tag; decisive satisfaction wins. |
| ALG-NONE | Counter-evidence for every child, empty identity, and a satisfying child despite unrelated fault. | `Unit` only with all counter-evidence; failed attempt/timeout never proves NOR. |
| ALG-CHAIN | Typed dependent outputs and causal stopping. | Source order preserved; later steps do not run after a non-satisfied step. |
| ALG-GATE | False, true, indeterminate, and faulted conditions. | False yields `Skipped`; true yields `Completed<T>`; condition I/F propagates. |
| ALG-LATCH | Empty read, capture, retain after R/I/F, and successful replacement. | Explicit state; only accepted success atomically replaces the captured tuple. |
| STA-01 | Two latch writers race with the same prior version. | One atomic commit; the other retries or reports a compare-and-swap conflict. |
| STA-02 | Captured evidence exceeds freshness. | `Indeterminate(stale-evidence, ...)` unless stricter policy requires fault. |
| REC-01 | Recursive goal has a static bound. | Accepted and bound appears in IR/graph. |
| REC-02 | Recursive goal has a decreasing well-founded measure. | Accepted with checker evidence. |
| REC-03 | Recursive goal has neither. | Static rejection. |
| PLN-01 | `all` children have no dependency, borrow, state, or effect conflicts. | Marked parallel-eligible. |
| PLN-02 | `all` children share write state or exclusive borrows. | Marked non-parallel with stable reasons. |
| PLN-03 | A chain connects incompatible output/input types. | Checker rejection before planning. |
| PLN-04 | Children consume an unallocated shared budget including retries. | Total accounting remains within parent limit or planning refuses. |
| PLN-05 | Requirements lack parent facts, invariants, or prior guarantees. | Explicit obligations are emitted; never assumed discharged. |

## Policy, waivers, and extensions

| ID | Scenario | Expected result |
| --- | --- | --- |
| POL-01 | Organization → team → repository → user layers only strengthen. | Deterministic monotonic composite policy. |
| POL-02 | A local layer widens authority, loosens a limit, weakens evidence, or relaxes type mode. | Rejected without a waiver. |
| WAV-01 | Exact scoped weakening has an authorized issuer, audit reference, active interval, and expiry. | Accepted only inside scope and time. |
| WAV-02 | Waiver is expired, premature, overbroad, unauthorized, or targets a non-waivable rule. | Rejected, not ignored. |
| EXT-01 | A derived extension fully lowers to core IR. | Extension presentation disappears; core meaning is checked and hashed. |
| EXT-02 | A supported native extension is present. | Must-understand node, rules, and identity remain in semantic IR. |
| EXT-03 | An unsupported native extension is present. | Artifact rejected before planning. |
| EXT-04 | An extension attempts to override core meaning or policy. | Descriptor/program rejected. |
