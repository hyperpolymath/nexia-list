#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# check-hypatia-findings.sh — severity counts + optional blocking decision for
# a Hypatia findings JSON file. Extracted from the hypatia-scan workflow so the
# gate decision is runnable (and provable) outside CI (issue #49).
#
# Semantics mirror the estate canonical reusable
# (hyperpolymath/standards .github/workflows/hypatia-scan-reusable.yml):
#   * advisory by default — findings are surfaced, never gate;
#   * when blocking is enabled (the workflow passes --block, gated on the
#     HYPATIA_BLOCK_ON_HIGH repository variable, mirroring the reusable's
#     `block-on-high` input) the decision refuses *critical* findings, and
#     additionally *high* findings when --high-too is given.
#
# Exit codes:
#   0 — no blocking finding under the selected policy (or advisory mode)
#   1 — blocking finding(s) present while --block is active
#   2 — the findings file is missing/malformed (fail loud, never silently pass)
set -u

findings_file=""
block="false"
high_too="false"

for arg in "$@"; do
  case "$arg" in
    --block) block="true" ;;
    --high-too) high_too="true" ;;
    -*) echo "unknown flag: $arg" >&2; exit 2 ;;
    *) findings_file="$arg" ;;
  esac
done

if [ -z "$findings_file" ] || [ ! -f "$findings_file" ]; then
  echo "usage: $0 FINDINGS_JSON [--block] [--high-too]" >&2
  exit 2
fi
command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 2; }

# Malformed input must fail LOUDLY: a scanner whose output we cannot read is a
# scanner in an unknown state, not a clean scanner (the verdict test of #49 —
# "can this check fail when something is actually wrong?").
if ! jq -e 'type == "array"' "$findings_file" >/dev/null 2>&1; then
  echo "::error::Hypatia findings file is not a JSON array: $findings_file"
  exit 2
fi

CRITICAL=$(jq '[.[] | select(.severity == "critical")] | length' "$findings_file")
HIGH=$(jq '[.[] | select(.severity == "high")] | length' "$findings_file")
MEDIUM=$(jq '[.[] | select(.severity == "medium")] | length' "$findings_file")
TOTAL=$(jq 'length' "$findings_file")

echo "hypatia findings: total=$TOTAL critical=$CRITICAL high=$HIGH medium=$MEDIUM"

blocking="$CRITICAL"
policy="critical"
if [ "$high_too" = "true" ]; then
  blocking=$((CRITICAL + HIGH))
  policy="critical+high"
fi

if [ "$block" = "true" ] && [ "$blocking" -gt 0 ]; then
  echo "::error::Hypatia found $blocking blocking finding(s) (policy: $policy) — refusing, HYPATIA_BLOCK_ON_HIGH is enabled"
  exit 1
fi

if [ "$blocking" -gt 0 ]; then
  echo "::warning::Hypatia found $blocking $policy finding(s) — ADVISORY only, this does not fail the check"
fi

exit 0
