use pyo3::prelude::*;
use wdl_ast::{AstToken, Document, Severity};
use std::io::Write;
use std::process::Command;

#[pyclass]
struct Diagnostic {
    #[pyo3(get)]
    severity: String,
    #[pyo3(get)]
    message: String,
}

#[pyclass]
struct LintWarning {
    #[pyo3(get)]
    rule: String,
    #[pyo3(get)]
    message: String,
}

#[pyclass]
struct ParsedDocument {
    #[pyo3(get)]
    version: String,
}

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

#[pyfunction]
fn lint(source: &str) -> PyResult<Vec<LintWarning>> {
    // Write source to a temp file and run sprocket lint on it
    let mut tmp = tempfile::NamedTempFile::with_suffix(".wdl")
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    tmp.write_all(source.as_bytes())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let path = tmp.path().to_owned();

    let output = Command::new("sprocket")
        .args(["lint", path.to_str().unwrap()])
        .output()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(
            format!("sprocket not found on PATH: {e}")
        ))?;

    let combined = String::from_utf8_lossy(&output.stderr).to_string()
        + &String::from_utf8_lossy(&output.stdout);

    // Parse lines like: "warning[RuleName]: message"
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

#[pymodule]
fn sprocket_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Diagnostic>()?;
    m.add_class::<ParsedDocument>()?;
    m.add_class::<LintWarning>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(lint, m)?)?;
    Ok(())
}