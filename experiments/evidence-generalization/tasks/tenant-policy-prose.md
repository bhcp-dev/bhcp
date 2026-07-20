# Prose treatment: tenant policy resolution

Repair `Policy::decide` in the dependency-free `tenant-policy` crate. Edit only
`src/lib.rs`, preserve the public API, add no dependency or unsafe code, and do not
use the network. Also keep the patch focused.

The implementation must satisfy this complete decision rule and obligation inventory:

- `tenant-isolation`: only enabled rules whose tenant exactly equals the request
  tenant are eligible; subject, action, and resource may use `*` as a wildcard.
- `default-deny`: no eligible rule returns denial with no selected rule.
- `specificity-before-priority`: maximize the number of exact matching subject,
  action, and resource fields before considering numeric priority.
- `priority-before-deny`: among equally specific rules, the greater numeric priority
  wins before the effect is considered.
- `deny-before-rule-id`: among rules tied on specificity and priority, denial wins.
- `stable-rule-id`: a remaining tie selects the lexicographically smaller rule ID.
- `insertion-order-independence`: loading the same rules in another order cannot
  change the decision.

Run Rustfmt, Clippy with warnings denied, and all visible tests. Report success only
when those checks pass; an independent withheld oracle and one-file policy judge run
after the session.
