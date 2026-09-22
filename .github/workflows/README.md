<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Workflows

## Product CI (required checks)

These build and test the actual product and are self-contained — they run
anywhere, including forks:

| Workflow | What it verifies |
|---|---|
| `rust-ci.yml` | `cargo fmt --check`, `clippy -D warnings`, `cargo test` (unit + golden contract + property tests), wasm32 build + wasm-bindgen |
| `ui-ci.yml` | ReScript compile under Bun, wasm core build, `bun test` (TEA update tests + golden-fixture contract tests), Bun bundle, Biome lint |

The same commands run locally: `just test`, `just build`, `just check`
(or the underlying `bun run …` / `cargo …` equivalents).

## Estate workflows (expected to no-op or fail outside hyperpolymath)

Everything else in this directory is governance/scanning plumbing tied to
private or sibling `hyperpolymath` repos (`standards` reusable workflows,
`hypatia`, `casket-ssg`, `a2ml`/`k9` validate actions, `.git-private-farm`,
`boj-server`). On forks or in isolated environments they cannot run and are
**not** indicators of product health:

`boj-build.yml`, `casket-pages.yml`, `dogfood-gate.yml`, `governance.yml`,
`hypatia-scan.yml`, `instant-sync.yml`, `mirror.yml`,
`push-email-notify.yml`, `secret-scanner.yml`

`codeql.yml` is standard GitHub scanning and runs anywhere — its matrix
covers **both** `javascript-typescript` and `rust` (the Rust core has been
scanned since #61; buildless `build-mode: none` is correct for CodeQL Rust).

Branch protection should require `rust-ci` and `ui-ci`; estate workflows
should stay non-required.

## Gate honesty (issue #49)

The verdict test for CI here: *can this check fail when something is actually
wrong?* Every check we own carries a tier label, and every gate is proven by
fail fixtures:

| Tier | Meaning | Examples |
|---|---|---|
| 🔴 GATE | Fails the run on findings | `empty-lint` (C0/NUL bytes), `hypatia-scan` **only when opted in**, `gate-self-test` |
| 🟡 CHECK | Fails on real errors; advisory otherwise | `groove-check` (invalid manifest JSON fails; missing endpoint only warns) |
| ℹ️ ADVISORY | Never fails; labelled so | `dogfood-summary`, Hypatia "Check for critical issues" |

The gate logic lives in runnable scripts (not inline workflow shell), so the
exact code that runs in CI can be exercised locally:

* `scripts/check-invisible-characters.sh` — byte-safe invisible-character scanner
* `scripts/check-groove-manifest.sh` — Groove manifest gate (exit 1 on invalid JSON)
* `scripts/check-hypatia-findings.sh` — Hypatia severity counts + blocking decision

**Prove it:** `scripts/test-ci-honesty.sh` (also `just test-ci-honesty`) runs
each gate against fixtures that MUST pass and fixtures that MUST FAIL, and
structurally checks the workflows still wire the gates in. The `gate-self-test`
job in `dogfood-gate.yml` runs the same proofs on every PR — if a future edit
turns a gate back into an annotation-only fake, CI goes red.

**Opting into the Hypatia gate:** set the repository variable
`HYPATIA_BLOCK_ON_HIGH=true` (Settings → Secrets and variables → Actions →
Variables). Then critical/high findings fail the scan — mirroring the
`block-on-high` input of the estate reusable in `hyperpolymath/standards`.
Default (and estate doctrine) is advisory: findings land on the Security →
Code scanning surface (SARIF category `hypatia`) instead.
