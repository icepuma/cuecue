# cuecue language specification

This document is normative for edition 2027 and for how cuecue treats edition v1. Examples illustrate; the executable form of each section lives in `tests/spec/` ([TESTING](TESTING.md)).

## 1. Terms

| Term | Meaning |
|---|---|
| unification, `&` | The meet of two values in the value lattice. Commutative, associative and idempotent. |
| disjunction, `\|` | The join of two values. Disjuncts marked `*` are defaults. |
| top `_`, bottom `_\|_` | The value that allows anything; the error value. |
| concrete, incomplete | A value with no choices left (`3`, `"a"`); a value that still has choices (`int`). Incomplete is an error only where a concrete value is required, such as export. |
| conjunct | One expression contributing to a field, with its scope and source span. A field's value is the unification of its conjuncts. |
| closedness | Structs reached through a definition (`#X`) reject fields the definition doesn't allow. |
| provenance | The source span and derivation steps (definition, pattern, comprehension, default, input) behind a conjunct. |
| edition | The syntax version of a file: `v1` or `2027`. |
| oracle | The official `cue` binary at v0.17.1. |

Other terms (fields, patterns, embedding, comprehensions, aliases, `let`) mean what they mean in the CUE spec.

## 2. Editions

- A file whose first token is `edition` is edition 2027 (§4.1). Every other file is edition v1.
- All files of one package share an edition. A mixed package is a `syntax` error that names one file of each edition.
- Both editions lower to the same core, so a package of either edition can import a package of the other.
- Only the declarations and rules in §4 differ between editions.

## 3. Edition v1

