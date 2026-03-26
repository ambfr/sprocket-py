import sprocket_py

# --- Test 1: Valid WDL ---
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
print(f"WDL version: {doc.version}")
print(f"Errors: {len(errors)}")

# --- Test 2: Invalid WDL (missing version) ---
invalid_wdl = """
workflow broken {
    call something
}
"""

doc2, errors2 = sprocket_py.parse(invalid_wdl)
print("\n=== Test 2: Missing version ===")
print(f"WDL version: {doc2.version}")
print(f"Errors found: {len(errors2)}")
for e in errors2:
    print(f"  [{e.severity}] {e.message}")

# --- Test 3: Syntax error ---
broken_wdl = """
version 1.1

workflow {
"""

doc3, errors3 = sprocket_py.parse(broken_wdl)
print("\n=== Test 3: Syntax error ===")
print(f"Errors found: {len(errors3)}")
for e in errors3:
    print(f"  [{e.severity}] {e.message}")