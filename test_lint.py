import sprocket_py

# Deliberately bad WDL - missing meta, parameter_meta, single quotes, curly brace commands
bad_wdl = """
version 1.1

task say_hello {
    input {
        String name
        String greeting
    }

    command {
        echo '~{greeting}, ~{name}!'
    }

    output {
        String result = read_string(stdout())
    }

    runtime {
        container: 'ubuntu:latest'
    }
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
}
"""

print("=== Parse check ===")
doc, errors = sprocket_py.parse(bad_wdl)
print(f"Version: {doc.version}")
print(f"Parse errors: {len(errors)}")

print("\n=== Lint warnings ===")
warnings = sprocket_py.lint(bad_wdl)
print(f"{len(warnings)} warnings found:")
for w in warnings:
    print(f"  [{w.rule}] {w.message}")