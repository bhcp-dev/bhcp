# BHCP v0 Semantics

Status: **Normative v0 contract**

Schema namespace: `bhcp/v0`

Canonical media type: deterministic CBOR

This document defines the behavior that a conforming BHCP v0 parser, checker,
planner, runtime, or SDK MUST implement. Capitalized requirement words have the
meaning defined by RFC 2119 and RFC 8174. Examples are informative unless explicitly
introduced as a rule.

## S1. Conformance and scope

A v0 implementation MUST declare its implemented features in a
`feature-manifest-document`. It MUST reject an artifact containing a required core
feature, native extension, or version it does not understand. It MUST NOT silently
erase semantics.

The normative processing pipeline is:

```text
profile-selected source → normalized canonical tokens → canonical AST
→ type/effect/policy elaboration → semantic IR
→ obligation/capability/state graph → execution graph → evidence graph
```

The canonical AST preserves source structure and source spans for diagnostics. The
semantic IR contains only resolved, elaborated meaning. It MUST NOT contain source
spans, formatting, comments, delimiters, sigils, keyword spellings, aliases, or
profile presentation. An implementation MAY retain those in a side table keyed by
IR reference ID.

Full theorem proving, unrestricted macros or grammar plugins, comprehensive
temporal/reactive logic, and universal workflow synthesis are outside v0. The full
v0 type system and all six core combinators are not optional.

Schema anchors: `feature-manifest-document`, `canonical-ast-document`,
`semantic-ir-document`, and all graph documents.

## S2. Goals and run outcomes

`Goal<I, O>` is a typed relation, not necessarily a mathematical function. Given an
input of `I`, it relates zero or more acceptable outputs of `O` to allowed state
transitions, an effect row, and evidence sufficient to discharge its obligations.
It does not require termination, determinism, uniqueness, or a particular plan.

A completed run has exactly one outcome:

| Outcome | Meaning |
| --- | --- |
| `Satisfied(output, evidence)` | The output has type `O`; all obligations are discharged by accepted, fresh evidence. |
| `Refuted(counterEvidence)` | Accepted counter-evidence proves that the goal cannot be satisfied for this run/input under the stated interpretation. |
| `Indeterminate(reason, partialEvidence)` | Neither satisfaction nor refutation is established; this includes exhaustion, missing evidence, and timeout. |
| `Faulted(error, trace)` | Evaluation or infrastructure violated its operational contract; the fault is not evidence of the goal's truth or falsity. |

Cancellation is an indeterminate reason unless cancellation itself causes a declared
fault. A timeout, crash, failed attempt, or absent verifier result MUST NOT be treated
as counter-evidence. Runtime outcomes conform to `runtime-outcome-document`.

## S3. Symbols, identities, labels, and references

Definitions use globally unique semantic names of the form
`namespace/name@version`. A domain predicate MUST have such an identity and a typed
signature. It MAY have a pure canonical definition, a verifier binding, or both. If
both exist, disagreement is a verifier fault and MUST be visible.

Source clauses MAY have human labels. Labels are lexical references and diagnostics;
they MUST NOT affect semantic identity. A label does affect meaning when it is also
an observable record field, variant tag, branch tag, or output name. Lowering MUST
assign every definition, clause, expression, type, and composition node a unique
structural `ref-id`. References use those IDs; CBOR cycles are forbidden.

Alpha-renaming of non-observable local binders is semantic-identity preserving.
Resolved symbol identities, observable names, and branch tags are not alpha-renamed.

Schema anchors: `symbol-id`, `ref-id`, `clause`, `binding`, and `reference`.

## S4. Types and checking profiles

### S4.1 Canonical type profile

The default is **infer then strict**: omitted local types are inferred, then the
fully elaborated program is checked without implicit dynamic conversions. A profile
selects one of:

- `strict`: no implicit `Dynamic`; every operation is statically justified.
- `infer-strict`: inference is allowed, then strict rules apply. This is canonical.
- `gradual`: explicit or boundary-introduced `Dynamic` is permitted; checks and
  possible faults are inserted at typed boundaries.
- `dynamic`: values may remain `Dynamic`; capability, policy, ownership, effect, and
  evidence checks still apply.

A goal MAY strengthen the profile mode (`dynamic` → `gradual` → `infer-strict` →
`strict`). Relaxing it requires policy permission or a valid waiver. Mode changes
MUST be represented in semantic IR.

### S4.2 Type forms

v0 has the following canonical type forms:

