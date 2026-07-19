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
v0 type system, minimal network kernel, proof checker, and standard derived prelude
are not optional.

“Required” does not mean “privileged.” The trusted composition mechanism is limited
to the network transition protocol, total pure reducer evaluation, sealed
observations, and generic derivation checking. The standard prelude is required
canonical BHCP code built on that mechanism; schedulers, named combinators,
persistence policies, and behavior-specific proof rules are outside the kernel.

Schema anchors: `feature-manifest-document`, `canonical-ast-document`,
`semantic-ir-document`, and all graph documents.

## S2. Goals, verdicts, and execution results

`Goal<I, O>` is a typed relation, not necessarily a mathematical function. Given an
input of `I`, it relates zero or more acceptable outputs of `O` to allowed state
transitions, an effect row, and evidence sufficient to discharge its obligations.
It does not require termination, determinism, uniqueness, or a particular plan.

A run has an operational result. A normally completed run contains exactly one
semantic verdict:

| Verdict | Meaning |
| --- | --- |
| `Satisfied(output, evidence)` | The output has type `O`; all obligations are discharged by accepted, fresh evidence. |
| `Refuted(counterEvidence)` | Accepted counter-evidence proves that the goal cannot be satisfied for this run/input under the stated interpretation. |
| `Unresolved(reason, partialEvidence)` | Neither satisfaction nor refutation is established; this includes exhaustion, missing evidence, and timeout. |

The operational result is `Completed(verdict)` or `Faulted(fault)`. A fault means
evaluation or infrastructure violated its operational contract; it is outside the
verdict because it says nothing about the goal's truth or falsity. The state names
within each category are adjectival and MUST remain distinct: execution is
`Completed | Faulted`, and a completed verdict is
`Satisfied | Refuted | Unresolved`.

Variant tags within one categorical family MUST use one grammatical role. The v0
state families use adjectives, including participial adjectives; noun payloads such
as `verdict`, `fault`, `reason`, and `derivation` are not state tags. New states MUST
follow the family convention instead of importing control-flow verbs.

Cancellation is an unresolved reason unless cancellation itself causes a declared
operational fault. A timeout, crash, failed attempt, or absent verifier result MUST
NOT be treated as counter-evidence. Implementations MUST NOT collapse refutation,
an unresolved verdict, and operational fault into a generic failure. Execution results
conform to `execution-result-document`.

## S3. Symbols, identities, labels, and references

Definitions use globally unique semantic names of the form
`namespace/name@version`. A domain predicate MUST have such an identity and a typed
signature. It MAY have a pure canonical definition, a verifier binding, or both. If
both exist, disagreement is a verifier fault and MUST be visible.

Source clauses MAY have human labels. Labels are lexical references and diagnostics;
they MUST NOT affect semantic identity. A label does affect meaning when it is also
an observable record field, variant tag, branch tag, or output name. Lowering MUST
assign every definition, clause, expression, type, and kernel network a unique
structural `ref-id`. References use those IDs; CBOR cycles are forbidden.
Clause labels within one goal MUST be unique so lexical references are unambiguous.
Renaming a label and all of its references together remains semantic-identity
preserving because lowering resolves those references to structural IDs.

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
- compile-time-only `Meta<DerivedForm,I,O>` and `Meta<NetworkShape,I,O>` lowering
  types, which MUST NOT occur in executable runtime IR; and
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
outcomes and network children. Persistent storage captures an owned value or a
policy-approved persistent share; it MUST NOT retain an expired borrow.

Omitted source qualifiers elaborate deterministically: `owned` means
`owned write unrestricted` in the goal's lexical lifetime; `shared` means
`shared read unrestricted`; a borrowed handle MUST state `read` or `write`; and an
omitted usage mode is `unrestricted`. Semantic IR always carries every qualifier.

### S4.5 Language mental model

| Familiar concept | Core lowering |
| --- | --- |
| nullable / missing | `Option<T>` or an explicit foreign-absence variant |
| exceptions | `Result<T,E>` for expected failure; `Faulted` execution for operational contract failure |
| Rust-like ownership | owned/shared/borrowed resource handles and lifetime constraints |
| garbage-collected object | shared resource identity; immutable records for pure values |
| object/class | nominal record plus namespaced predicates/goals; no implicit mutable fields |
| algebraic functional value | records, tuples, variants, pattern matching, generics, refinements |
| logic relation | goal relation and finite/verifier-backed quantification |
| actor | typed resource with message effects and state transitions |
| promise/async | goal output scheduled by a planner; ordering only when semantically declared |
| dynamic-language value | `Dynamic` plus explicit boundary checks and unchanged policy/effect rules |

Schema anchors: `type-definition`, `type`, `exact-number`, `machine-float-value`,
`value`, `resource-type`, `handle-type`, `meta-type`, and `type-mode`.

## S5. Expressions, functions, and predicates

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

