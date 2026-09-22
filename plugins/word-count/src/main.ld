;; SPDX-License-Identifier: MPL-2.0
;; word-count/src/main.ld — the reference λδ package for Nexia-List.
;; Counts words per note and can stamp the :word-count attribute on every
;; note. Requested grants (manifest.ld): :notes/read + :notes/write.

;; Words in a single note's content.
(def note-word-count
  (fn [n]
    (count (words (content n)))))

;; Notes shorter than `limit` words.
(def short-notes
  (fn [limit]
    (filter (fn [n] (< (note-word-count n) limit)) (notes))))

;; Stamp :word-count onto every note. Requires :notes/write; a sandbox that
;; was granted read-only access fails this with a structured capability
;; denial — never a panic, never a partial write.
(def annotate-word-counts!
  (fn []
    (map
      (fn [n] (set-attr! (:id n) "word-count" (note-word-count n)))
      (notes))))
