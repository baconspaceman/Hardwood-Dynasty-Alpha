# Importing rosters and years

Anyone can make a roster or a whole season and share it.

## Roster files (CSV or JSON)

```text
import roster examples/roster.csv           # add players to the teams named in the file
import roster examples/roster.csv replace   # replace those teams' rosters
```

CSV: first row is the header. Only `name` is required. Column names are matched loosely.

| Column(s) | Meaning |
|---|---|
| `team` / `club` | team abbreviation, city or nickname in the current league; `FA` or blank = free agent |
| `name` | "First Last" |
| `age`, `position` (PG/SG/SF/PF/C), `height_in` (inches, or `6'8"`, or cm), `weight_lb` | |
| `overall`, `potential` | 25-99; the engine builds a full set of attributes |
| `salary` (dollars), `years` | contract; otherwise a market contract is generated |
| `country` | code such as `USA`, `SRB`, `FRA` (see CONTENT/countries) |
| `archetype` | style id such as `sharpshooter`; otherwise guessed from stats/position |
| `ppg rpg apg spg bpg three_pa three_pct ft_pct fta tov mpg` | per-game stats; skills are inferred from these |
| any attribute key (`three_point`, `ball_handle`, `block`...) | sets that rating directly |

Lines starting with `#` are comments. Problems are reported in plain English (unknown teams, unknown
positions, unknown attributes) and the rest of the file still imports.

JSON: `{ "teams": [ { "name": "BOS", "players": [ { "name": "...", "overall": 80, "attrs": { "three_point": 90 } } ] } ], "free_agents": [] }`

## Year packs (JSON)

A **year pack** describes a whole season: the teams, rosters, a draft class, free agents, an optional
salary cap and settings overrides. It starts a *new* league:

```text
newpack examples/year_pack.json
export pack mypack.json          # export the current league as a pack to share
```

Fields: `name`, `author`, `description`, `year`, `salary_cap`, `settings`, `teams` (each with `city`,
`nickname`, `abbr`, `conference`, `market`, `players`), `free_agents`, `draft_class`, and an optional
embedded `mod`. See `examples/year_pack.json`.

## Using real statistics

The game ships **no** real names or data. If you have per-game stats for a real season (from a source
you may use), put them in the CSV columns above; the inference turns stat lines into skills: threes from
`three_pa`/`three_pct`, passing from `apg`, rebounding from `rpg`, rim protection from `bpg`, steals from
`spg`, free throws from `ft_pct`. Give an `overall` too, or one is estimated from production.

## Exporting

`export roster file.csv` writes every attribute for every player; `export pack file.json` writes a pack.
