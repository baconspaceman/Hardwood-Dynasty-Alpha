extends Control

const GMSeed = preload("res://scripts/data/gm_seed.gd")
const SetupShellStore = preload("res://scripts/core/setup_shell_store.gd")

var setup_state: Dictionary = {}
var year_spin_box: SpinBox
var status_label: Label
var setup_label: Label
var era_label: Label
var metric_values: Dictionary = {}
var metric_hints: Dictionary = {}
var franchise_tree: Tree
var rules_container: VBoxContainer

func _ready() -> void:
  RenderingServer.set_default_clear_color(Color("0a131c"))
  setup_state = SetupShellStore.load_state()
  theme = _build_theme()
  _build_ui()
  _refresh_ui()


func _build_ui() -> void:
  var scroll := ScrollContainer.new()
  scroll.set_anchors_preset(Control.PRESET_FULL_RECT)
  add_child(scroll)

  var shell := VBoxContainer.new()
  shell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  shell.size_flags_vertical = Control.SIZE_EXPAND_FILL
  shell.add_theme_constant_override("separation", 18)
  shell.custom_minimum_size = Vector2(1440, 920)
  scroll.add_child(shell)

  var margin := MarginContainer.new()
  margin.add_theme_constant_override("margin_left", 28)
  margin.add_theme_constant_override("margin_right", 28)
  margin.add_theme_constant_override("margin_top", 24)
  margin.add_theme_constant_override("margin_bottom", 36)
  shell.add_child(margin)

  var content := VBoxContainer.new()
  content.add_theme_constant_override("separation", 18)
  margin.add_child(content)

  content.add_child(_build_hero())
  content.add_child(_build_metrics())
  content.add_child(_build_secondary_row())
  content.add_child(_build_franchise_row())
  content.add_child(_build_footer_row())


func _build_hero() -> PanelContainer:
  var panel := _make_panel(Color("172836"), Color("395062"), 26)
  var row := HBoxContainer.new()
  row.add_theme_constant_override("separation", 18)
  panel.add_child(row)

  var left := VBoxContainer.new()
  left.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  left.add_theme_constant_override("separation", 12)
  row.add_child(left)

  var eyebrow := Label.new()
  eyebrow.text = "Prompt Loaded / Godot Test Build"
  eyebrow.add_theme_font_size_override("font_size", 13)
  eyebrow.add_theme_color_override("font_color", Color("87d6f0"))
  left.add_child(eyebrow)

  var title := Label.new()
  title.text = "Hardwood Dynasty Alpha"
  title.add_theme_font_size_override("font_size", 44)
  title.add_theme_color_override("font_color", Color("f8eddc"))
  left.add_child(title)

  var summary := Label.new()
  summary.text = "A real Godot prototype for a text-based basketball GM simulation. This build locks the start-year setup flow, previews the rules layer, and gives the repo a reproducible Windows export path."
  summary.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  summary.add_theme_font_size_override("font_size", 16)
  summary.add_theme_color_override("font_color", Color("c7d4dc"))
  left.add_child(summary)

  var tag_row := HFlowContainer.new()
  tag_row.add_theme_constant_override("h_separation", 10)
  tag_row.add_theme_constant_override("v_separation", 10)
  for text in ["Godot 4.6.2", "Control UI", "Windows Export", "Basketball GM Sim"]:
    tag_row.add_child(_make_chip(text))
  left.add_child(tag_row)

  var right_panel := _make_panel(Color("0f1923"), Color("2d4151"), 22)
  right_panel.custom_minimum_size = Vector2(360, 0)
  row.add_child(right_panel)

  var right := VBoxContainer.new()
  right.add_theme_constant_override("separation", 12)
  right_panel.add_child(right)

  var status_row := HBoxContainer.new()
  right.add_child(status_row)

  var status_title := Label.new()
  status_title.text = "Setup status"
  status_title.add_theme_font_size_override("font_size", 14)
  status_title.add_theme_color_override("font_color", Color("9eb2bf"))
  status_row.add_child(status_title)

  status_label = Label.new()
  status_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
  status_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  status_label.add_theme_font_size_override("font_size", 18)
  status_label.add_theme_color_override("font_color", Color("f8eddc"))
  status_row.add_child(status_label)

  var field_label := Label.new()
  field_label.text = "Starting year"
  field_label.add_theme_font_size_override("font_size", 14)
  right.add_child(field_label)

  year_spin_box = SpinBox.new()
  year_spin_box.min_value = 1950
  year_spin_box.max_value = 2025
  year_spin_box.step = 1
  year_spin_box.value = setup_state.get("selected_year", 1996)
  if int(year_spin_box.value) == 0:
    year_spin_box.value = 1996
  year_spin_box.value_changed.connect(_on_year_value_changed)
  right.add_child(year_spin_box)

  var quick_years := HFlowContainer.new()
  quick_years.add_theme_constant_override("h_separation", 10)
  quick_years.add_theme_constant_override("v_separation", 10)
  for year in GMSeed.SUPPORTED_YEARS:
    var year_button := Button.new()
    year_button.text = str(year)
    year_button.pressed.connect(_on_quick_year_pressed.bind(year))
    quick_years.add_child(year_button)
  right.add_child(quick_years)

  era_label = Label.new()
  era_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  era_label.add_theme_font_size_override("font_size", 15)
  era_label.add_theme_color_override("font_color", Color("c7d4dc"))
  right.add_child(era_label)

  var button_row := HBoxContainer.new()
  button_row.add_theme_constant_override("separation", 10)
  right.add_child(button_row)

  var lock_button := Button.new()
  lock_button.text = "Lock Starting Year"
  lock_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  lock_button.pressed.connect(_on_lock_year_pressed)
  button_row.add_child(lock_button)

  var reset_button := Button.new()
  reset_button.text = "Reset Setup"
  reset_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  reset_button.pressed.connect(_on_reset_pressed)
  button_row.add_child(reset_button)

  return panel


