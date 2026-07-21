# BHCP v0 conformance scenarios

This catalog is normative. A complete v0 implementation MUST provide executable
fixtures for every scenario ID and publish its result in a feature manifest and
evidence bundle. Until the complete parser, checker, planner, and runtime exist,
unimplemented cases remain the stable acceptance contract rather than executable
tests in this repository.

[`completion-manifest.txt`](completion-manifest.txt) closes that inventory for the
practical v0 roadmap. Its validator derives the explicit IDs below, expands each of
the five standard algebra rows into satisfied/refuted/unresolved/faulted instances,
and therefore requires exactly 99 scenario records. It also reconciles all 17 CDDL
root kinds, stable issue keys #99–#134, ten observable pipeline outcomes, and the
[`reference-program/`](reference-program/) artifact and feature inventory. Missing,
duplicate, or unknown records fail the focused contract test. The complete reference
program remains deliberately ahead of the partial front end and is not counted as
executable conformance yet. Its isolated extension source now executes through
complete derived lowering, and its already-implemented policy and typed artifact
boundaries validate now, while a
reviewed typed semantic projection, exact alternate-source normalization, closed
registry, content-bound extension rules, explicit policy-evidence producers,
obligation inventory, and separate eight-case outcome matrix prevent disconnected,
ill-typed, unauthorized, or category-collapsing future targets from passing on marker
presence alone.

Each positive case must check canonical AST, semantic IR, graph, execution result, and
deterministic bytes where those stages apply. Each negative case must check a stable
diagnostic code and must not emit a misleading later-stage artifact.

The first executable slice covers the source-to-IR portions of SYN-02, SYN-03,
ID-01, ID-02, and deterministic emission for CBOR-01 using
`fixtures/canonical-simple.bhcp` and the byte-level profile-preamble harness. The
explicit `fixtures/canonical-profile-preamble.bhcp` example proves that the fixed
scanner selects canonical syntax while retaining original source offsets and
artifact identity. The adjacent presentation fixture proves that
comments, formatting, and diagnostic labels do not affect semantic identity. The
checked-in `.ast.cbor` and `.ir.cbor` files are compiler output and are validated by
the same Rust harness as the 17 root diagnostic fixtures. The self-hosted `all`,
`any`, `none`, `chain`, and `gate` fixtures additionally exercise executable
portions of KRN-01 through KRN-09, KRN-12, KRN-13, ALG-ALL, ALG-ANY, ALG-NONE,
ALG-CHAIN, and ALG-GATE, including source-defined precedence, stable choice, typed
predecessor and parent-field edges, causal early stop, gate non-observation, empty
identities, and generic reducer re-evaluation. The `any` slice currently covers
homogeneous child outputs as a closed tagged winner record; `none` exposes canonical
`Unit`; each later `chain` child consumes its immediate predecessor's whole output;
and a unary gate infers `Excluded | Included<T>` from its child. Obligation-graph
proof coverage and unlisted scenarios remain normative acceptance requirements, not
claimed implementation support.

`goal-algebra/manifest.txt` and `tests/goal_algebra_conformance.rs` publish the
complete executable Phase 1 algebra boundary in one inventory. The harness compiles
all five derived forms, compares ten checked-in AST/IR artifacts byte for byte,
round-trips both root kinds, compares the four expressible explicit `§compose`
equivalents, and resolves named edge-case and generic proof-checker evidence in the
behavior-specific suites. It also pins the six generator inputs, the 17 schema-root
examples, and that phase's feature-manifest boundary without rewriting historical
artifacts when later graph stages land.

