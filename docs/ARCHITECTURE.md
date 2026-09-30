# Architecture

## Pipeline

```
source files ─▶ lexer ─▶ parser ─▶ lossless syntax tree (rowan)        cuecue-syntax
                                          │
                                          ▼
                          lowering: scopes, 2027 desugaring             cuecue-eval
                                          │
                                          ▼
                          IR: one core for both editions
                                          │
                                          ▼
                          evaluator ─▶ value graph ─▶ Value API
                                                         │
                          export (JSON, YAML) · diagnostics · why · test
```

The host (the CLI now; the language server and bindings later) reads files and inputs and hands them to `cuecue-eval` in memory.

## Crates

| Crate | Owns | Added in |
|---|---|---|
| `cuecue-syntax` | tokens, lexer, parser, syntax tree, typed AST; formatter (M6) | M0 |
| `cuecue-eval` | lowering, IR, evaluator, builtins and std, Value API, diagnostics data, JSON export | M0 |
| `cuecue` | CLI binary: loads files and inputs, renders diagnostics | M0 |
| `cuecue-lsp` | language server | M8 |
| `cuecue-encoding` | YAML, JSON Schema, OpenAPI, protobuf | M9 |
| `cuecue-mod` | manifest, lockfile, registry client | M10 |
| `cuecue-capi`, `cuecue-wasm`, `bindings/*` | embedding | M11 |

Split a crate further only when compile times or a second consumer require it, and record the split in DECISIONS.md.

## Invariants

1. Editions exist only in syntax and lowering. The IR and evaluator never branch on edition, except for the diagnostic in SPEC §4.11, which reads a flag on the conjunct.
2. `cuecue-eval` is pure. Its input is in-memory files plus supplied inputs; its output is values and diagnostics. It touches no filesystem, environment, clock, network or randomness, and holds no global mutable state.
3. Every IR node and every conjunct carries a span (file id plus byte range). Provenance comes from conjuncts, never from re-reading text.
4. Output is deterministic: the same input bytes give the same output bytes. Use ordered maps (`indexmap`) wherever order is observable, and match the oracle's field order for v1.
5. Errors are values. Bottom carries structured diagnostics; rendering happens at the edge (CLI, LSP).
6. Code reachable from user input returns errors; internal invariants use `debug_assert!`.
7. Identifiers and labels are interned as `u32` ids, and vertices live in arenas indexed by `u32`.

## Evaluator

M2.1 expands this sketch into a reviewed design.

- **Vertex:** one per field path of an evaluated value. Holds its conjuncts, its arcs (child fields, in order), its scalar and bound state, closedness information, and a status: unevaluated, evaluating, or final.
- **Conjunct:** an IR expression with its environment, span, definition context and provenance chain.
- **Environment:** the lexical scope chain. References resolve to vertices, never to copies.
- **Scheduling:** evaluating a vertex runs one task per conjunct. A task that needs something not yet known, such as a field set or a concrete scalar, waits on that vertex. When nothing can make progress, waiting tasks become `incomplete` or `cycle` diagnostics.
- **Disjunctions:** disjuncts evaluate lazily as forks of the vertex, are dropped at their first conflict, and are deduplicated. Defaults follow rules M0–M3 of the CUE spec. Tagged unions (SPEC §4.7) never fork.
- **Closedness:** each definition contributes a set of allowed labels; a field is allowed when every closed group of conjuncts allows it.
- **Sharing:** a reference to a final vertex that gains no extra conjuncts shares that vertex. Leaf values are hash-consed.
- **Functions:** a call instantiates the body with parameter bindings (SPEC §4.6); results are cached by argument.
- **Numbers:** see D-015.

Before designing, read: the CUE spec and `doc/ref/impl.md` at v0.17.1 (the formal model, typed feature structures), and the Go evaluator in `internal/core/adt` at v0.17.1 for semantic edge cases (D-005).

## Diagnostics

- `cuecue-eval` defines the data of SPEC §5: `Diagnostic { kind, path, message, labels, notes, help }`, where a label is `{ span, note, primary }` and a help entry may carry a suggested edit `{ span, replacement }`.
- Diagnostics are built from conjunct provenance inside the evaluator; `why` reads the same provenance.
- The CLI renders with `annotate-snippets` (D-012): the kind is the title id (`error[conflict]`), each file becomes a snippet, primary and secondary labels become annotations, notes and help become messages, and suggested edits become patches. `--diagnostics=json` serializes the data instead.
- The language server maps the primary label to the diagnostic range, the other labels to `relatedInformation`, and suggested edits to quick-fix code actions.

## Dependencies

Approved crates (D-014). Add each one in the milestone listed, not earlier. Exact versions live in `Cargo.lock`.

| Area | Crate | Milestone | Notes |
|---|---|---|---|
| Syntax tree | `rowan` | M1 | lossless tree from rust-analyzer; `cstree` is the fallback if the language server needs `Send` trees |
| Lexer, parser | none | M1 | hand-written (D-004) |
| Ordered maps, hashing | `indexmap`, `rustc-hash` | M0 | |
| Arenas | `la-arena` | M2 | typed `u32` indices |
| Numbers | `dashu` | M2 | D-015 |
| Regular expressions | `regex` | M2 | RE2-style syntax like Go's `regexp`; record each syntax difference as a test |
| JSON | `serde_json` | M2 | export and `encoding/json` |
| Diagnostics rendering | `annotate-snippets` | M3 | D-012 |
| "Did you mean" | `strsim` | M3 | closest field or identifier for `closed` and `reference` errors |
| CLI | `clap` | M0 | |
| Host-side errors | `thiserror` in libraries, `anyhow` in the CLI | M0 | user-facing diagnostics use SPEC §5, not these |
| Std: time | `jiff` | M5 | Go layout strings need our own formatter |
| Std: hashes, encodings | `sha2`, `sha1`, `md-5`, `hmac`, `ed25519-dalek`, `base64`, `hex`, `uuid` | M5 | |
| Std: net, text, csv, html | `ipnet`, `tabwriter`, `csv`, `html-escape` | M5 | `tabwriter` is a port of Go's package |
| Std: `text/template` | none yet | M5 | Q8 |
| YAML | `saphyr`, then our own emitter | M5 | D-016 |
| TOML | `toml` | M5 | |
| Protobuf | `protox`, `prost-reflect` | M9 | parses `.proto` without `protoc` |
| OpenAPI import | `oas3` | M9 | generating OpenAPI needs only `serde_json` |
| Incremental engine | `salsa` | M8 | D-013 |
| Language server | `tower-lsp-server`, or `lsp-server` with `lsp-types` | M8 | choose in M8.2; `tower-lsp` was last released in 2023 |
| Registry | `oci-client` | M10 | CUE modules are stored in OCI registries |
| Plugins | `wasmi` | M11 | pure-Rust WASM interpreter with fuel metering, usable inside the WASM build; `wasmtime` only if speed requires it |
| Bindings | `cbindgen`, `pyo3` with `maturin`, `wasm-bindgen`, `napi` | M11 | Go uses cgo over the C ABI |
| Tests | `insta`, `proptest`, `libtest-mimic`, `libfuzzer-sys`, `arbitrary` | M0–M2 | |
| Benchmarks | `criterion` | M7 | end-to-end comparisons use the `hyperfine` CLI |