Functions and predicates are total pure definitions in the expression calculus. A
function may return any declared type. A predicate returns `Bool` and MAY bind a
verifier. Purity is determined from an empty effect row, not from a `pure` assertion.
A verifier binding names an evidence producer with typed input/output and trust
requirements; it does not redefine the predicate. Kernel reducers are functions,
not privileged runtime callbacks.

A goal-level verifier receives the closed record `{ input: I, output: O }` for the
candidate `Goal<I,O>` and returns its declared evidence type. An omitted evidence
class or trust restriction MUST NOT be elaborated into a fabricated default. A
registered verifier is an external evidence producer, not an expression primitive or
kernel callback; registering one does not expand the trusted composition kernel.

Canonical verifier bindings own the verifier symbol, closed typed input, declared
evidence output, trust restrictions, and the structural obligations targeted by a
binding. These fields are program meaning and remain in canonical AST/semantic IR.
Project-local adapter declarations own only the host binding: one project-relative
executable path, an argv vector, project working scope, input/output media types, a
positive bounded timeout, a local effect ceiling, and the expected evidence kind.
The manifest binding MUST name the same verifier symbol and MUST NOT change its
canonical input, output, trust, targets, predicate, or evidence semantics.

A project adapter MUST NOT use a shell or encode a shell command string. Its
executable path MUST be lexically contained by the project and the runner MUST also
reject symlink or canonicalization escape before execution. Arguments are passed as
an argv vector without shell interpretation. Adapter effects are intersected with
the canonical goal and policy ceiling; a local declaration cannot grant authority
and MUST NOT grant ambient network. Unknown keys, duplicate symbols or fields,
missing fields, invalid media types, and unsupported effects are manifest errors.
The local manifest is deployment configuration rather than a canonical BHCP
artifact: it does not change semantic identity, but the resolved adapter artifact
and declaration MUST be retained as execution/evidence provenance. The v0 CDDL root
inventory therefore does not gain a project-manifest document.

The evaluator MAY provide fixed, versioned, total pure primitive definitions at the
bottom of expression evaluation for constructing and inspecting language values,
including sealed kernel observations and checked result construction.
Such primitives MUST be behavior-neutral, MUST NOT select an orchestration policy,
and MUST NOT be extensible implementation callbacks. Every orchestration decision
and precedence rule remains in an ordinary retained or compile-time-eliminated BHCP
definition.

Schema anchors: `expression`, `pattern`, `function-definition`,
`predicate-definition`, and `verifier-binding`.

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
| definitions | `§type`, `§function`, `§predicate`, `§goal`, `§use`, `§refines` |
| facts | `§input`, `§output`, `§resource`, `§state` |
| contracts | `§requires`, `§ensures`, `§invariant`, `§limit` |
| authority | `§allows`, `§forbids` |
| optimization/evidence | `§prefer`, `§verify`, `§case` |
| kernel composition | `§compose` |
| derived prelude | `§all`, `§any`, `§none`, `§chain`, `§gate` |
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
definition      = type-def | function-def | predicate-def | goal-def | syntax-def | profile-def
                | policy-def | waiver-def | extension-def | refines-decl ;
type-def        = "§type" qualified-name [ type-params ] "=" type ";" ;
function-def    = "§function" qualified-name [ type-params ] "(" [ parameters ] ")"
                  ":" type "=" expression ";" ;
predicate-def   = "§predicate" qualified-name [ type-params ] "(" [ parameters ] ")"
                  ":" "Bool" [ "=" expression ] [ verifier-binding ] ";" ;
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
verify-clause   = "§verify" [ label ] verifier-binding
                  [ "for" label-reference { "," label-reference } ] ";" ;
case-clause     = "§case" [ label ] "{" { binding | execution-expectation } "}" ";" ;
execution-expectation = "expect" ( "completed" verdict-state | "faulted" )
                      [ expression ] ";" ;
composition     = compose-expr ";" | all-expr ";" | any-expr ";" | none-expr ";"
                | chain-expr ";" | gate-expr ";" ;

compose-expr    = "§compose" "using" qualified-name composition-block ;
all-expr        = "§all" [ quantifier ] composition-block ;
any-expr        = "§any" [ quantifier ] composition-block ;
none-expr       = "§none" [ quantifier ] composition-block ;
chain-expr      = "§chain" composition-block ;
gate-expr       = "§gate" "when" expression unary-composition-block ;
composition-block = "{" { branch } "}" ;
unary-composition-block = "{" branch "}" ;
branch          = identifier "=" ( goal-call | composition-no-term ) ";" ;
composition-no-term = compose-expr | all-expr | any-expr | none-expr | chain-expr
                | gate-expr ;
quantifier      = ( "forall" | "exists" ) identifier "in" expression ;
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
label-reference = string ;
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
qualified-name  = semantic-component { ( "::" | "/" ) semantic-component } [ "@" version ] ;
semantic-component = identifier { "." identifier } ;
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

