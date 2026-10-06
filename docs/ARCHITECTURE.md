# Architecture

Hardwood Dynasty is a Rust library (`hardwood_dynasty`) plus a text front end (`hwd`). The library has
no I/O and no UI; any front end (terminal, GUI, web, bot) can drive it.

## Design principles

1. **Depth underneath, clarity on top.** Systems are deep; screens and settings explain themselves.
2. **Everything is data.** Rules, injuries, badges, archetypes, awards, franchises, life events and
   economy tables live in `Content` and can be patched by mods. Code only knows how to *apply* data.
3. **Deterministic.** One seed drives everything through a hand-written RNG (`rng.rs`); save, load and
   continue gives byte-identical results.
4. **Possession-level truth.** Every pro game is simulated play by play, so box scores, season stats,
   awards and records are all consistent. College and overseas use a fast team-level model by default.
5. **Era-relative ratings.** A 70 means "a good starter in his own league". The era profile decides how
   that league plays.

## Module map

| Module | Role |
|---|---|
| `rng` | seedable generator, gaussian, weighted choice |
| `settings` | typed settings registry, presets, plain-English help |
| `era` | rule timeline (patches by year), era "style" (pace, 3-point rate, percentages) |
| `economy` | cap/tax/apron tables, pay scale, market-value curve |
| `player`, `generate` | attributes, archetypes, badges, hidden traits, stat lines, player creation |
| `progression` | age curves, development, retirement odds |
| `injury` | injury catalog and the risk/severity/aftermath model |
| `events`, `life` | declarative event engine and the life sim built on it |
| `game` | the possession engine and its self-calibration |
| `fastsim` | team-level game model for college/overseas |
| `team`, `league` | the world: teams, staff, owners, players, history |
| `setup` | building a league for any start year |
| `season`, `playoffs` | schedules, daily simulation, playoffs and play-in |
| `offseason`, `draft`, `trade`, `finance` | contracts, free agency, draft lottery, trade AI, money |
| `college`, `overseas` | amateur and overseas worlds |
| `career` | roles, create-a-player, decisions |
| `story`, `leagueevents` | narrative engine, lockouts/expansion/mergers |
| `awards`, `records` | awards (data-driven), record book |
| `import` | CSV/JSON rosters, year packs, stat-to-rating inference |
| `calendar` | the state machine that moves time and stops for decisions |
| `content` | the content registry and the mod system |

## The calendar

`Preseason → RegularSeason → (PlayIn →) Playoffs → PostSeason → Draft → FreeAgency → Preseason(next)`.
`League::step()` performs one unit; `League::advance(Goal)` loops until a goal is reached or the human
must decide something (a draft pick, pending decision, illegal roster, life event).

## How the game engine stays on target

`game::calibrate` plays sample games and adjusts a small set of offsets (`Cal`) until league-wide pace,
2P%, 3P%, free-throw rate, turnovers, offensive rebounding and three-point volume match the era style.
The league re-runs it each preseason and again in-season so health and fatigue patterns are included.
Player *ratings* then decide who is better than whom.

## Saves

`League::save_json` / `load_json`. Content is rebuilt from the saved list of mods; settings values and
the whole world are saved. Old, forgotten retirees have their history folded into career lines to keep
saves smaller over many decades.
