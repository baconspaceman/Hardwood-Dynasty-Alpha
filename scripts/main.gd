extends Control

const GMSeed = preload("res://scripts/data/gm_seed.gd")
const LeagueSim = preload("res://scripts/core/league_sim.gd")
const SetupShellStore = preload("res://scripts/core/setup_shell_store.gd")

var game_state: Dictionary = {}

var year_spin_box: SpinBox
var status_label: Label
var headline_label: Label
var summary_label: Label
var selected_club_label: Label
var era_label: Label
var focus_option: OptionButton
var focus_hint_label: Label
var lock_year_button: Button
var start_button: Button
var advance_button: Button
var scout_button: Button
var trade_button: Button

var metric_values: Dictionary = {}
var metric_hints: Dictionary = {}
var syncing_browser_selection: bool = false

var browser_title_label: Label
var team_browser: Tree
var team_overview_label: Label
var roster_grid: GridContainer
var chemistry_value_label: Label
var chemistry_bar: ProgressBar
var buzz_value_label: Label
var buzz_bar: ProgressBar

var feed_container: VBoxContainer
var prospect_container: VBoxContainer
var recent_results_container: VBoxContainer

func _ready() -> void:
  RenderingServer.set_default_clear_color(Color("140d09"))
  game_state = SetupShellStore.load_state()
  theme = _build_theme()
  _build_ui()
  _refresh_ui()


func _build_ui() -> void:
  var scroll = ScrollContainer.new()
  scroll.set_anchors_preset(Control.PRESET_FULL_RECT)
  add_child(scroll)

  var shell = VBoxContainer.new()
  shell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  shell.size_flags_vertical = Control.SIZE_EXPAND_FILL
  shell.add_theme_constant_override("separation", 18)
  shell.custom_minimum_size = Vector2(1500, 980)
  scroll.add_child(shell)

  var margin = MarginContainer.new()
  margin.add_theme_constant_override("margin_left", 26)
  margin.add_theme_constant_override("margin_right", 26)
  margin.add_theme_constant_override("margin_top", 24)
  margin.add_theme_constant_override("margin_bottom", 36)
  shell.add_child(margin)

  var content = VBoxContainer.new()
  content.add_theme_constant_override("separation", 18)
  margin.add_child(content)

  content.add_child(_build_hero())
  content.add_child(_build_metrics())
  content.add_child(_build_league_row())
  content.add_child(_build_feed_row())


