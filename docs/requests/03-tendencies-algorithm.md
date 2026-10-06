# Request 3: Player tendencies, learned across years

## Goal
Ratings say what a player *can* do. **Tendencies** say what he *chooses* to do: how often he
shoots threes, drives, posts up, passes out of a double team, crashes the glass, gambles for
steals. Two players with the same ratings should play differently, and a player's tendencies should
change as he ages and as the era changes.

## Where things live today
- Players have 36 attributes, a position, archetype, and hidden traits (`Hidden` in `src/player.rs`).
- Team-level style lives in `Strategy` (`src/game.rs`). Player shot selection is derived from
  ratings and archetype, with no explicit per-player tendency vector.
- Progression (`src/progression.rs`) ages attributes. There is no tendency ageing.

## What to build
1. **A `Tendencies` struct on `Player`** (compact, serialised, default-filled for old saves).
   Suggested fields, each 0-100 and documented: three-point rate, mid-range rate, rim attack, post-up,
   pull-up vs catch-and-shoot, drive-and-kick, pick-and-roll ball handler / roller / popper, off-ball
   movement, transition push, offensive-glass crash, defensive gambling (steals vs staying home),
   help-defence aggression, foul-drawing, shot-taking selfishness (usage appetite), clutch appetite.
2. **Learning from history.** Given a player's season stat lines across years, fit tendencies:
   - Shot mix from attempts by zone and type; assists and turnover rates for creation role; usage.
   - Normalise by era first (3-point attempt rate means something very different in 1985 and 2020).
   - Use shrinkage: few games means lean toward the position/archetype prior, many games means
     trust the data. Explain this choice in a comment for beginners.
   - Output a **confidence** per tendency so the UI can say "estimated".
3. **An age and role curve.** Tendencies drift: young scorers take more shots, veterans take fewer
   and pass more; an injury (knees, Achilles) lowers rim attack. Model this as small yearly drift
   toward an age target plus coach-system pull (`Strategy`) and role changes (starter to bench).
4. **Use them in the sim.** `src/game.rs` should pick shot type, passer, and defensive action using
   tendencies *weighted* by ratings (a player with a high drive tendency and poor layup rating still
   drives, and is less efficient doing so). Keep league-level calibration on target.
5. **Generated players** (`src/generate.rs`) get tendencies from archetype plus noise so
   non-imported leagues work too. Imported players use the learned fit when stats exist.
6. **Moddable and visible.** Tendency definitions live in the content registry. `hwd player <id>`
   shows them with plain-English labels; coaches can set per-player tendency overrides ("let him
   shoot more threes") with a cost in chemistry and efficiency.

## Acceptance
- Determinism and save/load round-trip tests still pass; old saves load.
- A test: two same-rating players with different tendencies produce different shot charts.
- A backtest: fit tendencies from a seasons table and check the sim reproduces each player's
  shot mix within a tolerance.
- Docs: `docs/ARCHITECTURE.md` gets a Tendencies section; `docs/MODDING.md` explains the fields.
