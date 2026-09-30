# Roadmap

How this file works:

- The current milestone is the lowest-numbered one with an open task whose dependencies are done.
- Within a milestone, do tasks in order unless the milestone says they are parallel.
- A task should fit one agent session. If it won't, split it in place (M2.5a, M2.5b) before starting.
- **(review)** tasks end with the human's approval.
- Mark finished tasks `[x]`. A one-line note under a task for the next agent is welcome.

## M0 Bootstrap

Depends on: nothing.

- [x] **M0.1 Workspace.** Cargo workspace with `crates/cuecue-syntax`, `crates/cuecue-eval` and `crates/cuecue` (the binary); `rust-toolchain.toml` pinned to the current stable release; edition 2024; workspace lints with `unsafe_code = "forbid"`; `.gitignore` covering `target/` and `.oracle/`. *Done when* `cargo build`, `cargo test` and `cargo clippy --workspace --all-targets -- -D warnings` pass, and `cuecue --version` prints the version.
- [ ] **M0.2 CI.** GitHub Actions running the format check, clippy and tests on Linux and macOS, caching the oracle binary. *Done when* a pull request shows every check green.
- [ ] **M0.3 Oracle.** `scripts/oracle.sh` installs `cue` v0.17.1 into `.oracle/bin` (`GOBIN=$PWD/.oracle/bin go install cuelang.org/go/cmd/cue@v0.17.1`), refuses any other version, and implements `run <dir>` as described in TESTING.md. Check the message patterns in SPEC §5 against real oracle output and correct the table. *Done when* two runs on the same input produce identical bytes, and `tests/oracle-fixtures/` holds at least one input per v1 kind whose normalized output names that kind.
- [ ] **M0.4 Corpus.** `scripts/harvest.sh` checks out cue-lang/cue at v0.17.1, copies every `cue/testdata/**/*.txtar` into `tests/corpus/v1/` (keeping `.cue` sections, dropping `out/*`), copies CUE's LICENSE, and appends oracle sections. *Done when* all 484 archives are present, each with an oracle section or a skip reason, and rerunning the script changes nothing.
- [ ] **M0.5 Runner and ratchet.** `crates/cuecue-eval/tests/corpus.rs` built on `libtest-mimic`: one trial per archive, checked against `tests/corpus/status.txt` as described in TESTING.md. With the evaluator stubbed, every case is `fail`. *Done when* `cargo test` passes, editing any status line makes it fail, and `CUECUE_BLESS=1` rewrites `status.txt`.

## M1 Syntax, edition v1

Depends on: M0.

