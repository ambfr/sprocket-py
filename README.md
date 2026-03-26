# sprocket-py

Python bindings for [Sprocket](https://github.com/stjude-rust-labs/sprocket)'s WDL tooling, built with [PyO3](https://pyo3.rs/).

Exposes Sprocket's WDL parser and linter to Python developers — no Rust knowledge required.

```python
import sprocket_py

doc, errors = sprocket_py.parse(source)   # parse WDL, get version + diagnostics
warnings   = sprocket_py.lint(source)     # run lint checks, get rule violations
```

---

## Why this exists

WDL (Workflow Description Language) is widely used in bioinformatics to describe analysis pipelines. Sprocket is St. Jude's Rust-based WDL toolchain — it has a full parser, linter, validator, and LSP. But all of this is only accessible from Rust. Python developers (the majority of the bioinformatics community) have no way to programmatically parse or lint WDL files without shelling out to the command line.

sprocket-py bridges that gap.

---

## Installation

You need Rust and [maturin](https://github.com/PyO3/maturin) installed.

```bash
# Install maturin
pip install maturin

# Clone and build
git clone https://github.com/ambfr/sprocket-py
cd sprocket-py
maturin develop
```

For `lint()` to work, `sprocket` must also be installed and on your PATH:

```bash
cargo install sprocket
```

---

## Usage

### Parsing

```python
import sprocket_py

source = """
version 1.1

workflow hello {
    call say_hello
}

task say_hello {
    command <<< echo "Hello" >>>
    output { String out = read_string(stdout()) }
}
"""

doc, errors = sprocket_py.parse(source)

print(doc.version)          # "1.1"
print(len(errors))          # 0

for e in errors:
    print(e.severity)       # "error" | "warning" | "note"
    print(e.message)
```

### Linting

```python
warnings = sprocket_py.lint(source)

for w in warnings:
    print(w.rule)           # e.g. "MissingMeta"
    print(w.message)        # human-readable description
```

---

## API Reference

### `sprocket_py.parse(source: str) -> (ParsedDocument, list[Diagnostic])`

Parses a WDL source string. Always returns a tuple — even if the document has errors, a best-effort `ParsedDocument` is returned alongside the diagnostics. This mirrors `wdl-ast`'s design.

**ParsedDocument**
- `version: str` — WDL version string (e.g. `"1.1"`), or `"unknown"` if no version statement is present

**Diagnostic**
- `severity: str` — `"error"`, `"warning"`, or `"note"`
- `message: str` — human-readable description of the issue

---

### `sprocket_py.lint(source: str) -> list[LintWarning]`

Runs lint checks on a WDL source string. Currently implemented as a thin wrapper around the `sprocket lint` CLI — see [Development Notes](#development-notes) for details.

**LintWarning**
- `rule: str` — lint rule ID (e.g. `"MissingMeta"`, `"CommandSingleQuote"`)
- `message: str` — human-readable description

---

## Development Notes

These notes document decisions and API discoveries made during prototype development. They're here so the commit history makes sense and to inform the GSoC implementation work.

### The standalone `wdl` crate is archived

When I started, the obvious dependency was the `wdl` crate on crates.io. After trying to build against it, I found it was archived in September 2025 — all development moved to the Sprocket monorepo (`stjude-rust-labs/sprocket`). The correct crates are now `wdl-ast` and `wdl-lint` under `crates/` in that monorepo. `Cargo.toml` depends on them via a git dependency pointing at the live repo.

### Version extraction required reading wdl-ast source

I initially tried calling `.version()` directly on `Document`, which doesn't exist. Reading `crates/wdl-ast/src/lib.rs` in the Sprocket monorepo revealed the correct path: `document.version_statement()` returns `Option<VersionStatement>`, and you then call `.version().text()` via the `AstToken` trait. The `use wdl_ast::AstToken` import is required for `.text()` to be in scope — without it the compiler gives a confusing "method not found" error.

### The import name mismatch (hyphen vs underscore)

Early versions of the project had `name = "sprocket-py"` in both `[package]` and `[lib]` in `Cargo.toml`. Rust normalises hyphens to underscores in compiled `.so` filenames, so the actual module was `sprocket_py` but the `#[pymodule]` was registered as `sprocket-py`. This caused `import sprocket_py` to fail with a module initialiser error. Fix: set `[lib] name = "sprocket_py"` explicitly and make the `#[pymodule]` function name match.

### Why lint() uses the CLI

Ideally `lint()` would call `wdl-lint`'s `Linter` type directly via PyO3, the same way `parse()` calls `Document::parse()`. During prototyping I found that `Linter` is generic over a visitor context and holds mutable references that require careful ownership design to expose safely through PyO3 (all `#[pyclass]` types must be `'static + Send`). Rather than block on this, the prototype uses a subprocess wrapper around `sprocket lint` via `tempfile`. The GSoC implementation will replace this with direct crate bindings.

---

## Project structure

```
sprocket-py/
├── src/
│   └── lib.rs          # PyO3 bindings: parse(), lint(), Diagnostic, LintWarning, ParsedDocument
├── Cargo.toml          # Rust dependencies (pyo3, wdl-ast, tempfile)
├── test.py             # Tests for parse() — valid WDL, missing version, syntax errors
├── test_lint.py        # Tests for lint() — deliberately malformed WDL triggering multiple rules
└── README.md
```

---

## Roadmap (GSoC 2026)

- Replace CLI-based `lint()` with direct `wdl-lint` crate bindings
- Add `doc.workflows()`, `doc.tasks()`, `doc.imports()` accessors via AST visitor
- Python type stubs (`.pyi` files) for IDE autocomplete
- Cross-platform wheels via maturin + GitHub Actions CI
- PyPI publish

## License

MIT