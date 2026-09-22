#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# check-groove-manifest.sh — Groove protocol manifest gate logic, extracted
# from the dogfood-gate workflow so the *same* code that runs in CI can be
# exercised locally against fail fixtures (issue #49 — a gate that cannot be
# run is a gate that cannot fail).
#
# Exit codes:
#   0 — everything fine (advisories may have been printed; advisories do not gate)
#   1 — a manifest exists but is INVALID JSON (real error — this is the gate)
#   2 — internal/usage error (refuse a partial pass)
#
# Behaviour (faithful to rsr-template-repo/.github/workflows/groove-check.yml):
#   * canonical manifest location is www/.well-known/groove/manifest.json;
#     the repository-root path is accepted, with a warning, during migration.
#   * a manifest that does not parse as JSON is a HARD FAILURE (this is what
#     turns the check from an annotation-only fake into a real gate).
#   * a repo with production HTTP-server markers but no Groove endpoint is an
#     ADVISORY warning only — not every HTTP service is meant to be discovered.
set -u

root="${1:-}"
if [ -z "$root" ] || [ ! -d "$root" ]; then
  echo "usage: $0 REPO_ROOT" >&2
  exit 2
fi
cd "$root" || exit 2

command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 2; }

emit_output() {
  # Only write step outputs when running inside GitHub Actions.
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "$1=$2" >> "$GITHUB_OUTPUT"
  else
    echo "$1=$2"
  fi
}

HAS_MANIFEST="false"
HAS_GROOVE_CODE="false"
HAS_SERVER="false"
SERVICE_ID=""

MANIFEST=""
if [ -f "www/.well-known/groove/manifest.json" ]; then
  MANIFEST="www/.well-known/groove/manifest.json"
elif [ -f ".well-known/groove/manifest.json" ]; then
  MANIFEST=".well-known/groove/manifest.json"
  echo "::warning::Groove manifest at legacy root .well-known/ — canonical location is www/.well-known/ (run scripts/migrate-wellknown-to-www.sh)"
fi

if [ -n "$MANIFEST" ]; then
  HAS_MANIFEST="true"
  if ! jq empty "$MANIFEST" 2>/dev/null; then
    echo "::error file=$MANIFEST::Invalid JSON in Groove manifest"
    emit_output has_manifest "$HAS_MANIFEST"
    emit_output has_groove_code "$HAS_GROOVE_CODE"
    emit_output has_server "$HAS_SERVER"
    exit 1   # <-- THE GATE: invalid manifests fail the check.
  fi
  SERVICE_ID=$(jq -r '.service_id // "unknown"' "$MANIFEST")
fi

# Groove endpoint implemented in code?
if grep -rl 'well-known/groove' --include='*.rs' --include='*.ex' --include='*.zig' --include='*.v' --include='*.res' . 2>/dev/null | grep -v '^./scripts/' | head -1 | grep -q .; then
  HAS_GROOVE_CODE="true"
fi

# Production HTTP-server markers only. A bare `TcpListener` is deliberately NOT
# a signal (dominated by test/utility use); real servers pair the listener with
# a framework entry point.
if grep -rl 'Bandit\|Plug.Cowboy\|httpz\|vweb\|axum::serve\|actix_web\|hyper::Server\|rocket::build\|warp::serve' --include='*.rs' --include='*.ex' --include='*.zig' --include='*.v' . 2>/dev/null | head -1 | grep -q .; then
  HAS_SERVER="true"
fi

emit_output has_manifest "$HAS_MANIFEST"
emit_output has_groove_code "$HAS_GROOVE_CODE"
emit_output has_server "$HAS_SERVER"
if [ -n "$SERVICE_ID" ]; then
  emit_output service_id "$SERVICE_ID"
fi

if [ "$HAS_SERVER" = "true" ] && [ "$HAS_MANIFEST" = "false" ] && [ "$HAS_GROOVE_CODE" = "false" ]; then
  echo "::warning::This repo has server code but no Groove endpoint. Add www/.well-known/groove/manifest.json for service discovery. (ADVISORY — this does not fail the check.)"
fi

exit 0