## S8. Minimal network kernel and derived goal algebra

### S8.1 Kernel network

The only privileged composition form in semantic IR is a finite `kernel-network`.
It contains statically typed child goal invocations and the symbol of one total pure
BHCP reducer function. It contains no behavioral `kind`, implicit guard, implicit
dependency, built-in choice rule, quantified family, scheduling order, or planner
parallelism hint. It also contains no budget or policy decision; those belong to
clauses and analysis/execution artifacts. Every referenced reducer definition MUST
be resolved and present in semantic IR.

Each network is monomorphized. Conceptually its reducer has this exact signature,
where every child tag and output type is materialized as a closed record field:

```text
(parent: I,
 observations: {tag: Option<ExecutionResult<ChildOutput>>, ...})
    -> Reduction<O>
```

A record field's tag statically identifies its child; a present value is that
child's sealed `execution-result`, while absence means the child has not produced an
observation. The runtime `child-observation` envelope carries the structural child
ID and result before constructing this monomorphized record. The runtime invokes the
reducer with the parent input and this record. Child argument
expressions are total and pure, are evaluated only when that child is first
requested, and may read the parent input and already-present sealed observations.
Reading an absent observation or producing an argument of the wrong type is rejected
before execution when provable and is otherwise an operational fault. The reducer
returns exactly one reduction state:

| Reduction | Meaning |
| --- | --- |
| `Pending(requiredTags)` | The listed unique, known, unobserved child tags are exactly those whose results may next affect a conclusion. |
| `Concluded(result, derivation)` | The network has a terminal execution result justified by a kernel-checkable derivation. |

`Pending` and `Concluded` are adjectival states; neither is a run verdict. A pending
tag set MUST be non-empty. The runtime resolves every tag through the enclosing
network's unique tag-to-child-ID mapping before scheduling; reducers never receive or
manufacture child structural IDs. Its members MAY execute in any order or in parallel
only when their effects, ownership, state, policy, and budgets permit. On each new
observation the reducer runs again. A concluded reduction is terminal. Requesting an
unknown or already observed tag, returning an invalid derivation, or evaluating a
non-total reducer is an operational fault.

Child observations and evidence tokens are sealed. Reducers MAY pattern-match
execution/verdict states, inspect outputs and reasons, propagate an operational fault
opaquely, and reference accepted premise tokens. Operational trace contents and
timestamps are not reducer inputs to semantic choice: reducers MUST NOT branch on
them, alter them, or manufacture evidence, counter-evidence, or traces. Reducer source
constructs a conclusion from an execution result whose accepted evidence references
become its proposed premises; it cannot select or inspect the derivation ID. The
generic kernel deterministically derives that ID from the enclosing network plus the
exact parent input and sealed observation record, re-evaluates the exact
reducer with the same inputs, checks that the conclusion is identical, and verifies
that the premises exist, are accepted, and cover the obligations the result claims to
discharge. It then emits the checked `Concluded` reduction with a derivation containing
only that ID and the sealed premise references; there is no behavior-specific rule
tag. A concluded satisfied verdict MUST include that ID in `evidence`; a concluded
refuted verdict MUST include it in `counterEvidence`. The derivation can therefore
prove a premise-free logical identity without synthetic child evidence.

A goal containing only flat clauses is declarative and has no network. Its semantic
IR omits `body`; its input and output types are derived from its fact clauses. Empty
networks are permitted because a reducer can immediately conclude a logical identity
without requesting a child.

Composition quantifiers are stricter than logical quantifiers in S5: their domains
MUST normalize during elaboration to a statically known finite collection. They then
expand to explicit, deterministically ordered children before kernel-network IR and
semantic hashing. A verifier-backed or runtime-only domain is rejected as
composition input; bounded or well-founded recursive goals express traversal of a
runtime collection without adding a dynamic-family kernel primitive. A recursive
child reference MUST carry its own static bound or checker-accepted well-founded
decreasing measure. Unbounded recursion is rejected.

### S8.2 Semantic self-hosting

Named orchestration behavior is defined by versioned BHCP functions and derived
extensions in the standard prelude. `all`, `any`, `none`, `chain`, and `gate` are not
kernel node kinds. Their surface forms deterministically lower to kernel networks
whose reducers are ordinary total pure BHCP definitions.

The canonical profile binds its convenience forms to these v0 prelude symbols:

| Surface form | Compile-time BHCP lowerer | Runtime BHCP reducer |
| --- | --- | --- |
| `§all` | `bhcp/prelude.lower-all@0` | `bhcp/prelude.all-reducer@0` |
| `§any` | `bhcp/prelude.lower-any@0` | `bhcp/prelude.any-reducer@0` |
| `§none` | `bhcp/prelude.lower-none@0` | `bhcp/prelude.none-reducer@0` |
| `§chain` | `bhcp/prelude.lower-chain@0` | `bhcp/prelude.chain-reducer@0` |
| `§gate` | `bhcp/prelude.lower-gate@0` | `bhcp/prelude.gate-reducer@0` |

