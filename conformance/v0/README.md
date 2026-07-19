# BHCP v0 conformance scenarios

This catalog is normative. A complete v0 implementation MUST provide executable
fixtures for every scenario ID and publish its result in a feature manifest and
evidence bundle. Until the complete parser, checker, planner, and runtime exist,
unimplemented cases remain the stable acceptance contract rather than executable
tests in this repository.

Each positive case must check canonical AST, semantic IR, graph, execution result, and
deterministic bytes where those stages apply. Each negative case must check a stable
diagnostic code and must not emit a misleading later-stage artifact.

The first executable slice covers the source-to-IR portions of SYN-02, ID-01,
ID-02, and deterministic emission for CBOR-01 using
`fixtures/canonical-simple.bhcp`. The adjacent presentation fixture proves that
comments, formatting, and diagnostic labels do not affect semantic identity. The
checked-in `.ast.cbor` and `.ir.cbor` files are compiler output and are validated by
the same Rust harness as the 17 root diagnostic fixtures. The self-hosted `all`
fixture additionally exercises executable portions of KRN-01 through KRN-09,
KRN-12, and ALG-ALL, including source-defined precedence and generic reducer
re-evaluation. Obligation-graph proof coverage and all unlisted scenarios remain
normative acceptance requirements, not claimed implementation support.

The registered verifier slice additionally executes EVD-01 through EVD-04 for flat
contract clauses. It does not yet claim general obligation-graph construction,
process-backed project adapters, freshness windows, signatures, or full proof
coverage.

The canonical policy parser fixture `fixtures/canonical-policy.bhcp` executes the
authored source boundary for POL-01: explicit layer and inheritance syntax, stable
rule IDs, a diagnostic-only label, and closed typed rules lower to a validated source
policy document. The Rust composition suite executes the remaining POL-01/POL-02
boundaries over typed documents:
layer/source normalization, inheritance validation, restrictive joins, exact-number
limits, deny retention, decomposition-independent semantic identity, and adversarial
weakening rejection. Waiver application is still represented only by WAV-01/WAV-02
acceptance requirements.

## Syntax, identity, and encoding

| ID | Scenario | Expected result |
| --- | --- | --- |
| SYN-01 | Canonical source and a remapped keyword/sigil/delimiter profile lower to equivalent meaning. | Different AST presentation metadata; byte-identical normalized semantic IR and semantic ID. |
| SYN-02 | Source omits the profile preamble. | Exact profile `bhcp/canonical@0` is selected. |
| SYN-03 | Custom source has a malformed or non-first preamble. | Parse rejection before profile rules run. |
| ID-01 | Only whitespace, comments, labels, source spans, or formatting change. | Semantic ID unchanged; artifact ID changes when complete artifact metadata changes. |
| ID-02 | An observable output field, branch tag, effect, preference, policy, or native extension changes. | Semantic ID changes. |
| ID-03 | Alpha-equivalent local binders and the same tagged derived `all` branches in a different source order lower where effects make order unobservable. | Identical normalized kernel-network bytes and semantic ID. |
| ID-04 | Derived `chain` branches are permuted. | Different lowered network bytes and semantic ID. |
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
| OWN-04 | A derived retention goal attempts to persist an expiring borrow. | Rejected; ownership or approved persistent sharing is required. |
| EFF-01 | A pure goal calls an effectful child. | Effect propagates or the call is rejected; it is never hidden. |
| EFF-02 | Unsafe/foreign execution is allowed by policy. | Capability is visible and an evidence gap is emitted. |
| EFF-03 | A child allowance exceeds a parent prohibition. | Denied; parent ceiling and deny-wins are preserved. |

## Verification and evidence

