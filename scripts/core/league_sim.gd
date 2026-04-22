extends RefCounted
class_name LeagueSim

const GMSeed = preload("res://scripts/data/gm_seed.gd")

const SAVE_VERSION := 2
const TOTAL_WEEKS := 24

const FOCUS_EFFECTS := {
  "Balanced Reps": {"offense": 0, "defense": 0, "chemistry": 1, "development": 0},
  "Run and Gun": {"offense": 4, "defense": -1, "chemistry": 0, "development": 0},
  "Lockdown Shell": {"offense": -1, "defense": 4, "chemistry": 1, "development": 0},
  "Youth Movement": {"offense": -1, "defense": 0, "chemistry": 0, "development": 2},
  "Crash the Glass": {"offense": 1, "defense": 2, "chemistry": 0, "development": 0},
}

const TEAM_ANCHORS := {
  1984: {"slot-lal": 12, "slot-bos": 11, "slot-phi": 7, "slot-hou": 6, "slot-chi": -2},
  1996: {"slot-chi": 14, "slot-lal": 8, "slot-hou": 8, "slot-uta": 8, "slot-okc": 6, "slot-orl": 5},
  2024: {"slot-bos": 12, "slot-den": 10, "slot-okc": 9, "slot-min": 8, "slot-dal": 8, "slot-nyk": 6},
}

const STAR_ANCHORS := {
  1984: {
    "slot-lal": {"position": "PG", "archetype": "Showtime engine"},
    "slot-bos": {"position": "SF", "archetype": "Playmaking forward"},
    "slot-phi": {"position": "C", "archetype": "Power finisher"},
  },
  1996: {
    "slot-chi": {"position": "SG", "archetype": "Legendary two-way scorer"},
    "slot-lal": {"position": "C", "archetype": "Interior force"},
    "slot-uta": {"position": "PF", "archetype": "Pick-and-roll hammer"},
    "slot-hou": {"position": "C", "archetype": "Footwork savant"},
  },
  2024: {
    "slot-bos": {"position": "SF", "archetype": "Two-way alpha wing"},
    "slot-den": {"position": "C", "archetype": "Point-center creator"},
    "slot-okc": {"position": "PG", "archetype": "Slithery lead scorer"},
    "slot-min": {"position": "SG", "archetype": "Explosive downhill guard"},
  },
}

const FIRST_NAMES := [
  "Jalen", "Malik", "Andre", "Darius", "Tomas", "Noah", "Quentin", "Micah",
  "Julian", "Kendrick", "Devin", "Isaiah", "Roman", "Cade", "Elliot", "Marcus",
  "Terrance", "Khalil", "Nico", "Jordan", "Elijah", "Tyrese", "Jabari", "Owen",
]

const LAST_NAMES := [
  "Hines", "Booker", "Vale", "Mercer", "Ross", "Pryce", "Holloway", "Finch",
  "Monroe", "Vaughn", "Collier", "Sloan", "Gentry", "Wynn", "Santos", "Bishop",
  "Lennox", "Crawford", "Maddox", "Shepard", "Delgado", "Rivers", "Keller", "Parker",
]

const POSITION_ARCHETYPES := {
  "PG": ["Tempo engine", "Pick-and-roll surgeon", "Pace caller"],
  "SG": ["Volume bucket", "Movement scorer", "Two-way scorer"],
  "SF": ["Two-way wing", "Slasher", "Connector forward"],
  "PF": ["Stretch forward", "Glass cleaner", "Screen-and-roll bruiser"],
  "C": ["Rim anchor", "Low-post hub", "Vertical finisher"],
}

static func create_blank_state() -> Dictionary:
  return {
    "version": SAVE_VERSION,
    "selected_year": 0,
    "status": "awaiting-year",
    "generated_league": false,
    "user_team_id": "",
    "preview_team_id": GMSeed.FRANCHISE_SLOTS[0]["id"],
    "current_week": 1,
    "total_weeks": TOTAL_WEEKS,
    "focus": GMSeed.DEFAULT_FOCUS,
    "headline": "Choose a year and a club to start building your front office.",
    "note": "The save is waiting for a starting year and franchise selection.",
    "action_result": "No front-office moves logged yet.",
    "teams": [],
    "inbox": [],
    "prospects": [],
    "recent_results": [],
  }


