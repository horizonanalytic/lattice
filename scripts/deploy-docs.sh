#!/usr/bin/env bash
#
# Deploy docs to GitHub Pages
# Creates/updates the gh-pages branch with built documentation
#
# Usage:
#   ./scripts/deploy-docs.sh
#
# Prerequisites:
#   - mdbook installed (cargo install mdbook)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOCS_DIR="$PROJECT_ROOT/docs"

# Safety: ensure we never run destructive commands in the project root
safety_check() {
    local current_dir="$1"
    if [[ "$current_dir" == "$PROJECT_ROOT" ]] || [[ "$current_dir" == "$DOCS_DIR" ]]; then
        echo "ERROR: Refusing to run destructive command in project directory"
        exit 1
    fi
}

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Build the docs if source files exist
if [[ -f "$DOCS_DIR/src/SUMMARY.md" ]]; then
    log_info "Building documentation with mdbook..."
    cd "$DOCS_DIR"
    mdbook build
fi

# Determine the book output directory
# mdBook outputs to book/html/ when multiple renderers are configured, otherwise book/
if [[ -d "$DOCS_DIR/book/html" ]] && [[ -f "$DOCS_DIR/book/html/index.html" ]]; then
    BOOK_DIR="$DOCS_DIR/book/html"
elif [[ -d "$DOCS_DIR/book" ]] && [[ -f "$DOCS_DIR/book/index.html" ]]; then
    BOOK_DIR="$DOCS_DIR/book"
else
    log_error "No documentation found - neither book/html/ nor book/ contains index.html"
    exit 1
fi

log_info "Using documentation from: $BOOK_DIR"

# Get remote URL
cd "$PROJECT_ROOT"
REMOTE_URL=$(git remote get-url origin)

# Create a temporary directory and clone fresh
TEMP_DIR=$(mktemp -d)
log_info "Working in temporary directory: $TEMP_DIR"

# Cleanup function to ensure temp directory is removed
cleanup() {
    if [[ -n "${TEMP_DIR:-}" ]] && [[ -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR"
    fi
}
trap cleanup EXIT

cd "$TEMP_DIR"

# Initialize a new repo and set up gh-pages branch
git init
git remote add origin "$REMOTE_URL"

# Try to fetch existing gh-pages, or start fresh
if git ls-remote --exit-code --heads origin gh-pages >/dev/null 2>&1; then
    log_info "Fetching existing gh-pages branch..."
    git fetch origin gh-pages
    git checkout -b gh-pages origin/gh-pages
    # Clean everything (safety check first)
    safety_check "$(pwd)"
    git rm -rf . 2>/dev/null || true
else
    log_info "Creating new gh-pages branch..."
    git checkout --orphan gh-pages
fi

# Copy the built docs
log_info "Copying documentation..."
cp -r "$BOOK_DIR"/* .

# Add .nojekyll to prevent Jekyll processing
touch .nojekyll

# Commit and push
git add -A
git commit -m "Deploy docs $(date +%Y-%m-%d)" || { log_info "No changes to commit"; exit 0; }

log_info "Pushing to origin/gh-pages..."
git push origin gh-pages --force

# Return to project root (cleanup handled by trap)
cd "$PROJECT_ROOT"

log_success "Documentation deployed to gh-pages branch!"
log_info "Configure GitHub Pages at your repo Settings -> Pages"
log_info "Set source to 'Deploy from a branch' -> 'gh-pages' -> '/ (root)'"