func _build_hero() -> PanelContainer:
  var panel = _make_panel(Color("23140d"), Color("6b3d1f"), 28)
  var row = HBoxContainer.new()
  row.add_theme_constant_override("separation", 18)
  panel.add_child(row)

  var left = VBoxContainer.new()
  left.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  left.add_theme_constant_override("separation", 12)
  row.add_child(left)

  var eyebrow = Label.new()
  eyebrow.text = "ALPHA BUILD / FRONT OFFICE BASKETBALL SIM"
  eyebrow.add_theme_font_size_override("font_size", 13)
  eyebrow.add_theme_color_override("font_color", Color("ffb45c"))
  left.add_child(eyebrow)

  var title = Label.new()
  title.text = "Hardwood Dynasty Alpha"
  title.add_theme_font_size_override("font_size", 46)
  title.add_theme_color_override("font_color", Color("f8edd8"))
  left.add_child(title)

  headline_label = Label.new()
  headline_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  headline_label.add_theme_font_size_override("font_size", 24)
  headline_label.add_theme_color_override("font_color", Color("ffcf88"))
  left.add_child(headline_label)

  summary_label = Label.new()
  summary_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  summary_label.add_theme_font_size_override("font_size", 16)
  summary_label.add_theme_color_override("font_color", Color("dbcab6"))
  left.add_child(summary_label)

  var chip_row = HFlowContainer.new()
  chip_row.add_theme_constant_override("h_separation", 10)
  chip_row.add_theme_constant_override("v_separation", 10)
  for text in ["Godot 4.6.2", "Weekly Sim", "Franchise Save", "Trade Calls", "Prospect Board"]:
    chip_row.add_child(_make_chip(text))
  left.add_child(chip_row)

  var control_panel = _make_panel(Color("1a100b"), Color("4f2c17"), 22)
  control_panel.custom_minimum_size = Vector2(420, 0)
  row.add_child(control_panel)

  var controls = VBoxContainer.new()
  controls.add_theme_constant_override("separation", 12)
  control_panel.add_child(controls)

  var status_row = HBoxContainer.new()
  controls.add_child(status_row)

  var status_title = Label.new()
  status_title.text = "Save status"
  status_title.add_theme_font_size_override("font_size", 14)
  status_title.add_theme_color_override("font_color", Color("c9b49d"))
  status_row.add_child(status_title)

  status_label = Label.new()
  status_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
  status_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  status_label.add_theme_font_size_override("font_size", 18)
  status_label.add_theme_color_override("font_color", Color("f8edd8"))
  status_row.add_child(status_label)

  var year_label = Label.new()
  year_label.text = "Starting year"
  year_label.add_theme_font_size_override("font_size", 14)
  controls.add_child(year_label)

  year_spin_box = SpinBox.new()
  year_spin_box.min_value = 1950
  year_spin_box.max_value = 2025
  year_spin_box.step = 1
  year_spin_box.value = 1996
  year_spin_box.value_changed.connect(_on_year_value_changed)
  controls.add_child(year_spin_box)

  var quick_years = HFlowContainer.new()
  quick_years.add_theme_constant_override("h_separation", 10)
  quick_years.add_theme_constant_override("v_separation", 10)
  for year in GMSeed.SUPPORTED_YEARS:
    var year_button = Button.new()
    year_button.text = str(year)
    year_button.pressed.connect(_on_quick_year_pressed.bind(year))
    quick_years.add_child(year_button)
  controls.add_child(quick_years)

  var club_row = HBoxContainer.new()
  controls.add_child(club_row)

  var club_title = Label.new()
  club_title.text = "Selected club"
  club_title.add_theme_font_size_override("font_size", 14)
  club_title.add_theme_color_override("font_color", Color("c9b49d"))
  club_row.add_child(club_title)

  selected_club_label = Label.new()
  selected_club_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
  selected_club_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  selected_club_label.add_theme_font_size_override("font_size", 16)
  selected_club_label.add_theme_color_override("font_color", Color("f8edd8"))
  club_row.add_child(selected_club_label)

  era_label = Label.new()
  era_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  era_label.add_theme_font_size_override("font_size", 15)
  era_label.add_theme_color_override("font_color", Color("dbcab6"))
  controls.add_child(era_label)

  var focus_label = Label.new()
  focus_label.text = "Coaching focus"
  focus_label.add_theme_font_size_override("font_size", 14)
  controls.add_child(focus_label)

  focus_option = OptionButton.new()
  for label_text in GMSeed.get_focus_labels():
    focus_option.add_item(label_text)
  focus_option.item_selected.connect(_on_focus_selected)
  controls.add_child(focus_option)

  focus_hint_label = Label.new()
  focus_hint_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  focus_hint_label.add_theme_font_size_override("font_size", 14)
  focus_hint_label.add_theme_color_override("font_color", Color("dbcab6"))
  controls.add_child(focus_hint_label)

  var primary_actions = HBoxContainer.new()
  primary_actions.add_theme_constant_override("separation", 10)
  controls.add_child(primary_actions)

  lock_year_button = Button.new()
  lock_year_button.text = "Lock Year"
  lock_year_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  lock_year_button.pressed.connect(_on_lock_year_pressed)
  primary_actions.add_child(lock_year_button)

  start_button = Button.new()
  start_button.text = "Start Front Office"
  start_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  start_button.pressed.connect(_on_start_franchise_pressed)
  primary_actions.add_child(start_button)

  var live_actions = HBoxContainer.new()
  live_actions.add_theme_constant_override("separation", 10)
  controls.add_child(live_actions)

  advance_button = Button.new()
  advance_button.text = "Advance Week"
  advance_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  advance_button.pressed.connect(_on_advance_week_pressed)
  live_actions.add_child(advance_button)

  scout_button = Button.new()
  scout_button.text = "Scout Prospect"
  scout_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  scout_button.pressed.connect(_on_scout_pressed)
  live_actions.add_child(scout_button)

  trade_button = Button.new()
  trade_button.text = "Trade Call"
  trade_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  trade_button.pressed.connect(_on_trade_pressed)
  live_actions.add_child(trade_button)

  var reset_button = Button.new()
  reset_button.text = "Reset Save"
  reset_button.pressed.connect(_on_reset_pressed)
  controls.add_child(reset_button)

  return panel