static func create_franchise_state(year: int, user_team_id: String) -> Dictionary:
  var state = create_blank_state()
  var teams: Array = []

  for slot in GMSeed.FRANCHISE_SLOTS:
    teams.append(_build_team(slot, year))

  state["selected_year"] = year
  state["status"] = "in-season"
  state["generated_league"] = true
  state["user_team_id"] = user_team_id
  state["preview_team_id"] = user_team_id
  state["teams"] = teams
  state["headline"] = "Training camp opens for %s." % _get_team_by_id(teams, user_team_id)["market"]
  state["note"] = "Year %d locked. The front-office loop is active." % year
  state["action_result"] = "Franchise save created. Set a focus, scout, make calls, and advance the season."
  state["inbox"] = [
    {
      "tone": "warm",
      "title": "Owner Mandate",
      "body": "Win without burning future flexibility. The board expects visible progress this year.",
    },
    {
      "tone": "cool",
      "title": "League Office",
      "body": "Schedule loaded. The front office can now advance the season one week at a time.",
    },
  ]
  return state


static func set_focus(state: Dictionary, focus_label: String) -> Dictionary:
  var next_state: Dictionary = state.duplicate(true)
  next_state["focus"] = focus_label
  next_state["action_result"] = "Coaching focus changed to %s." % focus_label
  return next_state


static func scout_prospect(state: Dictionary) -> Dictionary:
  var next_state: Dictionary = state.duplicate(true)
  if next_state["status"] != "in-season":
    return next_state

  var user_team = _get_user_team(next_state)
  var scout_seed = "%d:%s:%d:%d" % [
    next_state["selected_year"],
    next_state["user_team_id"],
    next_state["current_week"],
    next_state["prospects"].size(),
  ]
  var rng = _make_rng(scout_seed)
  var position = ["PG", "SG", "SF", "PF", "C"][rng.randi_range(0, 4)]
  var rating = rng.randi_range(68, 82)
  var upside = rating + rng.randi_range(4, 12)
  var prospect = {
    "name": _build_player_name(scout_seed, position, rng.randi_range(0, 30)),
    "position": position,
    "current": rating,
    "upside": clampi(upside, rating, 95),
    "style": POSITION_ARCHETYPES[position][rng.randi_range(0, POSITION_ARCHETYPES[position].size() - 1)],
  }

  next_state["prospects"].insert(0, prospect)
  if next_state["prospects"].size() > 6:
    next_state["prospects"].resize(6)

  next_state["headline"] = "%s logged a new prospect report." % user_team["market"]
  next_state["action_result"] = "Scout filed a report on %s, a %s with %d current grade and %d upside." % [
    prospect["name"],
    prospect["position"],
    prospect["current"],
    prospect["upside"],
  ]
  _push_inbox(
    next_state,
    "Scout Report",
    "%s projects as a %s. Current grade %d, upside %d." % [
      prospect["name"],
      prospect["style"],
      prospect["current"],
      prospect["upside"],
    ],
    "cool",
  )
  return next_state


static func make_trade_call(state: Dictionary) -> Dictionary:
  var next_state: Dictionary = state.duplicate(true)
  if next_state["status"] != "in-season":
    return next_state

  var user_index = _find_team_index(next_state["teams"], next_state["user_team_id"])
  var partner_index = _find_trade_partner(next_state["teams"], user_index)
  if user_index == -1 or partner_index == -1:
    return next_state

  var user_team: Dictionary = next_state["teams"][user_index]
  var partner_team: Dictionary = next_state["teams"][partner_index]
  var outgoing_index = _pick_trade_piece(user_team, false)
  var incoming_index = _pick_trade_piece(partner_team, true)

  if outgoing_index == -1 or incoming_index == -1:
    next_state["action_result"] = "Trade call fizzled. Nobody wanted to move the right piece this week."
    return next_state

  var outgoing: Dictionary = user_team["roster"][outgoing_index]
  var incoming: Dictionary = partner_team["roster"][incoming_index]

  user_team["roster"].remove_at(outgoing_index)
  partner_team["roster"].remove_at(incoming_index)
  user_team["roster"].append(incoming)
  partner_team["roster"].append(outgoing)

  user_team["cap_room"] = snapped(float(user_team["cap_room"]) + float(outgoing["salary"]) - float(incoming["salary"]), 0.1)
  partner_team["cap_room"] = snapped(float(partner_team["cap_room"]) + float(incoming["salary"]) - float(outgoing["salary"]), 0.1)
  user_team["chemistry"] = clampi(int(user_team["chemistry"]) - 1 + int(incoming["overall"] > outgoing["overall"]), 55, 99)
  partner_team["chemistry"] = clampi(int(partner_team["chemistry"]) - 1 + int(outgoing["overall"] > incoming["overall"]), 55, 99)

  next_state["teams"][user_index] = _recalculate_team_identity(user_team)
  next_state["teams"][partner_index] = _recalculate_team_identity(partner_team)

  next_state["headline"] = "%s and %s completed a minor deal." % [user_team["market"], partner_team["market"]]
  next_state["action_result"] = "Trade call landed: %s acquired %s from %s for %s." % [
    user_team["market"],
    incoming["name"],
    partner_team["market"],
    outgoing["name"],
  ]
  _push_inbox(
    next_state,
    "Trade Call Completed",
    "%s joins the building while %s heads out. Cap room now sits at $%.1fM." % [
      incoming["name"],
      outgoing["name"],
      float(next_state["teams"][user_index]["cap_room"]),
    ],
    "warm",
  )
  return next_state


