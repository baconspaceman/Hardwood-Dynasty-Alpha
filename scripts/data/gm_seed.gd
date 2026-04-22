extends RefCounted
class_name GMSeed

const SUPPORTED_YEARS := [1984, 1996, 2024]

const ERA_PRESETS := {
  1984: {
    "label": "1984 / Rival dynasties and a generational rookie wing",
    "tone": "Half-court structure matters, interior play is premium, and the incoming class can bend the league.",
    "anchor": "The Lakefront-equivalent slot should not dominate yet, while a historic scoring-wing archetype enters the league.",
    "scouting": "The following draft should feel top-heavy with a franchise big man near the top.",
  },
  1996: {
    "label": "1996 / Championship peak and elite two-way wing dominance",
    "tone": "Veteran contenders run the top tier while power wings, isolation scorers, and anchor bigs still define title odds.",
    "anchor": "The Lakefront-equivalent slot should carry a legendary two-way scoring guard at or near peak form.",
    "scouting": "The following class should feel shallower than the current one but still useful for patient builders.",
  },
  2024: {
    "label": "2024 / Spacing, jumbo creators, and apron pressure",
    "tone": "Modern cap pressure, heliocentric creators, and floor spacing shape nearly every front-office decision.",
    "anchor": "Several slots should map to oversized creators or unicorn bigs, but all exposed names remain fictional.",
    "scouting": "The following class should tilt toward upside swings and developmental variance.",
  },
}

const COMMANDS := [
  {"label": "Roster Management", "description": "Review depth charts, salaries, and player archetypes."},
  {"label": "Trade Center", "description": "Move players and draft picks using direction-aware valuation."},
  {"label": "Free Agency", "description": "Sign players with soft-cap, tax, and Bird Rights constraints."},
  {"label": "Scouting", "description": "Inspect fictional prospects anchored to the next historical class."},
  {"label": "Draft Room", "description": "Control picks, boards, and long-term asset timing."},
  {"label": "Game Day", "description": "Simulate games, weeks, or seasons with standings and box scores."},
]

const FRANCHISE_SLOTS := [
  {"market": "Harbor City", "nickname": "Admirals", "conference": "Union", "division": "Atlantic", "slot": "slot-bos"},
  {"market": "Empire", "nickname": "Echo", "conference": "Union", "division": "Atlantic", "slot": "slot-nyk"},
  {"market": "Commonwealth", "nickname": "Crowns", "conference": "Union", "division": "Atlantic", "slot": "slot-phi"},
  {"market": "Granite Bay", "nickname": "Sentinels", "conference": "Union", "division": "Atlantic", "slot": "slot-bkn"},
  {"market": "Ironport", "nickname": "Captains", "conference": "Union", "division": "Atlantic", "slot": "slot-tor"},
  {"market": "Lakefront", "nickname": "Union", "conference": "Union", "division": "Central", "slot": "slot-chi"},
  {"market": "Motor City", "nickname": "Foundry", "conference": "Union", "division": "Central", "slot": "slot-det"},
  {"market": "Rivergate", "nickname": "Zephyrs", "conference": "Union", "division": "Central", "slot": "slot-ind"},
  {"market": "Steel Valley", "nickname": "Stags", "conference": "Union", "division": "Central", "slot": "slot-cle"},
  {"market": "Monument", "nickname": "Peaks", "conference": "Union", "division": "Central", "slot": "slot-mil"},
  {"market": "Gulf Coast", "nickname": "Tide", "conference": "Union", "division": "Southeast", "slot": "slot-mia"},
  {"market": "Sunbelt", "nickname": "Flight", "conference": "Union", "division": "Southeast", "slot": "slot-orl"},
  {"market": "Peach State", "nickname": "Pulse", "conference": "Union", "division": "Southeast", "slot": "slot-atl"},
  {"market": "Crown City", "nickname": "Paladins", "conference": "Union", "division": "Southeast", "slot": "slot-cha"},
  {"market": "Crescent Bay", "nickname": "Lanterns", "conference": "Union", "division": "Southeast", "slot": "slot-was"},
  {"market": "Cascade", "nickname": "Current", "conference": "Frontier", "division": "Northwest", "slot": "slot-por"},
  {"market": "Timberline", "nickname": "Foxes", "conference": "Frontier", "division": "Northwest", "slot": "slot-min"},
  {"market": "Summit", "nickname": "Peaks", "conference": "Frontier", "division": "Northwest", "slot": "slot-den"},
  {"market": "Aurora", "nickname": "Outpost", "conference": "Frontier", "division": "Northwest", "slot": "slot-okc"},
  {"market": "Saltline", "nickname": "Resonance", "conference": "Frontier", "division": "Northwest", "slot": "slot-uta"},
  {"market": "Desert", "nickname": "Blaze", "conference": "Frontier", "division": "Pacific", "slot": "slot-phx"},
  {"market": "Golden Coast", "nickname": "Signals", "conference": "Frontier", "division": "Pacific", "slot": "slot-gsw"},
  {"market": "Valley", "nickname": "Orbit", "conference": "Frontier", "division": "Pacific", "slot": "slot-sac"},
  {"market": "Shoreline", "nickname": "Breakers", "conference": "Frontier", "division": "Pacific", "slot": "slot-lac"},
  {"market": "Pacific", "nickname": "Helix", "conference": "Frontier", "division": "Pacific", "slot": "slot-lal"},
  {"market": "Lone Star", "nickname": "Relay", "conference": "Frontier", "division": "Southwest", "slot": "slot-dal"},
  {"market": "Canyon", "nickname": "Mirage", "conference": "Frontier", "division": "Southwest", "slot": "slot-sas"},
  {"market": "Mesa", "nickname": "Circuit", "conference": "Frontier", "division": "Southwest", "slot": "slot-hou"},
  {"market": "Bayou", "nickname": "Gales", "conference": "Frontier", "division": "Southwest", "slot": "slot-nop"},
  {"market": "Plainsfire", "nickname": "Syndicate", "conference": "Frontier", "division": "Southwest", "slot": "slot-mem"},
]

static func is_reasonable_year(year: int) -> bool:
  return year >= 1950 and year <= 2025

static func get_era_preset(year: int) -> Dictionary:
  return ERA_PRESETS.get(year, {})
