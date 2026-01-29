#!/bin/bash
# Documentation Quality Assurance Script
# Runs all documentation checks for Horizon Lattice

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "Horizon Lattice Documentation QA"
echo "========================================"
echo ""

FAILED=0

# 1. Check that all public items are documented
echo -e "${YELLOW}[1/5] Checking documentation completeness...${NC}"
if cargo doc --workspace --no-deps 2>&1 | grep -q "warning: missing documentation"; then
    echo -e "${RED}  ✗ Some public items are missing documentation${NC}"
    echo "  Run 'cargo doc --workspace --no-deps 2>&1 | grep missing' to see details"
    FAILED=1
else
    echo -e "${GREEN}  ✓ All public items documented${NC}"
fi

# 2. Run doc tests
echo ""
echo -e "${YELLOW}[2/5] Running doc tests...${NC}"
if cargo test --doc --workspace 2>&1; then
    echo -e "${GREEN}  ✓ All doc tests pass${NC}"
else
    echo -e "${RED}  ✗ Doc tests failed${NC}"
    FAILED=1
fi

# 3. Build mdBook
echo ""
echo -e "${YELLOW}[3/5] Building mdBook...${NC}"
cd "$PROJECT_ROOT/docs"
if mdbook build 2>&1; then
    echo -e "${GREEN}  ✓ mdBook builds successfully${NC}"
else
    echo -e "${RED}  ✗ mdBook build failed${NC}"
    FAILED=1
fi
cd "$PROJECT_ROOT"

# 4. Check links (if mdbook-linkcheck is installed)
echo ""
echo -e "${YELLOW}[4/5] Checking links...${NC}"
if command -v mdbook-linkcheck &> /dev/null; then
    cd "$PROJECT_ROOT/docs"
    if mdbook-linkcheck 2>&1; then
        echo -e "${GREEN}  ✓ No broken links${NC}"
    else
        echo -e "${RED}  ✗ Broken links found${NC}"
        FAILED=1
    fi
    cd "$PROJECT_ROOT"
else
    echo -e "${YELLOW}  ⚠ mdbook-linkcheck not installed, skipping${NC}"
    echo "  Install with: cargo install mdbook-linkcheck"
fi

# 5. Spell check (if cspell is installed)
echo ""
echo -e "${YELLOW}[5/5] Running spell check...${NC}"
if command -v cspell &> /dev/null; then
    cd "$PROJECT_ROOT/docs/src"
    if cspell "**/*.md" --config "$PROJECT_ROOT/.cspell.json" 2>&1; then
        echo -e "${GREEN}  ✓ No spelling errors${NC}"
    else
        echo -e "${RED}  ✗ Spelling errors found${NC}"
        FAILED=1
    fi
    cd "$PROJECT_ROOT"
else
    echo -e "${YELLOW}  ⚠ cspell not installed, skipping${NC}"
    echo "  Install with: npm install -g cspell"
fi

# Summary
echo ""
echo "========================================"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All documentation checks passed!${NC}"
    exit 0
else
    echo -e "${RED}Some documentation checks failed${NC}"
    exit 1
fi