func _build_metrics() -> HBoxContainer:
  var row = HBoxContainer.new()
  row.add_theme_constant_override("separation", 14)

  row.add_child(_create_metric_card("Week", "week", "Season clock", Color("ff8f3f")))
  row.add_child(_create_metric_card("Record", "record", "Current standings line", Color("ffc06e")))
  row.add_child(_create_metric_card("Cap Room", "cap", "Flexibility remaining", Color("f4e0b6")))
  row.add_child(_create_metric_card("Fan Buzz", "buzz", "Arena mood", Color("ff8f3f")))
  return row


func _build_league_row() -> HBoxContainer:
  var row = HBoxContainer.new()
  row.add_theme_constant_override("separation", 16)

  var browser_section = _make_section("League Table", "Browser & standings")
  browser_section["panel"].size_flags_horizontal = Control.SIZE_EXPAND_FILL
  browser_title_label = browser_section["title"]
  team_browser = Tree.new()
  team_browser.columns = 5
  team_browser.hide_root = true
  team_browser.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  team_browser.size_flags_vertical = Control.SIZE_EXPAND_FILL
  team_browser.custom_minimum_size = Vector2(0, 430)
  team_browser.item_selected.connect(_on_browser_selected)
  browser_section["body"].add_child(team_browser)
  row.add_child(browser_section["panel"])

  var detail_section = _make_section("Locker Room", "Selected club")
  detail_section["panel"].custom_minimum_size = Vector2(460, 430)

  team_overview_label = Label.new()
  team_overview_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  team_overview_label.add_theme_font_size_override("font_size", 15)
  team_overview_label.add_theme_color_override("font_color", Color("dbcab6"))
  detail_section["body"].add_child(team_overview_label)

  var chemistry_row = _make_progress_row("Locker Room")
  chemistry_value_label = chemistry_row["value"]
  chemistry_bar = chemistry_row["bar"]
  detail_section["body"].add_child(chemistry_row["panel"])

  var buzz_row = _make_progress_row("Arena Buzz")
  buzz_value_label = buzz_row["value"]
  buzz_bar = buzz_row["bar"]
  detail_section["body"].add_child(buzz_row["panel"])

  var roster_title = Label.new()
  roster_title.text = "Rotation"
  roster_title.add_theme_font_size_override("font_size", 18)
  roster_title.add_theme_color_override("font_color", Color("f8edd8"))
  detail_section["body"].add_child(roster_title)

  roster_grid = GridContainer.new()
  roster_grid.columns = 2
  roster_grid.add_theme_constant_override("h_separation", 10)
  roster_grid.add_theme_constant_override("v_separation", 10)
  detail_section["body"].add_child(roster_grid)

  row.add_child(detail_section["panel"])
  return row


func _build_feed_row() -> HBoxContainer:
  var row = HBoxContainer.new()
  row.add_theme_constant_override("separation", 16)

  var feed_section = _make_section("Front Office Feed", "Inbox")
  feed_container = VBoxContainer.new()
  feed_container.add_theme_constant_override("separation", 10)
  feed_section["body"].add_child(feed_container)
  row.add_child(feed_section["panel"])

  var right_column = VBoxContainer.new()
  right_column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  right_column.add_theme_constant_override("separation", 16)

  var prospect_section = _make_section("Prospect Board", "Scouting")
  prospect_container = VBoxContainer.new()
  prospect_container.add_theme_constant_override("separation", 10)
  prospect_section["body"].add_child(prospect_container)
  right_column.add_child(prospect_section["panel"])

  var result_section = _make_section("Recent Results", "Game day")
  recent_results_container = VBoxContainer.new()
  recent_results_container.add_theme_constant_override("separation", 10)
  result_section["body"].add_child(recent_results_container)
  right_column.add_child(result_section["panel"])

  row.add_child(right_column)
  return row


