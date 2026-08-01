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

`codeql.yml` is standard GitHub scanning and runs anywhere.

Branch protection should require `rust-ci` and `ui-ci`; estate workflows
should stay non-required.
