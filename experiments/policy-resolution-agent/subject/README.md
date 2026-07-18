# tenant-policy

`tenant-policy` is a dependency-free rule selector for a multi-tenant service.
Rules contain tenant, subject, action, and resource fields, a numeric priority, an
effect, and an enabled flag. Subject, action, and resource fields accept `*` as a
wildcard.

The public API is `Policy`, `Rule`, `Effect`, and `Decision`.
