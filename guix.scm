; SPDX-License-Identifier: MPL-2.0
;; guix.scm — GNU Guix package definition for nexia-list
;; Usage: guix shell -f guix.scm

(use-modules (guix packages)
             (guix build-system gnu)
             (guix licenses))

(package
  (name "nexia-list")
  (version "0.1.0")
  (source #f)
  (build-system gnu-build-system)
  (synopsis "nexia-list")
  (description "nexia-list — part of the hyperpolymath ecosystem.")
  (home-page "https://github.com/hyperpolymath/nexia-list")
  (license ((@@ (guix licenses) license) "MPL-2.0"
             "https://github.com/hyperpolymath/palimpsest-license")))
