# Beyond Human-Centric Programming

> Humans describe outcomes. Machines discover execution. Compilers verify the
> result.

Programming languages assume that a human is the primary author and maintainer of
implementation code. As machines become substantial software authors, that
assumption becomes an unnecessary constraint. BHCP explores a programming model in
which humans retain control of intent, policy, and judgment while machines are free
to discover implementations inside explicit boundaries.

The canonical program is not source text. It is a typed semantic object describing
goals, acceptable state transitions, authority, limits, preferences, and the
evidence required to establish a result. Human source, execution plans, audit
reports, and visual graphs are projections of that object.

> **Syntax is negotiable. Semantics are not.**

## Goals, not procedures

The primary abstraction is `Goal<I, O>`: a typed relation between an input, one or
more acceptable outputs, permitted state transitions and effects, and sufficient
evidence. It has a familiar function-like boundary without implying a deterministic
function or a unique result.

```text
§goal ImplementIssue {
    §input repository: owned Repository;
    §input issue: Issue;
    §output patch: Patch;

    §requires "clean-worktree": repository.clean;
    §ensures "acceptance": patch.addresses(issue.acceptanceCriteria);
    §invariant repository.history.preserved;

    §allows fs.read(repository), fs.write(repository), process.run(TestRunner);
    §forbids network, repository.history.rewrite;
    §limit changedFiles <= 12;
    §prefer 1: minimalDiff(patch);

    §verify "tests": with ExistingTests;
    §verify "types": with TypeChecker;
}
```

A planner may inspect, edit, test, repair, or use a cache. None of those strategies
defines the goal. An execution is acceptable only when it stays within authority and
produces evidence that discharges every obligation.

Goals compose without turning plans into source:

```text
§goal ShipChange {
    §input request: ChangeRequest;
    §output release: ReleaseReceipt;

    §chain {
        patch = ImplementIssue(issue = request.issue);
        checked = §all {
            tests = RunTests(patch = patch);
            security = ScanPatch(patch = borrow patch);
            policy = CheckPolicy(patch = borrow patch);
        };
        approved = §gate when request.risk == High {
            approval = HumanApproval(report = checked);
        };
        release = Publish(patch = move patch, approval = approved);
    };
}
```

The example says that implementation precedes independent checks, high-risk changes
require approval, and publication receives ownership of the patch. The planner may
parallelize the checks only when effects, state, and borrows do not conflict. The
source does not prescribe retry algorithms, model providers, prompts, or worker
topology. These readable forms are prelude syntax: after lowering, no `chain`, `all`,
or `gate` node remains. Their checked BHCP lowerers consume a source-independent
compile-time shape and produce the same minimal network IR that an explicit core
composition would produce.

That standard algebra is now published as one independently checkable conformance
set. Its manifest binds `all`, `any`, `none`, `chain`, and `gate` to canonical source,
deterministic AST/IR bytes, empty or invalid boundaries, and generic proof-checker
tamper evidence. The accompanying feature manifest distinguishes those implemented
forms from the completed obligation/capability/state-analysis graph builders and
their cross-graph consistency audit, while execution-graph, planning, and runtime
stages remain distinct.

## One meaning, several representations

BHCP keeps three often-confused layers separate:

- **Surface source** is a profile-selected human notation. Profiles may remap
  keywords, sigils, delimiters, aliases, and formatting.
- **Canonical AST** is the structurally canonical result of parsing normalized
  canonical tokens. Its artifact retains the selected profile plus source-oriented
  structure and spans for diagnostics; those presentation inputs do not enter
  semantic meaning.
- **Semantic IR** is elaborated meaning: resolved symbols, inferred canonical types,
  effects, policy, obligations, minimal kernel networks, and their versioned reducer
  functions. It excludes spelling, formatting, delimiters, sigils, and source spans.

The complete platform pipeline is:

```text
profile-selected source
  → normalized canonical tokens
  → canonical AST
  → type/effect/policy elaboration
  → semantic IR
  → obligation/capability/state graph
  → execution graph
  → evidence graph
```

