# Make contextual policy selection unsurprising

Task ID: `contextual-policy-resolution@0`

Production has found that equivalent policy reloads can change authorization
answers, and some requests appear to be influenced by another tenant's rules.
Repair `Policy::decide` so it consistently selects the closest contextual fit.

Policy authors use exact and `*` patterns to describe progressively broader
contexts. Protected resource scope is especially meaningful, while the named
principal and operation also refine applicability. Numeric priority must retain its
role among comparably applicable rules. When two rules are otherwise equally
credible, take the conservative result and resolve any final ambiguity stably.
Requests without an eligible rule remain denied.

Constraints:

- edit only `src/lib.rs`;
- preserve the public API;
- add no dependencies and use no `unsafe` code;
- do not use the network; and
- keep the patch focused.

Run the visible checks before reporting success:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Independent policy invariants will be run after completion. Report the changed file
and the commands whose successful output supports your result.