The byte-level scanner tests execute the fixed S9.1 selection boundary before any
profile-specific normalization: omission, an optional BOM, or an explicit canonical
preamble select exactly one profile, while invalid UTF-8, CRLF, Unicode whitespace,
truncation, aliases, duplicates, and misplaced directives fail with `BHCP0003` and
no artifact output. Exact custom symbols are selected without aliasing; unregistered
symbols fail closed with `BHCP0004`. The typed artifact harness in
`tests/profile_models.rs` round-trips every mapping category, common profile field,
and type mode through deterministic CBOR and pins stable malformed-document
diagnostics without closing feature negotiation. The finite profile-resolution model
in `tests/profile_contract.rs` pins the complete S9.1 decision boundary alongside the
executable registry. Positive vectors resolve exact single-parent syntax and profile
chains, safe token-coordinate overrides, nondecreasing type mode, and root-to-leaf
policy overlays. Adversarial vectors cover missing/cyclic parents, duplicate
coordinates, category errors, ambiguous or prefix-conflicting surfaces, recursive
aliases, core rebinding, unrelated child syntax, weaker child type mode, and duplicate
overlays. It specifies SYN-01/SYN-03 resolution behavior. The executable
`tests/profile_lowering.rs` harness and paired
`fixtures/profile-lowering-canonical.bhcp` / `fixtures/profile-lowering-words.bhcp`
sources close SYN-01 for one explicitly registered effective syntax: all six mapping
categories normalize before the canonical parser, comments and literals are inert,
original spans drive diagnostics, and equivalent canonical/custom programs share a
semantic ID. Adversarial effective maps fail as `BHCP9002`; mapped-away canonical
spellings fail as `BHCP0005`; registry omission remains `BHCP0004`.
`tests/profile_resolution.rs` makes the previously finite-only inheritance boundary
executable: registry insertion order cannot change the resolved value, child syntax
overrides flatten root to leaf, exact policy parents and overlays compose before
elaboration, and the resolved custom program matches canonical semantic identity.
Missing/cyclic parents, unrelated syntax, weaker modes, duplicate or missing overlays,
and inherited mapping conflicts fail atomically as `BHCP9003`/`BHCP9002`; attached
policy weakening retains its ordinary `BHCP8101`–`BHCP8107` code.
`tests/profile_adversarial.rs` and its checked-in
`tests/fixtures/profile_adversarial/manifest.txt` corpus make the prohibited behavior
boundary executable. Ambiguous aliases, recursion, canonical keyword capture, and
reserved-core rebinding fail before parsing with profile/syntax/mapping/rule context;
mapped-away source retains its original span; parser, macro, and semantic-override
artifact fields fail the closed model; and the CLI emits no partial formatted source.
`tests/profile_layout_conformance.rs` loads the two checked-in syntax/profile roots,
shared overlay, source forms, and formatter snapshots under `profile-layout/`. The
compact symbolic and spaced narrative forms retain different selected profiles,
labels, comments, formatting, AST bytes, IR artifact bytes, and artifact IDs while
sharing one policy-governed semantic ID. Every syntax, profile, AST, and IR artifact
round-trips through deterministic CBOR and its CDDL root. A meaningful overlay change
changes semantic identity; presentation-only edits do not; equivalent malformed forms
keep the same parser code/message while source and column remain presentation-sensitive.
`tests/profile_phase_audit.rs`, `profile-phase-audit.txt`, and the human-readable
[`profile-phase-audit.md`](profile-phase-audit.md) close SYN-08. The machine manifest
requires exactly three acceptance claims for each Phase 4 issue #41–#49, resolves
every named test function, audits local evidence links, and keeps incomplete-v0 and
unrestricted-macro/grammar-plugin non-goals consistent across public documentation.

The registered verifier slice additionally executes EVD-01 through EVD-06, including
capability-bounded project adapters and their deterministic evidence mapping. The
obligation builder constructs normalized contract, retained-case, verifier,
parent/child, and effective-policy nodes with exact audit provenance and open initial
status. Freshness windows, signatures, and full proof coverage remain unclaimed.

The manifest at `policy/manifest.txt` makes the complete no-waiver POL-01 through
POL-08 slice executable. Its canonical sources cover organization, team, repository,
and user layers plus all six closed category/operation pairs. The harness regenerates
and pins deterministic effective artifacts, proves equivalent decompositions share a
semantic identity while retaining distinct artifact identities, and proves a
meaningful restriction changes both identities. It also compiles `policy/program.bhcp`
under the baseline policy, validates the retained per-goal enforcement decision and
effective-policy identities against semantic IR, and checks stable authority and
limit denials. Every invalid weakening, topology conflict, incompatible unit, and
unsupported feature has a manifest-pinned diagnostic; source and canonical-CBOR CLI
composition must be byte-identical and fail atomically. Waiver application remains
represented only by WAV-01/WAV-02 acceptance requirements.

## Syntax, identity, and encoding

