#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# test-ci-honesty.sh — fail-fixture proofs for nexia-list's CI gates.
#
# Issue #49 verdict test: *can this check fail when something is actually
# wrong?*  This script answers it empirically, for every gate we own, by
# running the REAL gate logic (the same scripts the workflows invoke) against
# fixtures that MUST pass and fixtures that MUST FAIL, plus structural checks
# that the workflows still wire the gates in (drift alarms).
#
# Usage: scripts/test-ci-honesty.sh [REPO_ROOT]
# Exit:  0 = every proof passed; 1 = at least one gate could not fail when it
#        should (or passed when it must not); 2 = environment problem.
set -u

ROOT="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
SCANNER="$ROOT/scripts/check-invisible-characters.sh"
GROOVE="$ROOT/scripts/check-groove-manifest.sh"
HYPATIA="$ROOT/scripts/check-hypatia-findings.sh"
DOGFOOD="$ROOT/.github/workflows/dogfood-gate.yml"
HYPATIA_WF="$ROOT/.github/workflows/hypatia-scan.yml"

failures=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; failures=$((failures + 1)); }

expect_exit() {
  # expect_exit DESCRIPTION EXPECTED_CODE CMD...
  local desc="$1" want="$2"; shift 2
  "$@" >/dev/null 2>&1
  local got=$?
  if [ "$got" -eq "$want" ]; then pass "$desc (exit $got)"; else fail "$desc (wanted exit $want, got $got)"; fi
}

for f in "$SCANNER" "$GROOVE" "$HYPATIA" "$DOGFOOD" "$HYPATIA_WF"; do
  [ -f "$f" ] || { echo "missing: $f" >&2; exit 2; }
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "=== scanner [1/5] empty-linter (invisible characters) ==="

# Fixture: a clean source file — must produce NO findings.
mkdir -p "$WORK/clean"
printf 'fn main() { println!("clean"); }\n' > "$WORK/clean/main.rs"
"$SCANNER" "$WORK/clean" "$WORK/clean.res" "$WORK/clean.blk"
rc=$?
if [ "$rc" -eq 0 ] && [ ! -s "$WORK/clean.res" ] && [ ! -s "$WORK/clean.blk" ]; then
  pass "clean fixture yields no findings"
else
  fail "clean fixture should yield no findings (rc=$rc)"
fi

# Fixture: zero-width space (E2 80 8B) — warning tier, NOT blocking.
mkdir -p "$WORK/zwsp"
printf 'fn main() { let s = "a\xe2\x80\x8bb"; }\n' > "$WORK/zwsp/main.rs"
"$SCANNER" "$WORK/zwsp" "$WORK/zwsp.res" "$WORK/zwsp.blk"
rc=$?
if [ "$rc" -eq 0 ] && [ -s "$WORK/zwsp.res" ] && [ ! -s "$WORK/zwsp.blk" ]; then
  pass "ZWSP fixture is advisory (findings, no blocking)"
else
  fail "ZWSP fixture should be advisory-only (rc=$rc)"
fi

# Fixture: NUL byte — BLOCKING tier. This is the fail fixture: the gate must bite.
mkdir -p "$WORK/nul"
printf 'fn main() { let b = 0;\x00 }\n' > "$WORK/nul/main.rs"
"$SCANNER" "$WORK/nul" "$WORK/nul.res" "$WORK/nul.blk"
rc=$?
if [ -s "$WORK/nul.blk" ]; then
  pass "NUL fixture lands in blocking results — gate has input to fail on"
else
  fail "NUL fixture MUST land in blocking results (rc=$rc)"
fi

# Decision-rule honesty: the exact rule the workflow applies after the scan
# (`if [ "$BLOCKING" -gt 0 ]; then exit 1`). Blocking results exist => the
# rule would fire. Verified structurally below and empirically by the runner
# exercising the decision here.
BLOCKING=$(tr -cd '\0' < "$WORK/nul.blk" | wc -c)
expect_exit "NUL fixture trips the gate decision rule (blocking=$BLOCKING > 0 => workflow exits 1)" 0 \
  test "$BLOCKING" -gt 0

echo "=== drift [2/5] workflow wiring (structural honesty) ==="

if grep -q 'scripts/check-invisible-characters.sh' "$DOGFOOD"; then
  pass "dogfood-gate runs the shared scanner (not a silent inline copy)"
else
  fail "dogfood-gate.yml MUST call scripts/check-invisible-characters.sh"
fi
if grep -qE 'if \[ "\$BLOCKING" -gt 0 \]' "$DOGFOOD" && grep -q 'exit 1' "$DOGFOOD"; then
  pass "dogfood-gate empty-lint contains a real exit-1 gate"