static func advance_week(state: Dictionary) -> Dictionary:
  var next_state: Dictionary = state.duplicate(true)
  if next_state["status"] != "in-season":
    return next_state

  var week: int = next_state["current_week"]
  if week > int(next_state["total_weeks"]):
    next_state["status"] = "season-complete"
    next_state["headline"] = "Season complete. Reset the save or start a new era."
    next_state["action_result"] = "No more scheduled weeks remain in this alpha season."
    return next_state

  var matchups = _get_weekly_matchups(next_state["teams"], week)
  var results: Array = []

  for matchup in matchups:
    var result = _simulate_matchup(next_state, matchup["home"], matchup["away"], week)
    results.append(result)
    _replace_team(next_state["teams"], result["home_team"])
    _replace_team(next_state["teams"], result["away_team"])

  var user_result = _find_user_result(results, next_state["user_team_id"])
  if not user_result.is_empty():
    next_state["recent_results"].insert(0, user_result)
    if next_state["recent_results"].size() > 4:
      next_state["recent_results"].resize(4)
    _push_inbox(next_state, user_result["title"], user_result["body"], user_result["tone"])
    next_state["headline"] = user_result["headline"]
    next_state["action_result"] = user_result["body"]

  next_state["current_week"] = week + 1
  if next_state["current_week"] > int(next_state["total_weeks"]):
    next_state["status"] = "season-complete"
    _push_inbox(next_state, "Regular Season Closed", "The alpha season is complete. Review the standings, then reset into a new era.", "cool")

  return next_state


static func sort_standings(teams: Array) -> Array:
  var ordered: Array = teams.duplicate(true)
  ordered.sort_custom(_sort_team_desc)
  return ordered


static func get_team_by_id(state: Dictionary, team_id: String) -> Dictionary:
  return _get_team_by_id(state.get("teams", []), team_id)


static func get_user_team(state: Dictionary) -> Dictionary:
  return _get_team_by_id(state.get("teams", []), state.get("user_team_id", ""))


static func get_preview_team(state: Dictionary) -> Dictionary:
  return _get_team_by_id(state.get("teams", []), state.get("preview_team_id", ""))


static func _sort_team_desc(left: Dictionary, right: Dictionary) -> bool:
  if left["wins"] == right["wins"]:
    var left_diff = int(left["points_for"]) - int(left["points_against"])
    var right_diff = int(right["points_for"]) - int(right["points_against"])
    if left_diff == right_diff:
      return int(left["offense"]) + int(left["defense"]) > int(right["offense"]) + int(right["defense"])
    return left_diff > right_diff
  return int(left["wins"]) > int(right["wins"])


static func _build_team(slot: Dictionary, year: int) -> Dictionary:
  var team_rng = _make_rng("%d:%s" % [year, slot["id"]])
  var anchor_bonus = int(TEAM_ANCHORS.get(year, {}).get(slot["slot"], 0))
  var base_power = clampi(73 + team_rng.randi_range(-5, 6) + anchor_bonus, 64, 97)
  var offense = clampi(base_power + team_rng.randi_range(-3, 4), 62, 99)
  var defense = clampi(base_power + team_rng.randi_range(-4, 3), 62, 99)
  var chemistry = clampi(69 + team_rng.randi_range(-8, 10), 55, 96)
  var fan_support = clampi(60 + anchor_bonus + team_rng.randi_range(-7, 7), 38, 98)
  var payroll = float(team_rng.randi_range(128, 176))
  var cap_room = snapped(190.0 - payroll + float(team_rng.randi_range(-4, 6)), 0.1)

  var team = {
    "id": slot["id"],
    "market": slot["market"],
    "nickname": slot["nickname"],
    "conference": slot["conference"],
    "division": slot["division"],
    "slot": slot["slot"],
    "offense": offense,
    "defense": defense,
    "chemistry": chemistry,
    "fan_support": fan_support,
    "cap_room": cap_room,
    "payroll": payroll,
    "wins": 0,
    "losses": 0,
    "streak": 0,
    "points_for": 0,
    "points_against": 0,
    "direction": _direction_for_power(base_power),
    "roster": _build_roster(slot, year, base_power, team_rng),
    "last_result": "No games logged yet.",
  }
  return _recalculate_team_identity(team)


