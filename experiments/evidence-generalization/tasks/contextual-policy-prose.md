# Prose treatment: contextual policy resolution

Repair `Policy::decide` in the dependency-free `contextual-policy` crate. Edit only
`src/lib.rs`, preserve the public API, add no dependency or unsafe code, and do not
use the network.

The implementation must satisfy this complete decision ladder and obligation inventory:

- `tenant-isolation`: only rules whose tenant exactly equals the request tenant are
  eligible; a wildcard or foreign tenant is not eligible.
- `default-deny`: no eligible rule returns denial with no selected rule.
- `resource-before-subject`: an exact resource match outranks a wildcard resource,
  regardless of subject, action, or priority.
- `subject-before-action`: with equal resource specificity, an exact subject match
  outranks a wildcard subject.
- `action-before-priority`: with equal resource and subject specificity, an exact
  action match outranks a wildcard action.
- `priority-before-deny`: among rules with the same resource/subject/action shape,
  the greater numeric priority wins before the effect is considered.
- `deny-before-rule-id`: among rules tied through priority, denial wins.
- `stable-rule-id`: a remaining tie selects the lexicographically smaller rule ID.
- `insertion-order-independence`: loading the same rules in another order cannot
  change the decision.
- `disabled-rule-exclusion`: a disabled rule is never eligible.

Run Rustfmt, Clippy with warnings denied, and all visible tests. Report success only
when those checks pass; an independent withheld oracle and one-file policy judge run
after the session.
