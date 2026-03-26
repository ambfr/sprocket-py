// sprocket-py: PyO3 bindings exposing Sprocket's WDL parsing and linting to Python.
//
// Architecture overview:
//   - parse()  → native PyO3 binding to wdl_ast::Document::parse()
//               Returns structured ParsedDocument + Vec<Diagnostic> directly from the AST.
//   - lint()   → CLI-based wrapper around `sprocket lint`
//               See the comment above lint() for why this approach was chosen for the
//               prototype and what the GSoC work will replace it with.
//
// Dependencies:
//   - pyo3 0.22: Rust-Python FFI layer. Provides #[pyclass], #[pyfunction], #[pymodule].
//   - wdl-ast: Pulled from the live Sprocket monorepo (stjude-rust-labs/sprocket).
//             The standalone `wdl` crate was archived in Sept 2025; all development
//             moved to the Sprocket monorepo, so we depend on that directly via git.
//   - tempfile: Used by lint() to write WDL source to disk before invoking the CLI.

use pyo3::prelude::*;
use wdl_ast::{AstToken, Document, Severity};
use std::io::Write;
use std::process::Command;

// ---------------------------------------------------------------------------
// Python-visible types
// ---------------------------------------------------------------------------

/// Represents a single parse diagnostic (error, warning, or note) from wdl-ast.
///
/// Both `severity` and `message` are exposed as read-only Python attributes via
/// #[pyo3(get)]. We convert severity to a plain string rather than a Python enum
/// to keep the API simple for the prototype; a proper Severity enum is planned
/// for the GSoC implementation.
#[pyclass]
struct Diagnostic {
    #[pyo3(get)]
    severity: String,
    #[pyo3(get)]
    message: String,
}

/// Represents a single lint warning returned by `sprocket lint`.
///
/// `rule` is the lint rule ID (e.g. "MissingMeta"), `message` is the human-readable
/// description. These are parsed from the CLI's stderr/stdout output in lint().
///
/// In the full GSoC implementation, LintWarning will be populated directly from the
/// wdl-lint crate's Diagnostic type via PyO3, removing the CLI dependency entirely.
#[pyclass]
struct LintWarning {
    #[pyo3(get)]
    rule: String,
    #[pyo3(get)]
    message: String,
}

/// Represents a successfully parsed WDL document.
///
/// Currently exposes only `version`. The GSoC work will extend this with
/// `workflows()`, `tasks()`, and `imports()` accessors by traversing the
/// Document AST via wdl-ast's visitor pattern.
#[pyclass]
struct ParsedDocument {
    #[pyo3(get)]
    version: String,
}

// ---------------------------------------------------------------------------
// parse() — native PyO3 binding to wdl_ast::Document::parse()
// ---------------------------------------------------------------------------

