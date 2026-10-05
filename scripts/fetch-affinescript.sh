#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# Fetch and build the AffineScript compiler (with the affinescript-tea
# runtime) at the pinned commit into ./.affinescript, which
# scripts/build-ui.sh uses by default. Needs git, opam and an OCaml switch
# (the compiler's own build requirements); re-running is a no-op once the
# pinned commit is built.
set -euo pipefail

# hyperpolymath/affinescript: affinescript-tea, affinescript-router and the
# compiler fixes the UI needs (PRs #777/#778). Bump to the merge commit on main once they land.
AFFINESCRIPT_REPO="https://github.com/hyperpolymath/affinescript.git"
AFFINESCRIPT_REF="f21fdda8949ed2c29a50835be8ebbdecaab8e763"

root="$(cd "$(dirname "$0")/.." && pwd)"
dest="${AFFINESCRIPT_HOME:-$root/.affinescript}"

if [[ ! -d "$dest/.git" ]]; then
  git clone --filter=blob:none --no-checkout "$AFFINESCRIPT_REPO" "$dest"
fi
if [[ "$(git -C "$dest" rev-parse HEAD 2>/dev/null || true)" != "$AFFINESCRIPT_REF" ]]; then
  git -C "$dest" fetch --quiet origin "$AFFINESCRIPT_REF"
  git -C "$dest" -c advice.detachedHead=false checkout --quiet "$AFFINESCRIPT_REF"
fi
(
  cd "$dest"
  opam install . --deps-only --yes >/dev/null
  opam exec -- dune build bin/main.exe
)
echo "AffineScript compiler ready: $dest/_build/default/bin/main.exe"
