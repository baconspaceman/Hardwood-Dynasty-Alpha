extends RefCounted
class_name SetupShellStore

const SAVE_PATH := "user://setup_shell.cfg"
const VERSION := 1
const DEFAULT_STATE := {
  "version": VERSION,
  "selected_year": 0,
  "status": "awaiting-year",
  "generated_league": false,
  "note": "Wait for the user to provide a starting year before generating the league.",
}

static func load_state() -> Dictionary:
  var config := ConfigFile.new()
  var error := config.load(SAVE_PATH)

  if error != OK:
    return DEFAULT_STATE.duplicate(true)

  var state := DEFAULT_STATE.duplicate(true)
  for key in DEFAULT_STATE.keys():
    if config.has_section_key("setup", key):
      state[key] = config.get_value("setup", key)

  return state

static func save_state(state: Dictionary) -> void:
  var config := ConfigFile.new()
  for key in state.keys():
    config.set_value("setup", key, state[key])
  config.save(SAVE_PATH)

static func reset_state() -> Dictionary:
  if FileAccess.file_exists(SAVE_PATH):
    DirAccess.remove_absolute(SAVE_PATH)
  return DEFAULT_STATE.duplicate(true)