- primitives: `Bool`, `Text`, `Bytes`, `Unit`, `Timestamp`, and `Duration`;
- unbounded exact `Integer`, reduced `Rational<n,d>` (`d > 0`), and fixed-scale
  `Decimal<coefficient,scale>`;
- machine integers with signedness and width, and machine floats identified by
  `binary16`, `binary32`, `binary64`, or `binary128` plus their exact bit string;
- closed or open records, positional tuples, tagged variants, normalized unions and
  intersections, lists, sets, and maps;
- type parameters and applications, bounded generics, and refinements
  `{x: T | p(x)}` where `p` is total and pure;
- nominal identities keyed by resolved symbol and structural types keyed by shape;
- `Option<T> = None | Some(T)` and `Result<T,E> = Ok(T) | Err(E)`;
- `Dynamic`, whose use never suppresses runtime checks, and uninhabited `Never`;
- `Goal<I,O,effects,evidence>`, effect-row types, and evidence types;
- resources, state cells, and owned/shared/borrowed handles with access and lifetime.

Union and intersection members MUST be flattened, deduplicated, and sorted by their
normalized semantic encodings. A union containing a supertype removes subsumed
members. An intersection containing `Never` is `Never`. Record field names, tuple
positions, variant tags, nominal symbols, refinement predicates, numeric formats,
effects, ownership, access, and lifetimes are semantic.

Ordinary null or missing values MUST lower to `Option<T>`; core BHCP has no ambient
`null`. A foreign profile that distinguishes absence states MUST lower them to an
explicit tagged variant such as `Absent | Null | Undefined | Present(T)`.

Machine floats are never represented by a host-language float in semantic artifacts.
A float value is `(format, bits)`, preserving signed zero, infinities, and NaN payloads.
Exact rationals and decimals use their explicit canonical components.

### S4.3 Subtyping and identity

Structural records support width subtyping only when openness and mutability permit
it. Function-like goal inputs are contravariant, outputs covariant, and effect and
evidence requirements invariant unless an explicit proof rule establishes a safe
ordering. Nominal types are compatible only through declared `§refines` edges.
Refinement introduction requires proof of the predicate; elimination yields the base
type. `Never` is a subtype of every type; every type is a subtype of `Dynamic` only at
an explicit gradual/dynamic boundary.

### S4.4 Immutability, resources, and ownership

Values are immutable. Identity and mutation occur only through typed resources or
state cells. A handle has:

- mode: `owned`, `shared`, or `borrowed`;
- access: `read` or `write`;
- a resource type and region/lifetime; and
- usage: unrestricted, affine (at most once), or linear (exactly once).

An owned handle may be moved, ending the old binding. A shared handle cannot grant
write access. A read borrow may coexist with read borrows; a write borrow is
exclusive. No borrow may outlive its region or cross a goal boundary unless the goal
type declares the lifetime. Linear and affine obligations MUST be checked across all
outcomes and composition branches. Latch storage captures an owned value or a
policy-approved persistent share; it MUST NOT retain an expired borrow.

Omitted source qualifiers elaborate deterministically: `owned` means
`owned write unrestricted` in the goal's lexical lifetime; `shared` means
`shared read unrestricted`; a borrowed handle MUST state `read` or `write`; and an
omitted usage mode is `unrestricted`. Semantic IR always carries every qualifier.

### S4.5 Language mental model

| Familiar concept | Core lowering |
| --- | --- |
| nullable / missing | `Option<T>` or an explicit foreign-absence variant |
| exceptions | `Result<T,E>` for expected failure; `Faulted` for operational contract failure |
| Rust-like ownership | owned/shared/borrowed resource handles and lifetime constraints |
| garbage-collected object | shared resource identity; immutable records for pure values |
| object/class | nominal record plus namespaced predicates/goals; no implicit mutable fields |
| algebraic functional value | records, tuples, variants, pattern matching, generics, refinements |
| logic relation | goal relation and finite/verifier-backed quantification |
| actor | typed resource with message effects and state transitions |
| promise/async | goal output scheduled by a planner; ordering only when semantically declared |
| dynamic-language value | `Dynamic` plus explicit boundary checks and unchanged policy/effect rules |

Schema anchors: `type-definition`, `type`, `exact-number`, `machine-float-value`,
`value`, `resource-type`, `handle-type`, and `type-mode`.

## S5. Expressions and predicates

The expression calculus is small, deterministic, total, and pure. It includes:

- primitive and exact numeric literals, immutable records, tuples, variants, lists,
  sets, and maps;