static func _build_roster(slot: Dictionary, year: int, base_power: int, team_rng: RandomNumberGenerator) -> Array:
  var rotation_positions = ["PG", "SG", "SF", "PF", "C", "G", "F", "C"]
  var roster: Array = []

  for index in range(rotation_positions.size()):
    var slot_position = rotation_positions[index]
    var base_position = slot_position
    if slot_position == "G":
      base_position = "PG" if index % 2 == 0 else "SG"
    elif slot_position == "F":
      base_position = "SF" if index % 2 == 0 else "PF"

    var seeded_index = index + abs(int(hash("%d:%s" % [year, slot["id"]]))) % 13
    var name = _build_player_name(slot["id"], base_position, seeded_index)
    var overall = clampi(base_power + 6 - index * 3 + team_rng.randi_range(-4, 4), 58, 98)
    var age = clampi(team_rng.randi_range(20, 33) + int(index == 0), 19, 36)
    var potential = clampi(maxi(overall, overall + team_rng.randi_range(0, 12) - maxi(age - 25, 0)), overall, 99)
    var shot = clampi(overall + team_rng.randi_range(-5, 4), 55, 99)
    var defense = clampi(overall + team_rng.randi_range(-5, 4), 55, 99)
    var creation = clampi(overall + team_rng.randi_range(-6, 5), 50, 99)
    var mood = clampi(68 + team_rng.randi_range(-9, 10), 50, 98)
    var archetype = POSITION_ARCHETYPES[base_position][team_rng.randi_range(0, POSITION_ARCHETYPES[base_position].size() - 1)]

    var player = {
      "name": name,
      "position": base_position,
      "age": age,
      "overall": overall,
      "potential": potential,
      "shot": shot,
      "defense": defense,
      "creation": creation,
      "salary": snapped(maxf(1.3, float(overall - 56) * 0.72 + team_rng.randf_range(1.0, 4.5)), 0.1),
      "mood": mood,
      "archetype": archetype,
    }

    if index == 0:
      var star_profile: Dictionary = STAR_ANCHORS.get(year, {}).get(slot["slot"], {})
      if not star_profile.is_empty():
        player["position"] = star_profile["position"]
        player["archetype"] = star_profile["archetype"]
        player["overall"] = clampi(base_power + 9 + team_rng.randi_range(0, 3), 78, 99)
        player["potential"] = clampi(player["overall"] + team_rng.randi_range(1, 5), player["overall"], 99)
        player["shot"] = clampi(player["overall"] + team_rng.randi_range(-2, 4), 70, 99)
        player["defense"] = clampi(player["overall"] + team_rng.randi_range(-2, 4), 68, 99)
        player["creation"] = clampi(player["overall"] + team_rng.randi_range(-2, 4), 68, 99)
        player["salary"] = snapped(maxf(player["salary"], float(player["overall"] - 60) * 0.9), 0.1)
        player["mood"] = clampi(player["mood"] + 6, 50, 99)
    roster.append(player)

  roster.sort_custom(_sort_player_desc)
  return roster


static func _sort_player_desc(left: Dictionary, right: Dictionary) -> bool:
  return int(left["overall"]) > int(right["overall"])


static func _build_player_name(team_id: String, position: String, index: int) -> String:
  var key = "%s:%s:%d" % [team_id, position, index]
  var first_name = FIRST_NAMES[abs(int(hash("first:%s" % key))) % FIRST_NAMES.size()]
  var last_name = LAST_NAMES[abs(int(hash("last:%s" % key))) % LAST_NAMES.size()]
  return "%s %s" % [first_name, last_name]


