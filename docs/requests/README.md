# Handoff requests

These are **work orders, not finished features**. Each file is written so a brand-new local
Claude Code or Codex session can read it, understand the project, and start without any other
context. Nothing in this folder changes the game by itself.

## How to use one
1. Open a fresh session in a clone of this repo.
2. Tell it: *"Read `docs/ARCHITECTURE.md`, then do the work in `docs/requests/<file>`."*
3. Make it work on its own branch and open a draft PR. Keep `cargo test --release` green.

## The requests, in a sensible order
| # | File | What it is | Depends on |
|---|---|---|---|
| 1 | `01-attribute-thresholds.md` | Make ratings mean something: what counts as poor / average / good / elite per attribute, and how much it matters in the sim | nothing |
| 2 | `02-real-data-pipeline.md` | A way to build accurate rosters, stats and tendencies for real players from public datasets, kept out of the repo | 1 (helps) |
| 3 | `03-tendencies-algorithm.md` | Per-player play-style "tendencies" learned from a player's seasons, era by era | 2 (or generated data) |
| 4 | `04-3d-game-client-design.md` | Design for a real 3D game (Unreal/Unity/Godot) with a limited-view, over-the-shoulder camera | 1, 3 |
| 5 | `05-likeness-and-face-scan.md` | Placeholder faces, a community replacement system, and an optional on-device face scan | 4 |

The big plan is in `docs/ROADMAP.md`.

## Rules every request shares
- The engine stays **deterministic** (same seed gives the same game) and stays a **library** with no
  graphics code in it.
- **No real likenesses and no scraped data are committed to the repo.** Real-player data is
  built on the user's own machine from sources they download themselves.
- Every new setting needs a plain-English description (a test enforces that every setting is used).
- Anything tweakable goes through the mod system (`docs/MODDING.md`), not hard-coded.
- Add tests. Update docs. Don't add dependencies without saying why.
