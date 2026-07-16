# Beyond Human-Centric Programming

> Humans describe outcomes. Machines discover execution. Compilers verify the result.

Programming languages have historically been designed around one assumption: the
primary programmer is human. Their syntax, abstractions, tooling, and workflows
optimize for people writing, reading, debugging, reviewing, and maintaining
implementation code.

That assumption is beginning to weaken.

AI systems are becoming substantial authors of software. Humans increasingly define
goals, constraints, architecture, permissions, and acceptance criteria while machines
produce, modify, test, and maintain implementations. Yet those machines still work
through representations designed primarily for human cognition.

BHCP explores a programming model for what comes next: **beyond human-centric
programming**.

The objective is not to remove humans from software development. It is to stop
treating human source-code ergonomics as the primary constraint of the programming
system. Humans should remain in control of intent, policy, and judgment. Machines
should be free to discover implementations within those boundaries. Compilers and
verifiers should decide whether the result is acceptable.

## The thesis

Source code should not be the canonical representation of a program.

The canonical program should be a semantic object that describes:

- goals, inputs, and outputs;
- preconditions, postconditions, and invariants;
- permissions, prohibitions, and available resources;
- tests, verification requirements, and acceptable evidence;
- hard limits and optimization preferences; and
- provenance, uncertainty, and replay information.

Source code then becomes one human-oriented projection of that object. An execution
plan becomes another. A test suite, policy report, audit trail, or visual graph may be
another.

This shift separates what a program means from how a person writes it and how a
machine executes it.

> **Syntax is negotiable. Semantics are not.**

## The primary abstraction: a goal

The foundational abstraction is not a prompt, agent, function, task, or workflow. It
is a **goal**: a desired state together with the conditions under which that state
counts as achieved.

For example:

```text
§goal ImplementIssue {
    §input repository: Repository;
    §input issue: Issue;
    §output patch: Patch;

    §requires repository.clean;
    §requires issue.acceptance_criteria.count > 0;

    §ensures patch.addresses(issue.acceptance_criteria);
    §ensures repository.tests.all_pass;
    §ensures repository.typecheck.passes;

    §allows filesystem.read repository;
    §allows filesystem.write repository;
    §allows process.run TestRunner;

    §forbids network;
    §forbids repository.history.rewrite;

    §prefer minimal_diff;
    §prefer maintainability;

    §limit changed_files <= 12;
    §limit attempts <= 8;

    §verify ExistingTests;
    §verify GeneratedAcceptanceTests;
    §verify TypeChecker;
}
```

This program does not prescribe a sequence of steps. A planner may inspect the
repository, derive tests, implement a patch, run independent verifiers, repair
failures, and validate the final diff. Another planner may find a better route. Both
are valid only if they respect the declared capabilities and produce enough evidence
to satisfy every obligation.

This is the same separation that makes SQL powerful: the author declares the result,
while the system chooses scans, joins, indexes, and execution order. BHCP applies that
idea to semantic computation and software construction.

## A semantic programming platform

The envisioned system has several deliberately separate layers:

```text
Human source
    ↓
Syntax profile and macro expansion
    ↓
Canonical typed AST
    ↓
Canonical semantic IR
    ↓
Obligation and capability graph
    ↓
Execution planning
    ↓
Executable graph
    ↓
Runtime execution
    ↓
Evidence graph
```

The **semantic IR** is the stable interface of the platform. It represents program
meaning independently of surface syntax, model provider, agent framework,
implementation language, and execution backend. It should support deterministic
serialization, semantic hashing, optimization, caching, provenance, replay, and
interoperability.

The **obligation and capability graph** describes what must be true, what execution
may do, what it must never do, which resources are available, and which evidence can
establish success.

The **execution graph** is discovered by a planner. Its nodes may include
deterministic code, compilers, static analyzers, test runners, local or remote models,
tools, retrieval systems, databases, humans, approval gates, caches, and sandboxes.

The **evidence graph** is part of the result, not an afterthought. It records verifier
outputs, provenance, traces, costs, timing, resource use, replay metadata, unresolved
uncertainty, and the relationships between evidence and obligations. A plausible
answer without sufficient evidence does not satisfy the program.

## Four semantic categories

The core model can remain small by organizing programs around four categories:

1. **Facts** describe what exists: types, inputs, outputs, resources, and context.
2. **Obligations** describe what must be true: requirements, guarantees, limits,
   cases, and verification requirements.
3. **Permissions** describe what execution may and may not do through explicit
   capabilities.
4. **Preferences** rank valid plans without weakening correctness or policy.

Hard constraints and soft preferences must remain distinct. A cheaper or faster plan
is never preferable if it violates an obligation.

## Custom syntax, canonical meaning

Different people, teams, domains, and organizations should be able to use different
human-facing syntax without fragmenting program meaning. A syntax profile may change
keywords, sigils, delimiters, formatting, aliases, or constrained macro forms. Every
profile must lower into the same typed AST and semantic IR.

Two programs that differ only in surface syntax should have the same semantic hash.
A change to an obligation, permission, output, or evidence requirement must change
it.

Customization ends at the semantic boundary. A profile may rename a postcondition,
but it cannot redefine what a postcondition means. It may introduce a concise form
for read-only access, but it cannot weaken capability enforcement. Deterministic
parsing, reproducible builds, portable tooling, and secure compilation take priority
over unrestricted syntax extension.

This also creates a path toward executable organizational standards. A repository
could ship versioned syntax, culture, and policy profiles that enforce engineering,
security, testing, documentation, privacy, and review requirements at compile time.
Lower-level profiles may refine presentation and strengthen protected policy, but not
silently weaken it.