Those bindings are versioned semantic names, not compiler callbacks. The standard
prelude is canonical BHCP source checked by the same type, totality, purity, policy,
and normalization rules as project code. A lowerer runs during elaboration and
disappears after producing core IR. Every reachable reducer definition remains in
semantic IR and semantic identity. `§compose using f` is the sole core surface form;
it constructs the same network shape directly and requires `f` to have the exact
monomorphized reducer signature from S8.1.

Every lowerer has the compile-time signature:

```text
Meta<DerivedForm,I,O> -> Meta<NetworkShape,I,O>
```

`DerivedForm` contains only the parent input type, resolved typed child shapes, and
an optional typed condition. Quantifiers have already expanded. `NetworkShape`
contains only the output type, child shapes, and reducer symbol; it deliberately has
no network or child structural IDs. These closed nominal meta values are defined by
`derived-form-shape` and `network-shape`. A lowerer cannot observe tokens, comments,
diagnostic-only labels, spans, profiles, source order already proven unobservable,
ambient state, or planner data, and cannot allocate network or child IDs. After
validating the returned shape, the elaborator assigns deterministic structural IDs.
Meta values and executed lowerer definitions MUST NOT survive into executable
semantic IR or its semantic hash.

Derived lowering is restricted metaprogramming: it is typed, total, deterministic,
structurally recursive over a finite typed goal shape, has no I/O or ambient state,
and can only construct core IR. It cannot parse source text, introduce native
semantics, loosen policy, or bypass checking. Each instantiation is monomorphized to
its exact child record and output type before semantic hashing. The extension's
presentation disappears; equivalent fully lowered networks have the same semantic
identity.

The standard prelude defines these behaviors:

| Derived behavior | Network reduction |
| --- | --- |
| `all` | Initially requests every child. It concludes satisfied with a named product when all are satisfied, or refuted when any is refuted. |
| `any` | Initially requests every child. It concludes satisfied with a stable tagged winner when any is satisfied, or refuted when all are refuted. |
| `none` | Initially requests every child. It concludes satisfied with `Unit` when all are refuted, or refuted when any is satisfied. |
| `chain` | Requests one child at a time; only satisfaction enables the next child and binds its output. |
| `gate` | Is unary. A false pure condition concludes satisfied with `Excluded`; a true condition requests its one child and maps satisfaction to `Included<T>`. |

No block implicitly means `all` or `any`. A gate has exactly one child; multiple
guarded children require an explicit derived composition inside it.

A gate condition is a total pure `Bool` expression. It therefore yields true or
false, or faults if its operational evaluator violates its contract; the condition
itself is never an unresolved verdict. Evidence-dependent judgment MUST be modeled
as an explicit child goal whose execution result can be unresolved.

The empty identities are: `all {}` is satisfied with `{}`; `any {}` is refuted;
`none {}` is satisfied with `Unit`; and `chain {}` is satisfied with `Unit`. Nested
derived forms MAY normalize only when their prelude definition proves the rewrite
preserves output shape, observable tags, evidence, effects, and ordering.

### S8.3 Standard verdict laws

The standard reducers obey these laws after all still-relevant work has completed or
been safely cancelled:

| Behavior | Satisfied verdict | Refuted verdict | Otherwise |
| --- | --- | --- | --- |
| `all` | every relevant child completed satisfied | any child completed refuted | faulted if a relevant child faulted; otherwise completed unresolved |
| `any` | any child completed satisfied | every relevant child completed refuted | faulted if a relevant child faulted; otherwise completed unresolved |
| `none` | every relevant child completed refuted | any child completed satisfied | faulted if a relevant child faulted; otherwise completed unresolved |
| `chain` | each requested child completed satisfied in order | first requested child completed refuted | first causally relevant fault; otherwise completed unresolved; later children are never requested |
| `gate` closed | completed satisfied with `Excluded` | never | condition evaluation fault propagates |
| `gate` open | child satisfaction mapped to `Included<T>` | child refutation | child unresolution or fault propagates |

Thus a refuted branch can conclude `all` despite an unrelated fault, and a satisfied
branch can conclude `any` despite unrelated faults. A satisfied branch refutes
`none` despite unrelated faults. Implementations SHOULD cancel children no longer
listed by the reducer, but cancellation MUST be represented and MUST respect resource
cleanup.

### S8.4 Contracts, authority, and planning

Every network also obeys these rules:

1. A child requirement MUST be discharged by a previously observed guarantee, by a
   parent fact/invariant, or emitted as an explicit parent obligation.
2. Parent invariants hold before, during, and after every child transition.
3. Child authority is intersected with the parent ceiling. Prohibitions accumulate;
   deny wins.
