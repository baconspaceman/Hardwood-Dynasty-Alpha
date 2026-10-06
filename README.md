# Hardwood Dynasty

**A deep, era-accurate basketball simulation where you can be the GM, the coach, the owner, a scout,
a college coach, or a kid in high school chasing the NBA.**

Start in any season from **1946** to the present (and beyond). The rules of the game change exactly
when they really changed: no shot clock until 1954, no three-pointer until 1979, hand-checking until
2004, a salary cap only since 1984, the play-in tournament since 2020. Players, teams, money and
storylines evolve around you, decade after decade.

Everything is **moddable** (rules, injuries, life events, archetypes, badges, awards, even the league's
whole history), every setting explains itself, and you can **import your own rosters and years**.

> This is a text-based game for now: it runs in a terminal. The engine is a separate library, so a
> graphical front end can be built on top of it later.

---

## Quick start

You need [Rust](https://rustup.rs) (free; the installer takes a minute). Then:

```bash
git clone https://github.com/baconspaceman/Hardwood-Dynasty-Alpha
cd Hardwood-Dynasty-Alpha
cargo run --release
```

At the `>` prompt:

```text
new 1996 mygame        # start a league in the 1996-97 season (try 1962, 1984, 2024, 1946!)
role gm Chicago        # become General Manager of the Chicago team
roster                 # your players
advance week           # play a week.  Also: advance season, advance years 10
status                 # what needs your attention
help                   # every command
```

The game **stops and tells you why** whenever it needs you: your draft pick, an expiring contract, a
life event for your player, an illegal roster. Nothing happens behind your back that you didn't allow.

On Windows, `cargo build --release` produces `target\release\hwd.exe`, which you can double-click or
run from any terminal.

---

## Ways to play

| Role | Command | What you do |
|---|---|---|
| **Player** (create-a-player) | `create-player` | Build a player and live his whole life: **high school → college → draft or overseas → pro career → retirement**, including grades, family, money, taxes, agents, endorsements, stress and scandals |
| **General Manager** | `role gm <team>` | Trades, free agency, the draft, the cap, staff, scouting |
| **Head Coach** | `role coach <team>` | Starters, minutes, tempo, three-point emphasis, defense, practice focus |
| **Owner** | `role owner <team>` | A one-page money dashboard, ticket prices, budgets, the luxury tax, **Chase Mode** (see what spending 25M more costs and how much it improves title odds) |
| **Scout / Assistant** | `role scout <team>` | Reduce the fog around prospects; climb the ladder to GM or head coach |
| **College Coach / AD / Scout** | `role college_coach <school>` | Recruiting, NIL money (modern eras), the transfer portal, the national tournament |

Roles stack: an owner can run the bench and the front office too, or delegate (`delegate gm on`).
If you're fired, offers arrive. A player can retire and become a coach.

Want a story? `scenarios` lists situations like *The Long Rebuild*, *End of an Era* and *Franchise Player*.

---

## What's inside

* **A possession-by-possession game engine**: lineups, substitutions, fatigue, fouls and foul trouble,
  shot selection by skill, defense, rebounds, free throws, turnovers, steals, blocks, assists,
  technical and flagrant fouls, late-game fouling, overtime and in-game injuries. It **calibrates
  itself** to each era, so 1962 plays at 125 possessions a game and 1999 grinds at 89.
* **36-attribute players** (NBA-2K style), archetypes (Sharpshooter, Rim Protector, Point Center...),
  Bronze/Silver/Gold/Hall-of-Fame badges, hidden traits (work ethic, ego, injury-prone), potential you
  have to *scout*, and careers from teenager to Hall of Fame.
* **Injuries from a rolled ankle to a career-ending spinal condition**: 45 injury types, load
  management, playing through pain, permanent athleticism loss after ACLs and Achilles ruptures.
* **Real history**: franchises that move, merge and fold; a salary cap table from 1984; luxury tax and
  aprons; reserve clause → free agency → restricted free agency; draft lotteries for each era; high
  schoolers allowed when they really were; European and international players arriving by era; the
  EuroLeague and overseas leagues; college basketball with a national tournament.
* **A living economy**: owners with personalities, revenue and expenses, ticket prices, TV money,
  revenue sharing, and a salary cap that *keeps moving* after the last real season.
* **Storylines**: dynasties, rivalries, comebacks, "last dances", superteams, Cinderella runs.
* **A life sim** for your created player, driven by data you can edit.

---

## Make it yours (settings)

`settings` shows about 70 options in 12 categories. Every one explains itself:

```text
settings explain injuries.frequency
settings set injuries.frequency 2
settings preset hardcore        # arcade, balanced, sim, hardcore, sandbox
settings search draft
```

See [docs/SETTINGS.md](docs/SETTINGS.md) for the full list with what each low and high setting does.

## Mod it

Everything is data. Export the defaults, edit, drop your file in a `mods/` folder, and start a new league:

```bash
hwd mod export-defaults mod-templates   # editable JSON for every list in the game
hwd mod check mods/my_mod.json          # plain-English errors
hwd mod tuning                          # named parameters of the game engine
```

A mod can change the rules timeline, add new eras, injuries, badges, archetypes, awards, franchises,
countries and name pools, life stats/activities/events with choices, and engine tuning. See
[docs/MODDING.md](docs/MODDING.md) and `examples/mods/`.

## Import rosters and whole years

```text
import roster examples/roster.csv replace     # swap a team's roster (ratings optional!)
newpack examples/year_pack.json               # start a league from a shared "year pack"
export pack myleague.json                     # share your league as a pack
```

Give just an overall and a position, or real per-game stats, and the engine builds a full set of
attributes. See [docs/IMPORTING.md](docs/IMPORTING.md).

---

## Scripts and headless runs

```bash
hwd sim --year 1946 --years 80 --seed demo   # simulate 80 seasons of history (~40 seconds)
hwd run script.txt                           # run commands from a file
hwd rules 1962                               # the rulebook for any season
hwd calibrate 1950 1985 2024                 # check the engine against historical scoring
```

## Project layout

```text
src/              the engine (a Rust library)  - see docs/ARCHITECTURE.md
src/bin/hwd.rs    the text front end
tests/            calibration, determinism, import, career and history tests
docs/             guides; SETTINGS/TUNING/CONTENT/RULES are generated from the game itself
examples/         sample roster, year pack, settings file and mod
```

Names and teams are **fictional** (real cities, invented nicknames and players). Real data enters only
through files *you* import.

## Development

```bash
cargo test --release       # all tests (the long-career test takes ~20 s)
cargo fmt && cargo clippy
cargo run --release -- gen-docs docs   # regenerate the generated reference docs
```

Everything is deterministic: the same seed and the same choices give the same league.