| ID | Scenario | Expected result |
| --- | --- | --- |
| SYN-01 | Canonical source and a remapped keyword/sigil/delimiter profile lower to equivalent meaning. | Different AST presentation metadata; byte-identical normalized semantic IR and semantic ID. |
| SYN-02 | Source omits the profile preamble. | Exact profile `bhcp/canonical@0` is selected. |
| SYN-03 | Custom source has a malformed or non-first preamble. | Parse rejection before profile rules run. |
| SYN-04 | Canonical or custom source is formatted repeatedly through a validated resolved profile. | Byte-idempotent output; comments and mapped Unicode spellings survive; the canonical token stream, AST shape, semantic IR, and semantic ID remain equivalent. |
| SYN-05 | An effective profile introduces ambiguous/recursive aliases, captures canonical words, or rebinds reserved core symbols. | `BHCP9002` before program parsing; diagnostic identifies profile, syntax, mapping index, coordinate/surface, and violated rule; no AST/IR/source output. |
| SYN-06 | A syntax/profile artifact embeds parser code, unrestricted macros, or semantic overrides. | Closed-model `BHCP9001`; no registry activation or later artifact. |
| SYN-07 | Two checked-in profiles map and format one policy-governed goal with substantially different comments, labels, words, punctuation, widths, and indentation. | Selected profile and artifact IDs differ; resolved overlay, semantic projection, and semantic ID match; all syntax/profile/AST/IR roots round-trip deterministically. |
| SYN-08 | The Phase 4 implementation, evidence, documentation, issue graph, and milestone maturity are audited. | Every #41–#49 acceptance claim links to an existing named test; all local report links resolve; arbitrary grammar/plugins/macros remain non-goals; the bounded presentation milestone is complete without claiming complete v0. |
| ID-01 | Only whitespace, comments, labels, source spans, or formatting change. | Semantic ID unchanged; artifact ID changes when complete artifact metadata changes. |
| ID-02 | An observable output field, branch tag, effect, preference, policy, or native extension changes. | Semantic ID changes. |
| ID-03 | Alpha-equivalent local binders and the same tagged derived `all` branches in a different source order lower where effects make order unobservable. | Identical normalized kernel-network bytes and semantic ID. |
| ID-04 | Derived `chain` branches are permuted. | Different lowered network bytes and semantic ID. |
| CBOR-01 | Each root diagnostic fixture is encoded, decoded, and encoded again. | Deterministic bytes are identical and validate against `root-document`. |
| CBOR-02 | A content reference has `bhcp.hash/sha3-512@0` and another registered digest. | Both tags survive; understood digests verify; the default digest is 64 bytes. |
| CBOR-03 | A map uses duplicate keys, an indefinite length, or non-shortest integer encoding. | Canonical-wire rejection. |

## Types, expressions, ownership, and effects

`tests/type_checker.rs` makes TYP-01 through TYP-08 and NUM-01 executable at the
v0 type boundary. It covers inference without implicit `Dynamic`, explicit runtime
check descriptors recursively through composite types, nominal distinction,
open-record width subtyping, candidate-bound refinement evidence, exact closed
resource references, explicit `Option`/`Result` tags, the complete deterministic-CBOR
integer domain, exact numeric components, machine overflow, and unchanged float bits.
It also exercises all wire type forms,
generic arity/bounds, goal variance, canonical normalization, stable `BHCP4101`–
`BHCP4106` failures, semantic-IR schema validation, and type-sensitive identity.
The expression and ownership stages now consume this checked model; later effect and
runtime issues remain incomplete.

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

`tests/ownership_analysis.rs` makes OWN-01 through OWN-04 executable against
`own-01-read-overlap.bhcp` through `own-04-expired-retention.bhcp`. The same suite
covers all four argument modes, exact lifetime crossings (including shares), nested
branch joins without confusing sequential chain steps for overlap, recursive
transfer, affine and linear obligations, variant-backed handles, independent case
scopes, exact persistent-share approvals through nested initializer expressions,
pre-IR rejection, fully qualified handle types in semantic IR, and canonical handle
reference envelopes at the executable boundary.

`tests/effect_authority_analysis.rs` makes EFF-01 through EFF-03 executable before
semantic identity is computed. It proves deterministic-CBOR effect-set ordering,
child-to-parent propagation without kernel planner metadata, direct resource
projection, explicit parent ceilings, accumulated deny-wins prohibitions, and policy
checks over every propagated goal boundary. The same suite covers nominal
resource-scoped capabilities, unsafe evidence gaps, direct non-negative exact
dimensioned limits, and compatible same-priority preference objectives. Shared-budget
allocation and retry accounting remain PLN-04 work. PLN-05 obligation construction
is emitted as its own typed graph and is deliberately not represented as
`kernel-network` metadata.

`tests/capability_graph.rs` carries those EFF decisions across the graph boundary.
It checks one explicit request/final allow pair per execution-eligible effect,
structural resource coordinates, parent propagation, authored ceilings, deny-wins
context, effective-policy grants and exact source provenance, applied-waiver audit
data, unsafe/foreign/unsupported gaps, deterministic identities, and fail-closed
reconstruction.
Denied or unresolved authority remains a pre-IR error and therefore cannot be
reclassified as a planning grant.