## Effects and evidence are first-class

Agentic software makes two existing weaknesses of conventional programming more
urgent.

First, effects are often implicit. A tool can read secrets, mutate files, call a
network service, or rewrite repository history unless an external sandbox happens to
stop it. In BHCP, capabilities belong in the program. An execution path requiring an
undeclared effect is invalid.

Second, success is often self-reported. A model says it completed a task, while tests,
policy checks, or independent validation live outside the programming model. In BHCP,
verification is explicit. `ensures` states what must be true; `verify` identifies an
accepted evidence-producing procedure. The verifier contributes evidence—it does not
define correctness.

Evidence may be formally proven, mechanically verified, empirically tested,
statistically supported, model-judged, human-approved, or unresolved. The system
should represent those distinctions rather than collapse them into a boolean.

## Models, prompts, and agents are backends

BHCP is not a prompt language or an agent framework. Prompts, models, and agents are
possible execution strategies, not core semantic primitives.

A goal might be satisfied by deterministic code, a cache hit, a compiler, retrieval,
a model, a human, or a combination of them. The semantic program should avoid
prescribing a mechanism when it only cares about the outcome. Providers and runtimes
can then improve without requiring the program's meaning to change.

## Design principles

1. Meaning must not depend on formatting.
2. Syntax customization must not permit semantic redefinition.
3. Every accepted program must have a canonical representation.
4. Every executable effect must be declared.
5. Every successful goal must satisfy explicit postconditions.
6. Verification evidence must be first-class.
7. Soft preferences must remain distinct from hard constraints.
8. Models, prompts, and agents are backends, not core semantics.
9. A planner may discover procedure, but may not escape policy.
10. Programs must remain human-auditable even when machines author most of the
    implementation.
11. Deterministic tooling takes priority over syntactic cleverness.
12. The first system must solve one real workflow before attempting universal
    computation.

## What BHCP is not

BHCP is not primarily:

- another AI SDK or model router;
- another agent framework;
- a prompt or workflow language;
- a replacement syntax for TypeScript;
- a visual agent builder; or
- a coding-assistant interface.

Those may become integrations or implementation components. The core project is a
semantic programming platform with customizable human-facing syntax, a canonical IR,
goal-oriented execution planning, enforceable capabilities, and evidence-producing
verification.

## Adoption: runtime value before a new language

The language should not be the first adoption wedge. Developers rarely adopt a new
language because its design is elegant; they adopt systems that solve painful
problems.

The path begins with existing TypeScript, Python, Rust, and agent frameworks emitting
semantic IR indirectly. Developers gain execution tracing, reproducibility, caching,
verification, model portability, cost controls, capability enforcement, and evidence
bundles without learning new syntax.

As the IR becomes useful, developers can inspect it like a SQL query plan, compiler
IR, Terraform plan, or build graph. Direct authoring follows when a semantic goal is
clearer than its host-language equivalent. Organizational profiles come later, once
the semantic foundation is stable.

## Version zero

Version zero should prove the semantic model, not attempt to build the entire future
at once. Its required scope is:

- a fixed canonical syntax and small set of semantic primitives;
- goals, inputs, outputs, resources, requirements, guarantees, capabilities, limits,
  preferences, verifiers, and example cases;
- a canonical typed AST and semantic IR;
- deterministic serialization and semantic hashing;
- a simple planner and execution runtime;
- an evidence bundle and per-obligation satisfaction report;
- a TypeScript SDK; and
- CLI tools to parse, check, format, lower, inspect, run, and verify programs.

Keyword remapping, basic profile inheritance, and a visual graph inspector are
valuable next steps. Arbitrary grammar plugins, full theorem proving, unrestricted
macros, and universal workflow synthesis are explicitly deferred.

## The first prototype: coding-agent execution

The fastest credible prototype is a constrained coding-agent runtime.

Given a repository, issue, and acceptance criteria, the semantic goal requires an
implementation that changes only relevant files, adds appropriate tests, keeps the
existing suite passing, satisfies type checking and linting, stays within explicit
resource and iteration limits, and uses only declared capabilities.

An existing coding agent may serve as the first execution backend. BHCP instruments
its actions, captures mutations, runs independent verifiers, validates diff scope,
and emits an evidence graph containing the patch, test and type-check results, policy
violations, cost, timing, trace, and a satisfaction result for every obligation.

This prototype would be useful before a sophisticated planner or customizable
language exists. It would demonstrate the central claim: an agent's work becomes more
reliable when intent, authority, and proof of completion are part of the program
itself.

## Open questions

This project begins with research questions rather than pretending they are solved:

- How expressive can the IR become before reliable analysis and planning break down?
- How should conflicting obligations, inconclusive evidence, and partial success be
  represented?
- How can probabilistic planners be trusted without granting them authority to evade
  hard policy?
- Which parts of goal decomposition belong in source, and which should be synthesized?
- How should organization, team, repository, and user policies compose?
- How should semantic versions evolve while preserving old programs?
- What is a reusable library in a goal-oriented system: a goal, verifier, policy,
  capability, proof rule, execution template, or all of these?
- How should human judgment appear as a typed verifier rather than an informal
  external process?

## The invitation

Software is moving toward a world where humans design intent and machines increasingly
author implementation. We need programming systems that make that division explicit,
safe, inspectable, and verifiable.

BHCP is an exploration of that future: programs as meaning rather than text; goals
rather than prescribed workflows; capabilities rather than ambient authority; and
evidence rather than assertions of success.

The programming language is only one frontend. The durable artifact is the semantic
program.

> **Beyond human-centric programming.**
>
> Humans describe outcomes.  
> Machines discover execution.  
> Compilers verify the result.
