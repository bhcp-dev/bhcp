# Practical v0 analysis-graph completion audit

This report closes the bounded analysis-graph outcome in issue #118. It proves that
one validated governed `Compilation` deterministically reconstructs mutually
consistent obligation, capability, and state graphs. It does not construct an
execution graph, choose a plan, allocate a budget, perform storage or compare-and-
swap execution, enforce runtime capabilities, or assemble final evidence.

## Executable inventory

`completion-manifest.txt` contains 57 `graph` rows. They are the graph-applicable
projections of the normative CBOR/identity, ownership, effect, kernel/algebra,
recursion, policy/waiver, retention/state-analysis, and PLN-05 scenarios. Each row
names an exact Rust test, one or more of the three implemented graph roots, stable
diagnostic categories, and the frozen reference identity file. RET-01 and STA-01/02
cover only their static retention, authority, freshness, and atomic-transition
analysis; their competing-writer, retry, persistence, and result behavior remains
owned by the runtime roadmap.

The governed canonical and substantially remapped reference sources compile to the
same semantic IR and then produce the exact identities and deterministic-CBOR hashes
in `reference-program/graph-identities.txt`. Their AST artifacts remain distinct.
The identity file is recomputed by `tests/graph_completion.rs`; it is not trusted as
a self-reported success record.

## Checked correlation matrix

| Boundary | Executable check |
| --- | --- |
| Common input | Revalidate the typed IR, retained bytes, semantic/artifact identities, effective policy, waiver decisions, and then rebuild all three graphs from that one exact compilation before correlating any received graph. |
| Obligation/checker | Match nodes by structural clause/policy meaning and exact goal set rather than array position; retain the generic proof checker's exact transitive `depends-on` closure for every reported goal; reject missing, substituted, unknown-goal, or identity-rematerialized obligations. |
| Effects/decisions | Require a bijection from every execution-eligible IR effect to exactly one request and one final `allow` decision with the exact goal, typed resource, operation, and opaque parameters, plus the exact request edge. |
| Typed resources | Resolve capability and state resources through the full IR source coordinate—goal, binding/source clause, name, and canonical type bytes. Raw graph-local node-ID equality is never used as resource equivalence. |
| State authority | Require every authority node to name a capability node of kind `decision`, retain the identical effect/goal/operation, and join its state resource to the capability resource at the full typed coordinate. |
| Atomic transition | Require exact cell/read/candidate/CAS roles, pre/post versions, candidate and evidence edges, every CAS authority, all invariants, freshness, and atomicity. Existing IDs of the wrong graph type fail like dangling IDs. |
| Features | Derive the feature set from the three emitted graph headers and require exactly the obligation-, capability-, and state-builder identifiers. Planner, execution, storage/runtime, and final-evidence identifiers reject. |
| Reconstruction | After correlation, require each received graph and the complete set to equal deterministic reconstruction. Recomputing valid semantic/artifact IDs cannot bless a substitution. |

## Stable diagnostics

- `BHCP7501` — the compilation or materialized graph-set input is invalid.
- `BHCP7502` — a graph does not match the exact semantic-IR reconstruction.
- `BHCP7503` — a graph advertises a hidden or unsupported feature.
- `BHCP7504` — a cross-graph reference is dangling or names the wrong node type.
- `BHCP7505` — effect requests and capability decisions are not an exact bijection.
- `BHCP7506` — structural obligations or checker goal coordinates are incomplete.
- `BHCP7507` — state resources, authority, transitions, versions, or invariants disagree.

The adversarial suite removes obligations, changes effects, adds an execution feature,
substitutes dangling IDs, substitutes IDs that exist only as requests or resources,
substitutes a different valid decision, changes a transition version, and
rematerializes graph identities after every mutation. All fail before a consistency
report is returned.

## Identity and maturity boundary

Graph semantic identity excludes presentation and audit-only source decomposition as
defined by the graph model. Artifact identity retains source clauses, policy/waiver
provenance, and other auditable packaging. Existing builder tests cover reordered
presentation and policy decomposition controls; this audit composes those controls
with exact reference parity and three-graph reconstruction.

The output is analysis-only. The next stages remain the planner request/refusal
boundary, conflict and budget/retry planning, deterministic execution-graph
construction, capability-bounded execution, atomic state/CAS runtime, execution
lifecycle, evidence-graph assembly, runtime policy enforcement, SDK/CLI completion,
reference execution, security audit, and final certification.
