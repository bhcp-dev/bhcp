# Make policy resolution predictable

Task ID: `tenant-policy-resolution@0`

The authorization team reports that policy selection can feel surprising and may
occasionally cross tenant boundaries. Repair `Policy::decide` so it behaves like a
conservative multi-tenant policy engine.

An enabled rule may use `*` for a subject, action, or resource. Select the most
applicable matching rule for the requested tenant. Existing numeric priority must
remain meaningful, an explicit denial must not be casually bypassed by an equally
credible permission, and loading the same rules in another order must not change
the answer. Requests with no applicable rule remain denied.

Keep the solution unsurprising for policy authors and make deterministic choices
when several rules could apply.

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
