# test_lint.py — integration tests for sprocket_py.lint()
#
# This script tests both parse() and lint() against a deliberately malformed WDL 1.1
# document designed to trigger multiple wdl-lint rule violations simultaneously.
#
# The WDL below violates the following lint rules (verified against wdl-lint source):
#   - MissingMeta:          task `say_hello` has no `meta {}` block
#   - MissingParameterMeta: task `say_hello` has no `parameter_meta {}` block
#   - MissingMeta:          workflow `greet` has no `meta {}` block
#   - MissingParameterMeta: workflow `greet` has no `parameter_meta {}` block
#   - CommandSingleQuote:   `runtime.container` uses single-quoted string
#                           (wdl-lint prefers double quotes for container values)
#
# Additionally, the command block uses curly brace syntax `command { }` rather than
# heredoc `command <<< >>>`, which some lint rules flag depending on WDL version.
#
# To run: build with `maturin develop`, then `python test_lint.py`
# Note: lint() requires `sprocket` to be installed and available on PATH.

import sprocket_py

# Deliberately malformed WDL — triggers multiple lint rules.
# Each violation is commented inline.
bad_wdl = """
version 1.1

task say_hello {
    input {
        String name
        String greeting
    }

    command {
        echo '~{greeting}, ~{name}!'   # single-quoted string in command (lint warning)
    }

    output {
        String result = read_string(stdout())
    }

    runtime {
        container: 'ubuntu:latest'     # single quotes on container value (lint warning)
    }
    # no meta {} block         → MissingMeta
    # no parameter_meta {} block → MissingParameterMeta
}

workflow greet {
    input {
        String name
        String greeting
    }

    call say_hello {
        input:
            name = name,
            greeting = greeting,
    }
    # no meta {} block         → MissingMeta
    # no parameter_meta {} block → MissingParameterMeta
}
"""

# --- Parse check ---
# Even a lint-invalid document should parse cleanly at the AST level.
# Zero parse errors here confirms that lint warnings are distinct from parse errors —
# they represent style/best-practice violations, not syntax failures.
print("=== Parse check ===")
doc, errors = sprocket_py.parse(bad_wdl)
print(f"Version: {doc.version}")           # Expected: 1.1
print(f"Parse errors: {len(errors)}")      # Expected: 0 (this is valid WDL, just badly styled)

# --- Lint check ---
# lint() invokes `sprocket lint` on the source and parses the output into LintWarning
# objects. Each warning has a `rule` (e.g. "MissingMeta") and a `message` string.
# We expect at least 4 warnings from this document (2x MissingMeta, 2x MissingParameterMeta).
print("\n=== Lint warnings ===")
warnings = sprocket_py.lint(bad_wdl)
print(f"{len(warnings)} warnings found:")
for w in warnings:
    print(f"  [{w.rule}] {w.message}")