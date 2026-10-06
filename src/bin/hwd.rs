//! `hwd` - the Hardwood Dynasty text front end.
//!
//! Run `hwd` for the interactive menu-style prompt, `hwd help` for the command list, or use
//! one-shot commands such as `hwd rules 1962` and `hwd sim --year 1962 --years 10`.
//! This file only handles typing and printing; all game logic lives in the library.

use hardwood_dynasty::calendar::Goal;
use hardwood_dynasty::career::*;
use hardwood_dynasty::content::{export_defaults, load_mods_from_dir, Content};
use hardwood_dynasty::injury;
use hardwood_dynasty::league::*;
use hardwood_dynasty::player::*;
use hardwood_dynasty::settings::SettingValue;
use hardwood_dynasty::setup::NewLeagueOptions;
use hardwood_dynasty::team::*;
use hardwood_dynasty::trade::*;
use hardwood_dynasty::types::*;
use std::io::{BufRead, Write};

const DEFAULT_SAVE: &str = "league.hwdsave";

struct Session {
    league: Option<League>,
    save_path: String,
    mods_dir: String,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut s = Session { league: None, save_path: DEFAULT_SAVE.into(), mods_dir: "mods".into() };
    if args.is_empty() {
        repl(&mut s);
        return;
    }
    match args[0].as_str() {
        "run" => {
            let Some(path) = args.get(1) else {
                println!("Usage: hwd run <script.txt>   (one command per line)");
                return;
            };
            match std::fs::read_to_string(path) {
                Ok(text) => {
                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        println!("> {line}");
                        if !dispatch(&mut s, line) {
                            break;
                        }
                    }
                }
                Err(e) => println!("Can't read {path}: {e}"),
            }
        }
        "sim" => headless_sim(&args[1..]),
        "calibrate" => calibrate(&args[1..]),
        "gen-docs" => gen_docs(args.get(1).map(|s| s.as_str()).unwrap_or("docs")),
        _ => {
            let line = args.join(" ");
            dispatch(&mut s, &line);
        }
    }
}

fn repl(s: &mut Session) {
    println!("==============================================================");
    println!("  HARDWOOD DYNASTY  -  a basketball life & franchise simulator");
    println!("==============================================================");
    println!("New here? Type  quickstart  for a 2-minute tour, or  help  for all commands.");
    println!("Type  new  to start a league.   Type  quit  to leave.\n");
    let stdin = std::io::stdin();
    loop {
        let prompt = match &s.league {
            Some(l) => format!("[{}] > ", l.date_string()),
            None => "> ".to_string(),
        };
        print!("{prompt}");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !dispatch(s, line) {
            break;
        }
    }
}