- field/index selection and construction;
- exhaustive pattern matching;
- exact arithmetic, comparisons, Boolean logic, and typed collection operations;
- calls to pure definitions or domain predicates; and
- `forall`/`exists` over a statically finite domain or through a verifier binding that
  returns accepted evidence of the quantified claim.

Expression evaluation MUST terminate. Division by zero, overflow of an explicitly
machine-sized operation, invalid indexing, non-exhaustive matching, or failed dynamic
casts are typed results where declared, otherwise faults. There is no general
recursion, I/O, mutation, nondeterminism, foreign execution, unsafe operation, or
hidden divergence inside an expression. Well-founded recursion belongs to goal
composition (S8).

Predicate purity is determined from an empty effect row, not from a `pure` assertion.
An optional canonical predicate definition is an expression. A verifier binding
names an evidence producer with typed input/output and trust requirements; it does
not redefine the predicate.

Schema anchors: `expression`, `pattern`, `predicate-definition`, and
`verifier-binding`.

## S6. Effects and authority

An effect row is a normalized set of effect atoms, optionally with a row variable.
The empty row is pure. v0 atoms include filesystem read/write, network, process,
clock, randomness, state read/write, actor send/receive, foreign execution,
divergence, unsafe, and namespaced extensions. Each atom may carry a typed resource
scope.

`§allows` grants a capability ceiling; it does not require use. `§forbids` denies a
capability. Deny wins at every nesting and policy layer. Every execution node MUST
declare effects whose capabilities are granted and not forbidden. Effects inferred
from children are preserved in the parent row.

`unsafe` and unverified foreign execution require a policy-controlled capability.
They MUST add an evidence gap describing what could not be established; a goal with
an unresolved required gap cannot be `Satisfied`.

Schema anchors: `effect-row`, `effect`, `authority-clause`, `capability`, and
`capability-graph-document`.

## S7. Canonical language

Canonical source uses the `§` sigil, braces for blocks, semicolons as terminators,
UTF-8 NFC text, and flat typed clauses inside goals. The reserved vocabulary is:

| Role | Keywords |
| --- | --- |
| definitions | `§type`, `§predicate`, `§goal`, `§use`, `§refines` |
| facts | `§input`, `§output`, `§resource`, `§state` |
| contracts | `§requires`, `§ensures`, `§invariant`, `§limit` |
| authority | `§allows`, `§forbids` |
| optimization/evidence | `§prefer`, `§verify`, `§case` |
| composition | `§all`, `§any`, `§none`, `§chain`, `§gate`, `§latch` |
| meta/policy | `§syntax`, `§profile`, `§policy`, `§waiver`, `§extension`, `§extends` |

There is no generic `constraint` or `test` keyword. A precise contract clause lowers
to an obligation. `§verify` binds evidence producers. `§case` declares an executable
scenario and never defines correctness.

### S7.1 EBNF

The EBNF below is complete for canonical v0 syntax. Lexers MUST use longest match;
whitespace and comments separate tokens but are otherwise insignificant.

