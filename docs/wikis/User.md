<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Nexia-List for managing your mind

This page is for people **using Nexia-List to manage their own mind** — not building it. You write notes; Nexia-List keeps them, connects them, and (as you ask for more) reasons over them and brings the right one back at the right moment. If you want to work on Nexia-List itself, see [Developer](Developer).

The canonical getting-started guide is [QUICKSTART-USER.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/QUICKSTART-USER.adoc). This wiki is the signpost.

## The North Star

> *A note is a letter we send to our future self.* — Mark Bernstein, *The Tinderbox Way*

Everything in Nexia-List serves that one idea: helping the letter reach a richer future self. It runs on your device, in readable JSON you can keep forever — no account, no cloud, no company whose survival your archive depends on.

## What you get on day one (no code, ever)

The base experience — call it **L0** — is a complete, dignified thinking tool you can use for a lifetime without writing a single line of anything.

| You want to… | You do… |
|---|---|
| Get a thought down fast | Quick-capture it into an inbox — no ceremony, no canvas to arrange first |
| Connect ideas | Type `[[Note Title]]` to link notes; backlinks appear automatically |
| See relationships | Arrange notes on the spatial canvas (pan, zoom, double-click to create) |
| Find something | Search across your titles and content |
| Keep a live view | Save a search as an **Agent** — a query that keeps collecting matching notes for you |
| Take it with you | Your notebook is human-readable JSON; download it, keep it, move it anywhere |

You never have to learn a query language or see a parenthesis to get all of this.

## Progressive disclosure: you open the doors

Nexia-List is quiet by default and deep when you want it. Power is arranged as doors you choose to open — **you never see a parenthesis until you open one.** Panels for advanced features only appear when there's actually something to show. Nothing nags; nothing clutters the page until you ask for it.

| Level | What it adds | How it appears |
|---|---|---|
| **L0 — The letter** | Notes, `[[wikilinks]]`, search, canvas, quick-capture | Always on |
| **L1 — Living notes** | See-Also recall, duplicate detection, an inspector for tags/attributes | Appears only when there's something to surface |
| **L2 — Composer** | Command palette, typed reasoning edges, a reasoning view, outline/timeline | Behind a "power" door you turn on |
| **L3+ — Programmer** | λδ code cells, Smart Rules, shareable packages | Deeper doors, entirely optional |

## What opting in unlocks

As you open doors, the neglected half of the loop — **reasoning** and **resurfacing** — comes to life. These are planned capabilities; the design is settled (see the [Developer](Developer) page and the design docs):

- **Recall that writes back.** *See-Also* surfaces forgotten but related notes as you work; duplicate detection offers to merge near-copies. The archive stops being a graveyard you must excavate.
- **Notes that argue with you.** Draw typed, weighted edges (*supports*, *opposes*, *requires*…) between claims; a confidence value flows through the graph so you can ask *"why do I believe this?"* and see the argument. `0.5` means genuinely *Indeterminate*, not false.
- **Rules that tidy for you.** Smart Rules can auto-tag, classify, or file notes when conditions you set are met — always undoable, never silent.

All of it is local, opt-in, and computed *beside* your notes — delete the intelligence and it rebuilds; your letters are never touched.

## Get going

| If you want to… | Go to |
|---|---|
| Install and run in five minutes | [QUICKSTART-USER.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/QUICKSTART-USER.adoc) |
| Read the full vision | [docs/design/mind-management-plan.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md) |
| See where the project stands | [TOPOLOGY.md](https://github.com/hyperpolymath/nexia-list/blob/main/TOPOLOGY.md) |
| Look up a term | [Glossary](Glossary) |

Nexia-List is pre-release (v0.1, ~65% to MVP): you can create, edit, delete, link, and search notes today; the canvas supports pan/zoom, double-click create, and dragging notes, and it is keyboard-navigable. Your notes autosave locally and you can save or load them as a file. The recall and reasoning panels, the richer editor, and the desktop app are on the [roadmap](https://github.com/hyperpolymath/nexia-list/blob/main/ROADMAP.adoc), not shipped yet.

---

See also: [Home](Home) · [Lay-Public](Lay-Public) · [Glossary](Glossary)