func _refresh_ui() -> void:
  var selected_year = int(game_state.get("selected_year", 0))
  if selected_year > 0:
    year_spin_box.value = selected_year

  var preview_id = str(game_state.get("preview_team_id", GMSeed.FRANCHISE_SLOTS[0]["id"]))
  var preview_slot = GMSeed.get_franchise_slot(preview_id)
  if preview_slot.is_empty():
    preview_slot = GMSeed.FRANCHISE_SLOTS[0]

  var preview_year = int(year_spin_box.value)
  var preview_preset = GMSeed.get_era_preset(preview_year)
  var locked_preset = GMSeed.get_era_preset(selected_year)
  var focus_label = str(game_state.get("focus", GMSeed.DEFAULT_FOCUS))
  var focus_index = GMSeed.get_focus_labels().find(focus_label)
  if focus_index == -1:
    focus_index = 0
  focus_option.select(focus_index)
  focus_hint_label.text = GMSeed.get_focus_data(focus_label)["description"]

  var in_season: bool = str(game_state.get("status", "awaiting-year")) == "in-season"
  var season_complete: bool = str(game_state.get("status", "")) == "season-complete"

  status_label.text = _status_text()
  selected_club_label.text = "%s %s" % [preview_slot["market"], preview_slot["nickname"]]
  headline_label.text = str(game_state.get("headline", "Choose a year and a club to start building your front office."))
  summary_label.text = _summary_text(preview_slot)
  era_label.text = _era_text(preview_year, selected_year, preview_preset, locked_preset)

  lock_year_button.disabled = in_season or season_complete
  start_button.disabled = not GMSeed.is_reasonable_year(int(year_spin_box.value))
  advance_button.disabled = not in_season
  scout_button.disabled = not in_season
  trade_button.disabled = not in_season
  focus_option.disabled = not in_season
  year_spin_box.editable = not in_season and not season_complete

  _refresh_metrics(in_season)
  _refresh_browser(in_season)
  _refresh_team_detail(in_season, preview_slot)
  _refresh_feed()
  _refresh_prospects()
  _refresh_recent_results()


func _refresh_metrics(in_season: bool) -> void:
  if in_season:
    var user_team = LeagueSim.get_user_team(game_state)
    var current_week = int(game_state.get("current_week", 1))
    metric_values["week"].text = "Wk %d" % mini(current_week, int(game_state.get("total_weeks", LeagueSim.TOTAL_WEEKS)))
    metric_hints["week"].text = "of %d" % int(game_state.get("total_weeks", LeagueSim.TOTAL_WEEKS))
    metric_values["record"].text = "%d-%d" % [int(user_team["wins"]), int(user_team["losses"])]
    metric_hints["record"].text = user_team["last_result"]
    metric_values["cap"].text = "$%.1fM" % float(user_team["cap_room"])
    metric_hints["cap"].text = user_team["direction"].capitalize()
    metric_values["buzz"].text = "%d" % int(user_team["fan_support"])
    metric_hints["buzz"].text = str(game_state.get("action_result", ""))
    return

  metric_values["week"].text = "Pre"
  metric_hints["week"].text = "Choose an era"
  metric_values["record"].text = "--"
  metric_hints["record"].text = "No games yet"
  metric_values["cap"].text = "TBD"
  metric_hints["cap"].text = "Generated on save start"
  metric_values["buzz"].text = "Idle"
  metric_hints["buzz"].text = str(game_state.get("note", ""))


func _refresh_browser(in_season: bool) -> void:
  syncing_browser_selection = true
  team_browser.clear()
  team_browser.set_column_titles_visible(true)

  if in_season:
    browser_title_label.text = "League Table"
    team_browser.columns = 5
    team_browser.set_column_title(0, "Rank")
    team_browser.set_column_title(1, "Team")
    team_browser.set_column_title(2, "Record")
    team_browser.set_column_title(3, "Diff")
    team_browser.set_column_title(4, "Direction")

    var root = team_browser.create_item()
    var standings = LeagueSim.sort_standings(game_state.get("teams", []))
    for index in range(standings.size()):
      var team: Dictionary = standings[index]
      var item = team_browser.create_item(root)
      item.set_text(0, str(index + 1))
      item.set_text(1, "%s %s" % [team["market"], team["nickname"]])
      item.set_text(2, "%d-%d" % [int(team["wins"]), int(team["losses"])])
      item.set_text(3, str(int(team["points_for"]) - int(team["points_against"])))
      item.set_text(4, String(team["direction"]).capitalize())
      item.set_metadata(0, team["id"])
      if team["id"] == game_state.get("user_team_id", ""):
        item.set_custom_color(1, Color("ffcc7a"))
      if team["id"] == game_state.get("preview_team_id", ""):
        team_browser.set_selected(item, 0)
    syncing_browser_selection = false
    return

  browser_title_label.text = "Choose Your Franchise"
  team_browser.columns = 4
  team_browser.set_column_title(0, "Team")
  team_browser.set_column_title(1, "Conference")
  team_browser.set_column_title(2, "Division")
  team_browser.set_column_title(3, "Historical Slot")

  var root_pre = team_browser.create_item()
  for entry in GMSeed.FRANCHISE_SLOTS:
    var pre_item = team_browser.create_item(root_pre)
    pre_item.set_text(0, "%s %s" % [entry["market"], entry["nickname"]])
    pre_item.set_text(1, entry["conference"])
    pre_item.set_text(2, entry["division"])
    pre_item.set_text(3, entry["slot"])
    pre_item.set_metadata(0, entry["id"])
    if entry["id"] == game_state.get("preview_team_id", ""):
      team_browser.set_selected(pre_item, 0)
  syncing_browser_selection = false


