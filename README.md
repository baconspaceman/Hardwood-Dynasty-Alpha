# Hardwood Dynasty Alpha

A Godot-based alpha repository for a text-first basketball general manager simulation.

This pass converts the repo into a real game-engine project using Godot 4.6.2 and adds a working Windows export path. The prototype focuses on the front-office setup flow: pick a starting year, lock the save shell, review fictional franchise mappings, and preview the rules scaffolding for the deeper sim that follows.

## What Is In The Repo

- A Godot 4 project rooted at [project.godot](C:/game-making/Hardwood-Dynasty-Alpha/project.godot)
- A `Control`-based prototype scene for the basketball GM setup flow in [scenes/Main.tscn](C:/game-making/Hardwood-Dynasty-Alpha/scenes/Main.tscn)
- GDScript modules for era data, franchise slots, setup persistence, and rules previews in [scripts](C:/game-making/Hardwood-Dynasty-Alpha/scripts)
- A reproducible Windows export script in [scripts/build_windows.ps1](C:/game-making/Hardwood-Dynasty-Alpha/scripts/build_windows.ps1)
- GitHub templates and a Windows export workflow in [.github](C:/game-making/Hardwood-Dynasty-Alpha/.github)

## Quick Start

1. Open the project in Godot 4.6.2.
2. Run the main scene.
3. Choose a starting year and lock the setup shell.

For a local Windows export:

```powershell
.\scripts\build_windows.ps1 -GodotExecutable .\.tools\godot\editor\Godot_v4.6.2-stable_win64_console.exe
```

The build script expects Godot export templates for `4.6.2.stable` to be installed. The local test export generated during this pass lands at `build\HardwoodDynastyAlpha.exe`.

## Prototype Scope

This test build is intentionally narrow:

- Start-year selection for the historical era anchor
- Persistent setup shell saved to `user://setup_shell.cfg`
- A 30-franchise fictional league table mapped to hidden historical slot codes
- Preview cards for salary-cap rules, Bird Rights, trade logic, and progression direction

It is not the full sim yet. The deeper systems still need to be built:

- full roster generation by historical year
- fictional player identity generation
- game simulation and standings
- contracts, free agency, Bird Rights, and luxury-tax enforcement
- draft classes, picks, and AI trade negotiation

## Repository Shape

```text
.
|-- .github/                  Issue templates, PR template, CI
|-- docs/                     Vision, architecture, roadmap, data notes
|-- scenes/                   Godot scenes
|-- scripts/                  GDScript gameplay and build helpers
|-- export_presets.cfg        Windows export configuration
|-- icon.svg                  Project icon
`-- project.godot             Godot project root
```

## Next Build Priorities

- author the first full year pack, likely `1996`
- generate rosters and team direction from that year pack
- replace preview-only rules cards with real persistent state
- add a playable front-office command loop for rosters, trades, and simulation