```ebnf
program         = { use-decl | definition } ;
use-decl        = "§use" qualified-name [ "as" identifier ] ";" ;
definition      = type-def | predicate-def | goal-def | syntax-def | profile-def
                | policy-def | waiver-def | extension-def | refines-decl ;
type-def        = "§type" qualified-name [ type-params ] "=" type ";" ;
predicate-def   = "§predicate" qualified-name [ type-params ] "(" [ parameters ] ")"
                  ":" type [ "=" expression ] [ verifier-binding ] ";" ;
goal-def        = "§goal" qualified-name [ type-params ] [ "§refines" type-ref ]
                  goal-block ;
syntax-def      = "§syntax" qualified-name meta-block ;
profile-def     = "§profile" qualified-name [ "§extends" qualified-name ] meta-block ;
policy-def      = "§policy" qualified-name [ "§extends" qualified-name ] policy-block ;
waiver-def      = "§waiver" qualified-name policy-block ;
extension-def   = "§extension" qualified-name ( "derived" | "native" ) meta-block ;
refines-decl    = "§refines" type-ref type-ref ";" ;

goal-block      = "{" { goal-clause } "}" ;
goal-clause     = fact-clause | contract-clause | authority-clause | prefer-clause
                | verify-clause | case-clause | composition | goal-call-stmt ;
fact-clause     = fact-key [ label ] identifier ":" [ handle-mode ] type
                  [ "=" expression ] ";" ;
fact-key        = "§input" | "§output" | "§resource" | "§state" ;
contract-clause = contract-key [ label ] expression ";" ;
contract-key    = "§requires" | "§ensures" | "§invariant" | "§limit" ;
authority-clause= ( "§allows" | "§forbids" ) [ label ] effect-list ";" ;
prefer-clause   = "§prefer" [ integer ":" ] [ label ] expression ";" ;
verify-clause   = "§verify" [ label ] verifier-binding ";" ;
case-clause     = "§case" [ label ] "{" { binding | outcome-expectation } "}" ";" ;
outcome-expectation = "expect" outcome-tag [ expression ] ";" ;
composition     = all-expr ";" | any-expr ";" | none-expr ";" | chain-expr ";"
                | gate-expr ";" | latch-expr ";" ;

all-expr        = "§all" [ quantifier ] composition-block ;
any-expr        = "§any" [ quantifier ] composition-block ;
none-expr       = "§none" [ quantifier ] composition-block ;
chain-expr      = "§chain" composition-block ;
gate-expr       = "§gate" "when" expression composition-block ;
latch-expr      = "§latch" identifier [ ":" type ] latch-options composition-block ;
composition-block = "{" { branch } "}" ;
branch          = identifier "=" ( goal-call | composition-no-term ) ";" ;
composition-no-term = all-expr | any-expr | none-expr | chain-expr | gate-expr
                | latch-expr ;
quantifier      = ( "forall" | "exists" ) identifier "in" expression ;
latch-options   = [ "fresh" expression ] [ "initial" expression ] ;
goal-call-stmt  = [ identifier "=" ] goal-call ";" ;
goal-call       = type-ref "(" [ arguments ] ")" ;
arguments       = argument { "," argument } ;
argument        = identifier "=" [ "move" | "borrow" | "share" ] expression ;

policy-block    = "{" { policy-clause | authority-clause | contract-clause } "}" ;
policy-clause   = identifier [ label ] ( expression | meta-value ) ";" ;
meta-block      = "{" { identifier [ label ] meta-value ";" } "}" ;
meta-value      = literal | qualified-name | "[" [ meta-value { "," meta-value } ] "]"
                | "{" [ identifier ":" meta-value { "," identifier ":" meta-value } ] "}" ;
verifier-binding= "with" qualified-name [ "(" [ arguments ] ")" ] ;
binding         = identifier "=" expression ";" ;
label           = string ":" ;
effect-list     = effect-expr { "," effect-expr } ;
effect-expr     = qualified-name [ "(" [ expression { "," expression } ] ")" ] ;

expression      = let-expr | if-expr | match-expr | quant-expr | logic-or ;
let-expr        = "let" identifier [ ":" type ] "=" expression "in" expression ;
if-expr         = "if" expression "then" expression "else" expression ;
match-expr      = "match" expression "{" match-arm { match-arm } "}" ;
match-arm       = pattern [ "if" expression ] "=>" expression ";" ;
quant-expr      = ( "forall" | "exists" ) identifier "in" expression ":" expression ;
logic-or        = logic-and { "||" logic-and } ;
logic-and       = equality { "&&" equality } ;
equality        = relation { ( "==" | "!=" ) relation } ;
relation        = additive { ( "<" | "<=" | ">" | ">=" | "in" ) additive } ;
additive        = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative  = unary { ( "*" | "/" | "%" ) unary } ;
unary           = [ "!" | "-" ] postfix ;
postfix         = primary { "." identifier | "[" expression "]" | call-suffix } ;
call-suffix     = "(" [ expression { "," expression } ] ")" ;
primary         = literal | identifier | qualified-name | record | tuple | list
                | set | map | variant | "(" expression ")" ;
record          = "{" [ identifier ":" expression { "," identifier ":" expression } ] "}" ;
tuple           = "(" expression "," [ expression { "," expression } ] ")" ;
list            = "[" [ expression { "," expression } ] "]" ;
set             = "set" "{" [ expression { "," expression } ] "}" ;
map             = "map" "{" [ expression ":" expression
                  { "," expression ":" expression } ] "}" ;
variant         = identifier [ "(" [ expression { "," expression } ] ")" ] ;
pattern         = "_" | literal | identifier | variant-pattern | tuple-pattern
                | record-pattern ;
variant-pattern = identifier [ "(" [ pattern { "," pattern } ] ")" ] ;
tuple-pattern   = "(" pattern "," [ pattern { "," pattern } ] ")" ;
record-pattern  = "{" [ identifier [ ":" pattern ]
                  { "," identifier [ ":" pattern ] } ] "}" ;

type            = union-type ;
union-type      = intersection-type { "|" intersection-type } ;
intersection-type = prefix-type { "&" prefix-type } ;
prefix-type     = [ handle-mode ] primary-type [ refinement ] ;
handle-mode     = ( "owned" | "shared" | "borrowed" )
                  [ "read" | "write" ] [ usage-mode ] [ lifetime ] ;
usage-mode      = "unrestricted" | "affine" | "linear" ;
primary-type    = type-ref [ type-args ] | record-type | tuple-type | variant-type
                | goal-type | "(" type ")" ;
record-type     = "{" [ field-type { "," field-type } ] [ "," "..." ] "}" ;
field-type      = identifier [ "?" ] ":" type ;
tuple-type      = "(" type "," [ type { "," type } ] ")" ;
variant-type    = "variant" "{" variant-case { "," variant-case } "}" ;
variant-case    = identifier [ "(" [ type { "," type } ] ")" ] ;
goal-type       = "Goal" "<" type "," type [ "," effect-row [ "," type ] ] ">" ;
effect-row      = "!" "{" [ qualified-name { "," qualified-name } ] [ "|" identifier ] "}" ;
refinement      = "where" identifier "=>" expression ;
type-ref        = qualified-name ;
type-params     = "<" type-param { "," type-param } ">" ;
type-param      = identifier [ ":" type ] ;
type-args       = "<" type { "," type } ">" ;
parameters      = parameter { "," parameter } ;
parameter       = identifier ":" type ;
lifetime        = "'" identifier ;

literal         = "true" | "false" | "unit" | integer | rational | decimal
                | machine-float | string | bytes | timestamp | duration ;
rational        = integer "/" positive-integer ;
decimal         = integer ( "." digit { digit } ) "d" ;
machine-float   = "float" ( "16" | "32" | "64" | "128" ) "(" hex-bytes ")" ;
qualified-name  = identifier { ( "::" | "/" ) identifier } [ "@" version ] ;
identifier      = letter { letter | digit | "_" | "-" } ;
version         = digit { digit | "." | letter | "-" } ;
integer         = [ "-" ] positive-integer | "0" ;
positive-integer= nonzero-digit { digit } ;
string          = '"' { unicode-scalar | escape } '"' ;
bytes           = "h'" hex-bytes "'" ;
timestamp       = "time" string ;
duration        = "duration" string ;
hex-bytes       = hex-digit { hex-digit } ;
comment         = "//" { ? non-newline ? } | "/*" { ? non-closing sequence ? } "*/" ;
letter          = ? Unicode XID_Start or underscore ? ;
digit           = "0" | nonzero-digit ;
nonzero-digit   = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
hex-digit       = digit | "a" | "b" | "c" | "d" | "e" | "f"
                | "A" | "B" | "C" | "D" | "E" | "F" ;
unicode-scalar  = ? any Unicode scalar except quote, backslash, or control ? ;
escape          = "\\" ( '"' | "\\" | "/" | "b" | "f" | "n" | "r" | "t"
                | "u{" hex-digit { hex-digit } "}" ) ;
```