func _refresh_team_detail(in_season: bool, preview_slot: Dictionary) -> void:
  _clear_container(roster_grid)

  if in_season:
    var preview_team = LeagueSim.get_team_by_id(game_state, str(game_state.get("preview_team_id", "")))
    if preview_team.is_empty():
      preview_team = LeagueSim.get_user_team(game_state)

    team_overview_label.text = "%s %s\n%s / %s\nRecord %d-%d / %s\nCap room $%.1fM / Last result %s" % [
      preview_team["market"],
      preview_team["nickname"],
      preview_team["conference"],
      preview_team["division"],
      int(preview_team["wins"]),
      int(preview_team["losses"]),
      String(preview_team["direction"]).capitalize(),
      float(preview_team["cap_room"]),
      preview_team["last_result"],
    ]
    chemistry_value_label.text = "%d" % int(preview_team["chemistry"])
    chemistry_bar.value = int(preview_team["chemistry"])
    buzz_value_label.text = "%d" % int(preview_team["fan_support"])
    buzz_bar.value = int(preview_team["fan_support"])

    for player in preview_team["roster"]:
      roster_grid.add_child(_make_player_card(player))
    return

  team_overview_label.text = "%s %s\n%s / %s\nHistorical slot %s\nLock a year, then start this franchise to generate the roster and weekly season loop." % [
    preview_slot["market"],
    preview_slot["nickname"],
    preview_slot["conference"],
    preview_slot["division"],
    preview_slot["slot"],
  ]
  chemistry_value_label.text = "--"
  chemistry_bar.value = 0
  buzz_value_label.text = "--"
  buzz_bar.value = 0

  var placeholder_texts = [
    "Pick this club to become its GM.",
    "The generated roster will reflect the selected era once you start the save.",
    "After that, you can scout, trade, and advance week by week.",
    "The league browser becomes a live standings table once the season begins.",
  ]
  for text in placeholder_texts:
    roster_grid.add_child(_make_text_card(text))


func _refresh_feed() -> void:
  _clear_container(feed_container)
  var inbox: Array = game_state.get("inbox", [])
  if inbox.is_empty():
    feed_container.add_child(_make_text_card("No mail yet. Lock a year and start a franchise to open the front-office feed."))
    return

  for entry in inbox:
    feed_container.add_child(_make_feed_item(entry))


func _refresh_prospects() -> void:
  _clear_container(prospect_container)
  var prospects: Array = game_state.get("prospects", [])
  if prospects.is_empty():
    prospect_container.add_child(_make_text_card("Scout reports will appear here once you start making calls on the next class."))
    return

  for prospect in prospects:
    var card = _make_panel(Color("1c120d"), Color("6b3d1f"), 16)
    var box = VBoxContainer.new()
    box.add_theme_constant_override("separation", 4)
    card.add_child(box)

    var title = Label.new()
    title.text = "%s / %s" % [prospect["name"], prospect["position"]]
    title.add_theme_font_size_override("font_size", 16)
    title.add_theme_color_override("font_color", Color("f8edd8"))
    box.add_child(title)

    var body = Label.new()
    body.text = "Current %d / Upside %d / %s" % [
      int(prospect["current"]),
      int(prospect["upside"]),
      prospect["style"],
    ]
    body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    body.add_theme_color_override("font_color", Color("dbcab6"))
    box.add_child(body)
    prospect_container.add_child(card)


func _refresh_recent_results() -> void:
  _clear_container(recent_results_container)
  var results: Array = game_state.get("recent_results", [])
  if results.is_empty():
    recent_results_container.add_child(_make_text_card("Game results start landing here once the season is underway."))
    return

  for result in results:
    recent_results_container.add_child(_make_feed_item(result))


