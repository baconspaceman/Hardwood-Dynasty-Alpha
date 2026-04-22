# Architecture

## Engine Choice

This repository now uses Godot 4.6.2 for the prototype shell. The game itself is still text-heavy and systems-heavy, but using a real engine gives the project a concrete executable build path instead of a pure UI mockup.

## Core Boundaries

- [project.godot](C:/game-making/Hardwood-Dynasty-Alpha/project.godot)
  Defines the engine project, run target, and window configuration.
- [scenes/Main.tscn](C:/game-making/Hardwood-Dynasty-Alpha/scenes/Main.tscn)
  Owns the current alpha surface and setup flow.
- [scripts/data/gm_seed.gd](C:/game-making/Hardwood-Dynasty-Alpha/scripts/data/gm_seed.gd)
  Holds supported years, fictional franchise slots, command definitions, and era presets.
- [scripts/core/setup_shell_store.gd](C:/game-making/Hardwood-Dynasty-Alpha/scripts/core/setup_shell_store.gd)
  Persists the setup shell to `user://`.
- [scripts/main.gd](C:/game-making/Hardwood-Dynasty-Alpha/scripts/main.gd)
  Builds the current UI and binds setup, preview, and save behavior together.

## State Rules

- Saveable state stays simple and versioned.
- The current build only persists setup-shell data, not full league state.
- Historical anchoring data remains internal while user-facing team identities stay fictional.

## Build Rules

- Local Windows export is handled through [scripts/build_windows.ps1](C:/game-making/Hardwood-Dynasty-Alpha/scripts/build_windows.ps1).
- Export configuration is committed in [export_presets.cfg](C:/game-making/Hardwood-Dynasty-Alpha/export_presets.cfg).
- The generated executable should not be committed to the repository.
