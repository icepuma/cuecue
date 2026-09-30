# Decisions

Settled choices first, open questions last. Add an entry when you make a choice another agent could reasonably make differently. Status is **Accepted**, **Proposed** (waiting for the human), or **Superseded by D-xxx**. Keep entries short; the reason matters more than the choice.

## D-001 Two editions over one core

**Accepted.** A file is edition v1 (CUE v0.17.1) or edition 2027 ([SPEC](SPEC.md) §4). Both lower to one IR and one evaluator.

- Why: people value CUE's semantics and dislike its surface. One core lets the oracle verify the semantics and lets v1 and 2027 packages import each other.
- Consequence: every 2027 feature in SPEC states its v1 equivalent, or states that it has none.

## D-002 The oracle is `cue` v0.17.1

**Accepted.** For edition v1, the oracle's behavior settles anything the CUE spec leaves open. v0.17.1 was the latest stable release on 2026-09-30.

- Why: the spec has TODOs and gaps; the binary is precise.
- Consequence: moving to a newer oracle is a (review) task that re-harvests the corpus and re-blesses `status.txt`.

## D-003 Edition v1 first, then 2027

**Accepted.** M2 builds the v1 evaluator against the corpus; M4 adds 2027 on top.

- Why: the oracle gives v1 an exact target. 2027 features then build on a verified core, and most can be tested through their v1 equivalents.

## D-004 Lossless syntax tree, hand-written parser

**Accepted.** A `rowan` tree built by a hand-written lexer and recursive-descent parser.

- Why: the formatter, language server and migration tool need comments and whitespace. CUE's nested string interpolation and automatic commas are simpler to lex by hand than with a generator such as `logos`.

## D-005 A new evaluator, designed from the formal model

**Accepted.** Design from the CUE spec and `doc/ref/impl.md` at v0.17.1. Read the Go evaluator (`internal/core/adt`) to answer semantic questions, not as a structure to port.

- Why: the CUE team has said its earlier evaluator structure capped performance; a design built for speed is the point of this project.
- Consequence: M2.1 is a design task the human reviews.

## D-006 No overrides

**Accepted.** Defaults (`*`) remain the only values something else can replace.

- Why: every value stays traceable to its sources. This declines a request Pkl users raised about CUE.

## D-007 Functions are top-level, not values, and never recursive

**Accepted.** SPEC §4.6. Calls resolve in a namespace separate from fields.

- Why: closes the most requested ergonomic gap while keeping termination and the lattice semantics. Function values and higher-order functions wait for Q4.

## D-008 Tagged unions evaluate one arm

**Accepted.** SPEC §4.7.

- Why: disjunction search is the performance problem CUE users hit most, and a tag field is how most configurations already tell variants apart.

## D-009 Hermetic evaluation, no task runner

**Accepted.** Values enter only through source files and declared inputs (SPEC §4.8). There is no `cue cmd`, no tool files and no `tool/*` packages; in v1 packages, tool files are ignored with a warning.

- Why: Dagger's experience shows users want side effects in the language they already use. Hermetic evaluation keeps results reproducible and cacheable.

## D-010 Directory packages without ancestor merging

**Accepted.** A 2027 package is the files of one directory that share a package clause, all in one edition (SPEC §2, §4.12).

- Why: directory packages are what let policy live in its own file. Merging files from parent directories surprises readers and makes a package hard to read on its own.
- Note: this refines the published proposal, which suggested per-file imports instead of directory packages.

## D-011 Dependency versions live in the manifest

**Accepted.** `use` paths never carry versions (SPEC §4.4).

- Why: one place to change a version, with a lockfile pinning the rest.
- Note: this refines the published proposal's example, which put a version in the import path.

## D-012 Diagnostics are data; `annotate-snippets` renders them

**Accepted.** SPEC §5 defines the data. The CLI renders it with `annotate-snippets`; the language server maps it to LSP diagnostics.

