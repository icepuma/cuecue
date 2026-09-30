# Vision

## The problem

CUE gets the core of configuration right. Values, types and constraints are one kind of thing, and they combine by unification, which gives the same result in any order. Every value can be traced to the files that produced it. People who use CUE praise exactly this.

People who left CUE, or never adopted it, name the same problems again and again: it is slow on some configurations, its errors take theory to read, functions are awkward, it embeds natively only in Go, and learning it is a large up-front cost. The evidence is summarized below.

## What cuecue is

1. **Edition v1:** CUE as specified at v0.17.1, implemented in Rust and verified case by case against the official `cue` binary (the oracle).
2. **Edition 2027:** the same semantics with the changes recorded in [DECISIONS](DECISIONS.md): functions, tagged unions, typed inputs, built-in tests, `${}` interpolation, and errors that show every source of a value. Specified in [SPEC](SPEC.md) §4.
3. **One engine everywhere:** the CLI, the language server and every language binding call the same evaluator.

Each file declares its edition, and both editions lower to one core. Packages of either edition can import each other, and `cuecue migrate` converts v1 packages to 2027 mechanically.

## Goals for 1.0

| Goal | Measured by |
|---|---|
| Compatible | at least 95% of the v1 corpus passes, and every remaining failure has a written reason |
| Fast | never slower than the oracle on the benchmark suite, with lower peak memory |
| Readable errors | every conflict names all contributing sources, and `cuecue why` explains any field |
| Embeddable | the C, WASM/JavaScript, Python and Go bindings pass one shared smoke suite |
| Easy to adopt | every passing corpus case migrates to 2027 and produces identical output |

## Non-goals

- Turing completeness, recursion, or I/O during evaluation
- Overrides, or inheritance beyond defaults
- A task runner like `cue cmd`; side effects belong to the host program
- Compatibility with CUE's Go API
- Policy-engine features beyond constraints

## What people say about CUE

Collected in September 2026.

**Praised**

- Unification: composition that doesn't depend on order, and built-in deep merge ([HN](https://news.ycombinator.com/item?id=46265956), [Holos](https://holos.run/blog/why-cue-for-configuration/)).
- Types as values, with schema, data and policy in one language ([RCL](https://ruuda.nl/2024/a-reasonable-configuration-language), [DEVCLASS](https://devclass.com/2022/01/12/cue-language/)).
- Imports and exports for JSON Schema, OpenAPI and protobuf; used by Holos, Timoni and Grafana.

**Criticized**

- **Learning curve.** Dagger's founder named learning CUE as the top complaint from early users, and Dagger retired its CUE SDK once Go, Python and Node SDKs existed ([HN](https://news.ycombinator.com/item?id=46265956), [Dagger](https://dagger.io/blog/ending-cue-support/)).
- **Performance.** Slow on fairly small configurations for some users. The CUE team names disjunctions, repeated evaluation, closedness memory and duplicated work, and rewrote the evaluator (evalv3, default since v0.13) ([RCL](https://ruuda.nl/2024/a-reasonable-configuration-language), [discussion #2857](https://github.com/cue-lang/cue/discussions/2857), [v0.14.0 notes](https://github.com/cue-lang/cue/releases/tag/v0.14.0)).
- **Upstream bugs.** Memory leaks that stayed unfixed pushed Dagger away ([HN](https://news.ycombinator.com/item?id=46265956)).
- **Functions.** Written as `(#f & {x: 1}).out`; requests for real syntax have been open for years ([#1480](https://github.com/cue-lang/cue/issues/1480), [#484](https://github.com/cue-lang/cue/discussions/484)).
- **Error messages.** "incomplete value" and similar messages take theory to decode ([#550](https://github.com/cuelang/cue/issues/550), [HN](https://news.ycombinator.com/item?id=46588607)).
- **Go-flavored surface.** The module system and the `\(x)` interpolation syntax ([RCL](https://ruuda.nl/2024/a-reasonable-configuration-language)).
- **Tooling.** IDE support lagged, and outside Go, CUE is reached through a C wrapper around the Go runtime ([Pkl #7](https://github.com/apple/pkl/discussions/7), [Rawkode Academy](https://rawkode.academy/read/building-rust-cue-library), [#3102](https://github.com/cue-lang/cue/issues/3102)).
- **No overrides.** Pkl users asked why a value can't be set in more than one place ([Pkl #7](https://github.com/apple/pkl/discussions/7)). cuecue keeps this restriction (D-006).