`tests/expression_calculus.rs` makes EXP-01 and EXP-02 executable at the closed wire
calculus boundary. Its tables cover every expression constructor and pattern form,
both finite-quantifier paths, exact-number families, checked machine overflow,
invalid selection, failed casts, non-exhaustive matching, duplicate identities, and
unknown or recursively registered calls. Complete canonical source-expression parsing
and the full source-to-IR audit remain separate downstream conformance work.

`tests/function_predicate_elaboration.rs` connects that closed calculus to parsed
source definitions for the TYP-06, EXP-02, and EVD boundaries. It proves deterministic
forward resolution, registration-order-independent identity, bounded generic
inference and concrete specialization, validation of unused templates, retained
refinement evidence requirements, canonical predicate verifier input/configuration,
and fail-closed cycles, ambient calls, mismatches, inconsistent inference, bound
failures, and incomplete predicates. Verifier argument order normalizes away while
mode changes remain semantic. Source-expression syntax outside the currently parsed
slice and the full source-to-IR completion ledger remain later conformance work.

## Verification and evidence

| ID | Scenario | Expected result |
| --- | --- | --- |
| EVD-01 | Explicit verifier targets are reordered, then their labels and references are consistently renamed. | Targets resolve to normalized structural obligation IDs; semantic identity is unchanged while AST artifact identity changes. |
| EVD-02 | Registered verifiers accept, return accepted counter-evidence, remain inconclusive, or violate their operational contract. | Candidate decisions remain `Accepted | Rejected | Unresolved`; verifier faults remain operationally `Faulted` with partial evidence. |
| EVD-03 | A required verifier symbol is not registered. | No callback or command is inferred; a required `unsupported` evidence gap leaves its obligations unresolved. |
| EVD-04 | The same typed candidate, content references, timestamp, registry, and verifier outputs are checked twice. | Strongly validated evidence bundles, payload references, deterministic CBOR bytes, and artifact IDs are identical. |
| EVD-05 | A process adapter is registered in different registry orders and returns evidence for an explicit verifier target. | The request contains only the deterministic typed candidate and normalized structural targets; bundle bytes and audit-record order are identical, the item names the executable artifact, and provenance names the adapter declaration. |
| EVD-06 | A process adapter is absent, rejects, remains unresolved, faults, or emits malformed output. | CDDL-valid bundles and human inspection keep unsupported gaps, accepted refutations, unresolved gaps, verifier faults, and malformed-output faults distinct. |

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
| KRN-13 | A reducer uses typed literals/Boolean operations, sealed observation queries, an unsupported call in an unselected branch, or returns a satisfied value of the wrong output shape. | Closed total-pure operations evaluate deterministically; every branch is validated; unknown calls never dispatch to a host callback; output mismatch is rejected before a reduction is accepted. |
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
| PLN-05 | Requirements lack parent facts, invariants, or prior guarantees. | Deterministic explicit obligations and dependencies are emitted with open status; none is assumed discharged. |

The complete goal parser fixture covers the authored prerequisites for KRN-11,
REC-01..03, and PLN-03..05: quantifier domains, recursive goal references, typed
argument modes, limits, invariants, and nested composition all retain ordered source
spans in canonical AST. This is syntax evidence only. Static finiteness, decreasing
measures, chain compatibility, and shared-budget accounting remain assigned to their
checker, recursion, and planner roadmap owners. Obligation construction is executable
in `tests/obligation_graph.rs`.

## Policy, waivers, and extensions

