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
            HumanApproval(report = checked);
        };
        release = Publish(patch = move patch, approval = approved);
    };
}
```

The example says that implementation precedes independent checks, high-risk changes
require approval, and publication receives ownership of the patch. The planner may
parallelize the checks only when effects, state, and borrows do not conflict. The
source does not prescribe retry algorithms, model providers, prompts, or worker
topology.

## One meaning, several representations

BHCP keeps three often-confused layers separate:

- **Surface source** is a profile-selected human notation. Profiles may remap
  keywords, sigils, delimiters, aliases, and formatting.
- **Canonical AST** is the lossless, profile-independent result of parsing canonical
  tokens. It retains source-oriented structure and spans for diagnostics.
- **Semantic IR** is elaborated meaning: resolved symbols, inferred canonical types,
  effects, policy, obligations, and composition. It excludes spelling, formatting,
  delimiters, sigils, and source spans.

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

A run produces one of four outcomes: satisfied with output and evidence; refuted
with counter-evidence; indeterminate with a reason and partial evidence; or faulted
with an error and trace. Evidence may be formal, static, empirical, statistical,
model-judged, human-approved, or unresolved. A plausible output is not success.

## Version zero

v0 proves a coherent semantic foundation rather than a universal programming
system. Its required end-to-end scope includes:

- the full canonical type system: primitives; exact and machine numerics; records,
  tuples, variants, unions, intersections, collections, generics, refinements,
  nominal and structural identities; option, result, dynamic, never, goal, effect,
  evidence, resource, ownership, borrowing, and lifetime types;
- a small total pure expression calculus plus finite or verifier-backed
  quantification and namespaced, versioned domain predicates;
- goals, contracts, authority, budgets, preferences, verifiers, cases, recursive
  references, profiles, policies, waivers, and extensions;
- all six core combinators—`§all`, `§any`, `§none`, `§chain`, `§gate`, and
  `§latch`—including their output, state, evidence, and four-outcome behavior;
- a parser, formatter, type/effect/policy checker, semantic lowering pipeline,
  obligation and capability analysis, planner, runtime, evidence graph, Rust SDK,
  and CLI; and
- deterministic CBOR artifacts, semantic and artifact identities, conformance
  scenarios, and human-auditable diagnostics.

The first useful runtime remains a constrained coding-agent backend: it accepts an
issue and repository, enforces capabilities, captures changes, runs independent
verifiers, and returns a per-obligation evidence bundle. Existing languages and
agent frameworks should be able to emit or consume the IR before direct BHCP
authoring is common.

Full theorem proving, unrestricted macros and grammar plugins, comprehensive
temporal/reactive logic, and universal workflow synthesis are outside v0. They must
not be used to defer any core type or combinator semantics.

## Direction

BHCP is not a prompt language, model router, agent framework, visual workflow
builder, or replacement syntax for a conventional language. Models, prompts,
humans, compilers, caches, and deterministic code are execution backends. The core
project is a semantic programming platform: programs as meaning rather than text,
goals rather than prescribed workflows, capabilities rather than ambient authority,
and evidence rather than assertions of success.

The normative definition of this vision is [SEMANTICS.md](SEMANTICS.md); the v0 wire
contract is [`schemas/v0/`](schemas/v0/).
