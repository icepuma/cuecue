# Testing

Progress is measured by tests whose expected values come from outside cuecue: the oracle for edition v1, and the oracle run on v1 translations for most of edition 2027.

## Layers

| Layer | Where | Proves | Expected values come from |
|---|---|---|---|
| Unit | next to the code; `insta` snapshots for tree and IR dumps | one function or module | the author |
| Corpus | `tests/corpus/v1/` | edition v1 matches CUE | the oracle |
| Spec | `tests/spec/<section>/` | edition 2027 matches SPEC.md | the oracle on a v1 translation, or the author with review |
| Property | `crates/*/tests/props.rs` | lattice laws and equivalences | the laws |
| Fuzz | `fuzz/` | no panics, no hangs | — |
| Benchmarks | M7 | speed and memory against the oracle | — |

## The oracle

- `cue` v0.17.1 (`cuelang.org/go/cmd/cue`, needs Go 1.25 or newer). `scripts/oracle.sh` installs it into `.oracle/bin` and checks the version on every run (M0.3).
- `scripts/oracle.sh run <dir>` prints one of:
  - the output of `cue export --out json` for the `.cue` files in `<dir>`, or
  - normalized errors: one line per error, `<kind> <path>`, sorted. Kinds and message patterns are in SPEC §5. An unrecognized message becomes kind `other`, and the case stays `fail` until the mapping covers it.
- Only `scripts/oracle.sh` writes `oracle/*` sections. Expected values for edition v1 always come from the oracle.

## Corpus

- Source: `cue/testdata/**/*.txtar` from cue-lang/cue at v0.17.1: 484 archives in 19 areas (`basicrewrite`, `benchmarks`, `builtins`, `choosedefault`, `compile`, `comprehensions`, `cycle`, `definitions`, `disjunctions`, `eval`, `export`, `fulleval`, `inlinetest`, `interpolation`, `lists`, `packages`, `references`, `resolve`, `scalars`). They are Apache-2.0; `tests/corpus/v1/LICENSE` carries the notice.
- Harvesting (M0.4) keeps each archive's `.cue` sections, drops the Go-specific `out/*` sections, and appends the oracle's result as either

  ```
  -- oracle/export.json --
  {
      "a": 3
  }
  ```

  or

  ```
  -- oracle/error --
  conflict a.b
  incomplete c
  ```

- Cases that use a feature SPEC §3 excludes are `skip`, with the reason.

## Ratchet

`tests/corpus/status.txt` records the expected result of every case:

```
pass scalars/embed.txtar
fail cycle/structural.txtar  # structural cycle detection (M2.7)
skip eval/issue9999.txtar    # uses @experiment
```

The corpus test runs one `libtest-mimic` trial per case. A trial fails when its result differs from its line in either direction: a regression, or a case that now passes while still listed as `fail`. After a task moves cases, run

```
CUECUE_BLESS=1 cargo test -p cuecue-eval --test corpus
```

and commit `status.txt` with the task, so the diff shows what the task achieved. Filter by name as usual: `cargo test -p cuecue-eval --test corpus -- cycle/`.

## Spec tests (edition 2027)

- One directory per SPEC section, e.g. `tests/spec/4.6-functions/`, using the same txtar format.
- Equivalence cases carry the 2027 source plus its v1 translation (`-- v1/*.cue --`), written with the "v1 equivalent" rule of the SPEC section. The oracle's result for the translation is the expected result.
- Cases without a v1 equivalent (new errors, test blocks, exports, incomplete unions) state the expected result by hand in `-- expect/export.json --`, `-- expect/error --` or `-- expect/stdout --`. `CUECUE_BLESS=1` rewrites `expect/*` sections from cuecue's output; read the diff before committing it.
- Full rendered diagnostics (`-- expect/stderr --`) are compared only in `tests/spec/5-diagnostics/`. Everywhere else, tests compare kinds and paths.

## Property tests

`proptest` generates small values: scalars, bounds, structs, lists, and disjunctions with defaults. Required properties:

- `a & b == b & a`, `(a & b) & c == a & (b & c)`, `a & a == a`
- `a & _ == a`, and `a & _|_` is bottom
- `a | b == b | a`, `a | a == a`
- a tagged union equals its v1 equivalent whenever the tag is concrete (SPEC §4.7)
- `fmt(fmt(x)) == fmt(x)` (M6.2)
- `migrate(x)` evaluates to the same result as `x` for every passing corpus case (M12)

## Fuzzing

`cargo fuzz` targets `lex`, `parse` and `eval` (with a step budget) live in `fuzz/`. Lexer, parser and evaluator tasks require 5 minutes per affected target without a crash or hang.

## Debugging a corpus failure

1. Run the case alone: `cargo test -p cuecue-eval --test corpus -- <path> --nocapture`.
2. Run the oracle on the same files: `scripts/oracle.sh run <dir>`.
3. Shrink the input to the smallest file that still differs, and add it as a unit test next to the code you'll change.
4. Fix it, run the whole corpus, and bless `status.txt`.
