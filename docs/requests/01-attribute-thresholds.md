# Request 1: Rating thresholds ("what does a 50 mean?")

## Goal
Ratings should matter, but not too much. A 99 passer is the best passer ever and that should show.
A 50 passer is **not** the worst passer in the league: it is a perfectly usable, average-ish
NBA-level skill. Right now the engine treats ratings mostly as a smooth line; this request adds
named **tiers** so the numbers have meaning and the sim has sensible floors.

## Where things live today
- 36 attributes in `src/player.rs` (`attrs!` macro: close shot, layup, three point, pass accuracy,
  steal, block, speed, and so on, grouped into `Family`).
- `compute_overall` turns attributes into an overall rating.
- `src/game.rs` turns attributes into possession outcomes (shot make %, turnover %, steal %...).
- Tuning numbers are data-driven (`default_tuning` / `docs/TUNING.md`) and moddable.

## What to build
1. **A threshold table per attribute**, as data (a content list in `src/content.rs`, so mods can
   change it). For each attribute, five cut points, for example for `steal`:

   | Tier | Rating | Meaning |
   |---|---|---|
   | Weak | below 35 | a real liability |
   | Playable | 35-59 | fine; a 50 steal is "okay" |
   | Good | 60-74 | clearly above average |
   | Premier | 75-89 | an 85 steal is premier |
   | Elite | 90+ | amazing; league-best |

   Tiers will differ by attribute. A 60 vertical and a 60 free throw are not the same thing.
   Start from the existing league distributions, then adjust by hand.
2. **A floor and a ceiling curve in the sim.** Replace "rating maps linearly to effect" with a
   curve that is flat-ish at the bottom (a 50 is not a disaster), steepens through Good to
   Premier, and rewards Elite clearly. Keep the league average per era on target: the existing
   calibration (`calibrate`, `Cal`, `Refs` in `src/game.rs`) must keep landing within ~2%.
3. **Era-aware tiers.** A 40 three-point rating was fine in 1975 and is a liability today. Tiers
   for shooting and spacing should slide with the era (reuse the era anchors in `src/era.rs`).
4. **Show the tiers to the user.** Everywhere a rating is printed in `hwd`, add the tier word
   (and optionally colour). A `ratings` help screen explains the table in plain English.
5. **A setting** `sim.rating_impact` (slider): low = ratings matter less (more upsets, flatter
   league), high = ratings matter more. Default is the new balanced curve. Document low/high ends.

## Acceptance
- Tier table is data, moddable, and validated by `hwd mod check`.
- Calibration tests (`tests/calib_league.rs`, `tests/drift.rs`) still pass.
- New tests: a 50-rated player's stat line is clearly "playable"; a 99 clearly leads the league; an
  85 steal produces premier-level steal rates; monotonic (higher never produces worse results).
- `docs/TUNING.md` / `docs/SETTINGS.md` regenerated with `hwd gen-docs docs`.
