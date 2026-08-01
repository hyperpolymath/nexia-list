# SPDX-License-Identifier: MPL-2.0
# Justfile for nexia-list

# Default recipe — list available commands
import? "contractile.just"

default:
    @just --list

# Install Bun dependencies + wasm target for the core
setup:
    bun install --frozen-lockfile
    rustup target add wasm32-unknown-unknown || true

# Build the Rust core to WASM for the browser
build-wasm:
    bun run build:wasm

# Build the project (ReScript compile + Bun web bundle)
# Depends on build-wasm: the bundler resolves web/wasm/nexia_core.js.
build: build-wasm
    bun run build

# Run all tests (Rust core + UI)
# Depends on build: the UI tests import the generated *.res.js and the wasm
# bindings, so a clean checkout cannot go straight to `just test` without them.
# Both builds are incremental — a warm no-op costs well under a second.
test: build
    bun run test

# Rust core tests only — no build step, for a tight inner loop
test-rust:
    bun run test:rust

# Run the development server (http://localhost:5173)
run:
    bun run dev

# Static checks — Biome, rustfmt, clippy
# Mirrors rust-ci.yml exactly: --all-targets --features wasm, run from the
# workspace root. A narrower clippy here would pass locally and fail in CI.
check:
    bun run lint
    cargo fmt --check
    cargo clippy --all-targets --features wasm -- -D warnings

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
    @command -v bun >/dev/null 2>&1 && echo "  [OK] bun" || echo "  [FAIL] bun not found (need Bun 1.3+)"
    @command -v cargo >/dev/null 2>&1 && echo "  [OK] cargo" || echo "  [FAIL] cargo not found (need Rust stable)"
    @echo "Checking the WASM toolchain (the browser build depends on it)..."
    @rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown \
        && echo "  [OK] wasm32-unknown-unknown target" \
        || echo "  [FAIL] wasm32-unknown-unknown target missing — run 'just setup'"
    @just _wasm-bindgen-check
    @echo "Checking for hardcoded paths..."
    @grep -rn '/var/mnt/eclipse' --include='*.rs' --include='*.ex' --include='*.res' --include='*.gleam' --include='*.sh' --include='*.toml' . 2>/dev/null | grep -v 'Justfile' | head -5 || echo "  [OK] No hardcoded paths in source"
    @echo "Diagnostics complete."

# The wasm-bindgen CLI must match the wasm-bindgen crate in Cargo.lock exactly;
# a mismatch fails the browser build with a confusing schema error, so report
# the drift and the exact command that fixes it.
[private]
_wasm-bindgen-check:
    #!/usr/bin/env bash
    set -uo pipefail
    lock="$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock 2>/dev/null | grep '^version' | head -1 | cut -d'"' -f2)"
    cli="$(wasm-bindgen --version 2>/dev/null | awk '{print $2}')"
    if [ -z "${lock}" ]; then
        echo "  [WARN] wasm-bindgen not found in Cargo.lock — skipping version check"
    elif [ -z "${cli}" ]; then
        echo "  [FAIL] wasm-bindgen CLI not found — cargo install wasm-bindgen-cli --version ${lock} --locked"
    elif [ "${cli}" != "${lock}" ]; then
        echo "  [FAIL] wasm-bindgen CLI ${cli} != Cargo.lock ${lock}"
        echo "         cargo install wasm-bindgen-cli --version ${lock} --locked --force"
    else
        echo "  [OK] wasm-bindgen ${cli} (matches Cargo.lock)"
    fi

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
    #!/usr/bin/env bash
    set -uo pipefail
    grade="$(grep -oP '(?<=\*\*Current Grade:\*\* )[A-FX]' READINESS.md 2>/dev/null | head -1)"
    echo "${grade:-X}"

# Generate a shields.io badge markdown for the current CRG grade
# Looks for '**Current Grade:** X' in READINESS.md; falls back to X
crg-badge:
    #!/usr/bin/env bash
    set -uo pipefail
    grade="$(grep -oP '(?<=\*\*Current Grade:\*\* )[A-FX]' READINESS.md 2>/dev/null | head -1)"
    grade="${grade:-X}"
    case "${grade}" in
      A) color="brightgreen" ;; B) color="green" ;; C) color="yellow" ;;
      D) color="orange" ;; E) color="red" ;; F) color="critical" ;;
      *) color="lightgrey" ;;
    esac
    echo "[![CRG ${grade}](https://img.shields.io/badge/CRG-${grade}-${color}?style=flat-square)](https://github.com/hyperpolymath/standards/tree/main/component-readiness-grades)"
