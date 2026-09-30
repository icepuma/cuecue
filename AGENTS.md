# cuecue

cuecue is a Rust implementation of the CUE configuration language plus a new edition, 2027, built on the same semantics. Edition v1 must behave exactly like the pinned Go `cue` binary, the **oracle**. Edition 2027 adds declarations and rules that lower to the same core.

## Docs

- `docs/ROADMAP.md`: where work comes from. Milestones, tasks, and each task's *Done when*.
- `docs/SPEC.md`: read the sections for any language behavior you touch. For edition v1 it defers to the pinned CUE spec and the oracle.
- `docs/ARCHITECTURE.md`: read before adding a crate, module or dependency, or changing how crates talk to each other.
- `docs/TESTING.md`: read before writing tests, touching `tests/`, or debugging a corpus failure.
- `docs/DECISIONS.md`: settled choices and open questions. Read it before changing anything it covers.
- `docs/VISION.md`: goals, non-goals, and the research behind them.

## Work loop

1. **Pick.** If the human gave you a task ID, take it. Otherwise take the first open task in the current milestone of `docs/ROADMAP.md`. Done when you can state its *Done when* line and name the SPEC sections it touches.
2. **Red.** Write the test that proves the task, and watch it fail for the right reason.
3. **Green.** Make the smallest change that passes it.
4. **Check.** Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`. Done when all three are clean and the corpus ratchet reports no regressions.
5. **Record.** Tick the task in `docs/ROADMAP.md`, with a one-line note if the next agent needs it. Put new design choices in `docs/DECISIONS.md`. If you clarified language behavior, update `docs/SPEC.md` in the same commit.
6. **Commit.** One task per commit, with the message starting with the task ID, e.g. `M2.3: resolve lexical references`.

## Rules

- For edition v1, the oracle is ground truth. Only `scripts/oracle.sh` writes the `oracle/*` sections of the corpus.
- These need the human: **(review)** tasks, changes to edition 2027 in SPEC, crates missing from ARCHITECTURE §Dependencies, and the open questions in DECISIONS. Write your proposal into DECISIONS as Proposed, tell the human, and move to another task.
- Malformed input produces diagnostics. Code reachable from user input returns errors instead of panicking.
- `cuecue-eval` stays pure: file reading, environment, clock and network live in the CLI and other hosts.
- `unsafe` lives only in the C ABI crate (M11); every other crate forbids it.
