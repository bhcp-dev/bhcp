# BHCP v0 threat model

Status: active security boundary for the implemented repository slices and the
normative v0 contract. This document records abuse cases; `SEMANTICS.md` remains the
normative authority.

## Protected assets and trust boundaries

BHCP protects declared intent, effective policy restrictions, authority, artifact
identity, and acceptance evidence. Canonical source and deterministic artifacts may
cross untrusted storage or tooling. Profile normalization, policy composition,
waiver validation, planning, adapter execution, and evidence acceptance are separate
boundaries and must fail closed without returning partial later-stage artifacts.

The host supplies registered profiles, policy documents, authorization evidence,
verifier adapters, and an injected decision time. Those inputs are not trusted merely
because they came from the host: their exact typed shapes, identities, scope, and
authority must validate before they affect meaning.

## Waiver threats

- **scope amplification:** an attacker requests universe scope, a superset goal,
  resource, or operation set, or a broader `to` value than the exact authorized
  change. Validation requires the application scope to be a subset of both the
  source restriction and the typed change; wildcards and inferred changes reject.
- **rule or change substitution:** an attacker reuses a valid signature for another
  policy/rule pair, category, value, or weakening. Targets bind the exact
  `(source-policy-symbol, rule-id)`, category-specific `from`/`to` or removed value,
  and canonical scope into the signed artifact identity.
- **delegation-chain confusion:** an attacker splices chains, changes their root or
  final issuer, repeats a principal, or supplies a disconnected link. Chains must
  begin in every affected rule's authorized-issuer intersection, connect exactly,
  end at the document issuer, remain acyclic, and carry authorization evidence for
  every hop.
- **clock rollback:** an attacker reuses an earlier successful validation after
  expiry or lets different targets observe different times. The caller injects one
  normalized decision time per atomic application; no ambient clock or cached
  validity result is authoritative. The interval is half-open, so expiry equality
  rejects.
- **audit-reference substitution:** an attacker swaps the justification, audit
  record, signature, or delegation evidence while preserving the requested policy
  change. All such metadata enters waiver artifact identity and must validate before
  application; the effective artifact retains the waiver reference and decision
  time.
- **non-waivable downgrade:** an attacker presents stronger external authority for a
  rule whose governance is non-waivable or whose contributing issuer intersection is
  empty. Non-waivable status is final; delegation cannot create waiver authority.
- **partial acceptance:** an attacker mixes valid and invalid targets so a validator
  silently applies the valid subset. Validation and policy application are atomic;
  one invalid target produces no effective artifact.

## Other implemented boundaries

- Presentation profiles are closed typed data, not parser callbacks. Ambiguous or
  recursive aliases, core capture, executable macros, semantic overrides, and
  unregistered profiles reject before artifacts escape.
- Policy layers may only restrict. Widening, loosening, removal, allow-over-deny,
  type weakening, incompatible units, and invalid topology reject a whole layer.
- Process verifiers run only through exact project-relative registrations and a
  capability-bounded sandbox with no shell, `PATH`, ambient network, or inherited
  descriptor authority. Missing enforcement fails before launch.
- Reducer conclusions are re-evaluated by the generic checker against sealed child
  observations and typed outputs. Behavior-specific proof tags are not trusted.
- Ownership transfer is checked before semantic IR. An attacker cannot disguise an
  owned copy as `value`, move a shared or borrowed handle, grant write through a read
  handle, reuse a possibly moved binding after a branch join, or schedule an
  exclusive access beside another candidate. Diagnostics bind the resource,
  lifetime, branches, and both source locations.
- Persistent state cannot capture a borrow. A shared capture requires an exact
  goal/binding/lifetime approval supplied to the ownership boundary; naming a
  lifetime `persistent` does not create authority. Invalid ownership is atomic and
  produces no later artifact.
- Effects cannot disappear at a goal-call boundary. Canonical possible-effect rows
  propagate child atoms, substitute direct resource bindings, accumulate denies, and
  reject explicit parent-ceiling excess before semantic IR is emitted. Effective
  capability/prohibition scopes match nominal resource types and literal operations;
  unsafe or foreign atoms keep an unresolved evidence gap. Limits reject indirect,
  negative, or inexact maxima, and a same-priority preference group cannot mix
  incompatible objective types.

## Residual and deferred risks

The repository is not a complete v0 parser, graph analyzer, planner, runtime, proof
system, or execution graph. Effect rows are conservative declarations, not proof that
an implementation cannot perform undeclared host effects; runtime capability
enforcement remains required. Complete obligation-graph construction, signature
policy and key distribution, revocation services, durable clock attestation, full CDDL instance
interpretation, and scoped waiver application are deferred implementation work. A
deployment must not treat a normative artifact shape or decision vector as proof
that those runtime services already exist.