func _build_metrics() -> HBoxContainer:
  var row := HBoxContainer.new()
  row.add_theme_constant_override("separation", 14)

  row.add_child(_create_metric_card("Franchise Slots", "franchises", "Fictional league shell", Color("f28c38")))
  row.add_child(_create_metric_card("GM Commands", "commands", "Front-office actions", Color("87d6f0")))
  row.add_child(_create_metric_card("Supported Eras", "eras", "Ready-to-demo years", Color("d8d1c2")))
  row.add_child(_create_metric_card("Save State", "save", "Persistent setup shell", Color("f28c38")))

  return row


func _build_secondary_row() -> HBoxContainer:
  var row := HBoxContainer.new()
  row.add_theme_constant_override("separation", 16)

  var setup_panel := _make_section("Setup Phase", "Prompt contract")
  setup_panel.custom_minimum_size = Vector2(0, 240)
  setup_label = Label.new()
  setup_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  setup_label.add_theme_font_size_override("font_size", 15)
  setup_panel.get_child(0).add_child(setup_label)
  row.add_child(setup_panel)

  var commands_panel := _make_section("Command Surface", "General manager toolkit")
  commands_panel.custom_minimum_size = Vector2(0, 240)
  var command_list := VBoxContainer.new()
  command_list.add_theme_constant_override("separation", 10)
  for command in GMSeed.COMMANDS:
    var entry := _make_panel(Color("121d28"), Color("263a48"), 16)
    var entry_box := VBoxContainer.new()
    entry_box.add_theme_constant_override("separation", 4)
    entry.add_child(entry_box)

    var command_title := Label.new()
    command_title.text = command["label"]
    command_title.add_theme_font_size_override("font_size", 16)
    command_title.add_theme_color_override("font_color", Color("f8eddc"))
    entry_box.add_child(command_title)

    var command_body := Label.new()
    command_body.text = command["description"]
    command_body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    command_body.add_theme_color_override("font_color", Color("a9bbc7"))
    entry_box.add_child(command_body)
    command_list.add_child(entry)
  commands_panel.get_child(0).add_child(command_list)
  row.add_child(commands_panel)

  return row


func _build_franchise_row() -> HBoxContainer:
  var row := HBoxContainer.new()
  row.add_theme_constant_override("separation", 16)

  var franchise_panel := _make_section("Thirty Franchise Slots", "Fictional league shell")
  franchise_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  var franchise_wrapper := VBoxContainer.new()
  franchise_wrapper.size_flags_vertical = Control.SIZE_EXPAND_FILL
  franchise_tree = Tree.new()
  franchise_tree.columns = 4
  franchise_tree.hide_root = true
  franchise_tree.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  franchise_tree.size_flags_vertical = Control.SIZE_EXPAND_FILL
  franchise_tree.custom_minimum_size = Vector2(0, 360)
  franchise_wrapper.add_child(franchise_tree)
  franchise_panel.get_child(0).add_child(franchise_wrapper)
  row.add_child(franchise_panel)

  var rules_panel := _make_section("Mechanical Samples", "Rules engine")
  rules_panel.custom_minimum_size = Vector2(360, 360)
  rules_container = VBoxContainer.new()
  rules_container.add_theme_constant_override("separation", 10)
  rules_panel.get_child(0).add_child(rules_container)
  row.add_child(rules_panel)

  return row