func _create_metric_card(title_text: String, key: String, hint_text: String, accent: Color) -> PanelContainer:
  var panel = _make_panel(Color("241510"), accent.darkened(0.45), 22)
  panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

  var box = VBoxContainer.new()
  box.add_theme_constant_override("separation", 8)
  panel.add_child(box)

  var title = Label.new()
  title.text = title_text
  title.add_theme_font_size_override("font_size", 13)
  title.add_theme_color_override("font_color", Color("c9b49d"))
  box.add_child(title)

  var value = Label.new()
  value.text = "--"
  value.add_theme_font_size_override("font_size", 30)
  value.add_theme_color_override("font_color", Color("f8edd8"))
  box.add_child(value)

  var hint = Label.new()
  hint.text = hint_text
  hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  hint.add_theme_color_override("font_color", Color("dbcab6"))
  box.add_child(hint)

  metric_values[key] = value
  metric_hints[key] = hint
  return panel


func _make_section(title_text: String, kicker_text: String) -> Dictionary:
  var panel = _make_panel(Color("20130d"), Color("60361c"), 24)
  panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

  var box = VBoxContainer.new()
  box.add_theme_constant_override("separation", 14)
  panel.add_child(box)

  var kicker = Label.new()
  kicker.text = kicker_text.to_upper()
  kicker.add_theme_font_size_override("font_size", 12)
  kicker.add_theme_color_override("font_color", Color("ffb45c"))
  box.add_child(kicker)

  var title = Label.new()
  title.text = title_text
  title.add_theme_font_size_override("font_size", 24)
  title.add_theme_color_override("font_color", Color("f8edd8"))
  box.add_child(title)

  return {"panel": panel, "body": box, "title": title}


func _make_progress_row(label_text: String) -> Dictionary:
  var panel = _make_panel(Color("19110c"), Color("4f2c17"), 14)
  var box = VBoxContainer.new()
  box.add_theme_constant_override("separation", 6)
  panel.add_child(box)

  var top = HBoxContainer.new()
  box.add_child(top)

  var label = Label.new()
  label.text = label_text
  label.add_theme_font_size_override("font_size", 14)
  label.add_theme_color_override("font_color", Color("c9b49d"))
  top.add_child(label)

  var value = Label.new()
  value.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
  value.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  value.add_theme_font_size_override("font_size", 14)
  value.add_theme_color_override("font_color", Color("f8edd8"))
  top.add_child(value)

  var bar = ProgressBar.new()
  bar.min_value = 0
  bar.max_value = 100
  bar.show_percentage = false
  box.add_child(bar)

  return {"panel": panel, "value": value, "bar": bar}


func _make_player_card(player: Dictionary) -> PanelContainer:
  var card = _make_panel(Color("19110c"), Color("4f2c17"), 16)
  card.size_flags_horizontal = Control.SIZE_EXPAND_FILL

  var box = VBoxContainer.new()
  box.add_theme_constant_override("separation", 4)
  card.add_child(box)

  var title = Label.new()
  title.text = "%s / %s" % [player["name"], player["position"]]
  title.add_theme_font_size_override("font_size", 15)
  title.add_theme_color_override("font_color", Color("f8edd8"))
  box.add_child(title)

  var body = Label.new()
  body.text = "OVR %d | POT %d | Age %d | $%.1fM\n%s\nShot %d / Defense %d / Creation %d / Mood %d" % [
    int(player["overall"]),
    int(player["potential"]),
    int(player["age"]),
    float(player["salary"]),
    player["archetype"],
    int(player["shot"]),
    int(player["defense"]),
    int(player["creation"]),
    int(player["mood"]),
  ]
  body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  body.add_theme_color_override("font_color", Color("dbcab6"))
  box.add_child(body)
  return card


func _make_feed_item(entry: Dictionary) -> PanelContainer:
  var border = Color("6b3d1f")
  if entry.get("tone", "warm") == "cool":
    border = Color("557086")
  elif entry.get("tone", "warm") == "cold":
    border = Color("7a5044")

  var card = _make_panel(Color("19110c"), border, 16)
  var box = VBoxContainer.new()
  box.add_theme_constant_override("separation", 4)
  card.add_child(box)

  var title = Label.new()
  title.text = str(entry.get("title", "Update"))
  title.add_theme_font_size_override("font_size", 16)
  title.add_theme_color_override("font_color", Color("f8edd8"))
  box.add_child(title)

  var body = Label.new()
  body.text = str(entry.get("body", ""))
  body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  body.add_theme_color_override("font_color", Color("dbcab6"))
  box.add_child(body)
  return card


func _make_text_card(text_value: String) -> PanelContainer:
  var card = _make_panel(Color("19110c"), Color("4f2c17"), 16)
  var label = Label.new()
  label.text = text_value
  label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  label.add_theme_color_override("font_color", Color("dbcab6"))
  card.add_child(label)
  return card