| ID | Scenario | Expected result |
| --- | --- | --- |
| EVD-01 | Explicit verifier targets are reordered, then their labels and references are consistently renamed. | Targets resolve to normalized structural obligation IDs; semantic identity is unchanged while AST artifact identity changes. |
| EVD-02 | Registered verifiers accept, return accepted counter-evidence, remain inconclusive, or violate their operational contract. | Candidate decisions remain `Accepted | Rejected | Unresolved`; verifier faults remain operationally `Faulted` with partial evidence. |
| EVD-03 | A required verifier symbol is not registered. | No callback or command is inferred; a required `unsupported` evidence gap leaves its obligations unresolved. |
| EVD-04 | The same typed candidate, content references, timestamp, registry, and verifier outputs are checked twice. | Strongly validated evidence bundles, payload references, deterministic CBOR bytes, and artifact IDs are identical. |

## Kernel, derived goal algebra, state, and planning

For each standard derived behavior, scenarios `ALG-x-S`, `ALG-x-R`, `ALG-x-U`, and
`ALG-x-F` cover completed satisfied, refuted, and unresolved verdicts plus faulted
executions according to S8. Hand-written core networks and their derived surface
forms MUST lower to the same meaning.

| ID | Scenario | Expected result |
| --- | --- | --- |
| KRN-01 | Reducer receives no observations and returns `Pending` with multiple known child tags. | Tags resolve to children that are eligible together subject to effect/ownership/policy analysis. |
| KRN-02 | A pending reduction names an unknown, duplicate, or already observed child tag, or names no tags. | Stable kernel rejection; no execution result is emitted. |
| KRN-03 | A reducer returns `Concluded` with a forged token or invalid derivation. | Proof-check rejection and visible operational fault. |
| KRN-04 | Equivalent standard-prelude syntax and hand-written `§compose` source fully lower. | Byte-identical normalized kernel networks and semantic IDs. |
| KRN-05 | Reducer state names are inspected. | Only adjectival `pending` and `concluded` states occur. |
| KRN-06 | A premise-free reducer proves an empty logical identity. | The checker derives and seals the valid derivation ID from the network; that ID supplies the verdict's evidence or counter-evidence token. |
| KRN-07 | Kernel IR is inspected for derived or planner metadata. | No behavior kind, quantifier family, guard, dependency, budget, scheduling order, or parallelism hint is present. |
| KRN-08 | A concluded reduction is proof-checked. | The generic checker re-evaluates the referenced BHCP reducer and validates sealed premises; no behavior-specific proof-rule tag is accepted. |
| KRN-09 | A network reducer omits the parent input, uses a non-monomorphized observation record, or returns the wrong reduction type. | Static rejection before IR acceptance or execution. |
| KRN-10 | A reducer branches on an operational trace event, timestamp, or payload. | Static rejection; faults may be discriminated and propagated, but trace contents remain opaque to semantic choice. |
| KRN-11 | A composition quantifier has a statically finite domain, then a verifier-backed or runtime-only domain. | The static domain expands to explicit children before IR; the other domains are rejected and require bounded/well-founded recursive goals. |
| KRN-12 | A standard or extension lowerer consumes canonical typed shape data. | It receives `Meta<DerivedForm,I,O>`, returns `Meta<NetworkShape,I,O>`, cannot observe presentation or allocate network/child IDs, and leaves no meta value in runtime semantic IR. |
| RES-01 | A run completes without proof either way. | `Completed(Unresolved(...))`; it is neither refuted nor faulted. |
| RES-02 | Execution violates its operational contract. | `Faulted(...)` outside the semantic verdict; it is not counter-evidence. |
| ALG-ALL | Products, empty identity, fault-vs-unresolved precedence, and refutation despite unrelated fault. | Named product and evidence from every success; decisive refutation wins. |
| ALG-ANY | Tagged winner, empty identity, precedence, and success despite unrelated fault. | Stable winning branch tag; decisive satisfaction wins. |
| ALG-NONE | Counter-evidence for every child, empty identity, and a satisfying child despite unrelated fault. | `Unit` only with all counter-evidence; failed attempt/timeout never proves NOR. |
| ALG-CHAIN | Typed dependent outputs and causal stopping. | Source order preserved; later steps do not run after a non-satisfied step. |
| ALG-GATE | A unary gate's total pure condition is false, true, or faults; when true, its child may produce any execution result. | False yields `Excluded`; true requests exactly one child and yields `Included<T>` on satisfaction; child unresolution/fault propagates. An evidence-dependent condition must be an explicit child. |
| RET-01 | Derived retention reads empty state, captures satisfaction, and retains after refutation, unresolution, or fault. | Only completed satisfaction atomically replaces the captured tuple. |
| STA-01 | Two derived retention writers race with the same prior version. | One atomic commit; the other retries or reports a compare-and-swap conflict. |
| STA-02 | Captured evidence exceeds freshness. | `Completed(Unresolved(stale-evidence, ...))` unless stricter policy requires a fault. |
| REC-01 | Recursive goal has a static bound. | Accepted and bound appears in IR/graph. |
| REC-02 | Recursive goal has a decreasing well-founded measure. | Accepted with checker evidence. |
| REC-03 | Recursive goal has neither. | Static rejection. |
| PLN-01 | A derived `all` reducer returns one pending set whose children have no dependency, borrow, state, or effect conflicts. | The execution graph permits concurrent scheduling without adding a hint to semantic IR. |
| PLN-02 | One pending set contains children sharing write state or exclusive borrows. | Execution-graph conflict edges prevent concurrency and planner diagnostics report stable reasons. |
| PLN-03 | A chain connects incompatible output/input types. | Checker rejection before planning. |
| PLN-04 | Children consume an unallocated shared budget including retries. | Total accounting remains within parent limit or planning refuses. |
| PLN-05 | Requirements lack parent facts, invariants, or prior guarantees. | Explicit obligations are emitted; never assumed discharged. |