Implementations MUST diagnose duplicate observable names, unresolved names,
non-exhaustive matches, and reserved-keyword misuse before IR emission.

Schema anchors: `canonical-ast-document`, `ast-node`, and `token-span`.

## S8. Goal algebra

### S8.1 Core nodes

Every composition node is a first-class semantic IR form.

| Node | Result type and proof rule |
| --- | --- |
| `§all` | Conjunction. Output is a record/product keyed by branch names. Satisfaction requires evidence from every child. |
| `§any` | Constructive choice. Output is a tagged union keyed by branch names. Satisfaction names and proves one winning branch. |
| `§none` | Constructive NOR. Output is `Unit`. Satisfaction requires counter-evidence refuting every child. |
| `§chain` | Ordered dependent composition. Earlier named outputs bind later inputs; the output is the last observable binding (or an explicit record). |
| `§gate` | Implication controlled by a pure Boolean. Closed output is `Skipped`; open output is `Completed<T>`. |
| `§latch` | Atomic persistent state, explicitly `Empty` or `Captured<T>`, retaining the last accepted value, evidence, provenance, and freshness. |

`§all`, `§any`, and `§none` support finite quantified families. The domain MUST be
statically finite or verifier-backed with a finite witnessed enumeration. Recursive
goal references MUST carry a static bound or a checker-accepted well-founded
decreasing measure. Unbounded recursion is rejected.

### S8.2 Identities and normalization

- `all {}` is satisfied with `{}`; `any {}` is refuted; `none {}` is satisfied with
  `unit`; and an empty `chain` is satisfied with `unit`.
