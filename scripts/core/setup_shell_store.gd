extends RefCounted
class_name SetupShellStore

const SAVE_PATH := "user://hardwood_dynasty_alpha_state.json"
const LeagueSim = preload("res://scripts/core/league_sim.gd")

static func load_state() -> Dictionary:
  var base_state: Dictionary = LeagueSim.create_blank_state()
  if not FileAccess.file_exists(SAVE_PATH):
    return base_state

  var file = FileAccess.open(SAVE_PATH, FileAccess.READ)
  if file == null:
    return base_state

  var parsed = JSON.parse_string(file.get_as_text())
  if typeof(parsed) != TYPE_DICTIONARY:
    return base_state

  if int(parsed.get("version", 0)) != LeagueSim.SAVE_VERSION:
    return base_state

  for key in base_state.keys():
    if not parsed.has(key):
      parsed[key] = base_state[key]
  return parsed


static func save_state(state: Dictionary) -> void:
  var file = FileAccess.open(SAVE_PATH, FileAccess.WRITE)
  if file == null:
    return
  file.store_string(JSON.stringify(state))


static func reset_state() -> Dictionary:
  var base_state: Dictionary = LeagueSim.create_blank_state()
  save_state(base_state)
  return base_state
