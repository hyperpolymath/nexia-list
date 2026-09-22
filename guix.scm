;; SPDX-License-Identifier: MPL-2.0
;; Guix development environment for nexia-list.
;; Usage: guix shell -D -f guix.scm
;;
;; Covers the native side of the toolchain: Rust (stable) + the C/native
;; bits cargo links against (openssl, pkg-config) + git. Two pieces are
;; intentionally NOT in Guix because they are not packaged upstream:
;;   * Bun 1.3.x — installed via the pinned script (`curl -fsSL
;;     https://bun.sh/install | bash -s "bun-v1.3.14"`), exactly as CI does.
;;   * wasm32-unknown-unknown — added per-toolchain via
;;     `rustup target add wasm32-unknown-unknown` (Guix's rust does not
;;     ship the wasm std as a component).
;; CI (rust-ci.yml / ui-ci.yml) is the source of truth for those steps.

(use-modules (guix packages)
             (guix build-system gnu)
             (guix licenses)
             (gnu packages base)
             (gnu packages bash)
             (gnu packages rust)
             (gnu packages tls)
             (gnu packages pkg-config)
             (gnu packages version-control))

(package
  (name "nexia-list")
  (version "0.1.0")
  (source #f)
  (build-system gnu-build-system)
  (inputs (list coreutils bash git rust pkg-config openssl))
  (synopsis "nexia-list — a3ml-native notebook")
  (description "nexia-list: wasm-core notebook engine with an AffineScript UI
and the lambdadelta (λδ) plugin substrate — part of the hyperpolymath ecosystem.")
  (home-page "https://github.com/hyperpolymath/nexia-list")
  (license ((@@ (guix licenses) license) "MPL-2.0" "https://github.com/hyperpolymath/palimpsest-license")))
