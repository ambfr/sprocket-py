# Development Notes

Engineering log for sprocket-py. Documents the decisions, dead ends, and API discoveries made during prototype development. Written partly for my own reference and partly because GSoC mentors will read it.

---

## Environment setup

Platform: Windows 11 with WSL2 (Ubuntu 22.04) for the Linux build path; native Windows for testing.

Tools installed:
- Rustup (stable toolchain)
- Visual Studio Build Tools 2022 (required for the MSVC linker on Windows)
- Python 3.11 + virtualenv
- maturin 1.x (via pip)

The trickiest part of setup was getting maturin to find the correct Python interpreter inside a virtualenv on Windows. The fix is to activate the virtualenv first, then run `maturin develop` — it detects the active interpreter automatically.

---

## Discovering the correct wdl-ast crate

**Problem:** The obvious starting point was `wdl = "1.x"` on crates.io. I added it to `Cargo.toml` and the build failed — the crate resolved but the API I expected didn't exist.

**Investigation:** Looked at the crates.io page for `wdl` and found a deprecation notice dated September 2025: the standalone crate was archived and all development moved to the Sprocket monorepo at `github.com/stjude-rust-labs/sprocket`. The active crates are now workspace members under `crates/`:
- `crates/wdl-ast` — parser and AST
- `crates/wdl-lint` — lint rules
- `crates/wdl` — re-exports both (but this is the archived one on crates.io)

**Fix:** Use a git dependency in `Cargo.toml` pointing at the monorepo with `package = "wdl-ast"`:

```toml
wdl-ast = { git = "https://github.com/stjude-rust-labs/sprocket", package = "wdl-ast" }
```

This pins to `HEAD` of the monorepo, which means it always tracks the actively maintained codebase. The downside is that breaking changes upstream can break the build — something to address in GSoC by pinning to a specific rev or tag.

---

## Finding the version extraction API

**Problem:** I assumed `Document` had a `.version()` method. It doesn't. The compiler error was:

```
error[E0599]: no method named `version` found for struct `Document`
```

**Investigation:** Cloned the Sprocket monorepo and read `crates/wdl-ast/src/lib.rs`. Found that `Document` exposes:
- `version_statement() -> Option<VersionStatement>`
- `VersionStatement` has `.version() -> Version` (the AST node for the version token)
- `Version` implements `AstToken`, which has `.text() -> &str`

The `AstToken` trait is the key — it's not auto-imported, so you need `use wdl_ast::AstToken` explicitly in scope or `.text()` won't resolve. This wasted about 20 minutes before I found it in the trait docs.

**Final code:**
```rust
use wdl_ast::AstToken; // required for .text()

let version = document
    .version_statement()
    .map(|v| v.version().text().to_string())
    .unwrap_or_else(|| "unknown".to_string());
```

---

## The import name mismatch bug

**Problem:** After the first working build, `import sprocket_py` in Python produced:

```
ImportError: dynamic module does not define module export function (PyInit_sprocket_py)
```

**Investigation:** The `Cargo.toml` had:

```toml
[package]
name = "sprocket-py"   # hyphen

[lib]
name = "sprocket-py"   # hyphen here too — this is the problem
crate-type = ["cdylib"]
```

Rust normalises hyphens to underscores in the compiled `.so` filename (`sprocket_py.so`), but the `#[pymodule]` function was named `sprocket_py` (underscored). PyO3 looks for a C symbol named `PyInit_<module_name>` where `<module_name>` comes from the function name. The mismatch between the lib name and the function name caused the initialiser not to be found.

**Fix:** Set `[lib] name = "sprocket_py"` (underscore) explicitly, and make sure the `#[pymodule]` function name matches:

```toml
[lib]
name = "sprocket_py"
crate-type = ["cdylib"]
```

```rust
#[pymodule]
fn sprocket_py(m: &Bound<'_, PyModule>) -> PyResult<()> { ... }
```

This is now documented in PyO3's FAQ but isn't obvious when you first hit it.

---

## Why lint() is a CLI wrapper (not a native binding)

**Attempted approach:** Write `lint()` as a direct PyO3 binding to `wdl-lint`'s `Linter` type, the same way `parse()` wraps `Document::parse()`.

**Problem:** `wdl-lint`'s `Linter` is a struct that holds state through the visitor pattern. After reading `crates/wdl-lint/src/lib.rs`, I found that `Linter` holds mutable visitor references during a lint pass. PyO3's `#[pyclass]` requires all types to be `'static + Send` — i.e., no borrowed references, and safe to move across threads. Making `Linter` satisfy these constraints requires either an owned design (wrapping each lint pass in a fresh `Linter` instance) or a more careful ownership design that takes time to get right.

**Decision:** For the prototype, use `sprocket lint` via subprocess + `tempfile`. This works, is straightforward to test, and gets an end-to-end demo working quickly. The native binding is the right long-term solution and is explicitly planned for GSoC Week 3.

**Current implementation summary:**
1. Write WDL source to a `NamedTempFile` with `.wdl` extension (required by sprocket)
2. Invoke `sprocket lint <path>` as a subprocess
3. Parse stdout+stderr for lines matching `warning[RuleName]: message`
4. Return as `Vec<LintWarning>`

The `.wdl` extension requirement was discovered by trial and error — `sprocket lint` silently produces no output on files without a `.wdl` extension. Using `NamedTempFile::with_suffix(".wdl")` fixes this.

---

## Diagnostics are non-fatal

One thing that surprised me: `Document::parse()` in wdl-ast never returns an `Err`. It always returns `(Document, Vec<Diagnostic>)` — even if the WDL is completely broken, you get a best-effort `Document` back. This is intentional; the AST is designed for use in editors where you want to recover as much information as possible from malformed input (e.g. for the LSP).

This means `parse()` in our Python API can never raise a Python exception due to WDL parse errors — errors are communicated through the returned `Diagnostic` list. The only way `parse()` raises is if something goes wrong at the PyO3 level, which shouldn't happen in normal use.

---

## Commit log rationale

The commit history is small right now because a lot of the exploration above happened locally before I understood enough to commit meaningfully. Going forward, every API decision, bug fix, and test addition will be a separate commit with a message explaining the *why*, not just the *what*.