else
  fail "dogfood-gate.yml empty-lint lost its exit-1 gate"
fi
if grep -q 'ADVISORY' "$DOGFOOD"; then
  pass "non-gating summary job is labelled ADVISORY"
else
  fail "dogfood-gate summary must be labelled ADVISORY (non-gating)"
fi
if grep -q 'scripts/check-groove-manifest.sh' "$DOGFOOD"; then
  pass "dogfood-gate groove-check runs the shared manifest check"
else
  fail "dogfood-gate.yml groove-check MUST call scripts/check-groove-manifest.sh"
fi

echo "=== groove [3/5] manifest check ==="

# Fixture: valid canonical manifest — must pass.
mkdir -p "$WORK/g-ok/www/.well-known/groove"
printf '{"service_id":"demo","endpoints":[]}\n' > "$WORK/g-ok/www/.well-known/groove/manifest.json"
expect_exit "valid groove manifest passes" 0 "$GROOVE" "$WORK/g-ok"

# Fixture: INVALID JSON manifest — must FAIL. (The pre-fix workflow annotated
# ::error and stayed green; this is the fixture that proves the fix.)
mkdir -p "$WORK/g-bad/www/.well-known/groove"
printf '{"service_id": broken\n' > "$WORK/g-bad/www/.well-known/groove/manifest.json"
expect_exit "INVALID groove manifest FAILS the check" 1 "$GROOVE" "$WORK/g-bad"

# Fixture: server code without a manifest — advisory only, must still pass.
mkdir -p "$WORK/g-srv/src"
printf 'use axum; fn main() { axum::serve(listener, app); }\n' > "$WORK/g-srv/src/main.rs"
"$GROOVE" "$WORK/g-srv" > "$WORK/g-srv.out" 2>&1
rc=$?
if [ "$rc" -eq 0 ] && grep -q '::warning::' "$WORK/g-srv.out"; then
  pass "missing manifest with server code is advisory (exit 0 + warning)"
else
  fail "missing manifest with server code must be advisory, not failing (rc=$rc)"
fi

echo "=== hypatia [4/5] findings gate ==="

# Fixture: one critical finding.
cat > "$WORK/findings-critical.json" <<'JSON'
[{"rule_module":"demo","severity":"critical","type":"vuln","file":"src/x.rs","reason":"fixture"}]
JSON
expect_exit "advisory mode does not gate on critical" 0 "$HYPATIA" "$WORK/findings-critical.json"
expect_exit "--block refuses critical findings" 1 "$HYPATIA" "$WORK/findings-critical.json" --block

# Fixture: one HIGH finding only.
cat > "$WORK/findings-high.json" <<'JSON'
[{"rule_module":"demo","severity":"high","type":"vuln","file":"src/x.rs","reason":"fixture"}]
JSON
expect_exit "--block alone tolerates high-only findings" 0 "$HYPATIA" "$WORK/findings-high.json" --block
expect_exit "--block --high-too refuses high findings" 1 "$HYPATIA" "$WORK/findings-high.json" --block --high-too

# Fixture: clean scan blocks nothing.
printf '[]\n' > "$WORK/findings-clean.json"
expect_exit "clean findings never block" 0 "$HYPATIA" "$WORK/findings-clean.json" --block --high-too

# Fixture: malformed findings must fail LOUDLY (exit 2), never silently pass.
printf 'this is not json\n' > "$WORK/findings-broken.json"
expect_exit "malformed findings fail loudly" 2 "$HYPATIA" "$WORK/findings-broken.json" --block

echo "=== hypatia [5/5] workflow wiring (structural honesty) ==="

if grep -q 'HYPATIA_BLOCK_ON_HIGH' "$HYPATIA_WF"; then
  pass "hypatia-scan wires the HYPATIA_BLOCK_ON_HIGH opt-in gate"
else
  fail "hypatia-scan.yml MUST wire the HYPATIA_BLOCK_ON_HIGH gate"
fi
if grep -q 'scripts/check-hypatia-findings.sh' "$HYPATIA_WF"; then
  pass "hypatia-scan runs the shared findings gate script"
else
  fail "hypatia-scan.yml MUST call scripts/check-hypatia-findings.sh"
fi
if grep -q 'ADVISORY — does not gate' "$HYPATIA_WF"; then
  pass "hypatia advisory step is labelled ADVISORY — does not gate"
else
  fail "hypatia-scan.yml advisory step lost its honesty label"
fi

echo
if [ "$failures" -gt 0 ]; then
  echo "CI HONESTY: $failures proof(s) FAILED — a gate cannot fail when it should (or fails when it must not)."
  exit 1
fi
echo "CI HONESTY: all proofs passed — every gate can fail and every advisory is labelled."
exit 0