Edition v1 is CUE as defined by the [CUE language specification at v0.17.1](https://github.com/cue-lang/cue/blob/v0.17.1/doc/ref/spec.md). Where that spec is silent, marked TODO, or ambiguous, the oracle's behavior is the definition. Cover each such case with a test.

Not supported yet. The corpus marks cases that need these `skip`:

| Feature | Plan |
|---|---|
| `@experiment(...)` language experiments | not planned |
| `*_tool.cue` files, `cue cmd`, `tool/*` packages | not planned; tool files are ignored with a warning (D-009) |
| tag injection: `@tag`, `@if`, `-t` | open question Q6 |
| `@embed`, `@extern` | open question Q3 |
| dependencies fetched from a registry | M10 |

## 4. Edition 2027

Each feature states its grammar (EBNF, as in the CUE spec), its rules, its errors, and its **v1 equivalent**: v1 source that behaves the same. Spec tests use the v1 equivalent as their oracle, and `cuecue migrate` (M12) applies the rules in reverse. A feature without a v1 equivalent says so.

### 4.0 Example

`examples/deploy/deploy.cue`:

```cue
edition 2027
package deploy

use k8s "./k8s"
use "std/strings"

// Schema: types are values, as in v1.
#Service: {
    name!:     string & =~"^[a-z][a-z0-9-]{0,62}$"
    image!:    string
    env!:      "dev" | "staging" | "prod"
    replicas:  *2 | int & >=1 & <=50
    ports:     [...#Port]
    resources: #Resources
}

#Port: {
    name!:    string
    port!:    int & >0 & <65536
    protocol: *"TCP" | "UDP"
}

// The value of `tier` picks one arm (§4.7).
#Resources: union(tier) {
    small:  {cpu: "250m", memory: "256Mi"}
    large:  {cpu: "2", memory: "4Gi"}
    custom: {cpu!: string, memory!: string}
}

// Functions (§4.6).
fn labels(s: #Service) -> {[string]: string} = {
    "app.kubernetes.io/name": s.name
    "app.kubernetes.io/env":  s.env
}

fn deployment(s: #Service) -> k8s.#Deployment = {
    metadata: {name: s.name, labels: labels(s)}
    spec: {
        replicas: s.replicas
        selector: matchLabels: labels(s)
        template: {
            metadata: labels: labels(s)
            spec: containers: [{
                name:  s.name
                image: s.image
                ports: [for p in s.ports {name: p.name, containerPort: p.port}]
                resources: requests: {cpu: s.resources.cpu, memory: s.resources.memory}
            }]
        }
    }
}

// Supplied by the host (§4.8).
input version: string & =~"^v\\d+\\.\\d+\\.\\d+$"

services: [Name=string]: #Service & {name: Name}

services: api: {
    image: "ghcr.io/acme/api:${version}"
    env:   "prod"
    ports: [{name: "http", port: 8080}]
    resources: tier: "large"
}

services: worker: {
    image:    "ghcr.io/acme/worker:${version}"
    env:      "prod"
    replicas: 4
    resources: {tier: "custom", cpu: "500m", memory: "1Gi"}
}

// Policy: more constraints, in any order (§4.5).
services: [string]: {
    if self.env == "prod" {replicas: >=2}
}

// The only field `cuecue export` emits (§4.9).
export manifests: [for _, s in services {deployment(s)}]

// Run by `cuecue test` (§4.10).
test "prod services run at least two replicas" {
    assert len([for _, s in services if s.env == "prod" && s.replicas < 2 {s.name}]) == 0
}

test "images come from our registry" {
    assert len([for _, s in services if !strings.HasPrefix(s.image, "ghcr.io/acme/") {s.name}]) == 0
}

test "service names are validated" {
    assert fails(#Service & {name: "Bad_Name", image: "x", env: "dev"})
}
```

`examples/deploy/k8s/k8s.cue`, a minimal stand-in for real Kubernetes schemas:

```cue
edition 2027
package k8s

#Deployment: {
    apiVersion: "apps/v1"
    kind:       "Deployment"
    metadata: {
        name!:   string
        labels?: {[string]: string}
    }
    spec: {
        replicas: int & >=0
        selector: matchLabels: {[string]: string}
        template: {
            metadata: labels: {[string]: string}
            spec: containers: [...#Container]
        }
    }
}

#Container: {
    name!:  string
    image!: string
    ports?: [...{name?: string, containerPort!: int}]
    resources?: requests?: {cpu?: string, memory?: string}
}
```

```
cuecue export ./examples/deploy --input version=v1.2.3
cuecue test ./examples/deploy --input version=v1.2.3
```

### 4.1 Edition clause

```ebnf
SourceFile    = [ EditionClause "," ] { attribute "," } [ PackageClause "," ] { UseDecl "," } { Declaration "," } .
EditionClause = "edition" int_lit .
```

- Only comments may precede the edition clause.
- `2027` is the only valid edition. Any other number is a `syntax` error.

### 4.2 Contextual keywords

Edition 2027 adds `edition`, `use`, `fn`, `union`, `input`, `export`, `test`, `assert` and `self`. As with v1 keywords, each is special only in its grammatical position, so `test: 1` and `input: "x"` remain ordinary fields.

| Word | Special when |
|---|---|
| `edition` | first token of the file |
| `use` | in the preamble, followed by a string, or by an identifier and a string |
| `fn` | starting a top-level declaration, followed by an identifier and `(` |
| `input`, `export` | starting a top-level declaration, followed by an identifier and `:` |
| `test` | starting a top-level declaration, followed by a string and `{` |
| `assert` | starting a statement inside a test block |
| `union` | in expression position, followed by `(` |
| `self` | in expression position, always |

- A field named `self` must be written `"self"`, because `self` in an expression always means §4.5.
- A function can't be named after one of these words (`syntax` error).

### 4.3 Strings and interpolation

- In string and bytes literals, including multi-line ones, `${` starts an interpolation that ends at the matching `}`.
- `\$` is an escape for `$`, so `"\${x}"` is the literal text `${x}`.
- `\(` is a `syntax` error with the hint "use ${…}".
- In a literal delimited by N `#` characters, escapes start with `\` followed by N `#` (as in v1), and interpolation starts with `$`, N `#`, and `{`.

**v1 equivalent:** `${e}` → `\(e)`, and `\$` → `$`. Migration escapes each literal `${` as `\${`.

### 4.4 `use` declarations

```ebnf
UseDecl = "use" [ identifier ] string_lit .
```

- `use` declarations follow the package clause and precede all other declarations. A v1 `import` declaration is a `syntax` error in a 2027 file.
- Path forms:
  - `"std/<pkg>"` is the standard library package `<pkg>`, e.g. `"std/strings"` or `"std/encoding/json"`. The packages and functions are the same as in v1.
  - `"./dir"` or `"../dir"` is the package in that directory, relative to the importing file. The target must be inside the same module.
  - Any other path is a dependency declared in the module manifest (M10, Q2). Versions live in the manifest, never in the path (D-011).
- The package is bound to the given identifier, or else to the last element of the path.
- Duplicate bindings, unknown packages, and paths that leave the module are `reference` errors.

**v1 equivalent:** `use "std/strings"` → `import "strings"`; `use x "./dir"` → `import x "<module path>/<dir>"`.

### 4.5 `self`

- `self` is the value of the innermost enclosing struct literal, not counting comprehension bodies. At the top level of a file it is the package's value.
- It sees the fully unified value, so `self.env` in §4.0 sees the `env` that another conjunct sets.

**v1 equivalent:** a value alias on that struct literal, with a fresh name: `a: {x: 1, y: self.x}` → `a: S={x: 1, y: S.x}`.

### 4.6 Functions

```ebnf
FnDecl = "fn" identifier "(" [ Param { "," Param } [ "," ] ] ")" [ "->" Expression ] "=" Expression .
Param  = identifier ":" Expression .
```

Calls use the v1 call syntax: `f(a, b)` or `pkg.f(a)`.

- Functions are declared at the top level of a file and are visible in the whole package. They are exported unless their name starts with `_`.
- For `fn f(p1: T1, …, pn: Tn) -> R = B`, the call `f(a1, …, an)` evaluates `R & B` (or just `B` without `->`) in the declaration's scope plus one binding per parameter: `pi` is bound to `Ti & ai`. Nothing else enters the scope.
- Calls have their own namespace. In `name(…)`, `name` resolves to a function of the package, then to a builtin (`len`, `close`, …); fields are never callable. In `pkg.name(…)`, `name` resolves among `pkg`'s exported functions. This is why `labels: labels(s)` in §4.0 calls the function instead of referring to the field.
- Functions are not values. They can't be stored, passed, returned or unified.
- The call graph across all packages must be acyclic.
- Equal arguments give equal results, so the evaluator may cache calls.

Errors: a wrong number of arguments, or calling something that isn't a function → `call`; a cycle in the call graph → `recursion`, reported at one call site in the cycle; a function and another top-level declaration with the same name → `reference`.

**v1 equivalent** (calls within one package): `fn f(p: T) -> R = B` → `_f: {_p: T, out: R & B'}`, where `B'` is `B` with `p` replaced by `_p`; each call `f(a)` → `(_f & {_p: a}).out`. Use fresh names. The equivalence holds for results; the scope rule above is what the implementation follows.

### 4.7 Tagged unions

```ebnf
UnionExpr = "union" "(" identifier ")" "{" { UnionArm [ "," ] } "}" .
UnionArm  = [ "*" ] ( identifier | simple_string_lit ) ":" Expression .
```

Let `U = union(t) { n1: E1, …, nk: Ek }` be unified into a struct value `V`.

- `U` constrains the tag to `V.t: "n1" | … | "nk"`, with `*` on the name of the default arm, if one is marked.
- Once `V.t` is concrete, or at finalization if `V.t` has a default, `U` contributes exactly `{t: nj} & Ej` for the arm whose name equals `V.t`. The other arms are never evaluated.
- If `V.t` never becomes concrete and has no default, `U` reports `incomplete` at `V.t`, listing the arm names.
- A concrete `V.t` that names no arm is an `unknown-tag` error listing the arm names.
- Arm names are distinct, and at most one arm is marked `*` (`syntax` error otherwise).

**v1 equivalent:** `({t: "n1"} & E1) | … | ({t: "nk"} & Ek)`, with `*` on the default arm. The two agree whenever `V.t` is concrete; a property test checks this ([TESTING](TESTING.md)). When `V.t` isn't concrete, the union stays incomplete instead of searching the arms.

### 4.8 Inputs

```ebnf
InputDecl = "input" identifier ":" Expression .
```

- Top level only. Declares the field `identifier: Expression` and marks it as an input of the package.
- The host supplies inputs for the package it evaluates. The CLI takes `--input name=text` (a string) and `--input-json name=<json>`; bindings pass native values. A supplied value is one more conjunct on the field, with provenance `input <name>`.
- Supplying an undeclared input is an `input` error. A declared input that stays non-concrete fails export with `incomplete`, and the message says the input was not supplied.
- Inputs of imported packages can't be supplied; they act as plain fields.
- Evaluation reads nothing else from outside. Environment variables reach a configuration only through `--input`.

**v1 equivalent:** the field plus the supplied value as a second conjunct: `version: string & =~"…"` and `version: "v1.2.3"`.

### 4.9 Exports

```ebnf
ExportDecl = "export" identifier ":" Expression .
```

- Top level only. Declares the field and marks it exported.
- If the package being exported has at least one export declaration, `cuecue export` emits an object with exactly the exported fields, in declaration order (files in lexical order of their names). Otherwise it emits every regular field, as in v1.
- Export fails on any error in the package other than incompleteness. Incompleteness fails export only inside exported fields.

**v1 equivalent:** none; v1 can't hide a regular field from export without renaming it. Spec tests state the expected output directly.

### 4.10 Tests

```ebnf
TestDecl = "test" simple_string_lit "{" { TestStmt [ "," ] } "}" .
TestStmt = LetClause | "assert" Expression .
```

- Top level only. Test blocks add nothing to the package value; `eval` and `export` ignore them.
- `cuecue test [package] [--filter text] [--input …]` evaluates the package once, then runs each test: its `let`s and `assert`s evaluate in the package scope, in order.
- An `assert` passes when its expression evaluates to `true`. `false` fails the test; an incomplete or error value fails it with that diagnostic.
- `fails(x)` is a builtin available only inside test blocks. It is `true` when `x`, or any value inside `x`, is an error other than incompleteness, and `false` otherwise.
- Test names are unique within a package.
- The exit code is 0 when every test passes and 1 otherwise. A failed assert shows its span and the values of its operands. Output shape:

```
test prod services run at least two replicas ... ok
test images come from our registry ... ok
test service names are validated ... ok
3 passed, 0 failed
```

Errors: `assert` or `fails` outside a test block, and duplicate test names → `test`.

**v1 equivalent:** none.

### 4.11 Conflicting defaults

In v1, `(*1 | int) & (*2 | int)` keeps no default, and exporting it reports `incomplete`. In 2027, when a value needs its default and its defaults were dropped because they conflicted, the error is a `conflict` that cites both default markers. This applies when at least one marker is in a 2027 file. Only the diagnostic changes; the value is the same as in v1.

**v1 equivalent:** the same value; v1 reports `incomplete` instead.

### 4.12 Packages

- A 2027 package is the set of `.cue` files in one directory that share a package clause. Files in parent directories are not merged in (v1 merges them).
- `*_tool.cue` files have no special meaning.

**v1 equivalent:** a v1 package whose parent directories hold no files of the same package.

## 5. Diagnostics

Every diagnostic has:

- a **kind** from the table below. Kinds are stable; tests compare kinds and paths, never message text.
- a **path** when it concerns a value, e.g. `services.api.replicas`.
- a **message** whose first line names the path and the problem in plain words.
- a **label** for every conjunct that contributed: its span and a short note (`requires >=2`, `sets 1`). A conflict has at least one label per side.
- **notes** for provenance steps: the definition, pattern, comprehension condition, default or input that brought a conjunct in.
- optional **help**, including suggested edits, such as the closest allowed field for a `closed` error.

| Kind | Meaning | v1 oracle messages (`scripts/oracle.sh` holds the authoritative mapping) |
|---|---|---|
| `syntax` | the file doesn't parse, or breaks a static rule: an unused `let`, an edition or package rule | `expected …`, `missing …`, `illegal …`, `… not terminated`, `found packages …`, `unreferenced alias or let clause` |
| `reference` | unknown identifier, field, list index, import or package; duplicate declaration | `reference "…" not found`, `undefined field`, `index out of range`, `import failed`, `cannot find package` |
| `conflict` | values don't unify: mismatched values, types, bounds, validators, operands or list lengths | `conflicting values`, `invalid value`, `invalid operands`, `incompatible list lengths`, `empty disjunction`, `mismatched types` |
| `closed` | a closed struct doesn't allow the field | `field not allowed` |
| `required` | a required field is missing | `field is required but not present` |
| `incomplete` | a value isn't concrete where it must be, or refers to an unset optional field. Reference cycles such as `a: b, b: a` land here too | `incomplete value`, `non-concrete value`, `cannot reference optional field` |
| `cycle` | a structural cycle | `structural cycle` |
| `builtin` | a builtin or an arithmetic operation failed for a reason other than a conflict | `error in call to`, `failed arithmetic` |
| `call` | 2027: wrong arity, or calling something that isn't a function | — |
| `recursion` | 2027: a function reaches itself through calls | — |
| `unknown-tag` | 2027: a union tag names no arm | — |
| `input` | 2027: an undeclared input was supplied | — |
| `test` | 2027: test-only syntax outside a test block, or a duplicate test name | — |

Suppose `examples/deploy/hotfix.cue` contains:

```cue
edition 2027
package deploy

services: api: replicas: 1
```

`cuecue export` then reports the following. This is the content and shape; `annotate-snippets` decides the exact glyphs.

```
error[conflict]: services.api.replicas: conflicting values 1 and >=2
  ┌─ deploy.cue:74:38
  │     if self.env == "prod" {replicas: >=2}
  │                                      ^^^ requires >=2
  ├─ hotfix.cue:4:26
  │ services: api: replicas: 1
  │                          ^ sets 1
  = note: the condition is true because env is "prod" (deploy.cue:60:12)
  = help: run `cuecue why services.api.replicas` to see every source of this value
```

`cuecue why <path>` lists every conjunct of a field in source order, with its span and provenance, then the result. Without the hotfix:

```
services.api.replicas: 2
  deploy.cue:12:16  *2 | int & >=1 & <=50  via #Service (deploy.cue:8:1), pattern services: [Name=string] (deploy.cue:56:1)
  deploy.cue:74:38  >=2                    via pattern services: [string] (deploy.cue:73:1), if self.env == "prod" (true)
  result: default *2 (deploy.cue:12:16)
```

`--diagnostics=json` emits the same data (kind, path, message, labels with file, line, column and byte range, notes, help) for CI and other tools.

Open questions about the language are listed in [DECISIONS](DECISIONS.md).