func _make_chip(text_value: String) -> PanelContainer:
  var chip = _make_panel(Color("2b1a11"), Color("8b562e"), 999)
  var label = Label.new()
  label.text = text_value
  label.add_theme_font_size_override("font_size", 13)
  label.add_theme_color_override("font_color", Color("f6dec0"))
  chip.add_child(label)
  return chip


func _make_panel(background_color: Color, border_color: Color, radius: int) -> PanelContainer:
  var panel = PanelContainer.new()
  var style = StyleBoxFlat.new()
  style.bg_color = background_color
  style.border_color = border_color
  style.set_border_width_all(1)
  style.set_corner_radius_all(radius)
  style.content_margin_left = 18
  style.content_margin_right = 18
  style.content_margin_top = 18
  style.content_margin_bottom = 18
  panel.add_theme_stylebox_override("panel", style)
  return panel


func _build_theme() -> Theme:
  var app_theme = Theme.new()
  app_theme.set_color("font_color", "Label", Color("f2e4d3"))
  app_theme.set_font_size("font_size", "Label", 15)

  var button_normal = StyleBoxFlat.new()
  button_normal.bg_color = Color("f08a34")
  button_normal.border_color = Color("ffbe75")
  button_normal.set_border_width_all(1)
  button_normal.set_corner_radius_all(14)
  button_normal.content_margin_left = 14
  button_normal.content_margin_right = 14
  button_normal.content_margin_top = 10
  button_normal.content_margin_bottom = 10

  var button_hover = button_normal.duplicate()
  button_hover.bg_color = Color("ff9c4a")

  var button_pressed = button_normal.duplicate()
  button_pressed.bg_color = Color("cf7024")

  var button_disabled = button_normal.duplicate()
  button_disabled.bg_color = Color("7a5737")
  button_disabled.border_color = Color("8a6848")

  app_theme.set_stylebox("normal", "Button", button_normal)
  app_theme.set_stylebox("hover", "Button", button_hover)
  app_theme.set_stylebox("pressed", "Button", button_pressed)
  app_theme.set_stylebox("disabled", "Button", button_disabled)
  app_theme.set_color("font_color", "Button", Color("1a120b"))
  app_theme.set_color("font_disabled_color", "Button", Color("ddc5a8"))
  app_theme.set_font_size("font_size", "Button", 14)

  var input_style = StyleBoxFlat.new()
  input_style.bg_color = Color("120c08")
  input_style.border_color = Color("624026")
  input_style.set_border_width_all(1)
  input_style.set_corner_radius_all(14)
  input_style.content_margin_left = 12
  input_style.content_margin_right = 12
  input_style.content_margin_top = 8
  input_style.content_margin_bottom = 8

  app_theme.set_stylebox("normal", "LineEdit", input_style)
  app_theme.set_stylebox("focus", "LineEdit", input_style)
  app_theme.set_stylebox("normal", "SpinBox", input_style)
  app_theme.set_stylebox("normal", "Tree", input_style)
  app_theme.set_stylebox("panel", "Tree", input_style)
  app_theme.set_color("font_color", "LineEdit", Color("f8edd8"))
  app_theme.set_color("font_color", "Tree", Color("f8edd8"))
  app_theme.set_color("guide_color", "Tree", Color("6f4b31"))
  app_theme.set_color("title_button_color", "Tree", Color("f8edd8"))
  app_theme.set_font_size("font_size", "LineEdit", 16)
  app_theme.set_font_size("font_size", "Tree", 14)

  var progress_bg = StyleBoxFlat.new()
  progress_bg.bg_color = Color("251710")
  progress_bg.set_corner_radius_all(999)
  progress_bg.content_margin_top = 5
  progress_bg.content_margin_bottom = 5

  var progress_fill = StyleBoxFlat.new()
  progress_fill.bg_color = Color("f08a34")
  progress_fill.set_corner_radius_all(999)
  progress_fill.content_margin_top = 5
  progress_fill.content_margin_bottom = 5

  app_theme.set_stylebox("background", "ProgressBar", progress_bg)
  app_theme.set_stylebox("fill", "ProgressBar", progress_fill)
  app_theme.set_color("font_color", "ProgressBar", Color("f8edd8"))
  return app_theme


func _status_text() -> String:
  var status = str(game_state.get("status", "awaiting-year"))
  if status == "in-season":
    return "Week %d / %d" % [
      mini(int(game_state.get("current_week", 1)), int(game_state.get("total_weeks", LeagueSim.TOTAL_WEEKS))),
      int(game_state.get("total_weeks", LeagueSim.TOTAL_WEEKS)),
    ]
  if status == "year-locked":
    return "Year Locked"
  if status == "season-complete":
    return "Season Complete"
  return "Awaiting Setup"