func _build_footer_row() -> HBoxContainer:
  var row := HBoxContainer.new()
  row.add_theme_constant_override("separation", 16)

  var constraints_panel := _make_section("What The Repo Enforces", "Technical constraints")
  var constraints_box := VBoxContainer.new()
  constraints_box.add_theme_constant_override("separation", 10)
  for text in [
    "Wait for the player to choose a starting year before league generation.",
    "Keep real-world anchoring internal while exposed teams and players remain fictional.",
    "Persist setup state locally so future multi-season work has a clean save boundary.",
    "Treat salary cap, trade logic, and progression as explicit systems, not narrative fluff."
  ]:
    constraints_box.add_child(_make_bullet(text))
  constraints_panel.get_child(0).add_child(constraints_box)
  row.add_child(constraints_panel)

  var next_panel := _make_section("Next Build Priorities", "Production follow-up")
  var next_box := VBoxContainer.new()
  next_box.add_theme_constant_override("separation", 10)
  for text in [
    "Author the first complete historical year pack, likely 1996.",
    "Generate fictional rosters and contracts from that year pack.",
    "Replace rules preview cards with real persistent simulation state.",
    "Add roster, trade, draft, and game-sim command loops."
  ]:
    next_box.add_child(_make_bullet(text))
  next_panel.get_child(0).add_child(next_box)
  row.add_child(next_panel)

  return row


func _refresh_ui() -> void:
  var selected_year := int(setup_state.get("selected_year", 0))
  if selected_year > 0:
    year_spin_box.value = selected_year

  var preview_year := int(year_spin_box.value)
  var preview_preset := GMSeed.get_era_preset(preview_year)
  var active_preset := GMSeed.get_era_preset(selected_year) if selected_year > 0 else preview_preset
  var status_text := "Year %d Locked" % selected_year if selected_year > 0 else "Awaiting Year"

  status_label.text = status_text
  metric_values["franchises"].text = str(GMSeed.FRANCHISE_SLOTS.size())
  metric_hints["franchises"].text = "Fictional league shell"
  metric_values["commands"].text = str(GMSeed.COMMANDS.size())
  metric_hints["commands"].text = "Front-office actions"
  metric_values["eras"].text = str(GMSeed.SUPPORTED_YEARS.size())
  metric_hints["eras"].text = "1984, 1996, 2024"
  metric_values["save"].text = "Locked" if selected_year > 0 else "Idle"
  metric_hints["save"].text = str(setup_state.get("note", ""))

  if GMSeed.is_reasonable_year(preview_year):
    if preview_preset.is_empty():
      era_label.text = "Year %d is valid, but it still needs a dedicated authored data pack before full league generation can begin." % preview_year
    else:
      era_label.text = preview_preset["tone"]
  else:
    era_label.text = "Enter a historical start year between 1950 and 2025."

  var setup_lines := PackedStringArray()
  setup_lines.append("Rule: wait for a starting year before generating the league.")
  setup_lines.append("Selected year: %s" % ("Not chosen yet" if selected_year == 0 else str(selected_year)))
  if active_preset.is_empty():
    setup_lines.append("Era preset: custom year pending an authored data pack.")
    setup_lines.append("League shell: %d fictional franchises mapped to hidden historical slots." % GMSeed.FRANCHISE_SLOTS.size())
  else:
    setup_lines.append("Era preset: %s" % active_preset["label"])
    setup_lines.append("Anchor note: %s" % active_preset["anchor"])
    setup_lines.append("Scouting note: %s" % active_preset["scouting"])
  setup_label.text = "\n\n".join(setup_lines)

  _rebuild_franchise_tree()
  _rebuild_rules_preview(active_preset, selected_year)