4. Limits are shared budgets by default. A child allocation in the execution graph
   MUST be explicit, fit within the remaining parent budget, and account for retries
   and parallel work. Budget decisions do not mutate `kernel-network`.
5. Preferences compare valid results lexicographically in ascending integer-priority
   order. Within a priority, objectives are Pareto-combined unless policy supplies a
   deterministic aggregation function.
6. A multi-child pending set is parallel-eligible only when data dependencies,
   mutable state, exclusive borrows, linear use, and effects do not conflict. The
   execution graph records the relevant dependency, effect, and state edges; planner
   diagnostics explain any denied concurrency. Neither decision nor rationale is
   stored in `kernel-network`.
7. The sequence of pending sets returned across reducer evaluations is semantic;
   member order inside one pending set is not. Retries, speculative races, and
   fallback order are planner strategies only when state, effects, budgets, outputs,
   evidence, and reducer observations make them unobservable.

### S8.5 Persistent retention

Persistent retention is not composition syntax. The capability/executor boundary
provides effectful atomic state-read and compare-and-swap goals; they are ordinary
typed child goals, not kernel operations. A versioned prelude definition may derive
a `retain` or last-known-good behavior from those goals and a normal kernel network.

A retained key identifies one atomic persistent cell. `Empty` is explicit and is not
a missing field. On a completed satisfied candidate, value, accepted evidence,
provenance, capture time, freshness rule, and prior state ID commit atomically. A
completed refuted or unresolved candidate, or a faulted candidate execution, retains
the prior state. Reads beyond the freshness rule complete unresolved with a
stale-evidence reason unless policy requires an operational fault. Concurrent writers
serialize or use compare-and-swap over the prior state ID; lost updates are
forbidden. Storage captures an owned value or a policy-approved persistent share and
MUST NOT retain an expired borrow.

Schema anchors: `meta-type`, `derived-form-shape`, `network-shape`,
`kernel-network`, `child-observation`, `reduction`, `derivation`,
`execution-result`, `budget`, `preference`, `state-cell`, and `state-graph-document`.

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

Policy source documents apply in this fixed order: organization, team, repository,
user. Missing layers contribute the identity policy. Multiple documents in one layer
are ordered by canonical symbol solely for deterministic provenance; their
restrictions are joined without precedence. A source document symbol MUST be unique
in an artifact, rule IDs MUST be unique within that document, and the stable identity
of a source rule is the pair `(source-policy-symbol, rule-id)`. Equal local rule IDs
in different source documents are distinct. An `extends` chain is expanded before
layering, MUST stay within one layer, and MUST be acyclic; an inherited rule keeps
the identity of the document that declared it.

Every category has one closed operation and one typed value:

| category | operation | typed value | restrictive composition |
| --- | --- | --- | --- |
| requirement | `add` | requirement symbol, optional scope and canonical parameters | set union |
| evidence | `add` | obligation symbol, non-empty accepted evidence-class set, positive minimum, optional scope and canonical parameters | set union of independent evidence demands |
| prohibition | `deny` | effect symbol and optional scope | set union; denial always wins |
| capability | `narrow` | effect symbol and optional scope | intersection of allowed scopes for the same effect |
| limit | `tighten` | dimension, unit, non-negative exact maximum, and optional scope | minimum maximum for the same dimension, unit, and scope |
| type-mode | `strengthen` | `dynamic`, `gradual`, `infer-strict`, or `strict` | maximum in that listed order |

No other category/operation/value combination is a v0 policy rule. Policy parameters
are compared by their deterministic CBOR value. Evidence-class arrays and all scope
arrays are sorted sets with unique members. An evidence minimum MUST be greater than
zero. A limit maximum MUST be an exact non-negative number. Two overlapping limits
for the same dimension with different units are rejected in v0; implicit unit
conversion is forbidden.

A policy scope is the product of goal, resource, and operation dimensions. An omitted
dimension denotes its universe, a present array denotes exactly that set, and an
empty array makes the scope empty. Scope `A` is no broader than scope `B` exactly when
each set in `A` is a subset of the corresponding set in `B`, treating omission as the
universe. Capability narrowing intersects these products coordinate by coordinate.
An empty intersection is a valid empty capability ceiling, not an error and not an
implicit grant. A prohibition removes every matching capability regardless of any
grant, layer, source order, or waiver applied to some other rule.

Exact duplicate effective restrictions collapse. Their provenance is unioned;
`waivable` is the conjunction of the contributing flags, and, when all are waivable,
the effective authorized issuer set is the intersection of their non-empty source
issuer sets. An empty intersection makes the effective restriction non-waivable.
`authorized_issuers` MUST be absent when `waivable` is false and MUST be a non-empty
sorted set when it is true. Distinct additive requirements or evidence demands all
remain active. A statically provable contradiction between requirements is rejected;
an implementation that cannot prove consistency retains both and reports unresolved
at enforcement rather than silently choosing one.