static func _direction_for_power(power: int) -> String:
  if power >= 84:
    return "contending"
  if power >= 76:
    return "retooling"
  return "rebuilding"


static func _recalculate_team_identity(team: Dictionary) -> Dictionary:
  var rotation: Array = team["roster"].duplicate(true)
  rotation.sort_custom(_sort_player_desc)

  var top_count = mini(5, rotation.size())
  if top_count <= 0:
    return team

  var offense_sum = 0
  var defense_sum = 0
  var mood_sum = 0

  for index in range(top_count):
    offense_sum += int(rotation[index]["shot"]) + int(rotation[index]["creation"]) / 2
    defense_sum += int(rotation[index]["defense"])
    mood_sum += int(rotation[index]["mood"])

  team["offense"] = clampi(int(offense_sum / top_count), 60, 99)
  team["defense"] = clampi(int(defense_sum / top_count), 60, 99)
  team["chemistry"] = clampi(int((mood_sum / top_count) + 8), 55, 99)
  team["direction"] = _direction_for_power(int((team["offense"] + team["defense"]) / 2))
  return team


static func _get_weekly_matchups(teams: Array, week: int) -> Array:
  var rotation: Array = []
  for team in teams:
    rotation.append(team["id"])
  rotation.sort()

  for _index in range(maxi(0, week - 1)):
    rotation = _rotate_rotation(rotation)

  var matchups: Array = []
  for pair_index in range(int(rotation.size() / 2)):
    var home_id = rotation[pair_index]
    var away_id = rotation[rotation.size() - 1 - pair_index]
    if week % 2 == 0 and pair_index % 2 == 0:
      var temp = home_id
      home_id = away_id
      away_id = temp
    matchups.append({"home": home_id, "away": away_id})
  return matchups


static func _rotate_rotation(source: Array) -> Array:
  if source.size() <= 2:
    return source.duplicate(true)

  var rotated = source.duplicate(true)
  var fixed = rotated[0]
  rotated.remove_at(0)
  var tail = rotated[rotated.size() - 1]
  rotated.remove_at(rotated.size() - 1)
  rotated.insert(0, tail)
  rotated.insert(0, fixed)
  return rotated


static func _simulate_matchup(state: Dictionary, home_id: String, away_id: String, week: int) -> Dictionary:
  var home_team = _get_team_by_id(state["teams"], home_id).duplicate(true)
  var away_team = _get_team_by_id(state["teams"], away_id).duplicate(true)
  if home_team.is_empty() or away_team.is_empty():
    return {
      "home_team": home_team,
      "away_team": away_team,
      "user_result": {},
    }
  var rng = _make_rng("%d:%s:%s:%d" % [state["selected_year"], home_id, away_id, week])

  var home_offense = int(home_team["offense"])
  var home_defense = int(home_team["defense"])
  var away_offense = int(away_team["offense"])
  var away_defense = int(away_team["defense"])

  if home_id == state["user_team_id"]:
    var home_focus = FOCUS_EFFECTS.get(state["focus"], FOCUS_EFFECTS[GMSeed.DEFAULT_FOCUS])
    home_offense += int(home_focus["offense"])
    home_defense += int(home_focus["defense"])
  elif away_id == state["user_team_id"]:
    var away_focus = FOCUS_EFFECTS.get(state["focus"], FOCUS_EFFECTS[GMSeed.DEFAULT_FOCUS])
    away_offense += int(away_focus["offense"])
    away_defense += int(away_focus["defense"])

  var home_score = clampi(92 + int((home_offense - away_defense) * 0.55) + int(home_team["chemistry"] - 70) / 4 + rng.randi_range(-8, 10), 79, 138)
  var away_score = clampi(91 + int((away_offense - home_defense) * 0.55) + int(away_team["chemistry"] - 70) / 4 + rng.randi_range(-8, 10), 77, 136)
  if home_score == away_score:
    home_score += 1

  var home_win = home_score > away_score
  _apply_result(home_team, home_score, away_score, home_win)
  _apply_result(away_team, away_score, home_score, not home_win)

  if home_id == state["user_team_id"]:
    _apply_focus_progress(home_team, state["focus"], home_win)
  elif away_id == state["user_team_id"]:
    _apply_focus_progress(away_team, state["focus"], not home_win)

  var user_result = {}
  if home_id == state["user_team_id"] or away_id == state["user_team_id"]:
    var user_team = home_team if home_id == state["user_team_id"] else away_team
    var opponent = away_team if home_id == state["user_team_id"] else home_team
    var user_score = home_score if home_id == state["user_team_id"] else away_score
    var opponent_score = away_score if home_id == state["user_team_id"] else home_score
    var outcome = "W" if user_score > opponent_score else "L"
    var star = user_team["roster"][0]["name"]
    user_result = {
      "tone": "warm" if outcome == "W" else "cold",
      "title": "Week %d: %s %s-%s" % [week, outcome, user_score, opponent_score],
      "body": "%s %s %s %d-%d. %s set the tone." % [
        user_team["market"],
        "beat" if outcome == "W" else "fell to",
        opponent["market"],
        user_score,
        opponent_score,
        star,
      ],
      "headline": "%s %s %s in week %d." % [
        user_team["market"],
        "handled" if outcome == "W" else "dropped one to",
        opponent["market"],
        week,
      ],
    }
  return {
    "home_team": _recalculate_team_identity(home_team),
    "away_team": _recalculate_team_identity(away_team),
    "user_result": user_result,
  }