- Nested `all` and `any` nodes of the same kind MAY flatten only when doing so
  preserves observable output and branch tags. `none`, `chain`, `gate`, and `latch`
  never flatten across their boundary; NOR is not associative.
- Logical conjunction/disjunction are associative. Branch order in `all`, `any`, and
  `none` is commutative after dependency/effect analysis, while branch names and tags
  remain observable. Named products, tagged unions, `chain` order, state,
  preferences, evidence policy, and effect order where observable are not
  commutative.
- Every `any` winner and every child outcome in evidence uses the stable branch ID.
  A planner MUST NOT invent or erase tags.

### S8.3 Four-outcome propagation

Children may finish in any order. The following tables give the decisive rule after
all still-relevant work has completed or been safely cancelled. `F` means faulted and
`I` indeterminate. When both remain relevant and no truth/counter-proof decides the
composite, `F` takes precedence over `I`.

| Node | Satisfied | Refuted | Otherwise |
| --- | --- | --- | --- |
| `all` | every child S | any child R | F if any F; else I |
| `any` | any child S | every child R | F if any F; else I |
| `none` | every child R | any child S | F if any F; else I |
| `chain` | each step S in order | a step R | first causally relevant F; else I; later steps do not run |
| `gate` closed | `Satisfied(Skipped, condition evidence)` | never | condition I/F propagates |
| `gate` open | child S as `Completed<T>` | child R | child I/F propagates |
| `latch` update | atomic child S capture | child R, old value retained | child I/F, old value retained |

Thus a refuted branch proves `all` refuted even if an unrelated branch faults, and a
satisfied branch proves `any` satisfied despite unrelated faults. A satisfied branch
refutes `none` despite unrelated faults. Implementations SHOULD cancel irrelevant
work, but cancellation MUST be represented and MUST respect resource cleanup.

### S8.4 Nonlogical composition

Composition also obeys these rules:

1. A child requirement MUST be discharged by a prior guarantee in `chain`, by a
   parent fact/invariant, or emitted as an explicit parent obligation.
2. Parent invariants hold before, during, and after every child transition.
3. Child authority is intersected with the parent ceiling. Prohibitions accumulate;
   deny wins.
4. Limits are shared budgets by default. A child allocation MUST be explicit, fit
   within the remaining parent budget, and account for retries and parallel work.
5. Preferences compare valid results lexicographically in ascending integer-priority
   order (smaller integers first). Within a priority, objectives are Pareto-combined
   unless policy supplies a deterministic aggregation function.
6. `all` is parallel-eligible only when data dependencies, mutable state, exclusive
   borrows, linear use, and effects do not conflict. A graph records the decision and
   reasons.
7. `chain` order is semantic. Retries, speculative races, and fallback order are
   planner strategies unless the source makes them observable through state,
   effects, budgets, outputs, or evidence.

### S8.5 Latch state

A latch key identifies one atomic persistent cell. `Empty` is explicit and is not a
missing field. On a satisfied update, value, accepted evidence, provenance, capture
time, and freshness rule commit atomically. A refuted, indeterminate, or faulted
update retains the prior state. Reads beyond the freshness rule are
`Indeterminate(stale-evidence, partialEvidence)` unless policy requires a fault.
Concurrent writers serialize or use compare-and-swap over the prior state ID; lost
updates are forbidden.

Schema anchors: `composition-node`, `composition-kind`, `budget`, `preference`,
`state-cell`, and `state-graph-document`.

## S9. Profiles, policies, waivers, and extensions

### S9.1 Syntax selection

Profiles and policies are authored entirely in canonical BHCP. Noncanonical source
selects one exact syntax profile using this fixed, profile-independent ASCII preamble
as its first non-BOM bytes:

```text
#!bhcp-profile namespace/name@version
```

Omission selects `bhcp/canonical@0`. Preamble parsing permits only ASCII space and LF
and performs no aliasing. The selected profile is included in the AST artifact but
excluded from semantic identity.

v0 syntax profiles may remap keywords, the sigil, delimiters, terminators, formatting
rules, and aliases; inherit one exact parent; and attach policy overlays. They MUST
lower deterministically to canonical tokens. Arbitrary grammars, parser code,
unrestricted macros, ambiguous aliases, and core-semantic overrides are rejected.

### S9.2 Monotonic policy

Policy layers apply in order: organization, team, repository, user. Composition is
monotonic. A later layer may add requirements, evidence rules, or prohibitions;
tighten limits; narrow capabilities; or strengthen type mode. It MUST NOT weaken an
earlier layer without a valid waiver. Conflicts resolve to the stricter rule; deny
wins. The elaborated policy and every source layer remain in the artifact.