The semantic IR is the stable interoperability boundary. Deterministic encoding and
normalization make equivalent programs share a semantic identity even when their
surface profiles differ. Complete artifacts retain their own provenance-sensitive
identity.

The current Rust slice reaches policy-aware elaboration: a validated effective
policy can gate type mode, authority, prohibitions, and explicit dimensioned numeric
limits before semantic IR exists. Accepted IR retains its governing policy and
normalized decisions. Canonical possible-effect rows now preserve child effects,
direct typed-resource projection, parent ceilings, accumulated prohibitions, and
unsafe/foreign evidence gaps; direct exact limits and compatible Pareto preference
groups are checked without turning planner allocations or retries into kernel
metadata. The same boundary now has a closed v0 value/type checker:
canonical forms and exact numeric representations normalize without host-float
conversion; generic bounds, nominal/structural subtyping, refinements with explicit
candidate-bound evidence, recursive checked `Dynamic` crossings, and goal variance
are enforced; exact integers cover deterministic CBOR's complete integer domain;
and authored type definitions enter deterministic semantic IR. The practical S4-S9
front-end ledger and canonical/remapped reference program now reach governed semantic
IR, including retained calls, list construction, selection, and exhaustive matching.
Additional parsed S7 execution forms retain stable pre-IR diagnostics. The three
analysis graphs now rebuild and correlate through one exact compilation boundary;
execution-graph, planning, storage/CAS runtime, and final evidence work stay distinct
roadmap stages. The implemented
waiver path validates and applies exact typed
targets and changes across all six categories, scope containment, direct/delegated
issuer authority, injected half-open time validity, audit material, atomic rejection,
and identity effects. Executable source composes materialized inline policies and
applies materialized inline waivers through that same path before governed semantic
IR, with source presentation remaining artifact-only. A common safe-Rust graph model
now validates, deterministically encodes, identifies, and inspects every graph root
while preserving the distinction between semantic meaning and provenance-sensitive
packaging. Unrepresentable partial product-scope subtraction and graph construction
and planning remain separate roadmap work.

The presentation front end now has fixed raw-byte profile selection, typed
deterministic syntax/profile artifacts, and a closed registry that validates an
effective syntax map before lowering custom source to canonical tokens. The registry
resolves exact syntax/profile/policy parent chains root to leaf, enforces descendant
syntax and monotonic type mode, composes attached overlays through the ordinary policy
engine, and makes that resolution inspectable before elaboration. Lowering remains
NFC-aware and span-preserving without a profile callback. The formatter now renders
canonical or custom source through the resolved bounded layout, retains comments,
maps canonical tokens back to the selected surface, and proves an idempotent exact-token
round trip before returning bytes. The adversarial corpus now proves ambiguous aliases,
recursive mappings, core capture/rebinding, executable parser or macro payloads, and
semantic overrides fail atomically with auditable mapping diagnostics. The checked-in
symbolic and narrative layouts now compile one governed goal through distinct profile
and artifact identities to the same semantic identity, with formatting, comments,
labels, overlays, CBOR, and diagnostics pinned at their intended boundaries. The
[Phase 4 audit](conformance/v0/profile-phase-audit.md) links every presentation-layer
acceptance claim to executable evidence and reconciles the maturity boundary across
the repository. The bounded presentation milestone is complete; an unregistered
custom profile still fails closed, and the current Rust slice remains short of a
complete v0 system.

## Effects, policy, and evidence

Ambient authority and self-reported success are unacceptable foundations for
agentic software. BHCP therefore makes effects and evidence part of the program.
Purity is an empty effect row. Filesystem, network, process, state, nondeterminism,
divergence, unsafe operations, and foreign execution are explicit. Unsafe behavior
requires a policy-controlled capability and leaves a visible evidence gap.

Policy forms a ceiling around planning. Organization, team, repository, and user
layers compose monotonically: local layers may strengthen obligations, limits,
strictness, evidence rules, and prohibitions. A weakening requires an authorized,
scoped, expiring, auditable waiver.

