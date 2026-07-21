# BHCP

Beyond Human-Centric Programming (BHCP) is a semantic programming model in which
people declare outcomes, authority, limits, and required evidence while machines
discover acceptable executions.

This repository defines the normative v0 foundation and focused executable slices.
The Rust implementation accepts canonical clause goals plus self-hosted `all`,
homogeneous-output `any`, `none`, typed `chain`, and unary `gate`, emits validated
canonical AST and semantic IR, dispatches registered verifier bindings, and emits
deterministic evidence bundles. It also validates and normalizes every v0 wire type,
checks exact and machine values, generic bounds, refinements, nominal/structural
subtyping, and explicit `Dynamic` boundaries, and materializes checked `§type`
definitions in semantic IR. Canonical artifacts use deterministic CBOR and
algorithm-tagged semantic or artifact identities. A shared safe-Rust graph boundary
now decodes, validates, normalizes, identifies, and inspects obligation, capability,
state, execution, and evidence graph documents. It is not yet a complete v0 front
end, planner, runtime, or SDK: source-expression forms beyond the currently parsed
slice, execution-graph construction, planner budget allocation, and execution remain
roadmap work. Deterministic obligation graph, capability graph, and state-analysis
graph construction are implemented as separate, exactly reconstructable artifacts.

## Start here

- [VISION.md](VISION.md) is the short, aspirational description of the project and
  the product direction.
- [SEMANTICS.md](SEMANTICS.md) is the normative v0 language and platform contract.
  Implementations claiming v0 conformance must follow it.
- [`schemas/v0/`](schemas/v0/) is the machine-readable CDDL form of every v0
  platform artifact. Deterministic CBOR is the canonical wire representation.
- [`schemas/v0/examples/`](schemas/v0/examples/) contains CBOR diagnostic examples
  for every root document type.
- [`conformance/v0/`](conformance/v0/) is the normative scenario catalog; implemented
  slices include executable deterministic fixtures, including the complete no-waiver
  layered-policy boundary.