- [ ] **M1.1 Lexer.** Every token in the CUE spec's "Lexical elements": identifiers and keywords, number literals with multipliers (`1Ki`), string and bytes literals (single-line, multi-line, `#`-delimited, with interpolation), attributes, comments, and automatic comma insertion. *Done when* every corpus `.cue` file lexes without error tokens (except cases whose oracle result is `syntax`), each example in that spec section is a unit test, and the `lex` fuzz target runs 5 minutes clean.
- [ ] **M1.2 Parser.** Hand-written recursive descent that builds a `rowan` tree and recovers from errors. *Done when* every corpus file parses without diagnostics (except the oracle's `syntax` cases, which get at least one `syntax` diagnostic), `cuecue parse --tree <file>` prints the tree, and the `parse` fuzz target runs 5 minutes clean.
- [ ] **M1.3 Typed AST.** Typed accessors for every production of the CUE spec grammar. *Done when* printing the tree of any corpus file reproduces the file byte for byte.

## M2 Evaluator, edition v1

Depends on: M1.

- [ ] **M2.1 Evaluator design (review).** Expand ARCHITECTURE.md §Evaluator into a design covering vertices and conjuncts, scheduling, disjunctions and defaults, closedness, cycles, sharing, and numbers. Settle D-015 by running number edge cases through the oracle. *Done when* the human approves the design.
- [ ] **M2.2 Tracer bullet.** Lowering to IR, scalars, arithmetic, comparisons, bounds, `&` and `|` on scalars, JSON export, and `cuecue export <file>`. *Done when* the `scalars` area passes and `cuecue export` matches the oracle byte for byte on `a: 1 + 2`.
- [ ] **M2.3 Structs and references.** Regular, optional, required and hidden fields; embedding; lexical references; `let`; aliases; selectors and indexing. *Done when* the `references` and `resolve` areas pass.
- [ ] **M2.4 Definitions and closedness.** `#X`, `close()`, `...`, pattern constraints. *Done when* the `definitions` area passes.
- [ ] **M2.5 Disjunctions and defaults.** Including struct disjunctions and default rules M0–M3. *Done when* the `disjunctions` and `choosedefault` areas pass.
- [ ] **M2.6 Lists and comprehensions.** Open and closed lists, `for`, `if` and `let` clauses, dynamic fields, interpolation. *Done when* the `lists`, `comprehensions` and `interpolation` areas pass.
- [ ] **M2.7 Cycles.** Reference and structural cycles as in the CUE spec's "Cycles" section. *Done when* the `cycle` area passes.
- [ ] **M2.8 Builtins.** The CUE spec's builtin functions and validators. *Done when* the `builtins` cases that import no std package pass.
- [ ] **M2.9 Packages.** Package clauses, multi-file packages, ancestor merging, imports within a module, and a minimal `cue.mod/module.cue`. *Done when* the `packages` area passes.
- [ ] **M2.10 Remaining areas.** `eval`, `fulleval`, `export`, `compile`, `basicrewrite`, `benchmarks`, `inlinetest`. *Done when* at least 90% of the corpus passes and every `fail` line in `status.txt` has a reason.

## M3 Diagnostics

Depends on: M2.3. Runs in parallel with the rest of M2.

- [ ] **M3.1 Model and rendering.** The diagnostic data of SPEC §5, `annotate-snippets` rendering in the CLI, and `--diagnostics=json`. *Done when* every kind has a spec test with its rendered output.
- [ ] **M3.2 Provenance.** A label for every contributing conjunct, including those reached through definitions, patterns, embeddings, comprehensions and defaults. *Done when* the conflict example in SPEC §5 renders with both labels and the note.
- [ ] **M3.3 `cuecue why`.** *Done when* it prints the SPEC §5 example for the §4.0 package.
- [ ] **M3.4 Suggestions.** "Did you mean" help for `closed` and `reference` errors. *Done when* a misspelled field in a closed struct suggests the closest allowed field as a suggested edit.

## M4 Edition 2027

Depends on: M2, M3.1.

Each task is done when `tests/spec/<section>/` covers every rule and error of its SPEC section and passes.

- [ ] **M4.1 Edition clause and keywords** (SPEC §2, §4.1, §4.2).
- [ ] **M4.2 Interpolation** (§4.3).
- [ ] **M4.3 `use` declarations** (§4.4).
- [ ] **M4.4 `self`** (§4.5).
- [ ] **M4.5 Functions** (§4.6).
- [ ] **M4.6 Tagged unions** (§4.7), including the equivalence property test.
- [ ] **M4.7 Inputs** (§4.8) and their CLI flags.
- [ ] **M4.8 Exports** (§4.9).
- [ ] **M4.9 Tests** (§4.10) and `cuecue test`.
- [ ] **M4.10 Conflicting defaults and packages** (§4.11, §4.12).
- [ ] **M4.11 Acceptance.** Build `examples/deploy/` from SPEC §4.0. Write its v1 translation by hand and use the oracle's export of the translation as the expected output. *Done when* `cuecue export ./examples/deploy --input version=v1.2.3` matches it, `cuecue test ./examples/deploy --input version=v1.2.3` passes, and adding the `hotfix.cue` from SPEC §5 produces that diagnostic.

## M5 Standard library

Depends on: M2.8. Tasks run in parallel.

Port the v0.17.1 std packages except `tool/*`. Each task is done when every corpus case that uses its packages passes and each function has unit tests for its documented behavior.

- [ ] **M5.1** `strings`, `strconv`, `regexp`, `list`, `struct`, `path`
- [ ] **M5.2** `math`, `math/bits`
- [ ] **M5.3** `encoding/json`, `encoding/yaml`, `encoding/toml`, `encoding/csv`, `encoding/base64`, `encoding/hex`
- [ ] **M5.4** `crypto/sha256`, `crypto/sha512`, `crypto/sha1`, `crypto/md5`, `crypto/hmac`, `crypto/ed25519`
- [ ] **M5.5** `time`, `net`, `uuid`, `html`, `text/tabwriter`
- [ ] **M5.6** `text/template` (blocked on Q8)

## M6 CLI and formatter

Depends on: M2.2 (M6.2 needs only M1).

- [ ] **M6.1 Commands.** `eval`, `export` (`--out json|yaml`, `-e <expr>`), `vet`, `fmt`, `test`, `why`, `parse`; exit codes; `-` for stdin. *Done when* a curated subset of the oracle's CLI testscripts (`cmd/cue/cmd/testdata/script`, 422 at v0.17.1), with `cue` replaced by `cuecue`, passes. List the subset and the reasons for exclusions in `tests/cli/README.md`.
- [ ] **M6.2 Formatter.** Keeps comments; idempotent. *Done when* `cuecue fmt` output equals the oracle's `cue fmt` output for every corpus file, and formatting twice changes nothing.
- [ ] **M6.3 `eval` output.** *Done when* `cuecue eval` matches `cue eval` on every corpus case where both succeed.
- [ ] **M6.4 REPL.** `cuecue repl`. *Done when* it evaluates expressions against a loaded package and shows diagnostics.

## M7 Performance

Depends on: M2.

- [ ] **M7.1 Benchmarks.** `scripts/bench.sh` compares `cuecue` with the oracle using `hyperfine` on the `benchmarks` area and on a generated Kubernetes-style configuration with 10, 100 and 1000 services; `criterion` benches cover evaluator hot paths. Results go in `docs/PERF.md`. *Done when* the script runs in CI on demand and PERF.md records a baseline.
- [ ] **M7.2 Sharing.** Hash-consed leaf values and shared final vertices.
- [ ] **M7.3 Disjunctions.** Early pruning, deduplication, caching of calls and disjuncts.
- [ ] **M7.4 Tagged union fast path.** *Done when* a benchmark shows union cost independent of the number of arms.

M7 is done when every benchmark runs at least as fast as the oracle with lower peak memory.

## M8 Language server

Depends on: M3, M6.2.

- [ ] **M8.1 Incremental engine.** Parsing, lowering and package evaluation become `salsa` queries. *Done when* a test shows that editing one file re-runs only the queries that depend on it.
- [ ] **M8.2 LSP.** Diagnostics (secondary labels as related information), hover with the value and its `why`, go to definition, formatting, field-name completion from schemas, quick fixes from suggested edits. Choose the LSP crate first and record it in DECISIONS.md. *Done when* a generic LSP client exercises each feature on `examples/deploy`.
- [ ] **M8.3 Editor package.** A VS Code extension with a TextMate grammar for both editions.

## M9 Encodings

Depends on: M5.

- [ ] **M9.1 YAML** import and export (D-016); `export --out yaml` matches the oracle.
- [ ] **M9.2 JSON Schema** import and export.
- [ ] **M9.3 OpenAPI** generation (`encoding/openapi`) and import.
- [ ] **M9.4 Protobuf** import.

Each is done when it matches the oracle's `cue import`, `cue def` or `cue export` on a harvested sample.

## M10 Modules

Depends on: M4.3 and an answer to Q2.

- [ ] **M10.1 Manifest and lockfile** as decided in Q2.
- [ ] **M10.2 Registry client** built on `oci-client`, with an offline cache.
- [ ] **M10.3 Resolution** for `use` (2027) and `import` (v1). *Done when* a package that depends on a published CUE module builds offline from the cache and verifies lockfile hashes.

## M11 Embedding

Depends on: M4.

- [ ] **M11.1 Rust API.** A stable public API in `cuecue-eval` (session, inputs, values, diagnostics) with documented examples.
- [ ] **M11.2 C ABI.** `cuecue-capi` with a `cbindgen` header. This is the only crate allowed `unsafe`.
- [ ] **M11.3 WASM and JavaScript.** A `wasm-bindgen` package for browsers and Node (add `napi` only if WASM proves too slow).
- [ ] **M11.4 Python.** A `pyo3` wheel built with `maturin`.
- [ ] **M11.5 Go.** A cgo package over the C ABI.
- [ ] **M11.6 Plugins.** Custom builtins as WASM modules run by `wasmi` with fuel limits.

M11 is done when every binding passes one shared smoke suite in `tests/bindings/`.

## M12 Migration

Depends on: M4, M6.2.

- [ ] **M12.1 `cuecue migrate`.** Rewrites v1 packages to 2027 by applying SPEC §4's v1-equivalent rules in reverse, and reports what it can't convert (tool files, ancestor merging, experiments). *Done when* every passing corpus case migrates and evaluates to identical output, and migrated files are unchanged by `cuecue fmt`.

## M13 1.0

Depends on: all of the above.

- [ ] **M13.1 Spec freeze (review).** Edition 2027 is frozen, and every open question is answered or explicitly deferred.
- [ ] **M13.2 Release.** Versioned binaries for Linux, macOS and Windows, published crates and packages, and a documentation site.