static func _apply_result(team: Dictionary, scored: int, allowed: int, won: bool) -> void:
  var streak = int(team["streak"])
  team["wins"] = int(team["wins"]) + int(won)
  team["losses"] = int(team["losses"]) + int(not won)
  team["points_for"] = int(team["points_for"]) + scored
  team["points_against"] = int(team["points_against"]) + allowed
  team["fan_support"] = clampi(int(team["fan_support"]) + (2 if won else -2), 35, 99)
  team["chemistry"] = clampi(int(team["chemistry"]) + (1 if won else -1), 55, 99)
  if won:
    team["streak"] = streak + 1 if streak >= 0 else 1
  else:
    team["streak"] = streak - 1 if streak <= 0 else -1
  team["last_result"] = "%s %d-%d" % ["W" if won else "L", scored, allowed]

  for player in team["roster"]:
    player["mood"] = clampi(int(player["mood"]) + (2 if won else -2), 48, 99)


static func _apply_focus_progress(team: Dictionary, focus: String, won: bool) -> void:
  var focus_data = FOCUS_EFFECTS.get(focus, FOCUS_EFFECTS[GMSeed.DEFAULT_FOCUS])
  if int(focus_data["development"]) <= 0:
    return

  for player in team["roster"]:
    if int(player["age"]) <= 23 and int(player["overall"]) < int(player["potential"]):
      player["overall"] = clampi(int(player["overall"]) + (1 if won else 0), 55, int(player["potential"]))
      player["mood"] = clampi(int(player["mood"]) + 2, 48, 99)


static func _find_user_result(results: Array, user_team_id: String) -> Dictionary:
  for result in results:
    if not result["user_result"].is_empty():
      return result["user_result"]
  return {}


static func _push_inbox(state: Dictionary, title: String, body: String, tone: String) -> void:
  state["inbox"].insert(0, {"title": title, "body": body, "tone": tone})
  if state["inbox"].size() > 6:
    state["inbox"].resize(6)


static func _get_user_team(state: Dictionary) -> Dictionary:
  return _get_team_by_id(state["teams"], state["user_team_id"])


static func _get_team_by_id(teams: Array, team_id: String) -> Dictionary:
  for team in teams:
    if team["id"] == team_id:
      return team
  return {}


static func _find_team_index(teams: Array, team_id: String) -> int:
  for index in range(teams.size()):
    if teams[index]["id"] == team_id:
      return index
  return -1


static func _replace_team(teams: Array, updated_team: Dictionary) -> void:
  var index = _find_team_index(teams, updated_team["id"])
  if index != -1:
    teams[index] = updated_team


static func _find_trade_partner(teams: Array, user_index: int) -> int:
  var user_team: Dictionary = teams[user_index]
  for index in range(teams.size()):
    if index == user_index:
      continue
    var team: Dictionary = teams[index]
    if user_team["direction"] == "contending" and team["direction"] != "contending":
      return index
    if user_team["direction"] == "rebuilding" and team["direction"] == "contending":
      return index
  return 1 if user_index == 0 else 0


static func _pick_trade_piece(team: Dictionary, prefer_quality: bool) -> int:
  var roster: Array = team["roster"]
  if roster.size() < 4:
    return -1

  if prefer_quality:
    return 3
  return roster.size() - 1


static func _make_rng(seed_key: String) -> RandomNumberGenerator:
  var rng = RandomNumberGenerator.new()
  rng.seed = abs(int(hash(seed_key)))
  return rng