Normative terms such as **MUST**, **SHOULD**, and **MAY** are interpreted as in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) and
[RFC 8174](https://www.rfc-editor.org/rfc/rfc8174) when capitalized.

## Schema validation

The Rust schema harness checks the CDDL root inventory, parses and validates all 17
diagnostic fixtures, and verifies deterministic-CBOR round trips:

```sh
cargo test --test schema_fixtures
```

See [`schemas/v0/README.md`](schemas/v0/README.md) for the artifact inventory,
canonical encoding rules, and validation details.

## Executable slice

Install the pinned Rust toolchain, then run the CLI through Cargo:

```sh
mise install
cargo run -- parse conformance/v0/fixtures/canonical-simple.bhcp > /tmp/canonical-simple.ast.cbor
cargo run -- lower conformance/v0/fixtures/canonical-simple.bhcp > /tmp/canonical-simple.ir.cbor
cargo run -- inspect conformance/v0/fixtures/canonical-simple.bhcp
cargo run -- inspect /tmp/canonical-simple.ir.cbor
cargo run -- hash conformance/v0/fixtures/canonical-simple.bhcp
cargo run -- format conformance/v0/fixtures/canonical-simple-presentation.bhcp
cargo run -- policy inspect conformance/v0/fixtures/canonical-policy.bhcp
cargo run -- policy compose conformance/v0/fixtures/canonical-policy.bhcp > /tmp/effective-policy.cbor
cargo run -- policy inspect /tmp/effective-policy.cbor
cargo test --test policy_conformance
```

`parse` and `lower` emit deterministic CBOR conforming to the existing CDDL
`canonical-ast` and `semantic-ir` roots. `inspect` accepts either canonical source
or one of those `.cbor` artifacts, validates the artifact boundary, and renders a
concise Rust-owned outline of typed goal interfaces, structural clause IDs, lowered
conditions and effects, preferences, and expanded verifier targets. That outline is
presentation only and does not participate in semantic identity. `hash` emits the
single algorithm-tagged semantic identity as text.

`format` validates and canonicalizes the selected source before applying the resolved
whitespace rules, writes source text to stdout, reparses it, and rejects any change to
the canonical token stream or AST shape as `BHCP9004`. Canonical source needs no
registry arguments. Custom source supplies its syntax, profile, and any source-policy
CBOR artifacts explicitly after the source path; registry file order is irrelevant.
The formatter emits an exact custom preamble, inverts the resolved token map, retains
comments, wraps only between tokens, and honors `indent_width`, `line_width`, and
`final_newline`. The canonical profile uses the fixed `{ 4, 100, true }` layout and
preserves an explicit canonical preamble or leading BOM when present.

`policy compose` accepts one or more explicitly ordered canonical policy source or
source-policy CBOR inputs. It requires organization → team → repository → user
order, rejects unsupported features and weakening atomically, validates the
effective policy root, and writes deterministic CBOR only after the complete
composition succeeds. `policy inspect` accepts policy source or CBOR and renders
source rules or the effective layers, rules, identities, and exact tightening
provenance without exposing raw conversational CBOR.

The implemented source boundary supports namespaced/versioned goals;
typed `§input` and `§output` facts; `§requires`, `§ensures`, and `§limit` Boolean
expressions; `§allows` and `§forbids` effect atoms; ranked `§prefer`; and `§verify`
bindings with optional explicit contract-label targets. Scalar literals, binding
references, parentheses, unary `!`/`-`, and the
checked Boolean, comparison, and `+` operators form the clause-expression subset.
Closed record field types, one top-level `§all`, `§any`, `§none`, `§chain`, or
`§gate` body, and equivalent explicit `§compose` source for the first four forms
are also executable. `any` currently requires homogeneous child output types and
exposes its winner as `{output: T, tag: Text}`. A `none` goal with no declared output
facts has canonical `Unit`. Each later `chain` child has exactly one typed input
bound by `value`, `move`, `borrow`, or `share` to the immediate predecessor's whole
output; the first child is input-free. A gate has one total pure `Bool` condition,
one child, inferred `Excluded | Included<T>` output, and typed child arguments bound
to parent input fields. A false condition accepts no child observation; a true one
requests its child and propagates its semantic or operational result. Other
composition children remain zero-argument goal calls. Nested compositions, project
functions, and constructs outside the executable slice are rejected during lowering
with a stable diagnostic rather than erased. Parsing separately accepts complete S7
`§type`, general `§function`, `§predicate`, and standalone `§refines` definitions,
including generic bounds, structural and algebraic types, handles, refinement
predicates, verifier arguments, and exact source spans. `§type` definitions and
nominal `§refines` edges now pass through the closed v0 value/type checker and enter
semantic IR with normalized types, deterministic IDs, generic-bound checks, and
semantic identity sensitivity. Refinement introduction evaluates the checked total
comparison/Boolean subset and binds its evidence to both the normalized predicate
and exact candidate value. General functions and predicates over the parsed
expression slice now resolve forward references through a closed definition graph,
infer bounded generic arguments, emit only deterministic concrete specializations,
and enter semantic IR in symbol order. Unused generic templates are still checked;
cycles, unresolved or ambient calls, inconsistent inference, bound failures, and
result mismatches fail before IR with `BHCP4301`. Exact integers cover the complete
deterministic-CBOR `int` domain, including unsigned values above `i64::MAX`.
Independently of that still-partial source lowering, the closed v0 wire expression
checker covers every S5 expression and pattern form. It validates the complete tree
before evaluation; constructs immutable records, tuples, variants, lists, canonical
sets, and generic-key maps; evaluates exact Integer/Rational/Decimal operations and
checked machine-integer operations; enforces exhaustive patterns and lexical
bindings; and makes selection, casts, division, and overflow fault explicitly.
Pure calls resolve only through retained acyclic checked definitions. Quantifiers
use a static finite list/set or an exact finite-domain witness attached to a closed
verifier binding; neither path can invoke an ambient callback.
Before executable IR is emitted, the ownership pass now materializes every handle
qualifier and checks `value`, `move`, `borrow`, and `share` arguments through direct,
nested, recursive, conditional, and concurrency-candidate goal flow. It permits
overlapping reads, rejects exclusive-write conflicts and every post-move path,
requires linear consumption on all outcomes, keeps case scopes independent, and
rejects expired-borrow or unapproved-shared state retention with stable
`BHCP4401`–`BHCP4405` diagnostics. Executable handle data edges retain the exact
canonical qualifiers and advertise `bhcp/feature.ownership-analysis@0`.
Before IR identity is computed, the effect pass materializes canonical possible-effect
rows from authored ceilings and child calls, projects direct resource arguments across
goal boundaries, accumulates prohibitions with deny-wins semantics, and rejects child
effects outside an explicit parent ceiling. Effective-policy capability and
prohibition scopes are checked against the referenced resource's nominal type and
literal operation coordinate at every goal boundary. Unsafe and foreign atoms retain
an `unresolved` evidence class; dimensioned limits require direct non-negative exact
upper bounds; and objectives at one preference priority require compatible types.
These checks advertise `bhcp/feature.effect-authority-analysis@0`. Limits and
preferences remain semantic clauses: allocation, retries, obligation nodes, conflict
edges, and execution-graph decisions belong to their separately assigned roadmap
stages and are not added to `kernel-network`.
Validated semantic IR can now be lowered to a separate deterministic capability
graph. The builder emits one request and one final allow decision for every retained
possible effect, structural resource coordinates, authored and propagated ceiling
grants, applicable effective-policy grants and denial context, exact source-rule and
waiver audit provenance, and required unsafe/foreign/unsupported gaps. Parent
propagation is an explicit edge. Denied or unresolved authority still fails before
IR, and exact graph validation rebuilds the document from the retained compilation;
planning cannot add or change a grant. Source clause IDs, policy decomposition,
positional effective-rule indices, and applied-waiver presentation affect artifact
audit identity without renaming structural authority nodes.
Because materialized effect rows change semantic identity, active experiment
`contract.semantic-id` pins were recomputed. Their completed-study identities remain
unchanged in adjacent `.pre-effect-analysis` sidecars, and the frozen audit and
preregistration manifests point explicitly to those historical sidecars rather than
rewriting past evidence.
Predicate verifier arguments lower in canonical name order to a closed typed input
record plus retained mode/expression configuration. Argument order is not semantic;
the verifier symbol, types, modes, expressions, evidence output, and trust
restrictions are. A verifier-only predicate remains a declarative evidence boundary,
not a callable host implementation.
The same parser boundary now accepts complete S7 goal headers and clause ordering:
generic/refinement declarations, all four fact kinds and initializers, invariants,
limits, authority, preferences, verifier arguments, executable cases, standalone
goal calls, quantified composition, and recursively nested finite composition.
These additional goal forms retain deterministic AST structure and fail closed
before executable IR until explicitly assigned finite-domain and case-execution
stages land. The practical v0 completion-manifest slice, including the frozen
reference program, now reaches governed semantic IR. Direct recursive child calls
require a matching positive static limit or a
checked decreasing integer measure whose retained requirements or execution guard
prove the child remains non-negative before IR emission. Direct self-recursive gates
are supported for the finite Unit-output base-case slice; a closed base branch accepts
no observation for its unselected recursive child.

| Canonical definition | Implemented source slice | Explicitly deferred |
| --- | --- | --- |
| `§goal` / `§function` | Complete goal and general-function parsing/AST construction; the practical v0 source-to-IR ledger and canonical/remapped reference program execute through governed IR; parsed pure functions support retained calls, selection, lists, exhaustive matching, bounded generics, and deterministic specialization; ownership/resource flow, bounded recursion, effects, authority, exact limits, and preferences are checked before IR | Generic-goal execution, case execution, standalone call statements, unexpanded finite/nested composition, mutual recursion, execution graphs, and planner allocation/retry decisions |
| `§type` / `§predicate` / `§refines` | Complete parsing plus checked type/refinement lowering and parsed predicate elaboration: every v0 wire type normalizes, local generics enforce arity/bounds, total refinements retain candidate-bound evidence, and canonical predicate verifier interfaces/configuration materialize in semantic IR | Canonical expression forms outside the practical completion ledger retain stable pre-IR diagnostics until explicitly assigned |
| `§policy` | Complete canonical source parsing for layer, `§extends`, six closed typed rules, scopes/parameters, waivability, and issuers; inline and explicit composition, inspection, policy-aware elaboration, and governed semantic IR | Expression-valued policy clauses and enforcement beyond the compile-time/evidence boundary |
| `§syntax` / `§profile` | Complete closed source-definition lowering into identified typed artifacts and one atomically validated source-local syntax/profile/policy registry; fixed byte-level selection, exact one-parent resolution, monotonic attached overlays, resolved-profile inspection, span-aware custom-source compilation, and deterministic profile-aware formatting | Arbitrary grammars, executable macros, parser plugins, implicit parents, fallback search, and per-definition profile switching remain outside v0 |
| `§waiver` / `§extension` | Complete closed source-definition parsing for all six typed waiver changes, canonical scopes/targets, authority/time fields, and wire-compatible derived/native descriptors. Fully materialized inline waivers apply through the exact waiver engine with an injected decision time. Derived extensions execute one concrete total pure BHCP lowerer and disappear into checked core IR; explicitly registered native extensions retain an exact-schema, must-understand node. | Resolving frozen symbolic waiver references remains governed. Native support is closed to exact host registrations; planning and execution do not interpret unknown native payloads. |
| Other S7 definitions | None | `§use` declarations |

The Phase 4 decision boundary admits only one-token keyword, punctuation, and symbol
alias mappings. Exact single-parent chains resolve root to leaf; effective surface
spellings must be unambiguous and prefix-safe; core symbols cannot be rebound; and
formatting can change only insignificant whitespace. Profile children may select only
the same or a descendant syntax, may strengthen but not relax type mode, and append
unique policy overlays in an auditable root-to-leaf order before ordinary monotonic
policy composition. These rules are specified and executable as finite decision
vectors. The closed profile registry now resolves syntax/profile/policy parent chains,
flattens coordinate overrides, composes attached overlays through the ordinary policy
engine, and passes one validated effective syntax into the same canonical parser.

Before lexing, every source entry point scans the original bytes for the fixed
`#!bhcp-profile namespace/name@version` ASCII preamble. An optional UTF-8 BOM may
precede it; omission selects `bhcp/canonical@0`; and the directive must use ASCII
spaces and an LF terminator. Duplicate, misplaced, aliased, malformed, non-ASCII,
and invalid UTF-8 inputs fail as `BHCP0003` without emitting an artifact. The
scanner preserves original byte, line, and column offsets and source hashing while
masking only the accepted preamble for canonical lexing. The executable
[`canonical-profile-preamble.bhcp`](conformance/v0/fixtures/canonical-profile-preamble.bhcp)
example demonstrates explicit canonical selection. Exact custom profile symbols
are selected profile-independently. Unregistered symbols fail closed as `BHCP0004`;
registered effective syntaxes validate completely as `BHCP9002` before scanning,
then lower NFC keyword, sigil, delimiter, terminator, and alias surfaces without
touching comments or literals. Mapped-away canonical spellings fail as `BHCP0005`.
The paired
[`profile-lowering-canonical.bhcp`](conformance/v0/fixtures/profile-lowering-canonical.bhcp)
and [`profile-lowering-words.bhcp`](conformance/v0/fixtures/profile-lowering-words.bhcp)
fixtures compile to the same semantic identity while retaining different profile and
source-span artifact data.

The checked-in [`profile-layout`](conformance/v0/profile-layout) corpus makes that
identity boundary concrete for two deliberately different human layouts. The compact
symbolic profile and the spaced narrative profile select different keyword,
delimiter, terminator, alias, indentation, width, comment, and diagnostic-label
presentations for one policy-governed goal. Both retain their selected profile and
distinct AST/IR artifact identities, yet resolve the same overlay and produce the
same semantic ID. Their deterministic formatter snapshots, CBOR root round trips,
policy-change control, and matched parser diagnostic prove the invariance and
sensitivity rules stated normatively in S9.1.3 of [`SEMANTICS.md`](SEMANTICS.md).
The [Phase 4 completion audit](conformance/v0/profile-phase-audit.md) maps all 27
acceptance claims from issues #41–#49 to named executable tests, checks every local
evidence link, and pins consistent maturity and non-goal language. This completes the
bounded presentation-layer milestone; it does not make the repository a complete v0
implementation or admit arbitrary grammars, parser plugins, or unrestricted macros.

The Rust `profile` model decodes and emits both S9.1 root artifacts through the
repository deterministic-CBOR codec. It covers every closed mapping category,
bounded formatting, exact optional parents, ordered unique profile overlays, all
four type modes, common headers, and artifact-ID validation. Mapping coordinates
have one deterministic category/canonical order; feature IDs remain an open
negotiation set rather than a hard-coded allowlist. Unknown or duplicate fields,
invalid symbols or self-parents, duplicate or out-of-order mappings, duplicate
local overlays, bad formatting bounds, and illegal modes fail as `BHCP9001`.
Generic root-fixture validation maps those failures to `BHCP5002`. The parser-side
effective-map validator adds coordinate vocabulary, NFC lexical safety, ambiguity,
prefix, alias, core-override, and token-capture checks before emitting canonical
tokens. `ProfileRegistry` resolves exact parents root to leaf independent of
registration order, requires descendant syntax and nondecreasing type mode, rejects
duplicate/missing overlays, follows policy parents, and invokes the existing
monotonic composer before elaboration. Registry topology failures use `BHCP9003`;
policy weakening retains its category-specific `BHCP8101`–`BHCP8107` diagnostic.
The profile-source lowering API accepts only canonical policy, syntax, and profile
definitions, materializes deterministic syntax/profile artifact IDs, registers all
documents, and validates every registered leaf before exposing the source-local
registry. Source-defined syntax may name one exact parent with `§extends`; any parse,
topology, overlay, or effective-map failure returns no partial registry.
`render_profile_resolution` exposes the resolved chains, overlays, type mode, and
effective-policy identity. The formatter consumes that resolved leaf, canonicalizes
custom input without touching comments or literals, lays out canonical tokens through
the leaf formatting record, and maps them back to the selected surface. Its output is
idempotent, and an internal canonical-token plus AST-shape round trip prevents
formatting from becoming semantic.

The adversarial profile harness rejects ambiguous aliases, alias recursion, canonical
keyword capture, reserved-core rebinding, parser callbacks, unrestricted macros, and
semantic override fields before an AST, IR, or formatted source can be returned.
Effective-map `BHCP9002` diagnostics name the selected profile, syntax artifact,
offending `category:canonical=>surface` mapping, stable one-based mapping index, and
violated rule. A mapped-away spelling keeps its original program source point under
`BHCP0005` with the same context. Typed parser/macro/semantic payloads remain unknown
artifact fields and fail the closed `BHCP9001` model.

`bhcp.hash/sha3-512@0` is the default and only currently registered identity
algorithm, implemented through the pinned pure-Rust `sha3` crate. It provides a roughly 256-bit
post-quantum preimage margin. [`bhcp-project.toml`](bhcp-project.toml) is the explicit
algorithm-agility boundary: projects may select another algorithm once the Rust
implementation registers it; unknown selections fail before parsing.

For effective policy documents, semantic identity hashes only the normalized
`effective` restriction value. Requirements, evidence, effects, limits, type mode,
waivability, and authorized issuers therefore change it. Source decomposition,
content-addressed layers, rule provenance, labels, comments, formatting, and source
enumeration do not. The latter retained audit inputs do enter artifact identity;
the artifact ID field itself is excluded. The Rust policy API exposes both
recomputations and validates materialized IDs against the same projections.

The crate uses the `cddl` 0.10.6 parser from cddl-rs to reject malformed RFC 8610
schemas, the pure-Rust RustCrypto `sha3` 0.12.0 crate for SHA3-512, and
`unicode-normalization` 0.1.25 for normative NFC surface checks. The BHCP compiler,
deterministic CBOR codec, and fixture validator remain repository-owned
safe Rust; the repository contains no project-owned C, Ruby, or Node.js tooling. Run
every local acceptance check with:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

The `Rust quality` GitHub Actions workflow runs the same semantic gates with Rust
`1.97.1` from `.mise.toml` on every pull request and every update to `main`.
Formatting, Clippy, release, and the focused schema invariant remain independent.
The all-target test gate optimizes only SHA3/Keccak in the test profile, validates
that every root integration target appears exactly once, runs library/binary and
integration partitions concurrently, and reports their combined result through the
stable protected `Rust quality / Tests` context. The five required contexts remain:

- `Rust quality / Format`
- `Rust quality / Clippy`
- `Rust quality / Tests`
- `Rust quality / Release build`
- `Rust quality / 17-root CDDL fixtures`

The final check runs the 17-root fixture invariant directly; the complete test
plan also includes it. The canonical local command remains `cargo test
--all-targets`. Workflow actions are commit-pinned, and Cargo registry and Git
dependency caches are keyed by the pinned toolchain and `Cargo.lock`.

`cargo run --bin generate-fixtures` regenerates the checked-in AST and IR CBOR
artifacts for the canonical simple-goal and self-hosted
`all`/`any`/`none`/`chain`/`gate` fixtures. The semantic model defines the minimal
`kernel-network`: total pure reducers return adjectival `Pending | Concluded`
reduction states over factored execution results.

[`prelude/v0/all.bhcp`](prelude/v0/all.bhcp),
[`prelude/v0/any.bhcp`](prelude/v0/any.bhcp),
[`prelude/v0/none.bhcp`](prelude/v0/none.bhcp),
[`prelude/v0/chain.bhcp`](prelude/v0/chain.bhcp),
[`prelude/v0/gate.bhcp`](prelude/v0/gate.bhcp), and
[`prelude/v0/recursive-gate.bhcp`](prelude/v0/recursive-gate.bhcp) are parsed and checked as canonical
BHCP source. Their compile-time lowerers construct ID-free network shapes through
the restricted metamodel, disappear, and leave monomorphized runtime reducers in
semantic IR. The generic Rust kernel implements only typed sealed-observation
queries, result construction, tag-to-child resolution, and derivation sealing; the
prelude source determines behavior precedence and selects aggregation operations.
The `all`, `any`, `none`, and `chain` convenience forms and their explicit
`§compose` equivalents produce byte-identical semantic IR and semantic IDs. Gate
conditions are specialized into the retained reducer definition and never become
network metadata. Runtime tests cover pending requests, product, stable tagged
winner, `Unit`, last-step satisfaction, closed/open gate selection, all four empty
identities, decisive verdicts, causal early stop, fault/unresolved precedence,
typed predecessor and parent-field edges, non-observation, and generic
re-evaluation rejection of tampering:

The current retention lowerer accepts only handle-free predecessor outputs. A
nested owned, borrowed, or shared resource handle fails before IR, and received IR
revalidates the same boundary. Consuming moves and policy-approved persistent shares
remain coupled to the state/CAS runtime work in #124; this slice does not silently
copy them or treat an edge-mode label as authority.

```sh
cargo test --test self_hosted_all
cargo test --test self_hosted_any
cargo test --test self_hosted_none
cargo test --test self_hosted_chain
cargo test --test self_hosted_gate
```

The [goal-algebra conformance manifest](conformance/v0/goal-algebra/manifest.txt)
indexes all five canonical sources, their explicit `§compose` equivalents where the
surface model exposes one, the ten checked-in AST/IR artifacts, and named executable
empty/adversarial proof evidence. `tests/goal_algebra_conformance.rs` recompiles and
schema-round-trips every entry byte for byte, checks the exact six-fixture generator
and 17-root inventories, and preserves that phase's feature-manifest boundary. The
later obligation builder does not rewrite those historical algebra artifacts.

The reducer evaluator now statically checks every branch before execution, supports
typed literals, Boolean negation/conjunction/disjunction, equality, and total
conditionals, and exposes a closed behavior-neutral API over sealed observations for
stable winners, sequential missing-tag demand, counter-evidence aggregation, unit,
and checked result construction. Unknown calls cannot become host callbacks, even in
an unreachable branch. Every satisfied conclusion is checked against the network
output type before the generic derivation checker can accept it.

The retained reducer calls a small, fixed typed API for sealed-observation queries
and checked result construction. General source-defined functions and predicates in
the practical S5 ledger now elaborate into semantic IR; canonical expression forms
outside that bounded ledger retain stable pre-IR diagnostics. Adding derived
behaviors must not introduce behavior-specific Rust primitives.

The obligation-graph proof checker re-evaluates the exact retained reducer and binds
every derivation premise to the matching observed child, structural dependency, and
accepted evidence or counter-evidence. It revalidates semantic-IR, graph, candidate,
payload, producer, decision-time, contract-expression, and policy-minimum boundaries;
rejects target, identity, premise, reason, and dependency substitution; and preserves
satisfied, refuted, unresolved, and faulted results without behavior-specific proof
tags. Every evidence claim, item, and gap carries its sealed execution-instance
identity; built-in expression evidence additionally carries its typed input/output
context for deterministic re-evaluation. Every observed child result must agree
with the instance-specific aggregate statuses of that child's structural
obligations. The exact retained reducer remains authoritative for child composition;
its result combines with parent-local obligation dispositions under
`Refuted > Faulted > Unresolved > Satisfied`, so the checker rejects a weaker reducer
result and no child-discharge label becomes a behavior-specific parent rule. The obligation
builder emits deterministic open nodes for requirements,
guarantees, invariants, limits, retained cases, verifier bindings, parent/child
discharge dependencies, and effective policy requirements/evidence/limits. Generated
clause IDs and labels are audit data, not structural identity; duplicate policy rules
retain every exact source coordinate. This closes the obligation-graph proof-checker
slice, not the still-deferred planner, executor, evidence-graph assembler, or complete
v0 runtime.

The registered verification foundation resolves explicit `§verify ... for "label"`
targets to structural obligation IDs, re-evaluates total contract expressions over
typed input/output values, dispatches only host-registered evidence producers, and
emits strongly validated deterministic evidence-bundle CBOR. Accepted, rejected,
unresolved, and operationally faulted verifier outcomes remain distinct. Missing
registrations produce required unresolved gaps; arbitrary project commands are not
silently executed. Evidence timestamps are injected, and the implemented boundary
accepts canonical UTC timestamps at second precision.

Applicable effective-policy evidence rules add deterministic structural obligations
to that same boundary. Hosts explicitly bind each policy obligation symbol to one or
more registered verifier symbols; registration and binding order are unobservable.
Accepted items must use a policy-approved class and matching predicate, and the
positive minimum counts distinct bound producers. Evidence bundles expose the
effective rule index and every originating layer, policy symbol, and source rule.
Missing mappings or registrations remain required unresolved gaps; rejection and
operational fault behavior is unchanged.

This library boundary builds deterministic obligation, capability, and state-analysis
graphs, but not execution graphs. Their common typed wire boundary rejects unknown fields,
duplicate IDs, dangling local references, and cycles in obligation/execution graphs;
normalizes semantic sets; validates materialized semantic/artifact identities; and
exposes nodes, edges, references, provenance, and validation errors without a JSON
canonical intermediate. Planning, runtime enforcement, and final evidence-graph
assembly remain separate. Process-backed adapters now register through the generic
verifier registry rather than becoming kernel primitives. Dispatch deterministically
encodes the closed typed candidate `{ input, output }`, passes only the binding's
resolved structural targets and an explicit effective effect ceiling, and maps the
result into the same evidence-bundle model as an in-process verifier.

### Project-local verifier adapters

Canonical BHCP source declares a verifier's symbol, typed input/output evidence,
trust restrictions, and obligation targets. A project's `bhcp-project.toml` may bind
that symbol to a narrower local process envelope without turning a contract string
into a command:

For predicate bindings, named source arguments are normalized before expression-ID
allocation. Semantic IR retains their closed input record and each argument's mode
and checked expression as configuration; changing source order alone cannot change
semantic identity, while changing any retained coordinate does.

```toml
[[verifier_adapter]]
symbol = "example/verifier.check@0"
executable = "target/verifiers/example"
argv = ["verify", "--input", "-"]
working_scope = "project"
input_media_type = "application/vnd.bhcp.verification-request+cbor"
output_media_type = "application/vnd.bhcp.verifier-result+cbor"
timeout_ms = 30000
allowed_effects = ["bhcp-effect/fs.read@0", "bhcp-effect/process@0"]
evidence_kind = "static"
```

Declarations are sorted by verifier symbol and effect sets are normalized. Every
field is required. Executables are lexical project-relative paths; shells, command
strings, absolute/parent paths, ambient network, unknown effects or keys, duplicate
fields or symbols, invalid media types, and unbounded timeouts fail closed with a
project-manifest diagnostic.

`VerifierProcessRunner` canonicalizes the project root and exact executable, rejects
symlink escape, captures the executable and registration artifacts, passes argv
directly with no shell or `PATH` lookup, clears the environment, and sends one
deterministic CBOR request on standard input. The closed response protocol preserves
accepted, rejected, unresolved, and faulted states. Input, stdout, stderr, executable
size, wall-clock time, and cancellation are bounded and have stable, distinguishable
outcomes. Every execution record retains the exact declaration, obligation targets,
request, response when present, executable artifact, and exit code.

Each request carries both the verification subject's content reference and its exact
bytes. The runner validates their size and every digest before launch, and a producer
must judge those bytes rather than substitute an ambient project file. Evidence claims
therefore cannot name one caller-supplied subject while a registered adapter examines
another.

The runner compares the executable's device, inode, mode, size, and nanosecond
modification identity before and after artifact capture and again immediately before
launch, so same-length replacement is detected. The portable native launch still
reopens that canonical path after the final comparison; deployments must prevent
concurrent mutation of registered executables. Descriptor-based execution is a future
hardening option, not a property claimed by this slice.

Native adapters run only behind the packaged `bhcp-adapter-sandbox`, which closes every
inherited descriptor above standard error before installing the fail-closed OS sandbox.
Linux requires Landlock ABI v4 with full enforcement and seccomp; filesystem
access is restricted to the exact executable, read-only platform runtime paths, and
the project root only when its read/write effects were declared. Socket and network
operations are denied. macOS additionally requires `/usr/bin/sandbox-exec`; it denies
network and all non-project writes, and withholds common user/data roots unless project
read was declared while retaining the read-only OS runtime surface needed to load the
binary.
An unsupported or unavailable sandbox is an execution failure, never a silent direct
launch. The local declaration is not a new CDDL artifact and does not change semantic
ID.

Evidence items produced by an adapter identify the captured executable as the verifier
artifact and the exact normalized adapter declaration as provenance source. The
verification report also retains every process audit record, including declaration,
targets, request/response references, executable reference, and exit code. The host
injects the canonical UTC production timestamp; adapter output cannot supply or
override it. Fixed candidates, content references, timestamps, registrations, and
adapter outputs therefore produce byte-identical bundles regardless of registry
insertion order.

Operationally, a missing executable, canonical path escape, malformed or oversized
output, stderr flooding, nonzero exit, timeout, and cancellation are different
conditions; sandbox setup also fails closed as a process fault. Timeouts and
cancellation are unresolved completion reasons; process/protocol failures are faults.
A `BHCP7001` diagnostic means the request or registration violated the boundary before
any adapter process was started.

The canonical project-registry entry point is
`bhcp verify <contract> <goal> <candidate-cbor> <subject-file> <produced-at>`.
It discovers the nearest project manifest from the contract path, compiles the
contract, validates the typed candidate, registers the declared adapters, and emits
the retained canonical evidence bundle on standard output. Exit 0 means accepted,
3 rejected, 4 unresolved, and 5 faulted. The manifest may narrow the contract's
effective effect ceiling but cannot authorize an effect the contract did not allow.

## Coding-agent experiments

[`experiments/CONTROLLER.md`](experiments/CONTROLLER.md) documents the implemented
safe-Rust experiment controller. It freezes input, model, reasoning, sandbox,
toolchain, limit, arm-order, executable, and judge identities before launch;
creates a fresh oracle-free workspace per arm; records bounded external metrics;
rejects interrupted, contaminated, adaptive-oracle, and incomplete sessions; then
copies the frozen oracle and judges every arm symmetrically. Repository-facing
reports are create-once Markdown rather than checked-in JSON event streams.

[`experiments/minimal-coding-agent/`](experiments/minimal-coding-agent/) contains a
pinned, dependency-free Rust repository for the first controlled coding-agent
comparison. Its visible tests pass while an independent oracle rejects partial batch
commits, conflicting idempotency replays, and overflow rollback. The Markdown task
and canonical BHCP contract state the same requirements; the experiment isolates
whether BHCP makes completion claims more precise and mechanically checkable, not
whether hidden requirements can surprise an agent.

The fixture documentation defines the two-arm protocol and verifier boundary. The
generic dispatcher, bounded process integration, evidence-bundle model, and canonical
project-registry CLI are executable. The controller supplies the fixed candidate,
subject artifact, timestamp, and execution-graph identity; the contract and project
manifest select the bounded Rust, oracle, and change-policy producers.

The explicitly invoked repo skill at
[`.codex/skills/interpret-bhcp-contract/`](.codex/skills/interpret-bhcp-contract/)
turns lowered contracts into implementation and evidence matrices without becoming
a second semantic authority. It verifies the pinned identity, keeps implementation
state separate from verifier acceptance, and fails closed on unsupported or
unresolved contract boundaries.

[`experiments/policy-resolution-agent/`](experiments/policy-resolution-agent/)
adds a deliberately ambiguous authorization ticket. Its canonical contract pins
tenant isolation and a specificity → priority → deny → rule-ID precedence ladder
that the prose describes only as "most applicable," "conservative," and
"deterministic." This second fixture measures intent disambiguation rather than an
equal-information syntax comparison, and its checked oracle exposes five distinct
reasonable-but-policy-invalid implementations while retaining two passing control
invariants.

[`experiments/contextual-policy-agent/`](experiments/contextual-policy-agent/)
extends that benchmark to an ordered resource → subject → action specificity
lattice with ten withheld invariants. Pilot 006 records accepted prose and raw-BHCP
patches, one compact-skill run that collapsed the lattice and failed two invariants,
and a latest-main skill follow-up that passed all ten with substantially higher
token intake. The paired skill outcomes make run variance and ordered-obligation
retention explicit rather than reporting only a favorable sample.

[`experiments/in-session-evidence-agent/`](experiments/in-session-evidence-agent/)
is a deliberately small forward fixture for the registered evidence path. Its public,
withheld-oracle, and exact change-policy adapters are exposed through the canonical
project registry while independent controller judges remain the acceptance authority.
The preregistered forward 001 run produced a valid 0/1 negative: the model made no
edit, invoked no registered adapter, claimed no success, and all three semantic judges
rejected the unchanged starter. The result is retained without an adaptive replacement;
the post-run latest skill now documents the canonical registry workflow.

The machine-checked [Phase 2 evidence audit](experiments/phase-2-evidence-audit.md)
classifies all six pilots, four multi-seed attempts, and forward 001 without
retroactive promotion. Early pilots remain historical artifact replays; multi-seed
runs 001/002 are invalid, run 003 is excluded, and the hardened registered results
are valid negatives of 0/5 and 0/1. Positive in-session acceptance remains unproven,
and the small fixture-bound record supports no BHCP-versus-prose advantage.
BHCP v0 is not complete; Phase 2 closes a bounded evidence-loop implementation
and audit, not the complete parser, checker, planner, runtime, proof system, SDK,
or CLI.

The separate [evidence-generalization preregistration](experiments/evidence-generalization/preregistration.md)
freezes a four-task registered-use frame and a three-task equal-information
prose-versus-contract comparison, single-model arms, a 36-session schedule,
descriptive analysis, exclusions, stopping rules, and a zero-incremental-spend
resource budget for issues #92 and #93. The completed
[positive-use study](experiments/evidence-generalization/positive-use-results/README.md)
retains all twelve registered sessions with no exclusions: positive registry use
was 0/12 and in-session acceptance was 0/12 (two-sided 95% Clopper–Pearson
0.0000–0.2646), while all twelve negative claims were calibrated. This null result
is unaffected by the report's disclosed frozen-classifier limitation because every
controller recorded zero completed commands and no session retained any evidence
bundle. It does not upgrade the Phase 2 record or establish model, developer, or
language-wide generalization. The separate completed
[prose-versus-contract study](experiments/evidence-generalization/comparative-results/README.md)
retains all 24 sessions and twelve pairs with no exclusions. Neither arm repaired
any starter: every candidate remained byte-unchanged, every controller recorded
zero completed commands, every arm passed the public boundary and failed only the
withheld oracle, and all 24 negative claims were calibrated. Consequently both the
acceptance and calibration paired risk differences are +0.0000, with zero
discordants and two-sided exact McNemar p=1.000000. This retained null is a failure
of both representations to induce a repair in these fixtures, not evidence of
equivalence or a general BHCP-versus-prose effect.

The v0 policy wire and restriction algebra are now specified in
[`SEMANTICS.md`](SEMANTICS.md#s92-monotonic-policy) and the CDDL bundle. Six closed
category/operation/value shapes replace the former unrestricted policy value, and a
canonical effective policy separates semantic restrictions from retained source
layers, exact rule provenance, and waiver audit material. Strongly typed Rust source
and effective document models now validate that wire boundary, deterministic order,
and semantic/artifact identities at external CBOR input. Canonical `§policy` source
now lowers through the same model: an explicit layer, optional `§extends`, stable rule
IDs, all six typed category/operation/value forms, waivability, issuer lists, scopes,
and canonical parameters are parsed with retained AST spans. Comments, formatting,
and optional human labels do not enter the policy document. Canonical `§waiver`,
`§syntax`, `§profile`, and `§extension` definitions now parse into closed ordered AST
forms; expression-valued policy clauses and executable governance payloads still fail
before artifacts. Validated source documents now compose in fixed organization → team →
repository → user order. The composer resolves same-layer inheritance, rejects
missing/cyclic/cross-layer parents and duplicate sources, checks every later
capability/limit/type-mode rule for weakening before joining, intersects capability
scopes, takes exact minima and strongest modes, retains deny rules, collapses exact
duplicates with restrictive waiver governance, and emits canonical source-layer and
rule provenance plus semantic/artifact identities. The policy CLI composes ordered
source or CBOR inputs into validated deterministic bytes and human inspection names
every source, effective rule, type mode, identity, and tightening provenance.

The executable extension stage resolves derived descriptors to a source-defined
`Meta<DerivedForm,I,O> -> Meta<NetworkShape,I,O>` function, runs it in the restricted
total-pure evaluator, specializes its reducer, and retains only the resulting checked
kernel goal. Lowerer names, descriptor presentation, and generated IDs do not survive
semantic projection. Native descriptors require an exact symbol/schema registration;
their descriptor and deterministic payload remain visible in inspection and semantic
identity as a sorted unique must-understand node. Unsupported, schema-mismatched,
mixed, policy-override, and reserved-core forms fail before graph construction.

The Rust policy-aware compilation API applies a validated effective document before
semantic IR emission. It rejects source type mode below the effective minimum,
prohibited or ungranted authority across the final propagated effect row, and an
explicit dimensioned numeric `§limit` above its effective maximum with stable
`BHCP8201`–`BHCP8204` diagnostics. Resource scopes resolve retained binding IDs to
their nominal resource types, while literal operation scopes must match directly.
Accepted IR retains the effective policy identities and per-goal indices for every
applicable requirement, evidence demand, prohibition, capability, and limit. Equivalent policy
decompositions keep the same program semantic ID while their retained artifact IDs
remain auditable. Applicable evidence demands are dispatched through explicit
policy-obligation-to-verifier registry bindings and retain source-layer provenance in
the evidence bundle. The manifest-driven `conformance/v0/policy` suite pins the full
four-layer no-waiver composite, equivalent and meaningful-change identities, every
weakening diagnostic, source/CBOR CLI parity, schema round trips, and the resulting
per-goal enforcement decision.

The waiver boundary is now typed and executable for exact representable scopes. A
waiver names exact source-rule targets and category-specific changes,
uses policy-scope subset matching, carries direct or acyclic delegated issuer
authority plus authorization and audit material, and is evaluated atomically at one
injected time over `[not_before, expires_at)`. The waiver itself is artifact-only;
the normalized post-waiver effective restriction determines semantic identity. The
finite boundary vectors and [threat model](THREAT_MODEL.md) pin invalid intervals,
scope amplification, substitution, broken delegation, missing audit/authorization,
and non-waivable downgrade as rejection. The safe-Rust application path covers all
six weakening categories, finite delegated authority, injected time, atomic
multi-target application, deterministic identities/audit records, strong waiver-root
validation, and applied-waiver inspection. Partial product-scope subtraction that
cannot be represented by the current v0 effective-rule shapes is rejected rather
than approximated; broader execution-time enforcement remains later work.

Executable canonical source may carry its policy and fully materialized waivers
inline. The compiler orders and composes source policies, sorts waiver application
deterministically, requires one explicitly injected decision time, and then uses the
same policy-aware elaboration path as a separately supplied effective document.
Source/CBOR parity covers all six weakening categories, exact scopes, delegated
authority, interval edges, audit retention, and fail-closed unresolved or partially
representable changes. Inline and separately supplied policies cannot be mixed.

Policy composition fails atomically with an auditable diagnostic: `BHCP8101`
capability widening, `BHCP8102` limit loosening, `BHCP8103` type-mode weakening,
`BHCP8104` requirement removal, `BHCP8105` evidence removal, `BHCP8106` allow over
deny, `BHCP8107` incompatible limit units, or `BHCP8110` invalid source topology.
Weakening and unit-conflict messages identify the attempted rule, earlier authority,
and need for a waiver; no effective policy is returned from an invalid layer.

```bhcp
§policy example/repository@0 §extends example/base@0 {
  layer repository;
  rule network-ceiling "deployment network": capability narrow {
    effect: bhcp-effect/network@0,
    scope: { operations: [example/operation.fetch@0] }
  } waivable by ["security-team"];
  rule strict-types: type-mode strengthen strict nonwaivable;
}
```
The trusted composition boundary is deliberately narrow. A network carries its
structural ID, output type, finite typed children, and reducer symbol—nothing else.
It carries no behavior kind, quantifier family, guard, dependency list, budget,
scheduling order, or parallelism hint. Quantifiers expand to finite children before
IR; recursive bounds belong to the recursive child call; and budget/concurrency
decisions live in execution graphs. Pending reducers name stable child tags, which the
kernel resolves through the network; reducers never allocate child or derivation IDs.
The versioned `lower-retain@0` / `retain-reducer@0` prelude boundary now derives
retained-value behavior as a causal state-read, candidate, and compare-and-swap
network. Refuted, unresolved, faulted, stale, and CAS-conflict outcomes stop before a
write; only the satisfied path reaches compare-and-swap. The kernel gains no
retention, retry, state, or freshness behavior kind. Persistent storage, state-graph
execution, and runtime retry enforcement remain later stages. The deterministic
state-graph builder projects validated semantic IR into explicit resource, ownership,
borrow/move, invariant, cell, freshness, and guarded atomic-transition dependencies.
Mutable retention transitions bind the exact capability decision, prior read/version,
satisfied candidate and evidence, CAS endpoint, parent/resource invariants, and
stale/conflict outcomes; this analysis graph grants no authority and performs no
storage mutation.

## Practical v0 completion contract

[`conformance/v0/completion-manifest.txt`](conformance/v0/completion-manifest.txt)
freezes the milestone-7 implementation boundary. Its machine-checked inventory maps
all 99 normative scenario instances (including each standard algebra's
satisfied/refuted/unresolved/faulted matrix), all 17 wire roots, ten end-to-end
pipeline outcomes, and the reference-program features to stable issue keys #99–#134.
The validator rejects missing, duplicate, and unknown records and checks the root
inventory against both the CDDL and diagnostic fixtures. It also runs the existing
policy parser/composer and typed syntax, profile, waiver, and extension validators;
checks canonical/alternate source equivalence and cross-artifact registry links;
binds extension rules by content identity and policy evidence demands to explicit
producers; and audits reviewed types, direct limits, decreasing recursion, exact
effect scopes, ownership/data flow, obligations, and operational outcomes.

The nontrivial
[`conformance/v0/reference-program/`](conformance/v0/reference-program/) subject
crosses source, checking, governance, graph, planning, execution, evidence, SDK, CLI,
and conformance boundaries. Its canonical and substantially remapped sources now
compile with the derived extension to identical governed semantic IR while retaining
distinct AST artifact identities. End-to-end success remains a later claim: the same
frozen subject must pass through the public Rust SDK and CLI with deterministic
per-obligation evidence.

## Contributing and autonomous delivery

[AGENTS.md](AGENTS.md) is the canonical repository operating contract for humans
and autonomous workers. It defines authority order, meaningful TDD, the local Rust
gate, one-issue/one-PR delivery, native dependencies, atomic remote-ref claims,
independent review, fenced release, and post-merge consistency.

The versioned [project-loop profile](.codex/project-profile.md) provides the exact
`bhcp-dev/bhcp` adapter details used by reusable roadmap automation: issue queries,
readiness labels, lock namespaces, concurrency tokens, CI check names, review rules,
and completion criteria. Labels and comments mirror claims; they are never locks.

## Status

The executable slice is not a claim that the execution platform already exists. v0
is complete only when the parser, checker, planner, runtime, evidence machinery,
SDK, and CLI implement the complete type system, minimal kernel, proof checker, and
standard self-hosted prelude end-to-end, all records in the machine-checked
completion manifest are demonstrable, and the reference program passes the audited
pipeline. The checked-in contract defines that future claim; it does not make it
true today.
