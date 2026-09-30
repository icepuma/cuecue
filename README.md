# cuecue

A Rust implementation of the [CUE](https://cuelang.org) configuration language, and a proposed next edition of it.

**Status:** planning. There is no code yet; work starts at M0 in [the roadmap](docs/ROADMAP.md).

- **Edition v1** matches CUE v0.17.1 and is checked case by case against the official `cue` binary.
- **Edition 2027** keeps CUE's unification semantics and adds functions, tagged unions, typed inputs, built-in tests, `${}` interpolation, and error messages that show where every value came from.
- **One engine everywhere:** the CLI, the language server, and bindings for C, WASM/JavaScript, Python and Go share one evaluator.

| Doc | What's in it |
|---|---|
| [Vision](docs/VISION.md) | goals, non-goals, and what CUE users have said |
| [Spec](docs/SPEC.md) | edition v1 scope, edition 2027 features, diagnostics |
| [Roadmap](docs/ROADMAP.md) | milestones and tasks |
| [Architecture](docs/ARCHITECTURE.md) | pipeline, crates, evaluator sketch, dependencies |
| [Testing](docs/TESTING.md) | the oracle, the corpus, and how progress is measured |
| [Decisions](docs/DECISIONS.md) | settled choices and open questions |

Coding agents start at [AGENTS.md](AGENTS.md).

cuecue is an independent project, not affiliated with the CUE project or CUE Labs.