- Why `annotate-snippets`: maintained by the Rust project, renders rustc-style reports, and covers what SPEC §5 needs: labels from several files in one report, primary and secondary labels, notes, help, error codes with links, and suggested edits rendered as patches.
- Rejected: `miette` (labels from several files in one diagnostic are still an open issue, zkat/miette#193), `ariadne` (multi-file works, but no suggested-edit rendering; the repository moved to Codeberg), `codespan-reporting` (multi-file works, but no suggested-edit rendering).

## D-013 Pure evaluator now, `salsa` at M8

**Accepted.** `cuecue-eval` stays a pure function of files and inputs; M8 wraps it in `salsa` queries.

- Why: incremental computation needs pure queries, and adding salsa before the evaluator settles would double the work.

## D-014 Dependencies come from the approved list

**Accepted.** [ARCHITECTURE](ARCHITECTURE.md) §Dependencies lists approved crates and the milestone that adds each. Anything else needs a Proposed entry here first.

- Why: fewer crates, faster builds, smaller supply-chain surface.

## D-015 Numbers match the oracle's decimals

**Proposed** until M2.1 confirms it. Integers are arbitrary precision. Floats are decimals with 34 significant digits, like the oracle (`apd.BaseContext.WithPrecision(34)` in `internal/internal.go`). First candidate: `dashu` (integers and decimal floats with configurable precision and rounding). Fallback: `bigdecimal` with `num-bigint`.

- Rejected: `rust_decimal` (28 digits, too few).
- Still open: the oracle's rounding mode and how integer arithmetic interacts with the 34-digit context. M2.1 settles both with oracle tests.

## D-016 YAML: parse with `saphyr`, emit with our own writer

**Accepted.** `saphyr` parses (YAML 1.2, the successor to `yaml-rust2`, which is the fallback). Output uses our own emitter so `export --out yaml` can match the oracle byte for byte.

- Rejected: `serde_yaml` (deprecated since 2024), `serde_yml` (archived), `serde_yaml_ng` and `serde_norway` (no releases since 2024).

## D-017 Name

**Accepted.** The project, crates and binary are `cuecue`. Source files keep the `.cue` extension; the first line tells editions apart.

- cuecue is an independent project, not affiliated with the CUE project or CUE Labs.

## D-018 Align 2027's `self` with CUE's `aliasv2` experiment

**Proposed** (found during M0.4). CUE v0.17.1 has an `aliasv2` experiment that adds `self` with the meaning of SPEC §4.5, and also allows it inside list literals: `items: [1, 2, self[0]]` refers to the list (corpus `references/self.txtar`, skipped for v1 because it is an experiment). Proposal: change §4.5 to "the innermost enclosing struct or list literal", so 2027 matches upstream where both have `self`.

- Why: if upstream stabilizes `self`, v1 and 2027 agree instead of diverging.

## Open questions

These belong to the human. An agent that needs one answered writes a Proposed answer below it, tells the human, and moves to another task.

- **Q1 License.** Suggestion: Apache-2.0, matching the vendored corpus. Needed before the first public release.
- **Q2 Modules.** Manifest format, lockfile format, and whether to stay protocol-compatible with CUE's module registry. Needed before M10.
- **Q3 Embedding data files.** Should 2027 read JSON and YAML files from the module, like v1's `@embed`? Should v1 `@embed` and `@extern` be supported?
- **Q4 Richer functions.** Function values, higher-order functions, default parameters, named arguments. Not before 1.0.
- **Q5 Test inputs.** Should a test block be able to supply its own inputs?
- **Q6 v1 tag injection.** Support `@tag`, `@if` and `-t` for v1 compatibility?
- **Q7 Standard library names.** Keep v1's Go-style names (`strings.HasPrefix`) in 2027, or add a new naming style?
- **Q8 `text/template`.** No maintained Rust crate implements Go templates (`gtmpl` was last released in 2021). Implement a subset, or leave the package out?
- **Q9 Upstream experiments.** CUE v0.17.1 experiments overlap with 2027: `aliasv2` (`self`, D-018), `try`, `structcmp`, `explicitopen` and `testing`. When an experiment covers a 2027 feature, should 2027 adopt its syntax and meaning?