/// Parse a WDL source string and return a (ParsedDocument, list[Diagnostic]) tuple.
///
/// This is a direct binding to wdl_ast::Document::parse(), which returns both the
/// AST and a Vec of diagnostics in a single call. Diagnostics may be present even
/// when parsing succeeds (e.g. missing version statement is a warning, not a hard
/// error at the AST level).
///
/// Version extraction: we call version_statement() on the Document, then .version()
/// to get the VersionStatement node, then .text() via the AstToken trait. If no
/// version statement exists, we return "unknown" — this is intentional, as the
/// diagnostic list will already contain the relevant error message.
///
/// Note on API discovery: I initially tried to call .version() directly on Document,
/// but found that wdl-ast separates version access via version_statement() returning
/// an Option<VersionStatement>. This required reading the wdl-ast source in the
/// Sprocket monorepo directly (crates/wdl-ast/src/lib.rs) to find the correct path.
#[pyfunction]
fn parse(source: &str) -> PyResult<(ParsedDocument, Vec<Diagnostic>)> {
    let (document, diagnostics) = Document::parse(source);

    // Map wdl-ast Severity enum to plain strings for Python.
    // Severity has three variants: Error, Warning, Note.
    let py_diagnostics: Vec<Diagnostic> = diagnostics
        .iter()
        .map(|d| Diagnostic {
            severity: match d.severity() {
                Severity::Error => "error".to_string(),
                Severity::Warning => "warning".to_string(),
                Severity::Note => "note".to_string(),
            },
            message: d.message().to_string(),
        })
        .collect();

    // version_statement() returns Option<VersionStatement>.
    // If present, we chain .version().text() to get the raw version string (e.g. "1.1").
    // AstToken::text() is required here — it's a trait method, hence the `use wdl_ast::AstToken` import.
    let version = document
        .version_statement()
        .map(|v| v.version().text().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Ok((ParsedDocument { version }, py_diagnostics))
}

// ---------------------------------------------------------------------------
// lint() — CLI-based wrapper (prototype approach; will be replaced in GSoC)
// ---------------------------------------------------------------------------

/// Run lint checks on a WDL source string and return a list of LintWarning objects.
///
/// ## Why the CLI approach?
///
/// The ideal implementation would call the wdl-lint crate's Linter type directly via
/// PyO3, mirroring the approach used in parse(). During prototype development I
/// attempted this, but wdl-lint's Linter is generic over a visitor context and requires
/// mutable references that don't map cleanly to PyO3's ownership model without careful
/// design work (PyO3 requires all #[pyclass] types to be 'static + Send).
///
/// Rather than block on this, I implemented lint() as a thin subprocess wrapper around
/// `sprocket lint` using tempfile to materialise the WDL source on disk. This gets a
/// working end-to-end prototype while the deeper binding work is scoped to GSoC Week 3.
///
/// ## What GSoC Week 3 will change:
///
/// The CLI wrapper will be replaced by a direct PyO3 binding to wdl-lint's Linter,
/// with LintWarning populated from the crate's own Diagnostic type. This removes the
/// requirement for `sprocket` to be installed on PATH and makes lint() work anywhere
/// the Python package is installed.
///
/// ## Current output parsing:
///
/// `sprocket lint` emits diagnostics to stderr in the format:
///   warning[RuleName]: human-readable message
/// We parse these with a simple prefix-strip approach. Lines not matching this format
/// are silently ignored (e.g. file path headers, blank lines).
#[pyfunction]
fn lint(source: &str) -> PyResult<Vec<LintWarning>> {
    // Write WDL source to a named temp file with .wdl extension.
    // sprocket lint infers WDL version from the file content, but the .wdl extension
    // is required for it to recognise the file as a WDL document.
    let mut tmp = tempfile::NamedTempFile::with_suffix(".wdl")
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    tmp.write_all(source.as_bytes())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let path = tmp.path().to_owned();

    // Invoke `sprocket lint <path>`. This requires sprocket to be on PATH.
    // If sprocket is not found, we surface a clear error message rather than
    // a cryptic OS error.
    let output = Command::new("sprocket")
        .args(["lint", path.to_str().unwrap()])
        .output()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(
            format!("sprocket not found on PATH: {e}")
        ))?;

    // sprocket lint writes diagnostics to stderr; we combine both streams to be safe.
    let combined = String::from_utf8_lossy(&output.stderr).to_string()
        + &String::from_utf8_lossy(&output.stdout);

    // Parse lines of the form: "warning[RuleName]: message text"
    // We strip the "warning[" prefix, find the closing "]", extract the rule ID,
    // then take everything after "]: " as the message.
    let warnings = combined
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("warning[") {
                if let Some(bracket) = rest.find(']') {
                    let rule = rest[..bracket].to_string();
                    let message = rest[bracket + 2..].trim_start_matches(": ").to_string();
                    return Some(LintWarning { rule, message });
                }
            }
            None
        })
        .collect();

    Ok(warnings)
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Register all public types and functions under the `sprocket_py` module.
///
/// The module name here must match the `[lib] name` in Cargo.toml ("sprocket_py"),
/// which in turn must match the Python import name. This caused an import error
/// during early development when the crate name was "sprocket-py" (hyphen) but
/// the lib name defaulted to the same — Rust normalises hyphens to underscores in
/// the compiled .so filename, so the Python import `import sprocket_py` works only
/// when the lib name is explicitly set to "sprocket_py" in Cargo.toml.
#[pymodule]
fn sprocket_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Diagnostic>()?;
    m.add_class::<ParsedDocument>()?;
    m.add_class::<LintWarning>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(lint, m)?)?;
    Ok(())
}