| ID | Scenario | Expected result |
| --- | --- | --- |
| POL-01 | Organization → team → repository → user layers only strengthen. | Deterministic monotonic composite policy. |
| POL-02 | A local layer widens authority, loosens a limit, removes a requirement/evidence demand, allows a denied effect, or relaxes type mode. | Whole layer rejected without a waiver using `BHCP8101`–`BHCP8106`; diagnostic names later rule, earlier authority, attempted change, and waiver requirement. |
| POL-03 | Overlapping limits use incompatible units. | Whole layer rejected with auditable `BHCP8107`; no implicit conversion or partial effective policy. |
| POL-04 | Policy sources are duplicated or have missing, cyclic, or cross-layer inheritance references. | Composition rejected with stable `BHCP8110`; malformed source values remain `BHCP8001`. |
| POL-05 | Policy presentation, source order, decomposition, retained provenance, or an observable effective coordinate changes. | Presentation/order normalize to identical bytes; decomposition/provenance change artifact ID only; requirements, evidence, effects, limits, type mode, waivability, and issuers change semantic ID. Materialized and recomputed algorithm-tagged IDs match. |
| POL-06 | The policy CLI composes explicitly ordered source or canonical-CBOR inputs, inspects source/effective forms, or receives wrong layer order, unsupported features, malformed artifacts, or weakening. | Source/CBOR composition emits identical validated deterministic bytes; inspection names layers, rules, identities, and exact tightening provenance; every invalid case exits nonzero with a stable diagnostic and no partial artifact output. |
| POL-07 | Executable source is elaborated under an effective policy with an exact type-mode boundary, allowed/prohibited/ungranted authority, and a dimensioned numeric limit at or above its maximum. | Exact boundaries emit schema-valid IR retaining effective-policy identities and normalized per-goal decisions, then a deterministic capability graph with exact source/waiver audit provenance and one final grant per eligible effect; weaker type mode, prohibited authority, unresolved grant, and loose limits fail before IR with `BHCP8201`–`BHCP8204`; effective semantic changes alter program/graph semantics while source-only policy decomposition remains artifact provenance. |
| POL-08 | Applicable policy evidence demands have duplicate source contributors, multiple accepted classes or positive minima, registered or unavailable producers, and varying registration order. | Each effective demand becomes one normalized structural obligation retaining all source-layer provenance; only explicitly bound registered producers run, accepted class/minimum rules govern discharge, missing producers remain unresolved, rejection/fault remain distinct, and fixed inputs produce identical evidence-bundle bytes. |
| WAV-01 | Exact scoped weakening has an authorized issuer, audit reference, active interval, and expiry. | Accepted only inside scope and time. |
| WAV-02 | Waiver is expired, premature, overbroad, unauthorized, or targets a non-waivable rule. | Rejected, not ignored. |
| EXT-01 | A derived extension names a total pure BHCP lowering function, declares no native payload schema, and fully lowers to core IR. | Extension presentation disappears; core meaning is checked and hashed. Missing or invalid lowering is rejected. |
| EXT-02 | A supported native extension with a payload schema is present. | Must-understand node, rules, and identity remain in semantic IR; a native descriptor cannot provide a derived lowering. |
| EXT-03 | An unsupported native extension is present. | Artifact rejected before planning. |
| EXT-04 | An extension attempts to override core meaning or policy. | Descriptor/program rejected. |

`tests/governance_parser.rs` now makes the shared source-stage prerequisite for
POL/WAV/EXT and SYN-06/SYN-07 executable. The frozen policy, waiver, syntax, profile,
and extension sources parse into deterministic schema-valid canonical AST nodes;
syntax/profile projections and fully materialized waiver/extension definitions
round-trip through their typed wire boundaries; all six waiver changes, nested maps,
target order, authorization/delegation, content references, and exact field order
reject stably; and parser callbacks, unrestricted macros, semantic overrides,
expression-valued policy clauses, and invalid derived/native mixtures fail before
artifacts. Frozen exact-symbol artifact references remain deferred source bindings.
This remains parsing evidence for profile activation. Executable inline policy and
waiver lowering and extension execution are pinned separately below.

`tests/extension_lowering.rs` makes EXT-01 through EXT-04 and the extension path of
KRN-12 executable. It compiles the checked-in reference extension through a restricted
total-pure meta lowerer, validates and specializes the returned network/reducer, proves
lowerer identity and source order disappear after equivalent lowering, and rejects
missing lowerers, mixed modes, policy/core overrides, non-record calls, unsupported
native symbols, and payload-schema mismatches before later artifacts. Supported native
registrations retain sorted unique must-understand nodes whose exact descriptors and
deterministic payloads are inspectable, semantic-identity relevant, and retained in
schema-valid IR; tampered order, duplicate symbols, and optionalized received nodes
fail closed.

`tests/waiver_contract.rs` and `tests/waiver_application.rs` make the WAV-01/WAV-02
decision and application boundaries finite and executable. They pin exact
source-rule and weakening matching, subset/superset scopes, issuance and both
half-open time edges, direct and delegated issuer chains, authorization and audit
presence, non-waivable rules, all
six typed change categories, atomic rejection, deterministic identity/audit records,
strong root validation, and applied-waiver inspection. The checked-in waiver root
fixture uses the same closed CDDL shapes. The implementation rejects a narrower
partial scope when its complement cannot be represented exactly by the current v0
effective-rule shapes.

`tests/policy_waiver_lowering.rs` carries the same POL/WAV boundary from authored
source through composition, exact waiver application, policy-aware elaboration, and
semantic IR. It proves source/CBOR parity for all six weakening categories, direct
and delegated authority, injected half-open interval decisions, exact scopes and
audits, deterministic governance ordering, artifact-only presentation, effective
semantic identity, and atomic rejection of unresolved references or unrepresentable
partial scope changes.