Let `P ⊑ Q` mean that `Q` is at least as restrictive as `P`: requirement, evidence,
and prohibition sets are supersets; every capability ceiling is a subset; every
comparable maximum is no greater; and type mode is no weaker. This is a product of
set-inclusion orders, reversed inclusion for capabilities, the exact-number order for
limits, and the finite type-mode order. Each coordinate is antisymmetric, so the
strict part is irreflexive and acyclic. The join described above is idempotent,
commutative, and associative on normalized restrictions. Consequently grouping an
already validated layer-ordered sequence cannot change its effective policy. The
ordered monotonicity check MUST still examine every source layer before joining; it
rejects a `narrow`, `tighten`, or `strengthen` value that is broader, larger, or
weaker than the applicable earlier value, and implementations MUST NOT regroup inputs
to hide that invalid attempt. Source-layer groups, policy references within each
group, and provenance source sets are canonically sorted, so input enumeration order
affects neither identity. Source decomposition, content, and layer assignment remain
in artifact identity but do not affect the semantic join of accepted restrictions.

The empty effective policy contains no additive demands, prohibitions, capability
ceilings, limits, source layers, provenance, or waivers. Its materialized type-mode
entry is `dynamic` and non-waivable because no weakening below `dynamic` exists.
After valid waivers are applied to their exact source-rule identities, layers are
joined in order. A later layer that
states a broader capability, larger limit, weaker type mode, removal, or replacement
does not override the earlier restriction: it is either an additional restriction,
an exact duplicate, or a forbidden weakening. Conflicts resolve to the restrictive
join; deny wins.

`source-policy-document` (`form = source`) is the authored boundary.
`effective-policy-document` (`form = effective`) is the execution boundary. Its
`effective` member is canonical: category arrays are sorted and unique and its type
mode is materialized. Capability rules normalize to at most one intersected ceiling
per effect. Limit rules normalize by `(dimension, unit, scope)`; distinct overlapping
scopes remain separate and the minimum applicable maximum governs their overlap.
The document's `semantic_id` commits only to the `effective` member,
including effective waivability and issuer constraints. `source_layers` retains
content-addressed source documents grouped in organization → team → repository →
user order; `provenance` maps each canonical effective-rule index to sorted source
rule identities; and applied waiver references are retained. Those fields,
signatures, timestamps, justifications, and source decomposition contribute to
artifact identity but not semantic identity. Authoring enumeration order is
canonicalized away. Thus two auditable derivations may have the same effective
semantic identity without having the same artifact identity.

A waiver is valid only when it:

- identifies exact `(source-policy-symbol, rule-id)` rules and scope;
- states the precise weakening and justification;
- is issued by an authority permitted by the waived rule;
- starts no earlier than issuance and has an expiry;
- is unexpired at every affected decision;
- carries authorization material and is auditable; and
- does not waive a rule declared non-waivable.

Invalid, expired, overbroad, or unauthorized waivers are rejected, not ignored.
Waiver application never edits or erases a source document. It records the waiver in
the effective artifact and removes only the authorized weakening from the semantic
join. Waiving a collapsed restriction requires authorization from every contributing
source rule affected by the weakening. A waiver cannot manufacture a capability or
otherwise exceed the restriction-free identity policy.

### S9.3 Extensions

A derived extension has a namespaced/versioned identity and names a total pure BHCP
lowering function satisfying S8.2. It deterministically lowers completely to core IR
before checking and hashing. A content reference MAY retain its canonical source for
audit, but an opaque implementation callback is not a conforming derived lowering.
Its descriptor has `extension_kind = derived`, `must_understand = false`, a mandatory
`lowering` function symbol, and no native `payload_schema`. A native extension has
`extension_kind = native`, `must_understand = true`, a mandatory payload schema, no
derived lowering, and declared type, effect, policy, normalization, hashing, and
evidence behavior. Unsupported native extensions cause rejection. Extensions MUST
NOT override core meanings or loosen enclosing policy. `must_understand = false`
does not permit a compiler to ignore a derived use: it means no opaque feature
remains after the mandatory lowering succeeds. Missing, rejected, or unevaluated
lowering is an error.

Schema anchors: `syntax-document`, `profile-document`, `policy-document`,
`waiver-document`, and `extension-descriptor-document`.

## S10. Planning, execution, and evidence graphs

Lowering emits three analysis graphs with stable node and edge IDs:

- the obligation graph relates requirements, guarantees, invariants, limits, cases,
  verifiers, proof dependencies, and discharge status;
- the capability graph relates requested effects, resources, grants, denials, policy
  sources, and decisions; and
- the state graph relates cells/resources, ownership, borrows, transitions,
  invariants, retained-value operations, and freshness.

A planner request includes the semantic IR reference, input, graph references,
budgets, policy, available executors, and required features. A planner result is
either a typed execution graph or an explained refused/unresolved result. Planning
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

