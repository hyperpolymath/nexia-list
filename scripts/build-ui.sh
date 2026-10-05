#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# Build the web app into web/dist/: compile the AffineScript UI (ui/src) with
# the AffineScript compiler's Bun-ESM backend and assemble it with the TEA
# host, the Nexia host, the WASM core (web/wasm, from `bun run build:wasm`)
# and the static assets.
#
# The compiler is found at $AFFINESCRIPT_HOME (a hyperpolymath/affinescript
# checkout with `dune build` done); by default ./.affinescript, which
# scripts/fetch-affinescript.sh creates at the pinned commit.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
as_home="${AFFINESCRIPT_HOME:-$root/.affinescript}"
compiler="$as_home/_build/default/bin/main.exe"
tea_src="$as_home/affinescript-tea/src"
router_src="$as_home/affinescript-router/src"
dist="$root/web/dist"

if [[ ! -x "$compiler" ]]; then
  echo "error: AffineScript compiler not found at $compiler" >&2
  echo "  run scripts/fetch-affinescript.sh, or set AFFINESCRIPT_HOME to a built checkout" >&2
  exit 1
fi
if [[ ! -f "$root/web/wasm/nexia_core_bg.wasm" ]]; then
  echo "error: web/wasm is empty — run 'bun run build:wasm' first" >&2
  exit 1
fi

mkdir -p "$dist/wasm"
(
  cd "$root/ui/src"
  AFFINESCRIPT_STDLIB="$as_home/stdlib" AFFINESCRIPT_PATH="$tea_src:$router_src" \
    "$compiler" compile --bun-esm App.affine -o "$dist/app.bun.js"
)
cp "$tea_src/tea_host.js" "$dist/tea_host.js"
cp "$router_src/router_host.js" "$dist/router_host.js"
cp "$root/ui/host/nexia_host.js" "$dist/nexia_host.js"
cp "$root"/web/wasm/nexia_core.js "$root"/web/wasm/nexia_core_bg.wasm "$dist/wasm/"
for asset in index.html styles.css manifest.webmanifest service-worker.js icon.svg; do
  cp "$root/web/$asset" "$dist/$asset"
done
echo "web/dist built ($(du -sh "$dist" | cut -f1))"
