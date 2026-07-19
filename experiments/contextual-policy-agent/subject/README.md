# contextual-policy

`contextual-policy` is a dependency-free selector for a multi-tenant authorization
service. Rules contain tenant, subject, action, and resource fields, a numeric
priority, an effect, and an enabled flag. Subject, action, and resource accept `*`
as a wildcard.

The public API is `Policy`, `Rule`, `Effect`, and `Decision`.

