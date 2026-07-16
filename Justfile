# SPDX-License-Identifier: MPL-2.0
# Justfile for nexia-list

# Default recipe — list available commands
import? "contractile.just"

default:
    @just --list

# Install dependencies (Deno packages + wasm target for the core)
setup:
    deno task setup
    rustup target add wasm32-unknown-unknown || true

# Build the project (ReScript compile + esbuild web bundle)
build:
    deno task build

# Build the Rust core to WASM for the browser
build-wasm:
    deno task build:wasm

# Run all tests (Rust core + UI)
test:
    deno task test

# Run the development server (http://localhost:5173)
run:
    deno task dev

# Static checks — Deno lint, rustfmt, clippy
check:
    deno lint
    cd core && cargo fmt --check && cargo clippy -- -D warnings

# Publish docs/wikis/*.md to the GitHub wiki (docs/wikis/ is the source of truth)
#   just wiki-sync dry    # preview what would change, push nothing
#   just wiki-sync        # commit + push the wiki pages
# Override the target with NEXIA_WIKI_REMOTE.
wiki-sync mode="push":
    #!/usr/bin/env bash
    set -euo pipefail
    ROOT="$(pwd)"
    SRC="${ROOT}/docs/wikis"
    REMOTE="${NEXIA_WIKI_REMOTE:-https://github.com/hyperpolymath/nexia-list.wiki.git}"
    MODE="{{mode}}"
    WORK="$(mktemp -d)"
    trap 'rm -rf "${WORK}"' EXIT
    echo "=== nexia-list wiki-sync (${MODE}) -> ${REMOTE} ==="
    if ! git clone --quiet --depth 1 "${REMOTE}" "${WORK}"; then
        echo "!! wiki remote not initialised yet." >&2
        echo "   Visit the repo Wiki tab, save any page once to create it, then re-run." >&2
        exit 1
    fi
    # Only the Markdown pages are mirrored; README.adoc stays repo-only.
    cp "${SRC}"/*.md "${WORK}/"
    cd "${WORK}"
    git add -A
    if git diff --cached --quiet; then
        echo "wiki already up to date."
        exit 0
    fi
    echo "--- pending wiki changes ---"
    git status --short
    if [ "${MODE}" = "dry" ]; then
        echo "(dry run — nothing pushed)"
        exit 0
    fi
    git commit --quiet -m "docs(wiki): sync from docs/wikis/ @ $(git -C "${ROOT}" rev-parse --short HEAD)"
    git push --quiet origin HEAD
    echo "OK: wiki synced."

# Self-diagnostic — checks dependencies, permissions, paths
doctor:
    @echo "Running diagnostics for nexia-list..."
    @echo "Checking required tools..."
    @command -v just >/dev/null 2>&1 && echo "  [OK] just" || echo "  [FAIL] just not found"
    @command -v git >/dev/null 2>&1 && echo "  [OK] git" || echo "  [FAIL] git not found"
    @command -v deno >/dev/null 2>&1 && echo "  [OK] deno" || echo "  [FAIL] deno not found (need Deno 2.x)"
    @command -v cargo >/dev/null 2>&1 && echo "  [OK] cargo" || echo "  [FAIL] cargo not found (need Rust stable)"
    @echo "Checking for hardcoded paths..."
    @grep -rn '/var/mnt/eclipse' --include='*.rs' --include='*.ex' --include='*.res' --include='*.gleam' --include='*.sh' --include='*.toml' . 2>/dev/null | grep -v 'Justfile' | head -5 || echo "  [OK] No hardcoded paths in source"
    @echo "Diagnostics complete."

# Guided tour of key features
tour:
    @echo "=== nexia-list Tour ==="
    @echo ""
    @echo "1. Project structure:"
    @ls -la
    @echo ""
    @echo "2. Available commands: just --list"
    @echo ""
    @echo "3. Read README.adoc or README.md for full overview"
    @echo "4. Read EXPLAINME.adoc for architecture decisions"
    @echo "5. Run 'just doctor' to check your setup"
    @echo ""
    @echo "Tour complete! Try 'just --list' to see all available commands."

# Open feedback channel with diagnostic context
help-me:
    @echo "=== nexia-list Help ==="
    @echo "Platform: $(uname -s) $(uname -m)"
    @echo "Shell: $SHELL"
    @echo ""
    @echo "To report an issue:"
    @echo "  https://github.com/hyperpolymath/nexia-list/issues/new"
    @echo ""
    @echo "Include the output of 'just doctor' in your report."

# Run panic-attacker pre-commit scan
assail:
    @command -v panic-attack >/dev/null 2>&1 && panic-attack assail . || echo "WARN: panic-attack not found — install from https://github.com/hyperpolymath/panic-attacker"

# LLM context dump
llm-context:
    @echo "Project: nexia-list"
    @echo "License: MPL-2.0"
    @test -f README.adoc && head -30 README.adoc || test -f README.md && head -30 README.md || echo "No README found"


# Print the current CRG grade (reads from READINESS.md '**Current Grade:** X' line)
crg-grade:
    @grade=$$(grep -oP '(?<=\*\*Current Grade:\*\* )[A-FX]' READINESS.md 2>/dev/null | head -1); \
    [ -z "$$grade" ] && grade="X"; \
    echo "$$grade"

# Generate a shields.io badge markdown for the current CRG grade
# Looks for '**Current Grade:** X' in READINESS.md; falls back to X
crg-badge:
    @grade=$$(grep -oP '(?<=\*\*Current Grade:\*\* )[A-FX]' READINESS.md 2>/dev/null | head -1); \
    [ -z "$$grade" ] && grade="X"; \
    case "$$grade" in \
      A) color="brightgreen" ;; B) color="green" ;; C) color="yellow" ;; \
      D) color="orange" ;; E) color="red" ;; F) color="critical" ;; \
      *) color="lightgrey" ;; esac; \
    echo "[![CRG $$grade](https://img.shields.io/badge/CRG-$$grade-$$color?style=flat-square)](https://github.com/hyperpolymath/standards/tree/main/component-readiness-grades)"
