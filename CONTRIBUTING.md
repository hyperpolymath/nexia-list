<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Contributing to Nexia-List

## Development Setup

Prerequisites: [Deno](https://deno.land/) 2.x and [Rust](https://www.rust-lang.org/tools/install)
stable (plus the `wasm32-unknown-unknown` target for WASM builds). Deno is the
only JS toolchain — do not use npm/bun/yarn/pnpm. A Guix environment is
provided via `guix.scm` (`guix shell`) if you prefer reproducible shells.

```bash
# Clone the repository
git clone https://github.com/hyperpolymath/nexia-list.git
cd nexia-list

# Install dependencies
deno task setup

# Run the development server (http://localhost:5173)
deno task dev

# Build (ReScript + web bundle)
deno task build

# Verify setup
deno task lint
deno task test    # Rust core tests + UI tests
```

Equivalent `just` recipes exist: `just setup`, `just build`, `just test`,
`just run`, `just check`.

### Repository Structure
```
nexia-list/
├── core/                # Rust core — notes, backlinks, search, JSON storage
├── ui/                  # ReScript TEA-style UI (@rescript/react)
├── scripts/             # Deno build/dev scripts (esbuild)
├── web/                 # Browser entry + bundle output (dist/)
├── desktop/             # OPTIONAL Gossamer shell (external sibling checkout;
│                        # not built in this repo's CI)
├── docs/                # ADRs, reports
│   └── adr/             # Architecture decision records
├── tests/               # Cross-cutting tests
├── .well-known/         # Protocol files (ai.txt, security.txt, humans.txt)
├── .machine_readable/   # Contractiles, STATE/META/ECOSYSTEM checkpoints,
│                        # and governance metadata (see below)
├── .github/             # GitHub config and workflows
├── CHANGELOG.md
├── CODE_OF_CONDUCT.md
├── CONTRIBUTING.md      # This file
├── LICENSE
├── MAINTAINERS.adoc
├── README.adoc
├── ROADMAP.adoc
├── SECURITY.md
├── deno.json            # Deno tasks and import map
└── Justfile             # Task runner recipes
```

Governance and invariants are machine-readable: see
[`.machine_readable/`](.machine_readable/) (in particular `MUST.contractile`
and `INTENT.contractile`) and [`0-AI-MANIFEST.a2ml`](0-AI-MANIFEST.a2ml).

---

## How to Contribute

### Reporting Bugs

**Before reporting**:
1. Search existing issues
2. Check if it's already fixed in `main`
3. Determine which perimeter the bug affects

**When reporting**:

Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md) and include:

- Clear, descriptive title
- Environment details (OS, versions, toolchain)
- Steps to reproduce
- Expected vs actual behaviour
- Logs, screenshots, or minimal reproduction

### Suggesting Features

**Before suggesting**:
1. Check the [roadmap](ROADMAP.adoc)
2. Search existing issues and discussions
3. Consider which perimeter the feature belongs to

**When suggesting**:

Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md) and include:

- Problem statement (what pain point does this solve?)
- Proposed solution
- Alternatives considered
- Which perimeter this affects

### Your First Contribution

Look for issues labelled:

- [`good first issue`](https://github.com/hyperpolymath/nexia-list/labels/good%20first%20issue) — Simple Perimeter 3 tasks
- [`help wanted`](https://github.com/hyperpolymath/nexia-list/labels/help%20wanted) — Community help needed
- [`documentation`](https://github.com/hyperpolymath/nexia-list/labels/documentation) — Docs improvements
- [`perimeter-3`](https://github.com/hyperpolymath/nexia-list/labels/perimeter-3) — Community sandbox scope

---

## Development Workflow

### Branch Naming
```
docs/short-description       # Documentation (P3)
test/what-added              # Test additions (P3)
feat/short-description       # New features (P2)
fix/issue-number-description # Bug fixes (P2)
refactor/what-changed        # Code improvements (P2)
security/what-fixed          # Security fixes (P1-2)
```

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):
```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

---

## Questions?

See [MAINTAINERS.adoc](MAINTAINERS.adoc) for who to contact, and
[SECURITY.md](SECURITY.md) for reporting vulnerabilities.
