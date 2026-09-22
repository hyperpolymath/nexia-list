;; SPDX-License-Identifier: MPL-2.0
;; word-count/test/main.test.ld — harness tests. These run inside the λδ
;; harness against the fixture notebook built by
;; core/tests/lambdadelta_plugin_system.rs:
;;
;;   Alpha  = "one two three four five" → 5 words
;;   Beta   = ""                        → 0 words
;;   Gamma  = "just three here"         → 3 words
;;
;; assert-eq/assert RECORD into the report — an aborted run is itself a
;; failure, so a red report always tells the whole story.

(assert-eq 3 (count (notes)))

;; Pure reads under :notes/read.
(assert-eq 5 (note-word-count (note (resolve-title "Alpha"))))
(assert-eq 0 (note-word-count (note (resolve-title "Beta"))))
(assert-eq 2 (count (short-notes 5)))            ; Beta + Gamma are short
(assert-eq 3 (count (short-notes 10)))           ; everything is short of 10

;; The mutation path under :notes/write …
(annotate-word-counts!)
(assert-eq 5 (attr (note (resolve-title "Alpha")) :word-count))
(assert-eq 0 (attr (note (resolve-title "Beta")) :word-count))
