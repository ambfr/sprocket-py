use pyo3::prelude::*;
use wdl_ast::{AstToken, Document, Severity};

/// Represents a single diagnostic (error or warning) from parsing
#[pyclass]
struct Diagnostic {
    #[pyo3(get)]
    severity: String,
    #[pyo3(get)]
    message: String,
}

/// Represents the parsed WDL document
#[pyclass]
struct ParsedDocument {
    #[pyo3(get)]
    version: String,
}

/// Parse a WDL source string.
/// Returns (ParsedDocument, list of Diagnostics)
#[pyfunction]
fn parse(source: &str) -> PyResult<(ParsedDocument, Vec<Diagnostic>)> {
    let (document, diagnostics) = Document::parse(source);

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

    let version = document
        .version_statement()
        .map(|v| v.version().text().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Ok((ParsedDocument { version }, py_diagnostics))
}

/// The sprocket_py Python module
#[pymodule]
fn sprocket_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Diagnostic>()?;
    m.add_class::<ParsedDocument>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}