func _rebuild_franchise_tree() -> void:
  franchise_tree.clear()
  franchise_tree.set_column_title(0, "Franchise")
  franchise_tree.set_column_title(1, "Conference")
  franchise_tree.set_column_title(2, "Division")
  franchise_tree.set_column_title(3, "Historical Slot")
  franchise_tree.set_column_titles_visible(true)

  var root := franchise_tree.create_item()
  for entry in GMSeed.FRANCHISE_SLOTS:
    var item := franchise_tree.create_item(root)
    item.set_text(0, "%s %s" % [entry["market"], entry["nickname"]])
    item.set_text(1, entry["conference"])
    item.set_text(2, entry["division"])
    item.set_text(3, entry["slot"])


func _rebuild_rules_preview(active_preset: Dictionary, selected_year: int) -> void:
  for child in rules_container.get_children():
    child.queue_free()

  var rules := [
    {"title": "Salary Cap", "value": "$18.0M room", "body": "Soft-cap shell, luxury tax, and Bird Rights stay explicit."},
    {"title": "Bird Rights", "value": "Available", "body": "Cap exceptions survive once the player has earned retention rights."},
    {"title": "Trade Logic", "value": "Picks > veterans", "body": "Rebuilding teams should prefer long-run asset value over short-term salary ballast."},
    {"title": "Progression", "value": "Young growth window", "body": "High-potential youth improve while older players regress physically."},
  ]

  for rule in rules:
    var tile := _make_panel(Color("121d28"), Color("263a48"), 16)
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", 4)
    tile.add_child(box)

    var title := Label.new()
    title.text = rule["title"]
    title.add_theme_font_size_override("font_size", 14)
    title.add_theme_color_override("font_color", Color("9eb2bf"))
    box.add_child(title)

    var value := Label.new()
    value.text = rule["value"]
    value.add_theme_font_size_override("font_size", 20)
    value.add_theme_color_override("font_color", Color("f8eddc"))
    box.add_child(value)

    var body := Label.new()
    body.text = rule["body"]
    body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    body.add_theme_color_override("font_color", Color("c7d4dc"))
    box.add_child(body)
    rules_container.add_child(tile)

  if not active_preset.is_empty():
    var era_tile := _make_panel(Color("1b2b38"), Color("3d5567"), 16)
    var era_box := VBoxContainer.new()
    era_box.add_theme_constant_override("separation", 4)
    era_tile.add_child(era_box)

    var era_title := Label.new()
    era_title.text = "Active era"
    era_title.add_theme_font_size_override("font_size", 14)
    era_title.add_theme_color_override("font_color", Color("9eb2bf"))
    era_box.add_child(era_title)

    var era_value := Label.new()
    era_value.text = active_preset["label"]
    era_value.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    era_value.add_theme_font_size_override("font_size", 18)
    era_value.add_theme_color_override("font_color", Color("f8eddc"))
    era_box.add_child(era_value)

    var era_body := Label.new()
    era_body.text = "Current setup shell: %s" % ("locked" if selected_year > 0 else "preview only")
    era_body.add_theme_color_override("font_color", Color("c7d4dc"))
    era_box.add_child(era_body)
    rules_container.add_child(era_tile)


func _create_metric_card(title: String, key: String, hint: String, accent: Color) -> PanelContainer:
  var panel := _make_panel(Color("121d28"), accent.darkened(0.45), 22)
  panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

  var box := VBoxContainer.new()
  box.add_theme_constant_override("separation", 8)
  panel.add_child(box)

  var label := Label.new()
  label.text = title
  label.add_theme_font_size_override("font_size", 13)
  label.add_theme_color_override("font_color", Color("9eb2bf"))
  box.add_child(label)

  var value := Label.new()
  value.text = "--"
  value.add_theme_font_size_override("font_size", 28)
  value.add_theme_color_override("font_color", Color("f8eddc"))
  box.add_child(value)

  var hint_label := Label.new()
  hint_label.text = hint
  hint_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  hint_label.add_theme_color_override("font_color", Color("c7d4dc"))
  box.add_child(hint_label)

  metric_values[key] = value
  metric_hints[key] = hint_label
  return panel


func _make_section(title_text: String, kicker_text: String) -> PanelContainer:
  var panel := _make_panel(Color("121d28"), Color("2a3c49"), 24)
  panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

  var box := VBoxContainer.new()
  box.add_theme_constant_override("separation", 14)
  panel.add_child(box)

  var kicker := Label.new()
  kicker.text = kicker_text.to_upper()
  kicker.add_theme_font_size_override("font_size", 12)
  kicker.add_theme_color_override("font_color", Color("87d6f0"))
  box.add_child(kicker)

  var title := Label.new()
  title.text = title_text
  title.add_theme_font_size_override("font_size", 24)
  title.add_theme_color_override("font_color", Color("f8eddc"))
  box.add_child(title)

  return panel


