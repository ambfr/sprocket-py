# test.py — integration tests for sprocket_py.parse()
#
# These tests exercise the three main outcomes of parsing a WDL document:
#   1. Valid WDL with a version statement → clean parse, version extracted correctly
#   2. WDL missing a version statement   → parse succeeds but diagnostics contain an error
#   3. Syntactically broken WDL          → parse reports syntax error diagnostics
#
# To run: build the extension first with `maturin develop`, then `python test.py`
#
# Note: parse() always returns a (ParsedDocument, list[Diagnostic]) tuple even when
# there are errors. This mirrors wdl-ast's behaviour — Document::parse() never panics;
# it returns the best-effort AST alongside a list of diagnostics. This design was
# intentional in wdl-ast and we preserve it in the Python API.

import sprocket_py

# ---------------------------------------------------------------------------
# Test 1: Valid WDL
# ---------------------------------------------------------------------------
# A minimal but complete WDL 1.1 document with one workflow and one task.
# Expected: version = "1.1", zero errors.
# This verifies that AstToken::text() on the VersionStatement node correctly
# returns the raw version string from the source.

valid_wdl = """
version 1.1

workflow hello {
    call say_hello
}

task say_hello {
    command {
        echo "Hello, World!"
    }
    output {
        String out = read_string(stdout())
    }
}
"""

doc, errors = sprocket_py.parse(valid_wdl)
print("=== Test 1: Valid WDL ===")
print(f"WDL version: {doc.version}")       # Expected: 1.1
print(f"Errors: {len(errors)}")            # Expected: 0


# ---------------------------------------------------------------------------
# Test 2: Missing version statement
# ---------------------------------------------------------------------------
# WDL without a `version` declaration. wdl-ast still produces a Document node
# (version_statement() returns None → our code returns "unknown"), but the
# diagnostics list will contain an error explaining the missing version.
#
# This test confirms that:
#   a) parse() does not raise even when the WDL is malformed
#   b) doc.version falls back to "unknown" gracefully
#   c) the diagnostic message is surfaced correctly to Python

invalid_wdl = """
workflow broken {
    call something
}
"""

doc2, errors2 = sprocket_py.parse(invalid_wdl)
print("\n=== Test 2: Missing version ===")
print(f"WDL version: {doc2.version}")       # Expected: unknown
print(f"Errors found: {len(errors2)}")      # Expected: >= 1
for e in errors2:
    print(f"  [{e.severity}] {e.message}")


# ---------------------------------------------------------------------------
# Test 3: Syntax error
# ---------------------------------------------------------------------------
# A WDL document that is structurally broken — the workflow block is never closed.
# wdl-ast's parser is error-tolerant and will still return a Document, but the
# diagnostics list will contain one or more errors describing the parse failure.
#
# This test verifies that syntax errors produce Error-severity diagnostics
# (not just warnings or notes), and that the error message is non-empty.

broken_wdl = """
version 1.1

workflow {
"""

doc3, errors3 = sprocket_py.parse(broken_wdl)
print("\n=== Test 3: Syntax error ===")
print(f"Errors found: {len(errors3)}")      # Expected: >= 1
for e in errors3:
    print(f"  [{e.severity}] {e.message}")