A run either completes with a semantic verdict or faults operationally. Completed
verdicts are satisfied with output and evidence, refuted with counter-evidence, or
unresolved with a reason and partial evidence. A fault carries an error and trace but
makes no truth claim about the goal. Evidence may be formal, static, empirical,
statistical, model-judged, human-approved, or unresolved. A plausible output is not
success.

The completed Phase 2 slice implements and audits the bounded adapter, evidence, and
coding-agent controller path. Its hardened registered model results are valid
negatives, so positive in-session acceptance remains unproven and the current record
supports no BHCP-versus-prose advantage.
BHCP v0 is not complete; these measured limits are part of the evidence boundary,
not a claim that the broader runtime or a population effect already exists.

The evidence-generalization program is preregistered over four checked-in Rust
fixtures for registered use and the three representation-comparable fixtures for
the symmetric prose-versus-contract study. Its completed positive-use arm found
0/12 positive registry use and 0/12 in-session acceptance with no exclusions and
correctly negative claims in every session. The comparative arm completed all
twelve pairs with no exclusions: both representations left every starter unchanged,
failed only the withheld oracle, and made calibrated negative claims. Acceptance
and calibration therefore each have paired risk difference +0.0000, zero
discordants, and two-sided exact McNemar p=1.000000. This is not an equivalence
result or a BHCP-versus-prose effect claim; neither completed arm turns the frozen
repository frame into a population or model-wide claim.

## Version zero

v0 proves a coherent semantic foundation rather than a universal programming
system. Its required end-to-end scope includes:

- the full canonical type system: primitives; exact and machine numerics; records,
  tuples, variants, unions, intersections, collections, generics, refinements,
  nominal and structural identities; option, result, dynamic, never, goal, effect,
  evidence, resource, ownership, borrowing, and lifetime types;
- a small total pure expression calculus plus finite or verifier-backed
  quantification and namespaced, versioned functions and domain predicates;
- goals, contracts, authority, budgets, preferences, verifiers, cases, recursive
  references, profiles, policies, waivers, and extensions;
- a minimal execution-result-aware network kernel and generic proof checker, with `§all`, `§any`,
  `§none`, `§chain`, `§gate`, persistent retention, and future orchestration behavior
  defined by a versioned self-hosted BHCP prelude;
- a parser, formatter, type/effect/policy checker, semantic lowering pipeline,
  obligation and capability analysis, planner, runtime, evidence graph, Rust SDK,
  and CLI; and
- deterministic CBOR artifacts, semantic and artifact identities, conformance
  scenarios, and human-auditable diagnostics.

The machine-checked
[`conformance/v0/completion-manifest.txt`](conformance/v0/completion-manifest.txt)
turns this scope into a closed implementation inventory: 99 normative scenario
instances, 17 wire roots, ten observable pipeline stages, and one nontrivial
reference program are assigned to stable milestone-7 issue keys. The manifest and
reference program are a planned acceptance contract. Its isolated derived extension
now lowers through checked core IR and the native boundary is closed to exact
must-understand registrations; current fail-closed rejection of all other unsupported
definitions remains correct until their assigned implementations land.

The first useful runtime remains a constrained coding-agent backend: it accepts an
issue and repository, enforces capabilities, captures changes, runs independent
verifiers, and returns a per-obligation evidence bundle. Existing languages and
agent frameworks should be able to emit or consume the IR before direct BHCP
authoring is common.

Full theorem proving, unrestricted macros and grammar plugins, comprehensive
temporal/reactive logic, and universal workflow synthesis are outside v0. They must
not be used to defer the kernel, proof rules, or standard prelude semantics.

## Direction

BHCP is not a prompt language, model router, agent framework, visual workflow
builder, or replacement syntax for a conventional language. Models, prompts,
humans, compilers, caches, and deterministic code are execution backends. The core
project is a semantic programming platform: programs as meaning rather than text,
goals rather than prescribed workflows, capabilities rather than ambient authority,
and evidence rather than assertions of success.

The normative definition of this vision is [SEMANTICS.md](SEMANTICS.md); the v0 wire
contract is [`schemas/v0/`](schemas/v0/).
