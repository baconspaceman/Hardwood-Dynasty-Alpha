# Modding guide

A **mod** is a JSON file. Put it in a folder named `mods/` next to where you run `hwd`, then start a new
league (`new` loads every `.json` file in `mods/` in alphabetical order). Mods are saved with the league.

```bash
hwd mod export-defaults mod-templates   # every built-in list as editable JSON
hwd mod check mods/my_mod.json          # validates and explains problems in plain English
```

## Anatomy

```json
{
  "mod": { "name": "My Mod", "author": "me", "version": "1.0", "description": "what it does" },
  "settings_defaults": { "injuries.frequency": 1.5 },
  "tuning": { "game.fatigue_rate": 1.2 },
  "injuries":    { "add": [], "replace": [], "remove": ["iron_man"], "set": null },
  "life_events": { "add": [ ... ] }
}
```

Every list section accepts the same four verbs:

| Verb | Meaning |
|---|---|
| `add` | add entries (an entry with an existing id replaces it) |
| `replace` | replace entries by id (unknown ids are added) |
| `remove` | list of ids to delete |
| `set` | replace the *entire* list |

## What you can change

| Section | Contents |
|---|---|
| `settings`, `presets`, `settings_defaults` | the settings registry and default values |
| `rule_changes` | the timeline of rule patches (3-point line, shot clock, playoffs, draft, cap regime...) |
| `style_anchors` | how each era plays (pace, 3-point rate, shooting percentages) |
| `economy` | the cap/tax table, pay scale, max-contract percentages |
| `franchises`, `countries`, `name_pools` | the league's whole history, countries and names |
| `archetypes`, `badges` | player styles and what badges do |
| `injuries` | the injury catalog |
| `life_stats`, `life_activities`, `life_events` | the player life sim |
| `league_events` | lockouts, TV deals, shocks |
| `awards` | MVP-style awards defined by a weighted formula |
| `tuning` | named engine parameters ([TUNING.md](TUNING.md)) |

Reference lists of valid ids and attribute keys: [CONTENT.md](CONTENT.md),
[RULES_AND_HISTORY.md](RULES_AND_HISTORY.md), [SETTINGS.md](SETTINGS.md).

## Writing a life event (no Rust needed)

```json
{
  "id": "pickup_with_legends",
  "title": "Run with the legends",
  "text": "A group of retired legends invites {name} to their private summer runs.",
  "chance": 0.04,
  "stages": ["college", "pro"],
  "conditions": [ { "var": "life.reputation", "op": ">=", "value": 45 } ],
  "effects": [ { "target": "attr.shot_iq", "op": "add", "value": 2 } ],
  "choices": []
}
```

* `chance` is the probability **per month** (scaled by the "Life-sim depth" setting).
* `stages`: `high_school`, `college`, `pro`, `overseas`, `retired`, or `any`.
* Conditions can read: `age`, `year`, `ovr`, `potential`, `height`, `money`, `net_worth`, `mood`, `wear`,
  `years_pro`, `team.win_pct`, `life.<stat>`, `attr.<key>`, `custom.<name>`, `flag:<name>`.
  Operators: `>= <= > < == !=`.
* Effect targets: `life.<stat>`, `attr.<key>`, `money` (scaled by the era's cost of living), `endorsement`,
  `asset`, `mood`, `wear`, `fitness`, `potential`, `height`, `rel.<kind>`, `custom.<name>`, `flag`, `news`.
  Operations: `add`, `set`, `mul`, `flag_add`, `flag_remove`.
* Give the event `choices` (each with its own `effects`) and the game stops to ask the human. Each choice has
  an `explain` line shown in the menu.

## New life stats and activities

Add a stat (`life_stats`) such as `faith`, then an activity (`life_activities`) that raises it, and
events that read it. The "time budget" menu and `life` screen pick them up automatically.

## Designing a new era

Add `rule_changes` with a `year` and a `patch` of only the fields that change, for example a league
where threes are worth four points:

```json
{ "rule_changes": { "add": [ { "year": 1990, "id": "four-point-line", "title": "Four-point shots",
  "description": "Long threes are worth four.", "patch": { "three_value": 4 } } ] } }
```

Patch fields are the ones printed by `hwd rules <year>` (for example `shot_clock`, `three_point`,
`three_distance`, `playoff_teams`, `series_lengths`, `draft_rounds`, `lottery`, `hs_allowed`, `cap_type`,
`roster_max`, `games`).

## Tips

* Change one thing at a time and run `mod check`.
* Ids are the only thing that matters for replacing/removing; copy an entry from the exported
  templates and edit it.
* Calibration re-tunes the engine to an era's style, so rule mods that change scoring will mostly be
  absorbed; use `style_anchors` to change what "normal" looks like.
