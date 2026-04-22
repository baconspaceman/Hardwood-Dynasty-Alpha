extends RefCounted
class_name GMSeed

const SUPPORTED_YEARS := [1984, 1996, 2024]
const DEFAULT_FOCUS := "Balanced Reps"

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

const FOCUS_OPTIONS := [
  {
    "label": "Balanced Reps",
    "description": "Keep the rotation stable and avoid over-correcting after one rough week.",
  },
  {
    "label": "Run and Gun",
    "description": "Push pace, let the guards create early, and chase a louder offensive ceiling.",
  },
  {
    "label": "Lockdown Shell",
    "description": "Shrink the floor, protect the paint, and win ugly if needed.",
  },
  {
    "label": "Youth Movement",
    "description": "Prioritize developmental reps and bet on growth over short-term polish.",
  },
  {
    "label": "Crash the Glass",
    "description": "Lean into size, rebounding, and second-chance pressure.",
  },
]

const COMMANDS := [
  {"label": "Start Franchise", "description": "Lock a year and choose the club you want to run."},
  {"label": "Advance Week", "description": "Play the front office loop one week at a time with standings and results."},
  {"label": "Scout Prospect", "description": "Add a fresh report to the board for the next class."},
  {"label": "Make Trade Call", "description": "Probe the market and rebalance the roster."},
  {"label": "Browse League", "description": "Inspect every club through the standings and team browser."},
  {"label": "Persistent Save", "description": "Resume your franchise shell after reload without losing the current season."},
]

const FRANCHISE_SLOTS := [
  {"id": "harbor-city-admirals", "market": "Harbor City", "nickname": "Admirals", "conference": "Union", "division": "Atlantic", "slot": "slot-bos"},
  {"id": "empire-echo", "market": "Empire", "nickname": "Echo", "conference": "Union", "division": "Atlantic", "slot": "slot-nyk"},
  {"id": "commonwealth-crowns", "market": "Commonwealth", "nickname": "Crowns", "conference": "Union", "division": "Atlantic", "slot": "slot-phi"},
  {"id": "granite-bay-sentinels", "market": "Granite Bay", "nickname": "Sentinels", "conference": "Union", "division": "Atlantic", "slot": "slot-bkn"},
  {"id": "ironport-captains", "market": "Ironport", "nickname": "Captains", "conference": "Union", "division": "Atlantic", "slot": "slot-tor"},
  {"id": "lakefront-union", "market": "Lakefront", "nickname": "Union", "conference": "Union", "division": "Central", "slot": "slot-chi"},
  {"id": "motor-city-foundry", "market": "Motor City", "nickname": "Foundry", "conference": "Union", "division": "Central", "slot": "slot-det"},
  {"id": "rivergate-zephyrs", "market": "Rivergate", "nickname": "Zephyrs", "conference": "Union", "division": "Central", "slot": "slot-ind"},
  {"id": "steel-valley-stags", "market": "Steel Valley", "nickname": "Stags", "conference": "Union", "division": "Central", "slot": "slot-cle"},
  {"id": "monument-peaks", "market": "Monument", "nickname": "Peaks", "conference": "Union", "division": "Central", "slot": "slot-mil"},
  {"id": "gulf-coast-tide", "market": "Gulf Coast", "nickname": "Tide", "conference": "Union", "division": "Southeast", "slot": "slot-mia"},
  {"id": "sunbelt-flight", "market": "Sunbelt", "nickname": "Flight", "conference": "Union", "division": "Southeast", "slot": "slot-orl"},
  {"id": "peach-state-pulse", "market": "Peach State", "nickname": "Pulse", "conference": "Union", "division": "Southeast", "slot": "slot-atl"},
  {"id": "crown-city-paladins", "market": "Crown City", "nickname": "Paladins", "conference": "Union", "division": "Southeast", "slot": "slot-cha"},
  {"id": "crescent-bay-lanterns", "market": "Crescent Bay", "nickname": "Lanterns", "conference": "Union", "division": "Southeast", "slot": "slot-was"},
  {"id": "cascade-current", "market": "Cascade", "nickname": "Current", "conference": "Frontier", "division": "Northwest", "slot": "slot-por"},
  {"id": "timberline-foxes", "market": "Timberline", "nickname": "Foxes", "conference": "Frontier", "division": "Northwest", "slot": "slot-min"},
  {"id": "summit-peaks", "market": "Summit", "nickname": "Peaks", "conference": "Frontier", "division": "Northwest", "slot": "slot-den"},
  {"id": "aurora-outpost", "market": "Aurora", "nickname": "Outpost", "conference": "Frontier", "division": "Northwest", "slot": "slot-okc"},
  {"id": "saltline-resonance", "market": "Saltline", "nickname": "Resonance", "conference": "Frontier", "division": "Northwest", "slot": "slot-uta"},
  {"id": "desert-blaze", "market": "Desert", "nickname": "Blaze", "conference": "Frontier", "division": "Pacific", "slot": "slot-phx"},
  {"id": "golden-coast-signals", "market": "Golden Coast", "nickname": "Signals", "conference": "Frontier", "division": "Pacific", "slot": "slot-gsw"},
  {"id": "valley-orbit", "market": "Valley", "nickname": "Orbit", "conference": "Frontier", "division": "Pacific", "slot": "slot-sac"},
  {"id": "shoreline-breakers", "market": "Shoreline", "nickname": "Breakers", "conference": "Frontier", "division": "Pacific", "slot": "slot-lac"},
  {"id": "pacific-helix", "market": "Pacific", "nickname": "Helix", "conference": "Frontier", "division": "Pacific", "slot": "slot-lal"},
  {"id": "lone-star-relay", "market": "Lone Star", "nickname": "Relay", "conference": "Frontier", "division": "Southwest", "slot": "slot-dal"},
  {"id": "canyon-mirage", "market": "Canyon", "nickname": "Mirage", "conference": "Frontier", "division": "Southwest", "slot": "slot-sas"},
  {"id": "mesa-circuit", "market": "Mesa", "nickname": "Circuit", "conference": "Frontier", "division": "Southwest", "slot": "slot-hou"},
  {"id": "bayou-gales", "market": "Bayou", "nickname": "Gales", "conference": "Frontier", "division": "Southwest", "slot": "slot-nop"},
  {"id": "plainsfire-syndicate", "market": "Plainsfire", "nickname": "Syndicate", "conference": "Frontier", "division": "Southwest", "slot": "slot-mem"},
]

static func is_reasonable_year(year: int) -> bool:
  return year >= 1950 and year <= 2025


static func get_era_preset(year: int) -> Dictionary:
  return ERA_PRESETS.get(year, {})


static func get_focus_data(label: String) -> Dictionary:
  for option in FOCUS_OPTIONS:
    if option["label"] == label:
      return option
  return FOCUS_OPTIONS[0].duplicate(true)


static func get_focus_labels() -> Array:
  var labels: Array = []
  for option in FOCUS_OPTIONS:
    labels.append(option["label"])
  return labels


static func get_franchise_slot(franchise_id: String) -> Dictionary:
  for entry in FRANCHISE_SLOTS:
    if entry["id"] == franchise_id:
      return entry
  return {}