The optional `for` list on a goal-level `§verify` clause names contract-clause labels
in the same goal. Lowering MUST resolve them to unique structural obligation IDs,
deduplicate and deterministically order them, and reject unknown or non-contract
labels. Target order and consistent label renaming do not affect semantic identity.
Without `for`, the binding is goal-wide and may produce claims for every contract
obligation; satisfaction still requires accepted coverage for each obligation.

Total pure contract conditions are re-evaluated over the typed goal input and
candidate output. Targeted external evidence is additional required coverage: a
false condition or accepted refuting evidence refutes that candidate obligation; an
absent, unsupported, stale, or inconclusive verifier leaves it unresolved; and a
verifier operational-contract failure remains faulted rather than becoming
counter-evidence. For a fixed timestamp, content references, candidate, registry,
and verifier outputs, evidence-bundle bytes MUST be deterministic. Timestamps and
provenance remain artifact identity inputs.

Schema anchors: all `*-graph-document`, `planner-request-document`,
`planner-result-document`, `evidence-bundle-document`, and
`execution-result-document`.

## S11. Wire encoding, normalization, and identity

All platform artifacts MUST validate against the CDDL bundle in `schemas/v0/`.
Canonical wire bytes are deterministic CBOR under RFC 8949 §4.2. CDDL follows RFC
8610. Human CLI inspection and CBOR diagnostic notation are display formats, not
identity inputs.

Maps use deterministic key ordering. Definite lengths are required. Text is valid
UTF-8 NFC. Duplicate map keys are forbidden. Integers use the shortest encoding.
Semantic sets are arrays sorted by normalized deterministic-CBOR item bytes and have
no duplicates. References replace cyclic structures.

Before semantic hashing, an implementation MUST:

1. resolve all symbols and profile aliases;
2. execute every derived lowering and monomorphize it to `kernel-network` IR;
3. infer and materialize canonical types/effect rows;
4. alpha-normalize non-observable binders;
5. expand statically finite composition quantifiers, apply only prelude-proved
   network rewrites, and deterministically order the explicit child set;
6. normalize union/intersection members and policy clauses;
7. retain every reachable reducer definition and preserve the sequence of pending
   sets, observable names/tags, effects, preferences, policy, ownership/state
   semantics, and native extension nodes; and
8. remove source/profile presentation and provenance metadata.

Compile-time meta values and executed lowerer definitions are also removed at step
2; reachable runtime reducer definitions are retained by step 7.

There are two distinct identities:

- **Semantic ID** hashes normalized semantic meaning. It excludes syntax/profile
  presentation, formatting, source spans, comments, provenance, signatures, and
  artifact packaging.
- **Artifact ID** hashes the complete versioned document with its provenance and
  authorization material, except the artifact ID field itself.

Both hash normalized deterministic-CBOR bytes through an algorithm-tagged registry.
The default and only algorithm registered by the first executable foundation is
`bhcp.hash/sha3-512@0`, with a 64-byte digest. A project manifest MAY select another
registered algorithm; non-default algorithms are discouraged, and an implementation
MUST reject a selected algorithm it does not implement. Content references MAY carry
additional registered digests. Unknown algorithms in received artifacts are retained
but MUST NOT be treated as verified. A content reference includes media type, size,
and one or more digests; it is valid only if every claimed understood digest verifies.

Schema anchors: `semantic-id`, `artifact-id`, `digest`, `content-reference`, and every
document header.

## S12. Conformance requirements

A complete v0 suite MUST include scenarios for:

- two syntax profiles producing the same semantic ID;
- strict, gradual, dynamic, nominal, structural, refinement, option, and result
  typing;
- read/write borrow conflicts, ownership transfer, state mutation, pure/effectful
  boundaries, linear/affine paths, and unsafe evidence gaps;
- completed satisfied, refuted, and unresolved verdicts plus operational faults for
  kernel networks and every standard derived behavior;
- pending/concluded reducer validation, generic reducer re-evaluation, sealed
  evidence, invalid derivations,
  deterministic derived lowering, and equivalence with hand-written core networks;
- unary gate exclusion/inclusion, retained-value empty/capture/retain, atomic update, and stale
  evidence;
- chain type mismatch, bounded/well-founded recursion, budget allocation, effect
  conflict, and multi-child pending-set parallel eligibility;
- monotonic policies, forbidden weakening, valid/invalid waivers, and supported,
  unsupported, derived, and native extensions; and
- stable deterministic bytes, semantic-versus-artifact identity, and multiple
  algorithm-tagged digests.