func _make_panel(background_color: Color, border_color: Color, radius: int) -> PanelContainer:
  var panel := PanelContainer.new()
  var style := StyleBoxFlat.new()
  style.bg_color = background_color
  style.border_color = border_color
  style.set_border_width_all(1)
  style.set_corner_radius_all(radius)
  style.content_margin_left = 18
  style.content_margin_top = 18
  style.content_margin_right = 18
  style.content_margin_bottom = 18
  panel.add_theme_stylebox_override("panel", style)
  return panel


func _make_chip(text_value: String) -> PanelContainer:
  var chip := _make_panel(Color("10202b"), Color("324856"), 999)
  var label := Label.new()
  label.text = text_value
  label.add_theme_font_size_override("font_size", 13)
  label.add_theme_color_override("font_color", Color("d5e0e6"))
  chip.add_child(label)
  return chip


func _make_bullet(text_value: String) -> PanelContainer:
  var tile := _make_panel(Color("121d28"), Color("263a48"), 16)
  var label := Label.new()
  label.text = text_value
  label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
  label.add_theme_color_override("font_color", Color("c7d4dc"))
  tile.add_child(label)
  return tile


func _build_theme() -> Theme:
  var app_theme := Theme.new()
  app_theme.set_color("font_color", "Label", Color("dce5ea"))
  app_theme.set_font_size("font_size", "Label", 15)

  var button_normal := StyleBoxFlat.new()
  button_normal.bg_color = Color("f28c38")
  button_normal.border_color = Color("f5b068")
  button_normal.set_border_width_all(1)
  button_normal.set_corner_radius_all(14)
  button_normal.content_margin_left = 14
  button_normal.content_margin_right = 14
  button_normal.content_margin_top = 10
  button_normal.content_margin_bottom = 10

  var button_hover := button_normal.duplicate()
  button_hover.bg_color = Color("ff9b4f")

  var button_pressed := button_normal.duplicate()
  button_pressed.bg_color = Color("d6762a")

  app_theme.set_stylebox("normal", "Button", button_normal)
  app_theme.set_stylebox("hover", "Button", button_hover)
  app_theme.set_stylebox("pressed", "Button", button_pressed)
  app_theme.set_stylebox("disabled", "Button", button_pressed)
  app_theme.set_color("font_color", "Button", Color("101820"))
  app_theme.set_font_size("font_size", "Button", 14)

  var input_style := StyleBoxFlat.new()
  input_style.bg_color = Color("0f1821")
  input_style.border_color = Color("314553")
  input_style.set_border_width_all(1)
  input_style.set_corner_radius_all(14)
  input_style.content_margin_left = 12
  input_style.content_margin_right = 12
  input_style.content_margin_top = 8
  input_style.content_margin_bottom = 8

  app_theme.set_stylebox("normal", "LineEdit", input_style)
  app_theme.set_stylebox("normal", "SpinBox", input_style)
  app_theme.set_color("font_color", "LineEdit", Color("f8eddc"))
  app_theme.set_color("font_color", "SpinBox", Color("f8eddc"))
  app_theme.set_font_size("font_size", "LineEdit", 16)
  app_theme.set_font_size("font_size", "SpinBox", 16)

  return app_theme


func _on_quick_year_pressed(year: int) -> void:
  year_spin_box.value = year
  _refresh_ui()


func _on_year_value_changed(_value: float) -> void:
  _refresh_ui()


func _on_lock_year_pressed() -> void:
  var selected_year := int(year_spin_box.value)
  var note := ""
  var preset := GMSeed.get_era_preset(selected_year)

  if preset.is_empty():
    note = "Year %d recorded. The setup shell is locked, but the repo still needs a dedicated year pack before league generation." % selected_year
  else:
    note = "Year %d locked. %d fictional franchise slots are ready for historical anchoring." % [selected_year, GMSeed.FRANCHISE_SLOTS.size()]

  setup_state = {
    "version": SetupShellStore.VERSION,
    "selected_year": selected_year,
    "status": "year-locked",
    "generated_league": false,
    "note": note,
  }
  SetupShellStore.save_state(setup_state)
  _refresh_ui()


func _on_reset_pressed() -> void:
  setup_state = SetupShellStore.reset_state()
  year_spin_box.value = 1996
  _refresh_ui()
