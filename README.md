# Hardwood Dynasty Alpha

Hardwood Dynasty Alpha is a Godot-based basketball front-office prototype focused on the feel of running a fictional league that is historically anchored to real NBA eras.

The current build is no longer just a static setup screen. It now plays as a lightweight weekly franchise loop: choose an era, select a club, generate the league, set a coaching focus, scout prospects, make trade calls, and advance through a full alpha season with standings, inbox updates, roster cards, and persistent saves.

## Current Alpha Features

- A Godot 4 project rooted at [project.godot](C:/game-making/Hardwood-Dynasty-Alpha/project.godot)
- A warm, basketball-first `Control` UI in [scenes/Main.tscn](C:/game-making/Hardwood-Dynasty-Alpha/scenes/Main.tscn) and [scripts/main.gd](C:/game-making/Hardwood-Dynasty-Alpha/scripts/main.gd)
- A generated 30-team fictional league with era-based strength anchors in [scripts/core/league_sim.gd](C:/game-making/Hardwood-Dynasty-Alpha/scripts/core/league_sim.gd)
- Persistent franchise state saved to `user://hardwood_dynasty_alpha_state.json` through [scripts/core/setup_shell_store.gd](C:/game-making/Hardwood-Dynasty-Alpha/scripts/core/setup_shell_store.gd)
- Era, franchise, and coaching-focus seed data in [scripts/data/gm_seed.gd](C:/game-making/Hardwood-Dynasty-Alpha/scripts/data/gm_seed.gd)
- A reproducible Windows export script in [scripts/build_windows.ps1](C:/game-making/Hardwood-Dynasty-Alpha/scripts/build_windows.ps1)
- GitHub templates and CI in [.github](C:/game-making/Hardwood-Dynasty-Alpha/.github)

## Play Loop

1. Open the project in Godot 4.6.2 and run the main scene.
2. Choose a starting year or use one of the supported era buttons.
3. Pick a franchise from the browser.
4. Lock the year, then start the front office.
5. Adjust your coaching focus, scout prospects, make trade calls, and advance week by week.
6. Review the live standings, roster, team chemistry, fan buzz, inbox feed, and recent results as the season develops.

For a local Windows export:

```powershell
.\scripts\build_windows.ps1 -GodotExecutable .\.tools\godot\editor\Godot_v4.6.2-stable_win64_console.exe
```

The build script expects Godot export templates for `4.6.2.stable` to be installed. Local exports land at `build\HardwoodDynastyAlpha.exe`.

## Alpha Scope

This build is a playable front-office prototype, but it is still an alpha:

- Historical anchoring currently works through team and star strength profiles, not a full authored player database.
- Trade logic, scouting, progression, and game simulation are lightweight systems designed to make the prototype feel alive.
- Contracts, Bird Rights, free agency, full draft logic, playoffs, and deeper AI strategy still need a full implementation pass.

## Repository Shape

```text
.
|-- .github/                  Issue templates, PR template, CI
|-- docs/                     Vision, architecture, roadmap, data notes
|-- scenes/                   Godot scenes
|-- scripts/core/             Simulation and persistence
|-- scripts/data/             Era and franchise seed data
|-- scripts/build_windows.ps1 Windows export helper
|-- export_presets.cfg        Windows export configuration
|-- icon.svg                  Project icon
`-- project.godot             Godot project root
```

## Next Priorities

- Author the first fully bespoke year pack, likely `1996`, with real historical contract and attribute mapping behind fictional identities.
- Add full roster management flows for depth charts, minutes, and salary review.
- Expand transaction logic into real offer evaluation with picks, team direction, and soft-cap restrictions.
- Add season transitions: offseason, draft, free agency, and long-term save continuity.