func _summary_text(preview_slot: Dictionary) -> String:
  var status = str(game_state.get("status", "awaiting-year"))
  if status == "in-season" or status == "season-complete":
    var user_team = LeagueSim.get_user_team(game_state)
    return "Running %s %s in %d. Focus: %s. %s" % [
      user_team["market"],
      user_team["nickname"],
      int(game_state.get("selected_year", 0)),
      game_state.get("focus", GMSeed.DEFAULT_FOCUS),
      game_state.get("action_result", ""),
    ]

  return "Pick an era, choose a franchise from the browser, then start the front office. The active preview right now is %s %s." % [
    preview_slot["market"],
    preview_slot["nickname"],
  ]


func _era_text(preview_year: int, selected_year: int, preview_preset: Dictionary, locked_preset: Dictionary) -> String:
  if str(game_state.get("status", "")) == "in-season" or str(game_state.get("status", "")) == "season-complete":
    if not locked_preset.is_empty():
      return "%s\n%s" % [locked_preset["label"], locked_preset["tone"]]
    return "Year %d is active, but it still needs a dedicated authored data pack for full historical fidelity." % selected_year

  if GMSeed.is_reasonable_year(preview_year):
    if preview_preset.is_empty():
      return "Year %d is valid, but it still needs an authored year pack before full historical generation can happen." % preview_year
    return "%s\n%s" % [preview_preset["label"], preview_preset["anchor"]]
  return "Enter a historical start year between 1950 and 2025."


func _clear_container(container: Node) -> void:
  for child in container.get_children():
    child.queue_free()


func _commit_state() -> void:
  SetupShellStore.save_state(game_state)
  _refresh_ui()


func _on_year_value_changed(_value: float) -> void:
  if str(game_state.get("status", "")) != "in-season" and str(game_state.get("status", "")) != "season-complete":
    _refresh_ui()


func _on_quick_year_pressed(year: int) -> void:
  if year_spin_box.editable:
    year_spin_box.value = year
    _refresh_ui()


func _on_browser_selected() -> void:
  if syncing_browser_selection:
    return

  var item = team_browser.get_selected()
  if item == null:
    return

  var team_id = str(item.get_metadata(0))
  if team_id == "":
    return

  game_state["preview_team_id"] = team_id
  SetupShellStore.save_state(game_state)
  _refresh_ui()


func _on_lock_year_pressed() -> void:
  var selected_year = int(year_spin_box.value)
  if not GMSeed.is_reasonable_year(selected_year):
    return

  var preset = GMSeed.get_era_preset(selected_year)
  game_state["selected_year"] = selected_year
  game_state["status"] = "year-locked"
  game_state["generated_league"] = false
  game_state["teams"] = []
  game_state["current_week"] = 1
  game_state["headline"] = "Year %d locked. Choose a franchise and start the front office." % selected_year
  game_state["note"] = "The save is ready for a franchise start."
  game_state["action_result"] = "Historical setup is locked. Pick a club from the browser and start the save."
  if not preset.is_empty():
    game_state["headline"] = preset["label"]
  _commit_state()


func _on_start_franchise_pressed() -> void:
  var selected_year = int(game_state.get("selected_year", int(year_spin_box.value)))
  if not GMSeed.is_reasonable_year(selected_year):
    return

  var franchise_id = str(game_state.get("preview_team_id", GMSeed.FRANCHISE_SLOTS[0]["id"]))
  game_state = LeagueSim.create_franchise_state(selected_year, franchise_id)
  _commit_state()


func _on_focus_selected(index: int) -> void:
  if str(game_state.get("status", "")) != "in-season":
    return
  var selected_focus = focus_option.get_item_text(index)
  game_state = LeagueSim.set_focus(game_state, selected_focus)
  _commit_state()


func _on_advance_week_pressed() -> void:
  game_state = LeagueSim.advance_week(game_state)
  _commit_state()


func _on_scout_pressed() -> void:
  game_state = LeagueSim.scout_prospect(game_state)
  _commit_state()


func _on_trade_pressed() -> void:
  game_state = LeagueSim.make_trade_call(game_state)
  _commit_state()


func _on_reset_pressed() -> void:
  game_state = SetupShellStore.reset_state()
  year_spin_box.value = 1996
  _refresh_ui()