fn tokens(line: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut inq = false;
    for c in line.chars() {
        match c {
            '"' => inq = !inq,
            c if c.is_whitespace() && !inq => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Returns false to quit.
fn dispatch(s: &mut Session, line: &str) -> bool {
    let t = tokens(line);
    if t.is_empty() {
        return true;
    }
    let cmd = t[0].to_lowercase();
    let a: Vec<&str> = t[1..].iter().map(|x| x.as_str()).collect();
    match cmd.as_str() {
        "quit" | "exit" | "q" => return false,
        "help" | "?" => help(a.first().copied()),
        "quickstart" => quickstart(),
        "new" => cmd_new(s, &a),
        "load" => cmd_load(s, &a),
        "save" => cmd_save(s, &a),
        "rules" => cmd_rules(s, &a),
        "settings" | "setting" => cmd_settings(s, &a),
        "mod" | "mods" => cmd_mod(s, &a),
        "scenarios" => {
            for sc in League::scenarios() {
                println!("  {:<12} {} - {}", sc.id, sc.name, sc.text);
            }
        }
        _ => {
            if s.league.is_none() {
                println!("No league yet. Type  new  (or  load ) first. Try  help  for commands.");
            } else {
                let mut l = s.league.take().unwrap();
                let keep = league_command(s, &mut l, &cmd, &a);
                s.league = Some(l);
                return keep;
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------------------

fn pad(s: &str, n: usize) -> String {
    let mut o: String = s.chars().take(n).collect();
    while o.chars().count() < n {
        o.push(' ');
    }
    o
}

fn rpad(s: &str, n: usize) -> String {
    let c = s.chars().count();
    if c >= n {
        s.chars().take(n).collect()
    } else {
        format!("{}{}", " ".repeat(n - c), s)
    }
}

fn rating(l: &League, ovr: f64) -> String {
    if l.settings.text("display.rating_scale") == "scout" {
        format!("{:.0}", 20.0 + ((ovr - 25.0) / 74.0 * 60.0).clamp(0.0, 60.0))
    } else {
        format!("{ovr:.0}")
    }
}

fn height(l: &League, p: &Player) -> String {
    if l.settings.bool("display.imperial") {
        p.height_str()
    } else {
        format!("{}cm", (p.height_in as f64 * 2.54).round())
    }
}

fn hint(l: &League, text: &str) {
    if l.settings.bool("display.explain") {
        println!("  ({text})");
    }
}

fn parse_money(s: &str) -> Option<Money> {
    let t = s.trim().to_lowercase().replace(['$', ','], "");
    let (num, mult) = if let Some(n) = t.strip_suffix('m') {
        (n, 1_000_000.0)
    } else if let Some(n) = t.strip_suffix('k') {
        (n, 1_000.0)
    } else {
        (t.as_str(), 1.0)
    };
    num.parse::<f64>().ok().map(|v| (v * mult) as i64)
}

fn help(topic: Option<&str>) {
    if let Some(t) = topic {
        println!("(Detailed help for '{t}': type the command with no arguments to see its usage.)");
    }
    println!(
        "
GETTING STARTED
  new [year] [seed]       Start a league (any year from 1946).  e.g.  new 1984 mygame
  quickstart              A guided tour
  save [file] / load [file]
  scenarios               Story-driven starting situations

TIME
  advance [day|week|month|season|years N|playoffs|draft|fa]    Move the calendar
  status                  Where you are, your job and what needs attention
  decisions / decide <id> <option>      Answer pending choices
  events / event <id> <option>          Life events for your player

YOUR JOB  (pick any; careers can combine them)
  role <gm|coach|owner|scout|assistant> <team>     Run a pro team
  role <college_coach|ad|college_scout> <school>   Run a college program
  create-player           Build a player and live the whole story (HS -> college -> pro/overseas)

LOOK AROUND
  standings [east|west|college]   roster [team]   player <name>   box   news [n]
  league    leaders    awards [year]    history    story    odds    schedule
  draftboard    college    clubs    people [role]    records

PLAYER LIFE  (when you play as a created player)
  life                    Your stats, money, relationships, time budget
  allocate study=20 skill_work=30 ...   Split your time
  focus <skill>           Practice focus (shooting, defense, ...)
  agent hire              Hire an agent

GM / COACH / OWNER TOOLS
  trade <team> give <names|picks> get <names|picks>    (add 'confirm' to execute)
  fa [n]   sign <player> <years> <salary>   resign <player> <years> <salary>   waive <player>
  draftboard   pick <player>   autopick   scout <player>
  minutes <player> <min>   starters <p1..p5>   strategy <field> <value>
  hire <person-id> <role>   fire <person-id>   people
  money   owner   budget <item> <value>   chase <extra-salary>   odds

DATA
  settings [list|search <w>|set <key> <val>|explain <key>|preset <id>|changed]
  rules [year]                    The rulebook for a season
  import roster <file.csv|json> [replace]   import pack <file.json>   export roster|pack <file>
  mod export-defaults <dir> | mod check <file> | mod tuning
"
    );
}

fn quickstart() {
    println!(
        "
QUICKSTART
  1. new 1996 demo           start a league in the 1996-97 season (try 1962, 1984 or 2024!)
  2. role gm BOS             become General Manager of a team (abbreviations: see 'league')
  3. roster                  look at your team;  player <name> for details
  4. advance week            play a week;  advance season  to go all the way
  5. Whenever the game stops, it tells you why: a draft pick, an expiring contract, an event.
  6. settings                tune injuries, difficulty and realism. Every setting explains itself.
  Want to play AS a player instead?  create-player  starts you in high school.
  Want to run the money?  role owner BOS  then  money  and  chase 20M.
"
    );
}

// ---------------------------------------------------------------------------------------
// setup commands
// ---------------------------------------------------------------------------------------

fn load_mod_texts(dir: &str) -> Vec<String> {
    let mut v = vec![];
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut files: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).collect();
        files.sort();
        for f in files {
            if let Ok(t) = std::fs::read_to_string(&f) {
                v.push(t);
            }
        }
    }
    v
}

fn cmd_new(s: &mut Session, a: &[&str]) {
    let year = a.first().and_then(|y| y.parse::<i32>().ok()).unwrap_or(1996);
    let seed = a.get(1).map(|x| x.to_string()).unwrap_or_else(|| format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(1)));
    let preset = a.iter().find_map(|x| x.strip_prefix("preset=")).map(|x| x.to_string());
    let mods = load_mod_texts(&s.mods_dir);
    if !mods.is_empty() {
        println!("Loading {} mod pack(s) from '{}'.", mods.len(), s.mods_dir);
    }
    println!("Building the {year} league (seed '{seed}')...");
    let t0 = std::time::Instant::now();
    match League::new(NewLeagueOptions { name: "My League".into(), year, seed, preset, mods, overrides: Default::default() }) {
        Ok(l) => {
            println!("Done in {:.1}s: {} teams, {} players, {} colleges, {} overseas clubs.", t0.elapsed().as_secs_f64(), l.active_team_ids().len(), l.players.len(), l.colleges.len(), l.clubs.len());
            println!("Rules: {}.", rules_summary(&l.rules));
            println!("Next: 'league' to see the teams, then 'role gm <team>' (or owner/coach), or 'create-player'.");
            s.league = Some(l);
        }
        Err(e) => println!("Couldn't create the league: {e}"),
    }
}

fn cmd_save(s: &mut Session, a: &[&str]) {
    let Some(l) = &s.league else {
        println!("Nothing to save yet.");
        return;
    };
    let path = a.first().map(|x| x.to_string()).unwrap_or(s.save_path.clone());
    match l.save_json().and_then(|j| std::fs::write(&path, j).map_err(|e| e.to_string())) {
        Ok(()) => {
            println!("Saved to {path}.");
            s.save_path = path;
        }
        Err(e) => println!("Save failed: {e}"),
    }
}

fn cmd_load(s: &mut Session, a: &[&str]) {
    let path = a.first().map(|x| x.to_string()).unwrap_or(s.save_path.clone());
    match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|j| League::load_json(&j)) {
        Ok(l) => {
            println!("Loaded {path}: {}.", l.date_string());
            s.league = Some(l);
            s.save_path = path;
        }
        Err(e) => println!("Couldn't load {path}: {e}"),
    }
}

fn rules_summary(r: &hardwood_dynasty::era::Rules) -> String {
    format!(
        "{}, {} games, {}{}, shot clock {}, {} teams make the playoffs{}",
        r.league_name,
        r.games,
        if r.three_point { format!("3-pt line at {:.1} ft, ", r.three_distance) } else { "no 3-pt line, ".to_string() },
        if r.hand_checking { "hand-checking allowed" } else { "hand-checking banned" },
        if r.shot_clock > 0 { format!("{}s", r.shot_clock) } else { "none".into() },
        r.playoff_teams,
        if r.play_in { " (+ play-in)" } else { "" }
    )
}

fn cmd_rules(s: &mut Session, a: &[&str]) {
    let c = Content::default();
    let year = a.first().and_then(|y| y.parse::<i32>().ok()).or(s.league.as_ref().map(|l| l.year)).unwrap_or(2024);
    let r = c.rules(year);
    let st = c.style(year);
    println!("RULEBOOK for the {year}-{:02} season", (year + 1) % 100);
    println!("  League: {}   Games: {}   Quarter: {:.0} min   Overtime: {:.0} min", r.league_name, r.games, r.quarter_minutes, r.overtime_minutes);
    println!("  Shot clock: {}   Three-point line: {}   Hand-checking: {}   Zone defense: {}", if r.shot_clock > 0 { format!("{}s", r.shot_clock) } else { "none".into() }, if r.three_point { format!("{:.2} ft", r.three_distance) } else { "none".into() }, if r.hand_checking { "legal" } else { "banned" }, if r.zone_defense { "legal" } else { "illegal" });
    println!("  Playoffs: {} teams, series {:?}{}", r.playoff_teams, r.series_lengths, if r.play_in { ", with play-in" } else { "" });
    println!("  Draft: {} rounds, {:?}{}{}", r.draft_rounds, r.lottery, if r.hs_allowed { ", high schoolers eligible" } else { "" }, if r.territorial_picks { ", territorial picks" } else { "" });
    println!("  Free agency: {:?}   Cap: {:?}   Max contracts: {}   Luxury tax: {}   Aprons: {}", r.fa_regime, r.cap_type, r.max_contract, r.luxury_tax, r.aprons);
    let m = c.economy.season_money(&r, c.economy.table_cap(year), true);
    if m.cap_enforced {
        println!("  Salary cap: {}   Tax line: {}   Minimum salary: {}", fmt_money(m.cap), if r.luxury_tax { fmt_money(m.tax_line) } else { "none".into() }, fmt_money(m.min_salary));
    } else {
        println!("  No salary cap. Typical team payroll: {}   Average salary: {}", fmt_money(m.cap), fmt_money(m.avg_salary));
    }
    println!("  Style: pace {:.0} possessions, {:.0}% of shots are threes, league scoring {:.1} ppg", st.pace, st.three_rate * 100.0, st.ppg);
    println!("  Roster: {} max ({} dress)   Two-way slots: {}", r.roster_max, r.active_max, r.two_way_slots);
    println!("\nRule changes up to {year}:");
    let mut tl = c.rule_timeline.clone();
    tl.sort_by_key(|x| x.year);
    for ch in tl.iter().filter(|x| x.year <= year && x.year >= year - 12) {
        println!("  {}: {} - {}", ch.year, ch.title, ch.description);
    }
}

fn cmd_settings(s: &mut Session, a: &[&str]) {
    let content = Content::default();
    let mut tmp_settings = hardwood_dynasty::settings::Settings::with_defs(content.settings.clone());
    let settings = match &mut s.league {
        Some(l) => &mut l.settings,
        None => &mut tmp_settings,
    };
    let sub = a.first().copied().unwrap_or("list");
    match sub {
        "list" | "advanced" => {
            let adv = sub == "advanced";
            for (cat, name) in hardwood_dynasty::settings::CATEGORIES {
                println!("\n== {name} ==");
                for d in settings.defs().iter().filter(|d| d.category() == *cat && (adv || !d.advanced)) {
                    let cur = settings.raw(&d.key).map(|v| v.show()).unwrap_or_default();
                    println!("  {:<28} {:<10} {}", d.key, cur, d.name);
                }
            }
            println!("\n'settings explain <key>' tells you exactly what a setting does. 'settings advanced' shows more.");
        }
        "search" => {
            for d in settings.search(a.get(1).copied().unwrap_or("")) {
                println!("  {:<28} {}  -  {}", d.key, d.name, d.description);
            }
        }
        "explain" => match a.get(1).and_then(|k| settings.explain(k)) {
            Some(t) => println!("{t}"),
            None => println!("Unknown setting. Try 'settings search <word>'."),
        },
        "set" => {
            if a.len() < 3 {
                println!("Usage: settings set <key> <value>   (e.g. settings set injuries.frequency 2)");
                return;
            }
            let v = match a[2].parse::<f64>() {
                Ok(n) => SettingValue::Num(n),
                Err(_) => SettingValue::Text(a[2].to_string()),
            };
            match settings.set(a[1], v) {
                Ok(()) => println!("{} is now {}.", a[1], settings.raw(a[1]).map(|v| v.show()).unwrap_or_default()),
                Err(e) => println!("{e}"),
            }
        }
        "reset" => {
            if let Some(k) = a.get(1) {
                if k.contains('.') {
                    settings.reset(k);
                } else {
                    settings.reset_category(k);
                }
                println!("Reset.");
            }
        }
        "changed" => {
            for (d, v) in settings.changed() {
                println!("  {:<28} {} (default {})", d.key, v.show(), d.default.show());
            }
        }
        "preset" => {
            let presets = hardwood_dynasty::settings::builtin_presets();
            match a.get(1) {
                None => {
                    for p in &presets {
                        println!("  {:<10} {} - {}", p.id, p.name, p.description);
                    }
                }
                Some(id) => match presets.iter().find(|p| p.id == *id) {
                    Some(p) => {
                        settings.apply_preset(p);
                        println!("Applied preset '{}': {}", p.name, p.description);
                    }
                    None => println!("Unknown preset. Run 'settings preset' to list them."),
                },
            }
        }
        _ => println!("Usage: settings [list|advanced|search <word>|explain <key>|set <key> <value>|reset <key|category>|changed|preset [id]]"),
    }
}

fn cmd_mod(s: &mut Session, a: &[&str]) {
    match a.first().copied() {
        Some("export-defaults") => {
            let dir = a.get(1).copied().unwrap_or("mod-templates");
            match export_defaults(std::path::Path::new(dir)) {
                Ok(files) => {
                    println!("Wrote {} template files to {dir}:", files.len());
                    for f in files {
                        println!("  {f}");
                    }
                    println!("Edit them, then put your mod .json file in the '{}' folder and start a new league.", s.mods_dir);
                }
                Err(e) => println!("{e}"),
            }
        }
        Some("check") => {
            let Some(path) = a.get(1) else {
                println!("Usage: mod check <file.json>");
                return;
            };
            match std::fs::read_to_string(path) {
                Ok(text) => {
                    let mut c = Content::default();
                    match c.apply_mod_json(&text) {
                        Ok(w) if w.is_empty() => println!("{path}: looks good!"),
                        Ok(w) => {
                            println!("{path}: loads, with {} thing(s) to fix:", w.len());
                            for x in w {
                                println!("  - {x}");
                            }
                        }
                        Err(e) => println!("{path}: {e}"),
                    }
                }
                Err(e) => println!("Can't read {path}: {e}"),
            }
        }
        Some("tuning") => {
            println!("Game-engine tuning parameters (set in a mod's \"tuning\" section):");
            for t in hardwood_dynasty::game::default_tuning() {
                println!("  {:<34} default {:<7} range {}..{}  {}", t.key, t.default, t.min, t.max, t.description);
            }
        }
        Some("list") => {
            let _ = load_mods_from_dir(&mut Content::default(), std::path::Path::new(&s.mods_dir));
            println!("Mods folder: {}", s.mods_dir);
            for t in load_mod_texts(&s.mods_dir) {
                println!("  {}", t.chars().take(80).collect::<String>().replace('\n', " "));
            }
        }
        _ => println!("Usage: mod export-defaults [dir] | mod check <file> | mod tuning | mod list"),
    }
}

// ---------------------------------------------------------------------------------------
// league commands
// ---------------------------------------------------------------------------------------

fn pname(l: &League, id: PlayerId) -> String {
    l.p(id).name()
}

fn need_team(l: &League) -> Option<TeamId> {
    if l.user.team.is_none() {
        println!("You don't run a team yet. Try: role gm <team>   (see 'league' for team abbreviations).");
    }
    l.user.team
}

fn team_arg(l: &League, a: &[&str]) -> Option<TeamId> {
    match a.first() {
        Some(k) => {
            let r = l.find_team(&a.join(" ")).or_else(|| l.find_team(k));
            if r.is_none() {
                println!("No team matches '{}'. See 'league'.", a.join(" "));
            }
            r
        }
        None => l.user.team.or_else(|| {
            println!("Which team? e.g. roster BOS");
            None
        }),
    }
}

fn player_arg(l: &League, a: &[&str]) -> Option<PlayerId> {
    let q = a.join(" ");
    let r = l.find_player(&q);
    if r.is_none() {
        println!("No player matches '{q}'.");
    }
    r
}

fn league_command(s: &mut Session, l: &mut League, cmd: &str, a: &[&str]) -> bool {
    match cmd {
        "league" | "teams" => cmd_league(l),
        "status" => cmd_status(l),
        "advance" | "sim" | "a" => cmd_advance(l, a),
        "decisions" => cmd_decisions(l),
        "decide" => {
            if a.len() < 2 {
                println!("Usage: decide <id> <option-id>");
            } else if let Ok(id) = a[0].parse::<u32>() {
                match l.resolve_decision(id, a[1], false) {
                    Ok(m) => println!("{m}"),
                    Err(e) => println!("{e}"),
                }
            }
        }
        "events" => cmd_events(l),
        "event" => cmd_event(l, a),
        "standings" => cmd_standings(l, a),
        "roster" => {
            if a.first() == Some(&"college") || a.first() == Some(&"school") {
                cmd_college_roster(l, &a[1..]);
            } else if let Some(t) = team_arg(l, a) {
                cmd_roster(l, t);
            }
        }
        "player" | "p" => {
            if let Some(id) = player_arg(l, a) {
                cmd_player(l, id);
            }
        }
        "box" => cmd_box(l, a),
        "news" => cmd_news(l, a),
        "story" | "stories" => cmd_story(l),
        "awards" => cmd_awards(l, a),
        "history" => cmd_history(l),
        "leaders" => cmd_leaders(l),
        "odds" => {
            println!("Estimated championship odds (Monte Carlo from team ratings):");
            for (t, o) in l.title_odds_table().into_iter().take(10) {
                println!("  {:<24} {:>5.1}%", l.team(t).name(), o * 100.0);
            }
        }
        "schedule" => cmd_schedule(l, a),
        "draftboard" | "board" => cmd_draftboard(l),
        "pick" => {
            if let Some(id) = player_arg(l, a) {
                if !l.user_on_clock() {
                    println!("You're not on the clock.");
                } else {
                    match l.make_pick(id) {
                        Some(r) => println!("You select {} with pick #{}.", pname(l, r.player), r.overall),
                        None => println!("That pick failed."),
                    }
                }
            }
        }
        "autopick" => {
            if l.user_on_clock() {
                if let Some(r) = l.draft_next_ai() {
                    println!("Your staff selects {} with pick #{}.", pname(l, r.player), r.overall);
                }
            } else {
                println!("You're not on the clock.");
            }
        }
        "scout" => {
            if let Some(id) = player_arg(l, a) {
                match l.scout_prospect(id) {
                    Ok(m) => println!("{m}"),
                    Err(e) => println!("{e}"),
                }
            }
        }
        "fa" | "freeagents" => cmd_fa(l, a),
        "sign" => cmd_sign(l, a),
        "resign" => cmd_resign(l, a),
        "waive" => {
            if let (Some(t), Some(id)) = (need_team(l), player_arg(l, a)) {
                if l.p(id).team_id() != Some(t) {
                    println!("He isn't on your team.");
                } else {
                    l.waive_player(id, t);
                    println!("{} waived. Guaranteed money stays on your books.", pname(l, id));
                }
            }
        }
        "trade" => cmd_trade(l, a),
        "minutes" => {
            if a.len() >= 2 {
                if let (Some(id), Ok(m)) = (l.find_player(&a[..a.len() - 1].join(" ")), a[a.len() - 1].parse::<f64>()) {
                    match l.set_minutes(id, m) {
                        Ok(m) => println!("{m}"),
                        Err(e) => println!("{e}"),
                    }
                } else {
                    println!("Usage: minutes <player> <0-48>");
                }
            }
        }
        "starters" => {
            let names: Vec<String> = a.join(" ").split(',').map(|x| x.trim().to_string()).collect();
            let ids: Vec<PlayerId> = names.iter().filter_map(|n| l.find_player(n)).collect();
            match l.set_starters(ids) {
                Ok(m) => println!("{m}"),
                Err(e) => println!("{e}  (Usage: starters name1, name2, name3, name4, name5)"),
            }
        }
        "strategy" => {
            if a.len() >= 2 {
                match l.set_strategy_field(a[0], a[1]) {
                    Ok(m) => println!("{m}"),
                    Err(e) => println!("{e}"),
                }
            } else {
                if let Some(t) = need_team(l) {
                    let st = &l.team(t).strategy;
                    println!("Strategy: tempo {:+.1}, threes {:+.1}, inside {:.1}, glass {:.1}, defense {:?}, hack-a {}", st.tempo, st.three_emphasis, st.inside_focus, st.crash_glass, st.defense, st.hack_a);
                }
                println!("Change with: strategy <tempo|threes|inside|glass|hack|defense> <value>");
            }
        }
        "focus" => {
            if a.len() >= 2 {
                let f = a[a.len() - 1];
                if let (Some(id), Some(focus)) = (l.find_player(&a[..a.len() - 1].join(" ")), DevFocus::parse(f)) {
                    match l.set_dev_focus(id, focus) {
                        Ok(m) => println!("{m}"),
                        Err(e) => println!("{e}"),
                    }
                } else {
                    println!("Usage: focus <player> <{}>", DevFocus::ALL.iter().map(|f| f.name().split_whitespace().next().unwrap().to_lowercase()).collect::<Vec<_>>().join("|"));
                }
            } else if let Some(pid) = l.user.player {
                if let Some(f) = a.first().and_then(|x| DevFocus::parse(x)) {
                    let _ = l.set_dev_focus(pid, f);
                    println!("Your practice focus: {}.", f.name());
                } else {
                    println!("Usage: focus <skill>  ({})", DevFocus::ALL.iter().map(|f| f.name()).collect::<Vec<_>>().join(", "));
                }
            }
        }
        "people" => cmd_people(l, a),
        "hire" => {
            if a.len() >= 2 {
                if let (Ok(id), Some(role)) = (a[0].parse::<u32>(), staff_role(a[1])) {
                    match l.hire_staff(id, role) {
                        Ok(m) => println!("{m}"),
                        Err(e) => println!("{e}"),
                    }
                    return true;
                }
            }
            println!("Usage: hire <person-id> <coach|assistant|scout|trainer>   (see 'people')");
        }
        "fire" => {
            if let Some(Ok(id)) = a.first().map(|x| x.parse::<u32>()) {
                match l.fire_staff(id) {
                    Ok(m) => println!("{m}"),
                    Err(e) => println!("{e}"),
                }
            }
        }
        "money" | "finance" | "owner" | "cap" => cmd_money(l),
        "budget" => {
            if let (Some(t), true) = (need_team(l), a.len() >= 2) {
                if !(l.user.has(Role::Owner)) {
                    println!("Only the Owner sets the budget.");
                } else {
                    let v = parse_money(a[1]).map(|m| m as f64).or_else(|| a[1].parse::<f64>().ok());
                    match v {
                        Some(v) => match l.set_budget(t, a[0], v) {
                            Ok(m) => println!("{m}"),
                            Err(e) => println!("{e}"),
                        },
                        None => println!("'{}' isn't a number.", a[1]),
                    }
                }
            } else {
                println!("Usage: budget <payroll|tax|coaching|medical|scouting|facilities|marketing|ticket|concessions> <value>");
            }
        }
        "chase" => cmd_chase(l, a),
        "role" | "take" | "take-role" => cmd_role(l, a),
        "scenario" => {
            if let (Some(id), Some(t)) = (a.first(), l.user.team) {
                match l.apply_scenario(id, t) {
                    Ok(m) => println!("{m}"),
                    Err(e) => println!("{e}"),
                }
            } else {
                println!("Take a team role first, then: scenario <id>  (see 'scenarios').");
            }
        }
        "create-player" | "createplayer" => cmd_create_player(l, a),
        "life" => cmd_life(l),
        "allocate" => {
            let mut wanted = vec![];
            for kv in a {
                if let Some((k, v)) = kv.split_once('=') {
                    if let Ok(n) = v.parse::<f64>() {
                        wanted.push((k.to_string(), n));
                    }
                }
            }
            if wanted.is_empty() {
                println!("Usage: allocate study=20 skill_work=30 gym=10 social=15 family=10 rest=15  (percentages; they're rescaled to 100)");
            } else {
                match l.set_life_allocation(&wanted) {
                    Ok(m) => println!("{m}"),
                    Err(e) => println!("{e}"),
                }
            }
        }
        "agent" => {
            if let Some(pid) = l.user.player {
                let name = format!("Agent {}", l.p(pid).last);
                if let Some(life) = l.pm(pid).life.as_mut() {
                    life.agent = Some(hardwood_dynasty::life::AgentInfo { name: name.clone(), skill: 55.0 + (life.stat("reputation") * 0.3), fee_pct: 4.0, trust: 60.0 });
                    println!("You hire {name}: 4% fee, helps negotiate bigger contracts and endorsements.");
                }
            } else {
                println!("Only a player career has an agent.");
            }
        }
        "college" => cmd_college(l, a),
        "recruit" => {
            if a.len() >= 1 {
                let pts = a.last().and_then(|x| x.parse::<f64>().ok()).unwrap_or(10.0);
                let name = if a.last().and_then(|x| x.parse::<f64>().ok()).is_some() { a[..a.len() - 1].join(" ") } else { a.join(" ") };
                if let Some(id) = l.find_player(&name) {
                    match l.college_offer(id, pts) {
                        Ok(m) => println!("{m}"),
                        Err(e) => println!("{e}"),
                    }
                } else {
                    println!("No such recruit. See 'college recruits'.");
                }
            } else {
                println!("Usage: recruit <player> [points]");
            }
        }
        "nil" => {
            if let Some(Some(m)) = a.first().map(|x| parse_money(x)) {
                match l.college_set_nil(m) {
                    Ok(m) => println!("{m}"),
                    Err(e) => println!("{e}"),
                }
            }
        }
        "clubs" | "overseas" => cmd_clubs(l),
        "records" => {
            for (k, name) in hardwood_dynasty::records::SINGLE_GAME_CATS {
                match l.records.single_game.get(*k) {
                    Some(r) => println!("  {:<22} {:>3.0}  {} ({}, {})", name, r.value, r.holder, r.team, r.season),
                    None => println!("  {:<22} -", name),
                }
            }
        }
        "import" => cmd_import(l, a),
        "export" => cmd_export(l, a),
        "log" | "career" => {
            for e in &l.user.log {
                println!("  {e}");
            }
            println!("  Reputation: {:.0}/100", l.user.reputation);
        }
        "setauto" => {
            l.user.auto_decisions = a.first().map(|x| *x == "on").unwrap_or(true);
            println!("Auto-decisions: {}.", if l.user.auto_decisions { "ON (the game decides small choices for you)" } else { "OFF" });
        }
        "delegate" => {
            if let (Some(what), Some(v)) = (a.first(), a.get(1)) {
                let on = *v == "on";
                match *what {
                    "gm" => l.user.delegate_gm = on,
                    "coach" => l.user.delegate_coach = on,
                    _ => println!("delegate <gm|coach> <on|off>"),
                }
                println!("Owner delegation updated.");
            }
        }
        _ => {
            let _ = s;
            println!("Unknown command '{cmd}'. Type 'help'.");
        }
    }
    true
}

fn staff_role(s: &str) -> Option<StaffRole> {
    match s {
        "coach" | "headcoach" => Some(StaffRole::HeadCoach),
        "assistant" | "asst" => Some(StaffRole::AssistantCoach),
        "scout" => Some(StaffRole::Scout),
        "trainer" | "medical" => Some(StaffRole::Medical),
        _ => None,
    }
}

fn cmd_league(l: &League) {
    println!("{} - {}", l.rules.league_name, l.date_string());
    for conf in 0..2u8 {
        println!("\n{} Conference", l.conference_name(conf));
        for t in l.standings(Some(conf)) {
            let tm = l.team(t);
            println!("  {:<4} {:<26} rating {:>4.1}  payroll {:>8}  {}", tm.abbr, tm.name(), l.team_rating_full_health(t), fmt_money(l.payroll(t)), tm.direction.name());
        }
    }
}

fn cmd_status(l: &League) {
    println!("{}", l.date_string());
    println!("Cap {} | tax line {} | min salary {}", if l.money.cap_enforced { fmt_money(l.money.cap) } else { "none".into() }, if l.rules.luxury_tax { fmt_money(l.money.tax_line) } else { "none".into() }, fmt_money(l.money.min_salary));
    if l.user.roles.is_empty() {
        println!("You have no job yet. Try 'role gm <team>' or 'create-player'.");
    } else {
        println!("Your roles: {}", l.user.roles.iter().map(|r| r.name()).collect::<Vec<_>>().join(", "));
    }
    if let Some(t) = l.user.team {
        let tm = l.team(t);
        let r = &tm.record;
        println!("{}: {}-{}  rating {:.1}  payroll {}  owner approval {:.0}/100", tm.name(), r.w, r.l, l.team_rating_full_health(t), fmt_money(l.payroll(t)), tm.owner.approval);
        println!("Owner mandate: {}", tm.owner.mandate);
        let hurt: Vec<String> = tm.roster.iter().filter(|&&id| l.p(id).is_injured()).map(|&id| format!("{} ({})", l.p(id).name(), l.p(id).injury.as_ref().map(|i| i.name.clone()).unwrap_or_default())).collect();
        if !hurt.is_empty() {
            println!("Injured: {}", hurt.join("; "));
        }
        let exp: Vec<String> = tm.roster.iter().filter(|&&id| l.p(id).flags.contains("expiring")).map(|&id| l.p(id).name()).collect();
        if !exp.is_empty() {
            println!("Expiring contracts: {}", exp.join(", "));
        }
    }
    if let Some(c) = l.user.college {
        let co = &l.colleges[c as usize];
        println!("{} {}: {}-{}  prestige {:.0}", co.name, co.nickname, co.record.w, co.record.l, co.prestige);
    }
    if let Some(pid) = l.user.player {
        let p = l.p(pid);
        println!("Your player: {} ({} ovr, age {}) - {}", p.name(), rating(l, p.ovr as f64), l.age_of(pid), match &p.affiliation {
            Affiliation::HighSchool => format!("high school ({})", p.origin.school),
            Affiliation::College(c) => format!("college ({})", l.colleges[*c as usize].name),
            Affiliation::Nba(t) => format!("pro ({})", l.team(*t).name()),
            Affiliation::Overseas(c) => format!("overseas ({})", l.clubs[*c as usize].name),
            Affiliation::FreeAgent => "free agent".to_string(),
            _ => "retired".to_string(),
        });
    }
    if !l.decisions.is_empty() {
        println!("*** {} decision(s) waiting: type 'decisions' ***", l.decisions.len());
    }
    if let Some(pid) = l.user.player {
        if let Some(life) = &l.p(pid).life {
            if !life.pending.is_empty() {
                println!("*** {} life event(s) waiting: type 'events' ***", life.pending.len());
            }
        }
    }
}

fn cmd_advance(l: &mut League, a: &[&str]) {
    let goal = match a.first().copied().unwrap_or("day") {
        "day" | "d" => Goal::Days(1),
        "week" | "w" => Goal::Days(7),
        "month" | "m" => Goal::Days(30),
        "season" | "s" => Goal::EndOfSeason,
        "years" | "year" | "y" => Goal::Years(a.get(1).and_then(|n| n.parse().ok()).unwrap_or(1)),
        "playoffs" => Goal::UntilPhase(Phase::Playoffs),
        "regular" | "season-end" => Goal::UntilPhase(Phase::PostSeason),
        "draft" => Goal::UntilPhase(Phase::Draft),
        "fa" | "freeagency" => Goal::UntilPhase(Phase::FreeAgency),
        "preseason" => Goal::UntilPhase(Phase::Preseason),
        "step" => Goal::Step,
        n => match n.parse::<u32>() {
            Ok(d) => Goal::Days(d),
            Err(_) => {
                println!("Usage: advance [day|week|month|season|years N|playoffs|draft|fa|preseason|<days>]");
                return;
            }
        },
    };
    let t0 = std::time::Instant::now();
    let rep = l.advance(goal);
    // show the most interesting recent lines
    let user_lines: Vec<&String> = rep.lines.iter().rev().take(14).collect::<Vec<_>>().into_iter().rev().collect();
    for line in user_lines {
        println!("  {line}");
    }
    let news: Vec<&NewsItem> = l.news.iter().rev().filter(|n| matches!(n.kind.as_str(), "major" | "retirement" | "trade" | "story" | "user")).take(8).collect();
    if !news.is_empty() {
        println!("Headlines:");
        for n in news.iter().rev() {
            println!("  [{}] {}", n.season, n.text);
        }
    }
    if let Some(s) = rep.stopped {
        println!("\n>>> Stopped: {s}");
    }
    println!("({} steps in {:.2}s)  Now: {}", rep.steps, t0.elapsed().as_secs_f64(), l.date_string());
}

fn cmd_decisions(l: &League) {
    if l.decisions.is_empty() {
        println!("Nothing waiting on you.");
    }
    for d in &l.decisions {
        println!("\n#{}  {}\n  {}", d.id, d.title, d.text);
        for o in &d.options {
            println!("   - {:<22} {}   ({})", o.id, o.label, o.explain);
        }
        println!("  Answer with: decide {} <option>", d.id);
    }
}

fn cmd_events(l: &League) {
    let Some(pid) = l.user.player else {
        println!("No player career.");
        return;
    };
    match &l.p(pid).life {
        Some(life) if !life.pending.is_empty() => {
            for e in &life.pending {
                println!("\n[{}] {}\n  {}", e.event_id, e.title, e.text);
                for c in &e.choices {
                    println!("   - {:<12} {}  ({})", c.id, c.label, c.explain);
                }
                println!("  Answer with: event {} <option>", e.event_id);
            }
        }
        _ => println!("No life events waiting."),
    }
}

fn cmd_event(l: &mut League, a: &[&str]) {
    let Some(pid) = l.user.player else {
        println!("No player career.");
        return;
    };
    if a.len() < 2 {
        println!("Usage: event <event-id> <option>");
        return;
    }
    let defs = l.content.life.clone();
    let year = l.year;
    match hardwood_dynasty::life::resolve_event(l.pm(pid), &defs, year, a[0], a[1]) {
        Ok(log) => {
            for x in log {
                println!("  {x}");
            }
        }
        Err(e) => println!("{e}"),
    }
}

fn cmd_standings(l: &League, a: &[&str]) {
    if a.first() == Some(&"college") {
        println!("College top 25 (record, prestige):");
        for (i, c) in l.standings_college().into_iter().take(25).enumerate() {
            let co = &l.colleges[c as usize];
            println!("  {:>2}. {:<28} {:>2}-{:<2}  prestige {:.0}", i + 1, format!("{} {}", co.name, co.nickname), co.record.w, co.record.l, co.prestige);
        }
        return;
    }
    let confs: Vec<Option<u8>> = match a.first().copied() {
        Some("east") => vec![Some(0)],
        Some("west") => vec![Some(1)],
        _ => vec![Some(0), Some(1)],
    };
    for c in confs {
        println!("\n{} Conference", l.conference_name(c.unwrap()));
        println!("  {}  {}  {}  {}  {}  {}", pad("Team", 26), rpad("W", 3), rpad("L", 3), rpad("PCT", 5), rpad("DIFF", 6), "Strk");
        for t in l.standings(c) {
            let tm = l.team(t);
            let r = &tm.record;
            let mark = if l.is_user_team(t) { "*" } else { " " };
            println!("{mark} {}  {}  {}  {}  {}  {}", pad(&tm.name(), 26), rpad(&r.w.to_string(), 3), rpad(&r.l.to_string(), 3), rpad(&format!("{:.3}", r.pct()), 5), rpad(&format!("{:+.1}", r.diff_pg()), 6), if r.streak >= 0 { format!("W{}", r.streak) } else { format!("L{}", -r.streak) });
        }
    }
}

fn cmd_roster(l: &League, t: TeamId) {
    let tm = l.team(t);
    println!("{} ({}-{})  Coach: {}  GM: {}", tm.name(), tm.record.w, tm.record.l, l.coach_of(t).map(|c| c.name.clone()).unwrap_or_default(), l.gm_of(t).map(|c| c.name.clone()).unwrap_or_default());
    println!("{} {} {} {} {} {} {} {}  {}", pad("Player", 22), pad("Pos", 3), rpad("Age", 3), rpad("Ht", 5), rpad("OVR", 3), rpad("POT", 3), rpad("PPG", 5), rpad("Salary", 8), "Status");
    let mut ids = tm.roster.clone();
    ids.sort_by(|&a, &b| l.p(b).ovr.cmp(&l.p(a).ovr));
    for id in ids {
        let p = l.p(id);
        let st = p.seasons.iter().rev().find(|r| r.season == l.year && r.level == Level::Pro);
        let ppg = st.map(|r| r.stats.ppg()).unwrap_or(0.0);
        let status = match &p.injury {
            Some(i) => format!("OUT: {} ({}g)", i.name, i.games_remaining),
            None => if p.flags.contains("expiring") { "expiring".into() } else { String::new() },
        };
        let pot = if l.settings.bool("progression.hidden_potential") && !l.is_user_team(t) { "?".to_string() } else { rating(l, p.potential as f64) };
        println!("{} {} {} {} {} {} {} {}  {}", pad(&p.name(), 22), pad(p.position.name(), 3), rpad(&l.age_of(id).to_string(), 3), rpad(&height(l, p), 5), rpad(&rating(l, p.ovr as f64), 3), rpad(&pot, 3), rpad(&format!("{ppg:.1}"), 5), rpad(&fmt_money(p.current_salary()), 8), status);
    }
    println!("Payroll {}  (cap {}, tax line {})", fmt_money(l.payroll(t)), if l.money.cap_enforced { fmt_money(l.money.cap) } else { "none".into() }, if l.rules.luxury_tax { fmt_money(l.money.tax_line) } else { "none".into() });
}

fn cmd_player(l: &League, id: PlayerId) {
    let p = l.p(id);
    println!("{}  #{}  {}  {}  {} lb  born {}  {}", p.name(), id, p.position.name(), height(l, p), p.weight_lb, p.birth_year, p.hometown);
    let arch = l.content.archetypes.iter().find(|a| a.id == p.archetype).map(|a| a.name.clone()).unwrap_or_default();
    let (o, pt, sg) = l.scouted_view(l.user.team, id);
    let hide = l.settings.bool("progression.hidden_potential") && !p.user_controlled && l.user.team.map(|t| p.team_id() != Some(t)).unwrap_or(true) && p.team_id().is_none();
    println!("Archetype: {arch}   Overall {}   Potential {}{}", if hide { format!("~{o:.0}") } else { rating(l, p.ovr as f64) }, if hide || (l.settings.bool("progression.hidden_potential") && !p.user_controlled && p.team_id().map(|t| !l.is_user_team(t)).unwrap_or(true)) { format!("~{pt:.0}") } else { rating(l, p.potential as f64) }, if sg > 0.0 && hide { format!("  (scouting uncertainty ±{sg:.1})") } else { String::new() });
    println!("Status: {:?}   {}", p.affiliation, p.contract.as_ref().map(|c| format!("Contract: {} x {}yr", fmt_money(c.salary()), c.years_left())).unwrap_or_default());
    if let Some(i) = &p.injury {
        println!("INJURY: {}", injury::describe(i, l.settings.bool("injuries.hide_details")));
    }
    println!("Mood {:.0}/100  Wear {:.0}  Fitness {:.0}  Work ethic {}", p.mood.overall, p.wear, p.fitness, p.hidden.work_ethic);
    let mut by_fam: Vec<(Family, Vec<(Attr, u8)>)> = vec![];
    for f in Family::ALL {
        by_fam.push((f, p.attrs.iter().filter(|(a, _)| a.family() == f).collect()));
    }
    for (f, v) in by_fam {
        let parts: Vec<String> = v.iter().map(|(a, r)| format!("{} {}", a.name(), r)).collect();
        println!("  {:<12} {}", f.key(), parts.join(" | "));
    }
    if !p.badges.is_empty() {
        let b: Vec<String> = p.badges.iter().map(|b| format!("{} ({})", l.content.badges.iter().find(|d| d.id == b.id).map(|d| d.name.clone()).unwrap_or(b.id.clone()), b.tier.name())).collect();
        println!("Badges: {}", b.join(", "));
    }
    println!("\n{} {} {} {} {} {} {} {}", pad("Season", 7), pad("Level/Team", 26), rpad("G", 3), rpad("MPG", 5), rpad("PPG", 5), rpad("RPG", 5), rpad("APG", 5), rpad("FG%", 5));
    for r in p.seasons.iter().rev().take(12).rev() {
        println!("{} {} {} {} {} {} {} {}", pad(&format!("{}", r.season), 7), pad(&format!("{:?} {}", r.level, r.team), 26), rpad(&r.stats.g.to_string(), 3), rpad(&format!("{:.1}", r.stats.mpg()), 5), rpad(&format!("{:.1}", r.stats.ppg()), 5), rpad(&format!("{:.1}", r.stats.rpg()), 5), rpad(&format!("{:.1}", r.stats.apg()), 5), rpad(&format!("{:.3}", r.stats.fg_pct()), 5));
    }
    if !p.awards.is_empty() {
        println!("Honors: {}", p.awards.iter().map(|a| format!("{} {}", a.season, a.award)).collect::<Vec<_>>().join("; "));
    }
    if !p.injury_history.is_empty() {
        println!("Injury history: {}", p.injury_history.iter().map(|i| format!("{} {} ({}g)", i.season, i.name, i.games_missed)).collect::<Vec<_>>().join("; "));
    }
    if let Some(d) = &p.draft {
        println!("Drafted: {} round {} pick {} by {}", d.year, d.round, d.pick, d.team);
    }
}

fn cmd_box(l: &League, a: &[&str]) {
    let want_pbp = a.first() == Some(&"pbp");
    let bs = if let Some(u) = l.user.team {
        l.box_log.iter().rev().find(|b| b.home.team == u || b.away.team == u).or(l.box_log.last())
    } else {
        l.box_log.last()
    };
    let Some(bs) = bs else {
        println!("No games played yet.");
        return;
    };
    println!("{} {} @ {} {}{}", bs.away.name, bs.away.pts, bs.home.name, bs.home.pts, if bs.overtimes > 0 { format!(" ({}OT)", bs.overtimes) } else { String::new() });
    println!("Periods (home-away): {}", bs.periods.iter().map(|(h, a)| format!("{h}-{a}")).collect::<Vec<_>>().join("  "));
    for tb in [&bs.away, &bs.home] {
        println!("\n{}", tb.name);
        println!("{} {} {} {} {} {} {} {} {} {} {}", pad("Player", 18), rpad("MIN", 4), rpad("PTS", 3), rpad("REB", 3), rpad("AST", 3), rpad("STL", 3), rpad("BLK", 3), rpad("TO", 2), rpad("FG", 6), rpad("3P", 5), rpad("+/-", 4));
        for p in tb.players.iter().filter(|p| p.stats.g > 0) {
            let s = &p.stats;
            println!("{} {} {} {} {} {} {} {} {} {} {}{}", pad(&p.name, 18), rpad(&format!("{:.0}", s.min), 4), rpad(&s.pts.to_string(), 3), rpad(&s.reb().to_string(), 3), rpad(&s.ast.to_string(), 3), rpad(&s.stl.to_string(), 3), rpad(&s.blk.to_string(), 3), rpad(&s.tov.to_string(), 2), rpad(&format!("{}-{}", s.fgm, s.fga), 6), rpad(&format!("{}-{}", s.tpm, s.tpa), 5), rpad(&format!("{:+}", s.plus_minus), 4), if p.injured_in_game { "  (injured)" } else { "" });
        }
    }
    if want_pbp {
        for line in bs.pbp.iter().take(120) {
            println!("{line}");
        }
    } else if l.settings.bool("sim.play_by_play") {
        println!("\n(type 'box pbp' for play-by-play)");
    }
}

fn cmd_news(l: &League, a: &[&str]) {
    let n = a.first().and_then(|x| x.parse::<usize>().ok()).unwrap_or(15);
    let kind = a.get(1).copied();
    for item in l.news.iter().rev().filter(|i| kind.map(|k| i.kind == k).unwrap_or(true)).take(n).collect::<Vec<_>>().into_iter().rev() {
        println!("  [{} d{}] {}", item.season, item.day, item.text);
    }
}

fn cmd_story(l: &League) {
    let mut any = false;
    for s in l.stories.iter().rev().filter(|s| s.active).take(12) {
        any = true;
        print!("{}", l.story_text(s));
    }
    if !any {
        println!("No storylines yet. They emerge as the seasons unfold (see the 'Storyline intensity' setting).");
    }
}

fn cmd_awards(l: &League, a: &[&str]) {
    let year = a.first().and_then(|y| y.parse::<i32>().ok()).unwrap_or_else(|| if l.phase >= Phase::PostSeason || l.phase == Phase::Playoffs { l.year } else { l.year - 1 });
    let mut any = false;
    for aw in l.awards.iter().filter(|x| x.season == year) {
        any = true;
        let names: Vec<String> = aw.winners.iter().take(15).map(|(id, t)| format!("{}{}", pname(l, *id), if aw.winners.len() > 1 { format!(" [{}]", t + 1) } else { String::new() })).collect();
        println!("  {:<30} {}", aw.award, names.join(", "));
    }
    if !any {
        println!("No awards recorded for {year} yet.");
    }
}

fn cmd_history(l: &League) {
    println!("{} {} {} {} {}", pad("Season", 7), pad("Champion", 26), pad("Runner-up", 26), pad("MVP", 20), "Finals MVP");
    for h in l.history.iter().rev().take(25).rev() {
        println!("{} {} {} {} {}", pad(&h.season.to_string(), 7), pad(&h.champion, 26), pad(&h.runner_up, 26), pad(&h.mvp, 20), h.finals_mvp);
    }
    if l.history.is_empty() {
        println!("(No completed seasons yet.)");
    }
}

fn cmd_leaders(l: &League) {
    let mut c = l.award_candidates(false);
    c.retain(|x| x.games as f64 >= l.day.min(l.games_this_season as u32) as f64 * 0.4);
    let show = |title: &str, f: &dyn Fn(&hardwood_dynasty::awards::Candidate) -> f64, c: &Vec<hardwood_dynasty::awards::Candidate>| {
        let mut v: Vec<&hardwood_dynasty::awards::Candidate> = c.iter().collect();
        v.sort_by(|a, b| f(b).partial_cmp(&f(a)).unwrap());
        println!("{title}:");
        for x in v.iter().take(5) {
            println!("   {:<22} {:<5} {:.1}", pname(l, x.id), l.team(x.team).abbr, f(x));
        }
    };
    show("Points", &|x| x.ppg, &c);
    show("Rebounds", &|x| x.rpg, &c);
    show("Assists", &|x| x.apg, &c);
    show("Steals", &|x| x.spg, &c);
    show("Blocks", &|x| x.bpg, &c);
}

fn cmd_schedule(l: &League, a: &[&str]) {
    let Some(t) = team_arg(l, a) else { return };
    let mut games: Vec<&Fixture> = l.schedule.iter().filter(|f| f.home == t || f.away == t).collect();
    games.sort_by_key(|f| f.day);
    let upcoming: Vec<&&Fixture> = games.iter().filter(|f| !f.done).take(10).collect();
    println!("Next games for {}:", l.team(t).name());
    for f in upcoming {
        let (opp, at) = if f.home == t { (f.away, "vs") } else { (f.home, "@") };
        println!("  day {:>3}  {at} {}", f.day, l.team(opp).name());
    }
}

fn cmd_draftboard(l: &League) {
    let Some(d) = &l.draft else {
        println!("No draft is underway. The draft happens after the playoffs.");
        return;
    };
    if !d.lottery_text.is_empty() {
        println!("{}", d.lottery_text);
    }
    println!("Draft {} - pick {} of {}.  Prospects shown as YOUR scouts see them (~ = estimate).", d.year, d.next + 1, d.order.len());
    if let Some(s) = d.order.get(d.next) {
        println!("On the clock: {} (pick #{})", l.team(s.team).name(), s.overall);
    }
    println!("{} {} {} {} {} {}  {}", pad("Prospect", 22), pad("Pos", 3), rpad("Age", 3), rpad("Ht", 5), rpad("~OVR", 5), rpad("~POT", 5), "From");
    for (id, o, pt) in l.draft_board(l.user.team, 30) {
        let p = l.p(id);
        println!("{} {} {} {} {} {}  {}", pad(&p.name(), 22), pad(p.position.name(), 3), rpad(&(l.year + 1 - p.birth_year).to_string(), 3), rpad(&height(l, p), 5), rpad(&format!("{o:.0}"), 5), rpad(&format!("{pt:.0}"), 5), p.origin.school);
    }
    hint(l, "Use 'scout <name>' to reduce uncertainty, 'pick <name>' when you're on the clock, or 'autopick'.");
}

fn cmd_fa(l: &League, a: &[&str]) {
    let n = a.first().and_then(|x| x.parse::<usize>().ok()).unwrap_or(20);
    let mut v = l.free_agents.clone();
    v.sort_by(|&x, &y| l.p(y).ovr.cmp(&l.p(x).ovr));
    println!("{} {} {} {} {}  {}", pad("Free agent", 22), pad("Pos", 3), rpad("Age", 3), rpad("OVR", 3), rpad("Asks", 9), "Years");
    for id in v.into_iter().take(n) {
        let p = l.p(id);
        let (ask, yrs) = l.player_ask(id);
        println!("{} {} {} {} {}  {}", pad(&p.name(), 22), pad(p.position.name(), 3), rpad(&l.age_of(id).to_string(), 3), rpad(&rating(l, p.ovr as f64), 3), rpad(&fmt_money(ask), 9), yrs);
    }
    if let Some(t) = l.user.team {
        println!("Your payroll {} / cap {}.  Mid-level exception left: {}.", fmt_money(l.payroll(t)), fmt_money(l.money.cap), fmt_money((l.mle_amount() - l.team(t).exceptions.mle_used).max(0)));
    }
}

fn cmd_sign(l: &mut League, a: &[&str]) {
    if a.len() < 3 {
        println!("Usage: sign <player> <years> <salary>   e.g. sign \"Jalen Booker\" 3 8M");
        return;
    }
    let n = a.len();
    let (Some(sal), Ok(yrs)) = (parse_money(a[n - 1]), a[n - 2].parse::<u8>()) else {
        println!("Usage: sign <player> <years> <salary>");
        return;
    };
    if let Some(id) = l.find_player(&a[..n - 2].join(" ")) {
        match l.user_sign_free_agent(id, yrs, sal) {
            Ok(m) => println!("{m}"),
            Err(e) => println!("{e}"),
        }
    } else {
        println!("No such player.");
    }
}

fn cmd_resign(l: &mut League, a: &[&str]) {
    if a.len() < 3 {
        println!("Usage: resign <player> <years> <salary>");
        return;
    }
    let n = a.len();
    let (Some(sal), Ok(yrs)) = (parse_money(a[n - 1]), a[n - 2].parse::<u8>()) else {
        println!("Usage: resign <player> <years> <salary>");
        return;
    };
    if let Some(id) = l.find_player(&a[..n - 2].join(" ")) {
        match l.user_resign(id, yrs, sal) {
            Ok(m) => println!("{m}"),
            Err(e) => println!("{e}"),
        }
    }
}

fn parse_assets(l: &League, from: TeamId, words: &[&str]) -> Result<Vec<Asset>, String> {
    // "Name One, Name Two, 2027r1" separated by commas; picks as <year>r<round> (own pick) or <year>r<round>:<TEAM>
    let mut out = vec![];
    for part in words.join(" ").split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((y, rest)) = part.split_once('r') {
            if let Ok(year) = y.parse::<i32>() {
                let (round_s, team_s) = rest.split_once(':').unwrap_or((rest, ""));
                let round: u8 = round_s.parse().map_err(|_| format!("Bad pick '{part}'"))?;
                let original = if team_s.is_empty() { from } else { l.find_team(team_s).ok_or(format!("No team '{team_s}'"))? };
                out.push(Asset::Pick { year, round, original });
                continue;
            }
        }
        let id = l.find_player(part).ok_or(format!("No player matches '{part}'."))?;
        out.push(Asset::Player(id));
    }
    Ok(out)
}

fn cmd_trade(l: &mut League, a: &[&str]) {
    let Some(me) = need_team(l) else { return };
    if !l.user_controls_roster(me) {
        println!("Only a GM or Owner makes trades.");
        return;
    }
    let confirm = a.last() == Some(&"confirm");
    let a: Vec<&str> = if confirm { a[..a.len() - 1].to_vec() } else { a.to_vec() };
    let (Some(gi), Some(ti)) = (a.iter().position(|x| *x == "give"), a.iter().position(|x| *x == "get")) else {
        println!("Usage: trade <team> give <player, player, 2027r1> get <player, ...> [confirm]\n  Picks look like 2027r1 (your own 2027 first-rounder) or 2027r2:BOS.");
        return;
    };
    if gi == 0 || ti < gi {
        println!("Usage: trade <team> give <...> get <...>");
        return;
    }
    let Some(other) = l.find_team(&a[..gi].join(" ")) else {
        println!("No team '{}'.", a[..gi].join(" "));
        return;
    };
    let give = match parse_assets(l, me, &a[gi + 1..ti]) {
        Ok(v) => v,
        Err(e) => return println!("{e}"),
    };
    let get = match parse_assets(l, other, &a[ti + 1..]) {
        Ok(v) => v,
        Err(e) => return println!("{e}"),
    };
    let tr = TradeProposal { from: me, to: other, give, get };
    let ev = l.evaluate_trade(&tr);
    println!("You send: {}", tr.give.iter().map(|x| l.asset_label(x)).collect::<Vec<_>>().join("; "));
    println!("You get : {}", tr.get.iter().map(|x| l.asset_label(x)).collect::<Vec<_>>().join("; "));
    for e in &ev.errors {
        println!("  RULE: {e}");
    }
    println!("{}", ev.message);
    if !ev.accepted && ev.legal && l.settings.bool("difficulty.trade_assist") {
        let add = l.suggest_additions(&tr);
        if !add.is_empty() {
            println!("  Trade assistant: they'd likely accept if you also add: {}", add.iter().map(|x| l.asset_label(x)).collect::<Vec<_>>().join("; "));
        }
    }
    if ev.accepted {
        if confirm {
            l.execute_trade(&tr);
            println!("TRADE COMPLETE.");
        } else {
            println!("(Add the word 'confirm' to the end to make the trade.)");
        }
    }
}

fn cmd_people(l: &League, a: &[&str]) {
    let role = a.first().and_then(|r| staff_role(r));
    let mut v: Vec<&Person> = l.people.iter().filter(|p| p.team.is_none() && !p.retired && role.map(|r| p.role == r).unwrap_or(true)).collect();
    v.sort_by(|x, y| y.overall().partial_cmp(&x.overall()).unwrap());
    println!("Available staff (id, role, rating):");
    for p in v.into_iter().take(15) {
        println!("  #{:<5} {:<22} {:<16} {:>3.0}  asks {}", p.id, p.name, p.role.name(), p.overall(), fmt_money(p.salary));
    }
    if let Some(t) = l.user.team {
        let tm = l.team(t);
        println!("\nYour staff:");
        let mut ids: Vec<PersonId> = vec![];
        ids.extend(tm.gm);
        ids.extend(tm.head_coach);
        ids.extend(tm.assistants.iter());
        ids.extend(tm.scouts.iter());
        ids.extend(tm.trainer);
        for id in ids {
            let p = l.person(id);
            println!("  #{:<5} {:<22} {:<16} {:>3.0}", p.id, p.name, p.role.name(), p.overall());
        }
    }
}

fn cmd_money(l: &League) {
    let Some(t) = need_team(l) else { return };
    let tm = l.team(t);
    let payroll = l.payroll(t);
    println!("MONEY DASHBOARD - {}", tm.name());
    let f = &tm.finance;
    let last = tm.finance_history.last();
    let health = |margin: f64| if margin > 0.06 { "GREEN (healthy)" } else if margin > 0.0 { "YELLOW (thin)" } else { "RED (losing money)" };
    println!("  Payroll now     {:>10}   cap {}   tax line {}   1st apron {}   2nd apron {}", fmt_money(payroll), fmt_money(l.money.cap), if l.rules.luxury_tax { fmt_money(l.money.tax_line) } else { "n/a".into() }, if l.rules.aprons { fmt_money(l.money.first_apron) } else { "n/a".into() }, if l.rules.aprons { fmt_money(l.money.second_apron) } else { "n/a".into() });
    println!("  Budget target   {:>10}   tax tolerance {}", fmt_money(tm.budget.payroll_target), fmt_money(tm.budget.tax_tolerance));
    let tax = if l.rules.luxury_tax { l.content.economy.luxury_tax(&l.money, payroll, tm.tax_years >= 3) } else { 0 };
    if tax > 0 {
        println!("  LUXURY TAX if the season ended now: {}{}", fmt_money(tax), if tm.tax_years >= 3 { " (repeater rates!)" } else { "" });
    }
    println!("  This season so far: gate {}  concessions {}  playoffs {}  (home games {}, avg crowd {})", fmt_money(f.gate), fmt_money(f.concessions), fmt_money(f.playoffs), f.home_games, if f.home_games > 0 { f.attendance_total / f.home_games as u64 } else { 0 });
    if let Some(h) = last {
        println!("  Last season ({}): revenue {} | expenses {} | profit {}  -> {}", h.season, fmt_money(h.revenue()), fmt_money(h.expenses()), fmt_money(h.profit()), health(h.profit() as f64 / h.revenue().max(1) as f64));
        println!("     revenue: gate {} conc {} local TV {} national TV {} merch {} sponsors {} playoffs {} sharing {}", fmt_money(h.gate), fmt_money(h.concessions), fmt_money(h.local_tv), fmt_money(h.national_tv), fmt_money(h.merchandise), fmt_money(h.sponsors), fmt_money(h.playoffs), fmt_money(h.revenue_sharing));
        println!("     costs:   payroll {} tax {} staff {} facilities {} marketing {} operations {}", fmt_money(h.payroll), fmt_money(h.luxury_tax), fmt_money(h.staff), fmt_money(h.facilities), fmt_money(h.marketing), fmt_money(h.operations));
    } else {
        println!("  (A full financial statement appears after the first season.)");
    }
    println!("  Fans: hype {:.0}/100, loyalty {:.0}/100, arena {} seats (quality {:.0}), ticket level x{:.2}", tm.hype, tm.fan_loyalty, tm.arena.capacity, tm.arena.quality, tm.budget.ticket_price);
    println!("  Owner: {}  approval {:.0}/100  mandate: {}", tm.owner.name, tm.owner.approval, tm.owner.mandate);
    println!("  Title odds: {:.1}%", l.title_odds_for(t, 0.0) * 100.0);
    hint(l, "Owners: 'chase 20M' previews spending 20M more; 'budget ticket 1.2' sets prices; 'budget facilities 1.5' etc.");
}

fn cmd_chase(l: &League, a: &[&str]) {
    let Some(t) = need_team(l) else { return };
    let Some(extra) = a.first().and_then(|x| parse_money(x)) else {
        println!("Usage: chase <extra salary>   e.g. chase 25M   (shows tax bill, profit hit and title-odds gain)");
        return;
    };
    let p = l.chase_projection(t, extra);
    println!("CHASE MODE: adding {} of salary", fmt_money(extra));
    println!("  New payroll {}   (tax line {})", fmt_money(p.new_payroll), if l.rules.luxury_tax { fmt_money(p.tax_line) } else { "none".into() });
    println!("  Luxury tax: {} -> {}  (+{}){}", fmt_money(p.luxury_tax_before), fmt_money(p.luxury_tax_after), fmt_money(p.extra_tax), if p.repeater { "  [repeater rates]" } else { "" });
    println!("  TOTAL extra cost: {}", fmt_money(p.total_extra_cost));
    println!("  Projected profit: {} -> {}", fmt_money(p.profit_before), fmt_money(p.profit_after));
    println!("  Title odds: {:.1}% -> {:.1}%  (about {:.1} points of odds per $10M)", p.title_odds_before * 100.0, p.title_odds_after * 100.0, (p.title_odds_after - p.title_odds_before) * 100.0 / (extra as f64 / 10_000_000.0).max(0.01));
    if !p.apron_warning.is_empty() {
        println!("  WARNING: {}", p.apron_warning);
    }
    if !l.user.has(Role::Owner) {
        println!("  (You're not the owner. Ask them... or buy the team! Only owners can approve this spending.)");
    }
}

fn cmd_role(l: &mut League, a: &[&str]) {
    if a.len() < 2 {
        println!("Usage: role <gm|coach|owner|scout|assistant> <team>   or   role <college_coach|ad|college_scout> <school>");
        return;
    }
    let Some(role) = Role::parse(a[0]) else {
        println!("Unknown role '{}'.", a[0]);
        return;
    };
    let q = a[1..].join(" ");
    let res = match role {
        Role::CollegeCoach | Role::CollegeAd | Role::CollegeScout => {
            let ql = q.to_lowercase();
            let c = l.colleges.iter().find(|c| c.name.to_lowercase().contains(&ql) || c.nickname.to_lowercase() == ql).map(|c| c.id);
            match c {
                Some(c) => l.take_role(role, None, Some(c)),
                None => Err(format!("No college matches '{q}'. Try 'standings college'.")),
            }
        }
        _ => match l.find_team(&q) {
            Some(t) => l.take_role(role, Some(t), None),
            None => Err(format!("No team matches '{q}'. See 'league'.")),
        },
    };
    match res {
        Ok(m) => {
            println!("{m}");
            if let Some(t) = l.user.team {
                println!("Owner's mandate: {}", l.team(t).owner.mandate);
            }
        }
        Err(e) => println!("{e}"),
    }
}

fn cmd_create_player(l: &mut League, a: &[&str]) {
    // key=value arguments, all optional
    let mut spec = CreatePlayerSpec::default();
    if a.is_empty() {
        println!(
            "CREATE A PLAYER
  create-player name=\"Alex Rivers\" pos=SG height=77 arch=two_way_wing talent=3 start=hs9 country=USA work=70 points=three:20,mid:15,athletic:10

  name      first and last name             pos     PG SG SF PF C
  height    inches (77 = 6'5\")              talent  1 (role player) .. 5 (generational)
  arch      archetype id (see below)        start   hs9 hs10 hs11 hs12 college overseas pro
  points    extra skill points by family: inside, mid, three, playmaking, perimeter_d, interior_d, rebounding, athletic, mental
            budget by talent: 1=40  2=50  3=60  4=70  5=80
  work      work ethic 1-100 (how fast you improve)
Archetypes: {}
Defaults are used for anything you leave out. Run again with your choices.",
            l.content.archetypes.iter().map(|x| x.id.clone()).collect::<Vec<_>>().join(", ")
        );
        return;
    }
    for kv in a {
        let Some((k, v)) = kv.split_once('=') else { continue };
        match k {
            "name" => {
                let mut it = v.split_whitespace();
                spec.first = it.next().unwrap_or("Alex").into();
                spec.last = it.collect::<Vec<_>>().join(" ");
                if spec.last.is_empty() {
                    spec.last = "Rivers".into();
                }
            }
            "pos" => spec.position = Position::parse(v).unwrap_or(Position::SG),
            "height" => spec.height_in = v.parse().unwrap_or(77),
            "arch" | "archetype" => spec.archetype = v.into(),
            "talent" => spec.talent = v.parse().unwrap_or(3),
            "start" => spec.start = v.into(),
            "country" => spec.country = v.into(),
            "town" | "hometown" => spec.hometown = v.replace('_', " "),
            "work" => spec.work_ethic = v.parse().unwrap_or(70),
            "points" => {
                for part in v.split(',') {
                    if let Some((f, n)) = part.split_once(':') {
                        spec.points.push((f.to_string(), n.parse().unwrap_or(0)));
                    }
                }
            }
            _ => println!("(ignoring unknown option '{k}')"),
        }
    }
    // If the user did not name their name, keep default. Make the family name unique-ish.
    match l.create_player(&spec) {
        Ok(id) => {
            let p = l.p(id);
            println!("Created {} ({}, {}, overall {}, potential {}): {}.", p.name(), p.position.name(), p.height_str(), p.ovr, if l.settings.bool("progression.hidden_potential") { "?".to_string() } else { p.potential.to_string() }, talent_label(spec.talent));
            println!("You're a {} now. Use 'life' to see your world, 'allocate ...' to split your time, 'advance month' to live a month.", match spec.start.as_str() { "college" => "college freshman", "pro" => "pro rookie", "overseas" => "pro overseas", _ => "high schooler" });
        }
        Err(e) => println!("Couldn't create the player: {e}"),
    }
}

fn cmd_life(l: &League) {
    let Some(pid) = l.user.player else {
        println!("You have no player career. Start one with: create-player");
        return;
    };
    let p = l.p(pid);
    let Some(life) = &p.life else {
        println!("The life sim is off for this player.");
        return;
    };
    println!("{} - {} (age {}), {}{}", p.name(), life.stage.label(), l.age_of(pid), life.school, if matches!(life.stage, hardwood_dynasty::life::LifeStage::HighSchool | hardwood_dynasty::life::LifeStage::College) { format!(", class {}", life.class_year) } else { String::new() });
    if !life.eligible {
        println!("*** ACADEMICALLY INELIGIBLE: raise your grades to play! ***");
    }
    println!("Basketball: overall {}  potential {}  mood {:.0}  fitness {:.0}", rating(l, p.ovr as f64), if p.user_controlled { rating(l, p.potential as f64) } else { "?".into() }, p.mood.overall, p.fitness);
    println!("\nLIFE STATS");
    for sd in &l.content.life.stats {
        if sd.id.starts_with("dev_") {
            continue;
        }
        let v = life.stat(&sd.id);
        let bar = "#".repeat(((v - sd.min) / (sd.max - sd.min).max(1.0) * 20.0) as usize);
        println!("  {:<16} {:>6.1}  {:<20} {}", sd.name, v, bar, sd.description);
    }
    println!("\nTIME BUDGET (per month)");
    for a in &l.content.life.activities {
        println!("  {:<12} {:>4.0}%   {}", a.id, life.allocation.get(&a.id).copied().unwrap_or(0.0), a.description);
    }
    let f = &life.finance;
    println!("\nMONEY: cash {}  invested {}  assets {}  debt {}  net worth {}  endorsements {}/yr  (taxes paid this year {})", fmt_money(f.cash), fmt_money(f.investments), fmt_money(f.assets), fmt_money(f.debt), fmt_money(f.net_worth()), fmt_money(f.endorsements), fmt_money(f.taxes_this_year));
    if let Some(ag) = &life.agent {
        println!("Agent: {} (skill {:.0}, fee {}%)", ag.name, ag.skill, ag.fee_pct);
    }
    println!("\nPEOPLE");
    for r in &life.relationships {
        println!("  {:<22} {:<10} closeness {:>3.0}  {}", r.name, r.kind, r.closeness, r.note);
    }
    println!("\nRECENT");
    for e in life.timeline.iter().rev().take(8).rev() {
        println!("  {} (age {}): {}", e.season, e.age, e.text);
    }
}

fn cmd_college(l: &League, a: &[&str]) {
    match a.first().copied() {
        Some("recruits") => {
            let mut v: Vec<&Player> = l.players.iter().filter(|p| p.affiliation == Affiliation::HighSchool && p.custom.get("class").copied().unwrap_or(0.0) as u8 == 12 && !p.user_controlled).collect();
            v.sort_by(|x, y| x.custom.get("rank").copied().unwrap_or(999.0).partial_cmp(&y.custom.get("rank").copied().unwrap_or(999.0)).unwrap());
            println!("Top high-school seniors (rank, name, position, height, estimated overall/potential):");
            for p in v.into_iter().take(25) {
                let (o, pt, _) = l.scouted_view(l.user.team, p.id);
                println!("  #{:<4} {:<22} {:<3} {:<5} ~{:.0}/~{:.0}  {}", p.custom.get("rank").copied().unwrap_or(0.0), p.name(), p.position.name(), p.height_str(), o, pt, p.origin.school);
            }
        }
        _ => {
            let Some(c) = l.user.college.or_else(|| l.standings_college().first().copied()) else {
                println!("No colleges in this league.");
                return;
            };
            let co = &l.colleges[c as usize];
            println!("{} {}  {}-{}  prestige {:.0}  facilities {:.0}  coach {} (recruiting {:.0})", co.name, co.nickname, co.record.w, co.record.l, co.prestige, co.facilities, co.coach.name, co.coach.recruiting);
            if l.nil_available() {
                println!("NIL budget {} (spent {})", fmt_money(co.nil_budget), fmt_money(co.nil_spent));
            }
            println!("Recruiting points: {:.0}.  ('college recruits' lists the class; 'recruit <name> <points>' makes an offer.)", co.recruiting_points);
            cmd_college_roster(l, &[]);
        }
    }
}

fn cmd_college_roster(l: &League, a: &[&str]) {
    let c = if a.is_empty() { l.user.college } else { l.colleges.iter().find(|c| c.name.to_lowercase().contains(&a.join(" ").to_lowercase())).map(|c| c.id) };
    let Some(c) = c else {
        println!("Which school? e.g. roster college Northern Plains");
        return;
    };
    let co = &l.colleges[c as usize];
    println!("{} roster:", co.name);
    let mut ids = co.roster.clone();
    ids.sort_by(|&a, &b| l.p(b).ovr.cmp(&l.p(a).ovr));
    for id in ids {
        let p = l.p(id);
        let st = p.seasons.iter().rev().find(|r| r.season == l.year && r.level == Level::College);
        println!("  {:<22} {:<3} class {}  ovr {}  {}", p.name(), p.position.name(), hardwood_dynasty::college::class_of(p), rating(l, p.ovr as f64), st.map(|r| format!("{:.1} ppg {:.1} rpg {:.1} apg", r.stats.ppg(), r.stats.rpg(), r.stats.apg())).unwrap_or_default());
    }
}

fn cmd_clubs(l: &League) {
    println!("Overseas pro clubs (EuroLeague-level clubs marked *):");
    let mut v: Vec<&hardwood_dynasty::overseas::Club> = l.clubs.iter().collect();
    v.sort_by(|a, b| a.tier.cmp(&b.tier).then(a.country.cmp(&b.country)));
    for c in v.into_iter().take(40) {
        println!("  {}{:<28} {:<4} tier {}  {}-{}  titles {}  EuroLeague titles {}", if c.tier == 1 { "*" } else { " " }, c.name, c.country, c.tier, c.record.w, c.record.l, c.titles.len(), c.euro_titles.len());
    }
}

fn cmd_import(l: &mut League, a: &[&str]) {
    if a.len() < 2 {
        println!("Usage: import roster <file.csv|file.json> [replace]   |   import pack <file.json> (starts a NEW league from a year pack)");
        return;
    }
    let replace = a.contains(&"replace");
    match std::fs::read_to_string(a[1]) {
        Ok(text) => match a[0] {
            "roster" => {
                let res = if a[1].to_lowercase().ends_with(".json") { l.import_roster_json(&text, replace) } else { l.import_roster_csv(&text, replace) };
                match res {
                    Ok(r) => println!("{}", r.summary()),
                    Err(e) => println!("{e}"),
                }
            }
            "pack" => println!("To start a new league from a year pack, use: newpack {}", a[1]),
            _ => println!("Import what? roster or pack."),
        },
        Err(e) => println!("Can't read {}: {e}", a[1]),
    }
}

fn cmd_export(l: &League, a: &[&str]) {
    if a.len() < 2 {
        println!("Usage: export roster <file.csv>   |   export pack <file.json>");
        return;
    }
    let res = match a[0] {
        "roster" => std::fs::write(a[1], l.export_rosters_csv()),
        "pack" => std::fs::write(a[1], serde_json::to_string_pretty(&l.export_year_pack(&l.name)).unwrap_or_default()),
        _ => return println!("Export what? roster or pack."),
    };
    match res {
        Ok(()) => println!("Wrote {}.", a[1]),
        Err(e) => println!("Couldn't write: {e}"),
    }
}

// ---------------------------------------------------------------------------------------
// headless tools
// ---------------------------------------------------------------------------------------

fn headless_sim(args: &[String]) {
    let mut year = 1996;
    let mut years = 5;
    let mut seed = "headless".to_string();
    let mut preset = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--year" => {
                year = args.get(i + 1).and_then(|x| x.parse().ok()).unwrap_or(year);
                i += 1;
            }
            "--years" => {
                years = args.get(i + 1).and_then(|x| x.parse().ok()).unwrap_or(years);
                i += 1;
            }
            "--seed" => {
                seed = args.get(i + 1).cloned().unwrap_or(seed);
                i += 1;
            }
            "--preset" => {
                preset = args.get(i + 1).cloned();
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    let t0 = std::time::Instant::now();
    let mut l = match League::new(NewLeagueOptions { year, seed, preset, ..Default::default() }) {
        Ok(l) => l,
        Err(e) => return println!("{e}"),
    };
    let rep = l.advance(Goal::Years(years));
    if let Some(s) = rep.stopped {
        println!("stopped: {s}");
    }
    println!("{:<7} {:<3} {:<26} {:<20} {:>6} {:>8}", "Season", "Tms", "Champion", "MVP", "ppg", "Cap");
    for h in &l.history {
        println!("{:<7} {:<3} {:<26} {:<20} {:>6.1} {:>8}", h.season, h.teams, h.champion, h.mvp, h.avg_ppg, fmt_money(h.cap));
    }
    println!("Simulated {} seasons in {:.1}s ({} players created).", l.history.len(), t0.elapsed().as_secs_f64(), l.players.len());
}

fn calibrate(args: &[String]) {
    use hardwood_dynasty::game::*;
    use hardwood_dynasty::generate::*;
    use hardwood_dynasty::rng::Rng;
    let content = Content::default();
    let years: Vec<i32> = args.iter().filter_map(|a| a.parse().ok()).collect();
    for season in if years.is_empty() { vec![1950, 1985, 2024] } else { years } {
        let mut rng = Rng::new(season as u64);
        let teams: Vec<GameTeam> = (0..30)
            .map(|id| {
                let mut players = vec![];
                for i in 0..13u32 {
                    let target = rng.gauss(if i < 5 { 66.0 } else if i < 9 { 56.0 } else { 48.0 }, 5.0).clamp(35.0, 92.0);
                    let spec = GenSpec::new(season, 26, target, target, OriginKind::College);
                    let p = generate_player(&content, &mut rng, id * 100 + i, &spec, 1.0);
                    players.push(GamePlayer::from_player(&p, &content.badges, 1.0, 0.0));
                }
                assign_rotation(&mut players, false, &content.rules(season));
                GameTeam { id: id as u16, name: format!("T{id}"), players, strategy: Strategy::default() }
            })
            .collect();
        let refs = Refs::from_players(teams.iter().flat_map(|t| t.players.iter()));
        let mut ctx = GameContext {
            refs,
            cal: Cal::default(),
            season,
            rules: content.rules(season),
            style: content.style(season),
            tune: Tune::from_map(&content.tuning),
            home_court: 2.5,
            randomness: 1.0,
            star_power: 1.0,
            fatigue: 1.0,
            foul_rate: 1.0,
            hot_hand: true,
            clutch: true,
            possession_detail: 1.0,
            in_game_injuries: false,
            playoffs: false,
            neutral_site: false,
            play_by_play: false,
        };
        calibrate_ctx(&mut ctx, &teams, &mut rng, season, &content);
    }
}

fn calibrate_ctx(ctx: &mut hardwood_dynasty::game::GameContext, teams: &[hardwood_dynasty::game::GameTeam], rng: &mut hardwood_dynasty::rng::Rng, season: i32, content: &Content) {
    use hardwood_dynasty::game::*;
    calibrate(ctx, teams, rng, 150, 3);
    let mut m = LeagueMeasure::default();
    for g in 0..600 {
        let a = (g * 7) % 30;
        let b = (a + 1 + (g * 3) % 29) % 30;
        m.add(&simulate_game(ctx, &teams[a], &teams[b], rng));
    }
    let st = content.style(season);
    println!("{season}: ppg {:.1} (target {:.1}) | pace {:.1} ({:.1}) | 3PAr {:.3} ({:.3}) | fg2 {:.3} ({:.3}) | fg3 {:.3} ({:.3}) | FT rate {:.3} ({:.3}) | TOV {:.3} ({:.3})", m.ppg(), st.ppg, m.pace(), st.pace, m.three_rate(), st.three_rate, m.fg2(), st.fg2, m.fg3(), st.fg3, m.ft_rate(), st.ft_rate, m.tov_rate(), st.tov);
}

/// Write reference docs generated from the real data, so they can never drift from the game.
fn gen_docs(dir: &str) {
    use hardwood_dynasty::settings::*;
    let c = Content::default();
    let _ = std::fs::create_dir_all(dir);
    let mut s = String::from("# Settings reference\n\n_Generated by `hwd gen-docs` from the game's own settings registry. Do not edit by hand._\n\nIn the game: `settings` lists them, `settings explain <key>` explains one, `settings set <key> <value>` changes one, and `settings preset <id>` applies a bundle.\n\n");
    s += "## Presets\n\n";
    for p in &c.presets {
        s += &format!("- **{}** (`{}`): {}\n", p.name, p.id, p.description);
    }
    for (cat, name) in CATEGORIES {
        s += &format!("\n## {name}\n\n| Setting | What it does | Low / Off | High / On | Default |\n|---|---|---|---|---|\n");
        for d in c.settings.iter().filter(|d| d.category() == *cat) {
            let (lo, hi) = match &d.kind {
                SettingKind::Choice { options } => (String::new(), options.iter().map(|o| format!("`{}`: {}", o.id, o.explain)).collect::<Vec<_>>().join("<br>")),
                SettingKind::Slider { min, max, .. } => (format!("{min}: {}", d.low_note), format!("{max}: {}", d.high_note)),
                SettingKind::Toggle => (d.low_note.clone(), d.high_note.clone()),
            };
            s += &format!("| `{}`{} | {} | {} | {} | {} |\n", d.key, if d.advanced { " (advanced)" } else { "" }, d.description, lo, hi, d.default.show());
        }
    }
    let _ = std::fs::write(format!("{dir}/SETTINGS.md"), s);

    let mut t = String::from("# Engine tuning parameters\n\n_Generated by `hwd gen-docs`._ Set any of these in a mod's `\"tuning\"` section.\n\n| Key | Default | Range | Meaning |\n|---|---|---|---|\n");
    for p in hardwood_dynasty::game::default_tuning() {
        t += &format!("| `{}` | {} | {} to {} | {} |\n", p.key, p.default, p.min, p.max, p.description);
    }
    let _ = std::fs::write(format!("{dir}/TUNING.md"), t);

    let mut r = String::from("# Rule changes timeline\n\n_Generated by `hwd gen-docs`._ Each entry is a patch to the rule book; mods can add, replace or remove entries (`rule_changes`).\n\n| Season | Id | Change |\n|---|---|---|\n");
    let mut tl = c.rule_timeline.clone();
    tl.sort_by_key(|x| x.year);
    for x in tl {
        r += &format!("| {} | `{}` | **{}**: {} |\n", x.year, x.id, x.title, x.description);
    }
    r += "\n## Franchise history\n\n| Franchise | Seasons | Identities |\n|---|---|---|\n";
    for f in &c.franchises {
        let ids: Vec<String> = f.eras.iter().map(|(y, i)| format!("{y}: {} {}", i.city, i.nickname)).collect();
        r += &format!("| `{}` | {}-{} | {} |\n", f.key, f.first_year(), f.last_year.map(|y| y.to_string()).unwrap_or_else(|| "now".into()), ids.join("; "));
    }
    let _ = std::fs::write(format!("{dir}/RULES_AND_HISTORY.md"), r);

    let mut ct = String::from("# Content reference (what mods can change)\n\n_Generated by `hwd gen-docs`._\n\n");
    ct += "## Attributes\n\n| Key | Name | Family |\n|---|---|---|\n";
    for a in Attr::ALL {
        ct += &format!("| `{}` | {} | `{}` |\n", a.key(), a.name(), a.family().key());
    }
    ct += "\n## Archetypes\n\n";
    for a in &c.archetypes {
        ct += &format!("- `{}` **{}**: {}\n", a.id, a.name, a.description);
    }
    ct += "\n## Badges\n\n";
    for b in &c.badges {
        ct += &format!("- `{}` **{}** ({}): {} Requires: {}\n", b.id, b.name, b.category, b.description, b.requires.iter().map(|(k, v)| format!("{k} {v}+")).collect::<Vec<_>>().join(", "));
    }
    ct += "\n## Injuries\n\n| Id | Injury | Severity | Median games | Career-ending chance |\n|---|---|---|---|---|\n";
    for i in &c.injuries {
        ct += &format!("| `{}` | {} | {} | {} | {:.1}% |\n", i.id, i.name, i.severity.name(), i.median_games, i.career_ending * 100.0);
    }
    ct += "\n## Awards\n\n";
    for a in &c.awards {
        ct += &format!("- `{}` **{}** (from {}): {:?}\n", a.id, a.name, a.first_year, a.metric);
    }
    ct += "\n## Life stats\n\n";
    for s in &c.life.stats {
        ct += &format!("- `{}` **{}** ({}-{}): {}\n", s.id, s.name, s.min, s.max, s.description);
    }
    ct += "\n## Life activities (monthly time budget)\n\n";
    for a in &c.life.activities {
        ct += &format!("- `{}` **{}**: {} Effects: {:?}\n", a.id, a.name, a.description, a.effects);
    }
    ct += &format!("\n## Life events ({})\n\n", c.life.events.len());
    for e in &c.life.events {
        ct += &format!("- `{}` **{}** [{}] chance {:.1}%/month{}\n", e.id, e.title, e.stages.join(","), e.chance * 100.0, if e.choices.is_empty() { String::new() } else { format!(", {} choices", e.choices.len()) });
    }
    let _ = std::fs::write(format!("{dir}/CONTENT.md"), ct);
    println!("Wrote SETTINGS.md, TUNING.md, RULES_AND_HISTORY.md and CONTENT.md to {dir}/.");
}
