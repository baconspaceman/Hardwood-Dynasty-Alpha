# Roadmap

## Where we are (done)
The Rust simulation engine: era-correct rules from 1946, a moving salary cap, possession-level
games with per-era calibration, injuries (including career-ending), progression, playoffs and
play-in, draft, free agency, trades, finance and owner mode, college, overseas, careers and life
sim, storylines, mods, import/export, and the `hwd` text interface.

## How the next work is organised
Detailed, copy-and-paste-ready work orders live in `docs/requests/` (start with its `README.md`).
Each one can be handed to a local Claude Code or Codex session on its own branch.

## Phase 1: Make the numbers mean something
- **Rating thresholds** (`requests/01`): named tiers per attribute (playable / good / premier /
  elite), a flatter floor so a 50 is not a disaster, era-aware tiers, a `sim.rating_impact` slider.
- Fold in balance fixes found by real play-testing (see "Play-test backlog" below).

## Phase 2: Real, accurate players
- **Data pipeline** (`requests/02`): a separate tool that turns datasets the user downloads into
  roster and year-pack files, with licence notes, name matching, era normalisation and a backtest.
- **Tendencies** (`requests/03`): per-player play-style learned from years of stats, drifting with
  age and role, and used by the sim for shot selection and defence.

## Phase 3: A real game you can see and play
- **Client design and proof of concept** (`requests/04`): pick the engine (Unreal, Unity or Godot),
  define the Rust boundary, build a real-time on-court layer, and prove the over-the-shoulder,
  limited-view camera ("you are the player, you cannot see the whole court") plus a broadcast view.
- **Faces and likeness** (`requests/05`): generic placeholder faces, a community face-pack format,
  and an opt-in, on-device face scan. No real likenesses ship with the game.
- Then: animation set, arenas and crowds, commentary, controller feel, replay system.

## Phase 4: Depth the engine does not have yet
- Contract negotiation minigame (counter-offers, sign-and-trade for the user).
- Draft-day trades and a full expansion draft.
- Referee crews, travel and schedule quirks, in-season tournament as its own competition.
- Locker-room cliques and player-vs-player relationships in the life sim.
- Smarter AI trades (multi-team deals, needs-based and cap-aware).
- Women's pro and college leagues.
- Possession-level simulation for college and overseas games (today they use the faster
  team-level sim).

## Phase 5: Community
- A public library of mods, year packs, settings files and (separately, user-hosted) face packs.
- Mod validator improvements and an in-game mod browser.
- Online leagues and shared saves, if the client proves popular.

## Play-test backlog
Collected as the author plays. Add items here as issues in the repo, then turn the best into
requests in `docs/requests/`.

## Principles that do not change
Deterministic simulation, a UI-free library, everything moddable, every setting explained in plain
English, no real likenesses and no scraped data in the repository.
