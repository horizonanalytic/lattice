#!/bin/bash
# Check that complete code examples in documentation compile correctly
#
# This script extracts Rust code blocks from markdown files that contain
# a full `fn main()` function and attempts to compile them to catch API
# drift between docs and implementation.
#
# Code snippets (fragments without fn main) are skipped as they're meant
# to show partial code in context.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Create a temporary directory for test files
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "Checking documentation examples..."
echo "Temp directory: $TEMP_DIR"

# Create a Cargo project for testing
mkdir -p "$TEMP_DIR/src"
cat > "$TEMP_DIR/Cargo.toml" << EOF
[package]
name = "doc-examples-check"
version = "0.1.0"
edition = "2024"

[dependencies]
horizon-lattice = { path = "$PROJECT_ROOT/crates/horizon-lattice", features = ["networking", "multimedia"] }
EOF

# Files to check (high-priority getting-started and tutorial docs)
DOC_FILES=(
    "docs/src/introduction.md"
    "docs/src/getting-started/installation.md"
    "docs/src/getting-started/first-app.md"
    "docs/src/tutorials/hello-world.md"
)

# Generate a lib.rs that includes all extracted code as modules
echo "// Auto-generated file to check doc examples compile" > "$TEMP_DIR/src/lib.rs"
echo "#![allow(unused)]" >> "$TEMP_DIR/src/lib.rs"
echo "#![allow(dead_code)]" >> "$TEMP_DIR/src/lib.rs"
echo "" >> "$TEMP_DIR/src/lib.rs"

EXAMPLE_COUNT=0

for doc_file in "${DOC_FILES[@]}"; do
    full_path="$PROJECT_ROOT/$doc_file"
    if [[ ! -f "$full_path" ]]; then
        echo "Warning: $doc_file not found, skipping"
        continue
    fi

    echo "Processing: $doc_file"

    # Create a safe module name from the file path
    safe_name=$(echo "$doc_file" | sed 's/[^a-zA-Z0-9]/_/g')

    # Extract code blocks and process them
    block_num=0
    in_block=0
    current_block=""

    while IFS= read -r line || [[ -n "$line" ]]; do
        if [[ "$line" =~ ^\`\`\`rust,ignore ]]; then
            in_block=1
            current_block=""
            continue
        fi

        if [[ "$line" =~ ^\`\`\` ]] && [[ $in_block -eq 1 ]]; then
            in_block=0

            # Only process blocks that contain fn main() - these are complete examples
            if echo "$current_block" | grep -q "fn main()"; then
                block_num=$((block_num + 1))
                mod_name="${safe_name}_example_${block_num}"
                block_file="$TEMP_DIR/src/${mod_name}.rs"

                echo "  Found complete example (block $block_num)"

                # Write the module file
                {
                    echo "// From: $doc_file (example $block_num)"
                    echo "#![allow(unused)]"
                    echo "#![allow(dead_code)]"
                    echo ""
                    # Replace fn main() with a test function to avoid multiple mains
                    echo "$current_block" | sed 's/fn main()/fn _doc_example_main()/'
                } > "$block_file"

                # Add module declaration to lib.rs
                echo "mod ${mod_name};" >> "$TEMP_DIR/src/lib.rs"

                EXAMPLE_COUNT=$((EXAMPLE_COUNT + 1))
            fi
            continue
        fi

        if [[ $in_block -eq 1 ]]; then
            current_block="${current_block}${line}
"
        fi
    done < "$full_path"
done

if [[ $EXAMPLE_COUNT -eq 0 ]]; then
    echo ""
    echo "No complete examples found to check."
    exit 0
fi

echo ""
echo "Found $EXAMPLE_COUNT complete examples to check."
echo "Attempting to compile..."

cd "$TEMP_DIR"

# Try to compile (check only, no need to build fully)
if cargo check 2>&1; then
    echo ""
    echo "All $EXAMPLE_COUNT documentation examples compile successfully!"
    exit 0
else
    echo ""
    echo "ERROR: Some documentation examples failed to compile!"
    echo "Please update the documentation to match the current API."
    exit 1
fi