## Policy, waivers, and extensions

| ID | Scenario | Expected result |
| --- | --- | --- |
| POL-01 | Organization → team → repository → user layers only strengthen. | Deterministic monotonic composite policy. |
| POL-02 | A local layer widens authority, loosens a limit, removes a requirement/evidence demand, allows a denied effect, or relaxes type mode. | Whole layer rejected without a waiver using `BHCP8101`–`BHCP8106`; diagnostic names later rule, earlier authority, attempted change, and waiver requirement. |
| POL-03 | Overlapping limits use incompatible units. | Whole layer rejected with auditable `BHCP8107`; no implicit conversion or partial effective policy. |
| POL-04 | Policy sources are duplicated or have missing, cyclic, or cross-layer inheritance references. | Composition rejected with stable `BHCP8110`; malformed source values remain `BHCP8001`. |
| POL-05 | Policy presentation, source order, decomposition, retained provenance, or an observable effective coordinate changes. | Presentation/order normalize to identical bytes; decomposition/provenance change artifact ID only; requirements, evidence, effects, limits, type mode, waivability, and issuers change semantic ID. Materialized and recomputed algorithm-tagged IDs match. |
| WAV-01 | Exact scoped weakening has an authorized issuer, audit reference, active interval, and expiry. | Accepted only inside scope and time. |
| WAV-02 | Waiver is expired, premature, overbroad, unauthorized, or targets a non-waivable rule. | Rejected, not ignored. |
| EXT-01 | A derived extension names a total pure BHCP lowering function, declares no native payload schema, and fully lowers to core IR. | Extension presentation disappears; core meaning is checked and hashed. Missing or invalid lowering is rejected. |
| EXT-02 | A supported native extension with a payload schema is present. | Must-understand node, rules, and identity remain in semantic IR; a native descriptor cannot provide a derived lowering. |
| EXT-03 | An unsupported native extension is present. | Artifact rejected before planning. |
| EXT-04 | An extension attempts to override core meaning or policy. | Descriptor/program rejected. |