Schema validation MUST use the repository-owned Rust harness, generate or maintain
at least one valid instance of every root document type, and round-trip
representative diagnostic instances through deterministic CBOR without changing
canonical bytes. Until the Rust harness implements general RFC 8610 evaluation, its
declared validation scope MUST remain explicit and implemented artifacts MUST also
pass their strongly typed boundary validators.

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
    §verify "exact": with bhcp.verifier/expression@0;
}
```

### A.2 Derived prelude behavior

```text
§all { build = Build(); docs = BuildDocs(); };
§any { cache = FetchCache(); source = Build(); };
§none { malware = DetectMalware(); policyViolation = DetectViolation(); };
§chain { patch = Edit(); checked = Check(patch = borrow patch); saved = Save(patch = move patch); };
§gate when change.risk == High { approval = Approve(change = change); };
```

The first form is equivalent after lowering to explicit core source:

```text
§compose using bhcp/prelude.all-reducer@0 {
    build = Build();
    docs = BuildDocs();
};
```

The `all` output is `{build: BuildOutput, docs: DocsOutput}`. The `any` output is
`cache(CacheOutput) | source(BuildOutput)`. The `none` output is `Unit`. The gate is
`Excluded | Included<Approval>`. Each convenience form invokes its versioned
standard-prelude lowerer from S8.2, then leaves only a `kernel-network` and the
reachable reducer definition in semantic IR. Persistent retention is expressed by
calling a prelude goal backed by state-read and compare-and-swap capabilities, not
by a composition keyword or kernel operation.

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

The first quantifier is valid only when `workspace.packages` normalizes to a finite
collection during elaboration; it then expands to explicit children before kernel
IR. A runtime-only package collection requires a bounded or well-founded recursive
goal like `WalkTree`. Each recursive child carries its own checked decreasing
measure; neither a quantified family nor a network-wide recursion mode survives
lowering.

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

### A.6 Monotonic policy vectors

These examples use `U` for an omitted (universal) scope dimension. They are
normative semantic vectors; the CDDL diagnostic fixture supplies the positive wire
shape.

| earlier | later | result | status |
| --- | --- | --- | --- |
| requirements `{lint}` | add `{signed-commits}` | `{lint, signed-commits}` | valid |
| evidence `{static ≥ 1}` | add `{human-approved ≥ 1}` | both independent demands | valid |
| network operations `{fetch, publish}` | narrow to `{fetch}` | `{fetch}` | valid |
| network operations `{fetch}` | state `{fetch, publish}` | `{fetch}`; later statement cannot widen | forbidden weakening |
| attempts ≤ 5 | tighten attempts ≤ 3 | attempts ≤ 3 | valid |
| attempts ≤ 3 | state attempts ≤ 5 | attempts ≤ 3; later statement cannot loosen | forbidden weakening |
| `gradual` | strengthen to `strict` | `strict` | valid |
| `strict` | state `gradual` | `strict`; later statement cannot weaken | forbidden weakening |
| allow network `{fetch}` | deny network `{fetch}` | denied | valid; deny precedence |
| time ≤ 10 seconds | time ≤ 500 milliseconds | none | rejected; v0 has no implicit unit conversion |
| non-waivable deny | waiver naming that rule | none | rejected waiver |
| source `org/policy@0:r1` and `repo/policy@0:r1` | add both | two distinct source identities | valid |
| no source documents | identity policy | empty restrictions and non-waivable `dynamic` mode | valid |

## Appendix B. Vision-to-contract traceability

| v0 commitment | Normative section | Principal CDDL rule(s) |
| --- | --- | --- |
| goal relation, three verdicts, and factored operational fault | S2 | `goal-definition`, `verdict`, `execution-result-document` |
| type modes and complete type system | S4 | `type-mode`, `type`, `value`, `handle-type` |
| pure expression calculus, functions, and predicates | S5 | `expression`, `function-definition`, `predicate-definition`, `verifier-binding` |
| effects, authority, unsafe gaps | S6 | `effect-row`, `authority-clause`, `capability`, `evidence-gap` |
| canonical vocabulary and grammar | S7 | `canonical-ast-document`, `ast-node` |
| minimal network kernel and reductions | S8.1 | `kernel-network`, `child-observation`, `reduction`, `derivation` |
| self-hosted standard goal algebra | S8.2–S8.3 | `meta-type`, `derived-form-shape`, `network-shape`, `function-definition`, `kernel-network`, `execution-result` |
| composition of contracts/policy/budgets/preferences | S8.4 | `clause`, `budget`, `preference`, graph rules |
| persistent retention/freshness | S8.5 | `state-cell`, `state-node`, `state-transition` |
| profiles and fixed preamble | S9.1 | `syntax-document`, `profile-document`, `syntax-mapping` |
| monotonic policy and waivers | S9.2 | `policy-document`, `waiver-document` |
| derived/native extensions | S9.3 | `extension-descriptor-document`, `extension-node` |
| platform analysis/execution/evidence artifacts | S10 | all graph, planner, evidence document roots |
| deterministic CBOR and identities | S11 | `document-header`, `semantic-id`, `artifact-id`, `content-reference-document` |
| conformance scenarios and complete v0 boundary | S12 | `feature-manifest-document`, every root document |
