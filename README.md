# DjQlint — Django Query Linter

🌍 **English** · [Русский](README.ru.md)

A fast, production-grade static analyzer (written in Rust) that detects
inefficient Django ORM queries and database anti-patterns in Python code,
**without importing or running your project**. It parses source with
[`tree-sitter`](https://tree-sitter.github.io/) and ships first-class
**SARIF** output for GitHub Advanced Security, SonarQube and GitLab.

```text
  ! `if pending:` evaluates the full QuerySet just to test for rows
    ,-[app/views.py:16:8]
 15 |     pending = Book.objects.filter(status="pending")
 16 |     if pending:
    :        ^^^|^^^
    :           `-- truthiness test loads every row
 17 |         notify_admins()
    `----
  help: Use `if pending.exists():` — it issues a cheap `SELECT 1 … LIMIT 1`
        instead of fetching and caching the whole result set.
```

## Rules

| Code   | Name                  | What it catches |
|--------|-----------------------|-----------------|
| `N001` | `n-plus-one`          | Iterating a QuerySet and traversing a relation per row (`for b in qs: b.author.name`) without `select_related`/`prefetch_related`. |
| `P001` | `in-memory-counting`  | `len(queryset)` / `list(queryset)` — pulls every row into Python. Use `.count()` / `.exists()`. |
| `P002` | `redundant-truthiness`| `if queryset:` evaluates and caches the whole result set just to test for rows. Use `.exists()`. |

## Install & run

```bash
cargo build --release

./target/release/djqlint path/to/project        # pretty console output
./target/release/djqlint -f json   .             # machine-readable JSON
./target/release/djqlint -f sarif -o out.sarif . # SARIF for CI
./target/release/djqlint --list-rules
```

Exit code is `1` when findings exist (override with `--exit-zero`), `2` on
internal error, `0` when clean — ready to gate a CI job.

## Configuration — `.djqlint.toml`

```toml
[rules]
disabled = ["P002"]          # turn rules off by code

[files]
exclude = ["/migrations/", "/tests/"]
respect_gitignore = true     # default
```

## Suppressing findings inline

```python
total = len(qs)                       # noqa: djqlint-P001   (one rule)
total = len(qs)                       # noqa                 (all rules on line)
total = len(qs)                       # djqlint: disable     (all djqlint rules)
total = len(qs)                       # djqlint: disable=P001,N001
```

## Architecture

```
src/
├── main.rs              # thin binary: parse args -> djqlint::run
├── lib.rs              # run(): orchestrates config, walk, analyze, report
├── cli.rs              # clap (derive) CLI definition
├── config.rs           # .djqlint.toml (serde + toml)
├── diagnostic.rs       # Diagnostic / Severity / FileReport (shared types)
├── engine/
│   ├── mod.rs          # FileUnit context + parallel driver (rayon)
│   ├── walker.rs       # .gitignore-aware FS walk (ignore crate)
│   ├── parser.rs       # tree-sitter parsing + query compilation
│   ├── queryset.rs     # heuristic QuerySet data-flow model (variable tracking)
│   ├── suppress.rs     # noqa / djqlint:disable directive parsing
│   └── util.rs         # AST helpers (node text, pre-order cursor walk)
├── rules/
│   ├── mod.rs          # `trait Rule`, registry, shared query helpers
│   ├── queries/*.scm   # tree-sitter S-expression patterns (include_str!)
│   ├── n001_nplus1.rs
│   ├── p001_in_memory_count.rs
│   └── p002_redundant_truthiness.rs
└── reporters/
    ├── console.rs      # miette graphical report (source-annotated)
    ├── json.rs         # stable JSON
    └── sarif.rs        # SARIF 2.1.0
```

### How analysis flows

1. **Walk** (`engine::walker`) — `ignore::WalkBuilder` collects `*.py`,
   honouring `.gitignore` and config excludes.
2. **Fan out** (`engine::analyze_files`) — `rayon` runs every file in parallel.
   Each file gets a fresh `tree-sitter` parser (parsers are `Send`, not `Sync`).
3. **Parse + model** — `tree-sitter` is error-recovering, so malformed Python
   never panics; we then build a conservative `Bindings` model that tracks which
   local variables hold a QuerySet (`qs = User.objects.filter(...)`), to a
   fixpoint so transitive assignments are captured.
4. **Run rules** — each `Rule` owns a pre-compiled tree-sitter `Query` (immutable
   and `Sync`, so it is shared across threads). Rules match coarse syntactic
   shapes via the query, then refine using the data-flow model.
5. **Suppress** — findings on lines carrying a `noqa`/`djqlint:disable` directive
   are dropped.
6. **Report** — console (miette), JSON, or SARIF.

### Adding a rule

1. Drop an S-expression in `src/rules/queries/<code>.scm`.
2. Implement `trait Rule` (compile the query in `new()` with
   `include_str!`, match in `check()`), see `p001_in_memory_count.rs` for the
   canonical pattern.
3. Register it in `rules::build_rules`.

## CI

* **GitHub Actions** — `.github/workflows/ci.yml` builds, lints, tests, then
  uploads SARIF to code scanning.
* **GitLab CI** — `.gitlab-ci.yml` emits SARIF as a SAST report.

## Development

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo run -- examples/views.py        # demo fixture exercising every rule
```

## License

MIT.