A waiver is valid only when it:

- identifies exact rules and scope;
- states the precise weakening and justification;
- is issued by an authority permitted by the waived rule;
- starts no earlier than issuance and has an expiry;
- is unexpired at every affected decision;
- carries authorization material and is auditable; and
- does not waive a rule declared non-waivable.

Invalid, expired, overbroad, or unauthorized waivers are rejected, not ignored.

### S9.3 Extensions

A derived extension has a namespaced/versioned identity and deterministically lowers
completely to core IR before checking and hashing. A native extension is a
namespaced/versioned, must-understand IR node with declared type, effect, policy,
normalization, hashing, and evidence behavior. Unsupported native extensions cause
rejection. Extensions MUST NOT override core meanings or loosen enclosing policy.

Schema anchors: `syntax-document`, `profile-document`, `policy-document`,
`waiver-document`, and `extension-descriptor-document`.

## S10. Planning, execution, and evidence graphs

Lowering emits three analysis graphs with stable node and edge IDs:

- the obligation graph relates requirements, guarantees, invariants, limits, cases,
  verifiers, proof dependencies, and discharge status;
- the capability graph relates requested effects, resources, grants, denials, policy
  sources, and decisions; and
- the state graph relates cells/resources, ownership, borrows, transitions,
  invariants, latch operations, and freshness.

A planner request includes the semantic IR reference, input, graph references,
budgets, policy, available executors, and required features. A planner result is
either a typed execution graph or an explained refusal/indeterminate result. Planning
does not grant authority.

Every execution node declares its typed inputs/outputs, effects, capability decision,
budget allocation, executor, dependencies, and expected evidence. Execution graph
edges are reference IDs, never object cycles. Runtime traces bind actual events to
these nodes.

An evidence bundle contains typed claims, evidence items, verifier identity and
version, subject/content references, provenance, freshness, trust classification,
gaps, and edges to discharged or refuted obligations. Evidence status is per
obligation. `§case` results may appear as evidence only when an obligation explicitly
accepts their verifier class; cases never create obligations.

Schema anchors: all `*-graph-document`, `planner-request-document`,
`planner-result-document`, `evidence-bundle-document`, and
`runtime-outcome-document`.

## S11. Wire encoding, normalization, and identity

All platform artifacts MUST validate against the CDDL bundle in `schemas/v0/`.
Canonical wire bytes are deterministic CBOR under RFC 8949 §4.2. CDDL follows RFC
8610. JSON and CBOR diagnostic notation are display formats, not identity inputs.

Maps use deterministic key ordering. Definite lengths are required. Text is valid
UTF-8 NFC. Duplicate map keys are forbidden. Integers use the shortest encoding.
Semantic sets are arrays sorted by normalized deterministic-CBOR item bytes and have
no duplicates. References replace cyclic structures.

Before semantic hashing, an implementation MUST:

1. resolve all symbols and profile aliases;
2. infer and materialize canonical types/effect rows;
3. alpha-normalize non-observable binders;
4. flatten and sort associative, unordered logical nodes when S8 permits;
5. normalize union/intersection members and policy clauses;
6. preserve `chain` order, observable names/tags, effects, preferences, policy,
   ownership/state semantics, and native extension nodes; and
7. remove source/profile presentation and provenance metadata.

There are two distinct identities:

- **Semantic ID** hashes normalized semantic meaning. It excludes syntax/profile
  presentation, formatting, source spans, comments, provenance, signatures, and
  artifact packaging.
- **Artifact ID** hashes the complete versioned document with its provenance and
  authorization material, except the artifact ID field itself.

Both hash normalized deterministic-CBOR bytes through an algorithm-tagged registry.
`sha2-256` is mandatory and uses a 32-byte digest. Content references MAY carry
additional registered digests. Unknown algorithms are retained but MUST NOT be
treated as verified. A content reference includes media type, size, and one or more
digests; it is valid only if every claimed understood digest verifies.

Schema anchors: `semantic-id`, `artifact-id`, `digest`, `content-reference`, and every
document header.

## S12. Conformance requirements

A complete v0 suite MUST include scenarios for:

- two syntax profiles producing the same semantic ID;
- strict, gradual, dynamic, nominal, structural, refinement, option, and result
  typing;
- read/write borrow conflicts, ownership transfer, state mutation, pure/effectful
  boundaries, linear/affine paths, and unsafe evidence gaps;
