;; SPDX-License-Identifier: PMPL-1.0-or-later
(ecosystem
  (metadata
    (version "0.1.0")
    (last-updated "2026-03-02"))
  (project
    (name "nexia-list")
    (purpose "Cross-platform personal knowledge management inspired by Tinderbox")
    (role application))
  (related-projects
    (project "rescript-tea" (relationship dependency) (purpose "TEA architecture for ReScript"))
    (project "cadre-tea-router" (relationship dependency) (purpose "URL-based navigation for TEA apps"))
    (project "bunsenite" (relationship potential-consumer) (purpose "Nickel-based configuration schemas"))))
