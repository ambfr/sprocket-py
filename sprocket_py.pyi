from typing import List, Tuple

class Diagnostic:
    """A parse diagnostic (error or warning) from a WDL document."""
    severity: str   # "error", "warning", or "note"
    message: str

class LintWarning:
    """A lint warning from a WDL document."""
    rule: str       # e.g. "MetaSections", "HereDocCommands"
    message: str

class ParsedDocument:
    """A successfully parsed WDL document."""
    version: str    # e.g. "1.1" or "unknown"

def parse(source: str) -> Tuple[ParsedDocument, List[Diagnostic]]:
    """
    Parse a WDL source string.

    Returns a tuple of (ParsedDocument, list of Diagnostics).
    If the document has errors, they will appear in the diagnostics list.

    Example:
        doc, errors = sprocket_py.parse(wdl_source)
        if errors:
            for e in errors:
                print(f"[{e.severity}] {e.message}")
    """
    ...

def lint(source: str) -> List[LintWarning]:
    """
    Lint a WDL source string using Sprocket's lint rules.

    Returns a list of LintWarnings. Returns an empty list if the
    document has parse errors (lint is skipped in that case).

    Requires the sprocket CLI to be installed and on PATH.

    Example:
        warnings = sprocket_py.lint(wdl_source)
        for w in warnings:
            print(f"[{w.rule}] {w.message}")
    """
    ...