- satisfaction, refutation, indeterminacy, and fault for every combinator;
- gate skipping and latch empty/capture/retain, atomic update, and stale evidence;
- chain type mismatch, bounded/well-founded recursion, budget allocation, effect
  conflict, and `all` parallel eligibility;
- monotonic policies, forbidden weakening, valid/invalid waivers, and supported,
  unsupported, derived, and native extensions; and
- stable deterministic bytes, semantic-versus-artifact identity, and multiple
  algorithm-tagged digests.

Schema validation MUST use the repository-pinned `cddl` tool, generate or maintain at
least one valid instance of every root document type, and round-trip representative
diagnostic instances through deterministic CBOR without changing canonical bytes.

## Appendix A. Canonical examples

These examples supplement the grammar and rules; they do not weaken them.

### A.1 Simple goal and verifier binding

```text
§predicate bhcp.example/nonEmpty@0(value: Text): Bool
    = value != "" with bhcp.verifier/eval@0;

§goal Greet {
    §input name: Text;
    §output greeting: Text;
    §requires "name": bhcp.example/nonEmpty@0(name);
    §ensures "prefix": greeting == "Hello, " + name;
    §verify "exact" with bhcp.verifier/expression@0;
}
```

### A.2 Every combinator

```text
§all { build = Build(); docs = BuildDocs(); };
§any { cache = FetchCache(); source = Build(); };
§none { malware = DetectMalware(); policyViolation = DetectViolation(); };
§chain { patch = Edit(); checked = Check(patch = borrow patch); saved = Save(patch = move patch); };
§gate when change.risk == High { approval = Approve(change = change); };
§latch lastGreen: Build fresh duration "PT24H" { candidate = Test(build = build); };
```

The `all` output is `{build: BuildOutput, docs: DocsOutput}`. The `any` output is
`cache(CacheOutput) | source(BuildOutput)`. The `none` output is `Unit`. The gate is
`Skipped | Completed<Approval>`. The latch is `Empty | Captured<Build>` with evidence,
provenance, and freshness.

### A.3 Refinement and result

```text
§type Port = Integer where p => 0 < p && p < 65536;
§type ParsePort = Goal<Text, Result<Port, ParseError>>;
```

### A.4 Finite and recursive composition

```text
§all forall package in workspace.packages {
    checked = CheckPackage(package = package);
};

§goal WalkTree §refines Goal<Node, Unit> {
    §input node: Node;
    §limit depth <= 64;
    §all forall child in node.children {
        walked = WalkTree(node = child); // checker proves child.depth < node.depth
    };
}
```

### A.5 Ownership and effects

```text
§goal Persist {
    §input patch: owned affine Patch;
    §resource repository: borrowed write 'run Repository;
    §output receipt: Result<Receipt, StorageError>;
    §allows fs.read(repository), fs.write(repository);
    §forbids network;
    §chain {
        checked = Check(patch = borrow patch);
        receipt = Write(repository = borrow repository, patch = move patch);
    };
}
```

## Appendix B. Vision-to-contract traceability

| v0 commitment | Normative section | Principal CDDL rule(s) |
| --- | --- | --- |
| goal relation and four outcomes | S2 | `goal-definition`, `runtime-outcome-document` |
| type modes and complete type system | S4 | `type-mode`, `type`, `value`, `handle-type` |
| pure expression calculus and predicates | S5 | `expression`, `predicate-definition`, `verifier-binding` |
| effects, authority, unsafe gaps | S6 | `effect-row`, `authority-clause`, `capability`, `evidence-gap` |
| canonical vocabulary and grammar | S7 | `canonical-ast-document`, `ast-node` |
| six combinators and outcomes | S8 | `composition-node`, `composition-kind`, `runtime-outcome` |
| composition of contracts/policy/budgets/preferences | S8.4 | `clause`, `budget`, `preference`, graph rules |
| latch persistence/freshness | S8.5 | `state-cell`, `state-node`, `state-transition` |
| profiles and fixed preamble | S9.1 | `syntax-document`, `profile-document`, `syntax-mapping` |
| monotonic policy and waivers | S9.2 | `policy-document`, `waiver-document` |
| derived/native extensions | S9.3 | `extension-descriptor-document`, `extension-node` |
| platform analysis/execution/evidence artifacts | S10 | all graph, planner, evidence document roots |
| deterministic CBOR and identities | S11 | `document-header`, `semantic-id`, `artifact-id`, `content-reference-document` |
| conformance scenarios and complete v0 boundary | S12 | `feature-manifest-document`, every root document |
