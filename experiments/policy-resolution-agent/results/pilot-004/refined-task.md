# Repair authorization rule selection

Task ID: `tenant-policy-resolution@0`

Production reports point to two related failures in `Policy::decide`: a request may
be influenced by rules owned by another tenant, and equivalent rule reloads can
produce a different answer. Repair the selector so policy authors get the
conservative, deterministic result they would reasonably expect from the existing
rule fields.

The data model is intentional. Enabled rules belong to one tenant; subject, action,
and resource patterns accept `*`; numeric priority expresses author intent; and an
explicit denial is meaningful. Resolve overlapping rules in a stable way that
respects those concepts. A request without an applicable rule remains denied.

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
