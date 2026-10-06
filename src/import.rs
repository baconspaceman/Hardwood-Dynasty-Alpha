//! Importing and exporting: custom rosters, whole "year packs", and stat-based ratings.
//!
//! Anyone can make a roster or a year and share it:
//! * a **roster file** (CSV or JSON) lists teams and players (ratings are optional: give just
//!   overall + position + height, or even real per-game stats, and the engine fills in a full
//!   set of attributes);
//! * a **year pack** (JSON) describes a whole season: teams, rosters, a draft class, free agents,
//!   and optional rule/cap overrides.
//!
//! Every problem found is reported in plain English with the line or team it belongs to.

use crate::content::{ListPatch, ModInfo, ModPack};
use crate::contract::*;
use crate::franchise::{Franchise, Identity};
use crate::generate::*;
use crate::league::*;
use crate::player::*;
use crate::rng::Rng;
use crate::setup::NewLeagueOptions;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One player in an import file. Everything except `name` is optional.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerEntry {
    pub name: String,
    pub age: Option<i32>,
    pub position: Option<String>,
    /// Height in inches (or feet-inches like 6'8").
    pub height_in: Option<f64>,
    pub weight_lb: Option<u16>,
    pub overall: Option<f64>,
    pub potential: Option<f64>,
    pub salary: Option<Money>,
    pub years: Option<u8>,
    pub country: Option<String>,
    pub archetype: Option<String>,
    pub college: Option<String>,
    /// Per-game stats used to infer ratings when attributes are not given.
    pub stats: Option<StatsEntry>,
    /// Direct attribute overrides by key (e.g. {"three_point": 88}).
    pub attrs: BTreeMap<String, f64>,
    pub draft_year: Option<i32>,
    pub draft_pick: Option<u16>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StatsEntry {
    pub mpg: Option<f64>,
    pub ppg: Option<f64>,
    pub rpg: Option<f64>,
    pub apg: Option<f64>,
    pub spg: Option<f64>,
    pub bpg: Option<f64>,
    pub fg_pct: Option<f64>,
    pub three_pa: Option<f64>,
    pub three_pct: Option<f64>,
    pub ft_pct: Option<f64>,
    pub fta: Option<f64>,
    pub tov: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TeamEntry {
    pub name: String,
    pub abbr: String,
    pub city: String,
    pub nickname: String,
    /// "East"/"West" or 0/1.
    pub conference: Option<String>,
    pub market: Option<f64>,
    pub lon: Option<f64>,
    pub lat: Option<f64>,
    pub players: Vec<PlayerEntry>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RosterFile {
    pub teams: Vec<TeamEntry>,
    pub free_agents: Vec<PlayerEntry>,
}

/// A complete season, ready to start.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct YearPack {
    pub name: String,
    pub author: String,
    pub description: String,
    pub year: Season,
    /// Salary cap for the year in dollars (otherwise the era table is used).
    pub salary_cap: Option<Money>,
    pub teams: Vec<TeamEntry>,
    pub free_agents: Vec<PlayerEntry>,
    pub draft_class: Vec<PlayerEntry>,
    /// Setting overrides applied to the new league, by key.
    pub settings: BTreeMap<String, crate::settings::SettingValue>,
    /// Extra mod content applied with the pack.
    pub r#mod: Option<ModPack>,
}

#[derive(Clone, Debug, Default)]
pub struct ImportReport {
    pub teams_touched: usize,
    pub players_created: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl ImportReport {
    pub fn summary(&self) -> String {
        let mut s = format!(
            "Imported {} players across {} teams.",
            self.players_created, self.teams_touched
        );
        for w in &self.warnings {
            s += &format!("\n  warning: {w}");
        }
        for e in &self.errors {
            s += &format!("\n  ERROR: {e}");
        }
        s
    }
}

// ------------------------------------------------------------------------------------------
// CSV
// ------------------------------------------------------------------------------------------

/// Split one CSV line respecting double quotes.
pub fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut inq = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if inq && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => inq = !inq,
            ',' if !inq => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn parse_height(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some((f, i)) = s.split_once('\'') {
        let feet: f64 = f.trim().parse().ok()?;
        let inch: f64 = i.trim().trim_end_matches('"').trim().parse().unwrap_or(0.0);
        return Some(feet * 12.0 + inch);
    }
    if let Some((f, i)) = s.split_once('-') {
        if let (Ok(feet), Ok(inch)) = (f.trim().parse::<f64>(), i.trim().parse::<f64>()) {
            if feet <= 8.0 {
                return Some(feet * 12.0 + inch);
            }
        }
    }
    let v: f64 = s.parse().ok()?;
    if v > 120.0 {
        Some(v / 2.54) // centimeters
    } else {
        Some(v)
    }
}

/// Parse a roster CSV into teams. Column names are matched loosely (case-insensitive).
pub fn parse_roster_csv(text: &str) -> Result<(RosterFile, Vec<String>), String> {
    let mut lines = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
    let header = lines.next().ok_or("The CSV file is empty.")?;
    let cols: Vec<String> = split_csv_line(header)
        .into_iter()
        .map(|c| c.to_lowercase().replace([' ', '-'], "_"))
        .collect();
    if !cols.iter().any(|c| c == "name" || c == "player") {
        return Err("The CSV needs a 'name' column in the first row. Example header: team,name,age,position,height_in,overall,potential,salary,years,country".into());
    }
    let mut warnings = vec![];
    let mut file = RosterFile::default();
    for (ln, line) in lines.enumerate() {
        let row = split_csv_line(line);
        let get = |keys: &[&str]| -> Option<String> {
            for k in keys {
                if let Some(i) = cols.iter().position(|c| c == k) {
                    if let Some(v) = row.get(i) {
                        if !v.is_empty() {
                            return Some(v.clone());
                        }
                    }
                }
            }
            None
        };
        let num = |keys: &[&str]| -> Option<f64> {
            get(keys).and_then(|v| v.replace(['$', ','], "").parse::<f64>().ok())
        };
        let name = match get(&["name", "player"]) {
            Some(n) => n,
            None => {
                warnings.push(format!("Row {}: no name, skipped.", ln + 2));
                continue;
            }
        };
        let mut e = PlayerEntry {
            name,
            ..Default::default()
        };
        e.age = num(&["age"]).map(|a| a as i32);
        e.position = get(&["position", "pos"]);
        e.height_in = get(&["height_in", "height", "ht"]).and_then(|h| parse_height(&h));
        e.weight_lb = num(&["weight_lb", "weight", "wt"]).map(|w| w as u16);
        e.overall = num(&["overall", "ovr", "rating"]);
        e.potential = num(&["potential", "pot"]);
        e.salary = num(&["salary", "salary_usd"]).map(|s| s as i64);
        e.years = num(&["years", "contract_years"]).map(|y| y as u8);
        e.country = get(&["country", "nationality"]);
        e.archetype = get(&["archetype"]);
        e.college = get(&["college", "school"]);
        let st = StatsEntry {
            mpg: num(&["mpg", "min", "minutes"]),
            ppg: num(&["ppg", "pts", "points"]),
            rpg: num(&["rpg", "reb", "trb"]),
            apg: num(&["apg", "ast"]),
            spg: num(&["spg", "stl"]),
            bpg: num(&["bpg", "blk"]),
            fg_pct: num(&["fg_pct", "fg%"]),
            three_pa: num(&["three_pa", "3pa", "tpa"]),
            three_pct: num(&["three_pct", "3p%", "3p_pct", "tp_pct"]),
            ft_pct: num(&["ft_pct", "ft%"]),
            fta: num(&["fta"]),
            tov: num(&["tov", "tpg"]),
        };
        if st.ppg.is_some() || st.rpg.is_some() || st.apg.is_some() {
            e.stats = Some(st);
        }
        // direct attribute columns
        for a in Attr::ALL {
            if let Some(v) = num(&[a.key()]) {
                e.attrs.insert(a.key().to_string(), v);
            }
        }
        let team = get(&["team", "club", "franchise"]).unwrap_or_default();
        if team.is_empty()
            || team.eq_ignore_ascii_case("fa")
            || team.eq_ignore_ascii_case("free agent")
        {
            file.free_agents.push(e);
        } else if let Some(t) = file
            .teams
            .iter_mut()
            .find(|t| t.name.eq_ignore_ascii_case(&team) || t.abbr.eq_ignore_ascii_case(&team))
        {
            t.players.push(e);
        } else {
            file.teams.push(TeamEntry {
                name: team.clone(),
                abbr: team.clone(),
                players: vec![e],
                ..Default::default()
            });
        }
    }
    Ok((file, warnings))
}

// ------------------------------------------------------------------------------------------
// Turning entries into players
// ------------------------------------------------------------------------------------------

/// Guess the archetype from a stat line.
fn archetype_from_stats(e: &PlayerEntry, pos: Position, height: f64) -> &'static str {
    let Some(s) = &e.stats else {
        return match pos {
            Position::PG => "floor_general",
            Position::SG => "scoring_guard",
            Position::SF => "two_way_wing",
            Position::PF => "glass_cleaner",
            Position::C => "rim_protector",
        };
    };
    let (ppg, rpg, apg, bpg, spg, t3a) = (
        s.ppg.unwrap_or(8.0),
        s.rpg.unwrap_or(3.0),
        s.apg.unwrap_or(2.0),
        s.bpg.unwrap_or(0.3),
        s.spg.unwrap_or(0.7),
        s.three_pa.unwrap_or(2.0),
    );
    if height >= 82.0 {
        if bpg >= 1.6 {
            return "rim_protector";
        }
        if apg >= 4.5 {
            return "playmaking_big";
        }
        if t3a >= 3.5 {
            return "stretch_big";
        }
        if ppg >= 18.0 {
            return "post_scorer";
        }
        return if rpg >= 9.0 {
            "glass_cleaner"
        } else {
            "athletic_finisher"
        };
    }
    if apg >= 7.0 && height < 79.0 {
        return "floor_general";
    }
    if apg >= 5.5 && height >= 79.0 {
        return "point_forward";
    }
    if t3a >= 6.0 && ppg >= 15.0 {
        return "sharpshooter";
    }
    if spg >= 1.6 {
        return "lockdown_defender";
    }
    if ppg >= 22.0 {
        return "pure_scorer";
    }
    if t3a >= 4.0 && spg >= 1.0 {
        return "three_and_d";
    }
    if rpg >= 8.0 {
        return "glass_cleaner";
    }
    match pos {
        Position::PG => "combo_guard",
        Position::SG => "scoring_guard",
        _ => "two_way_wing",
    }
}

/// Rough overall rating from a stat line when none is given.
fn overall_from_stats(s: &StatsEntry, mpg_default: f64) -> f64 {
    let ppg = s.ppg.unwrap_or(8.0);
    let rpg = s.rpg.unwrap_or(3.0);
    let apg = s.apg.unwrap_or(2.0);
    let stk = s.spg.unwrap_or(0.7) + s.bpg.unwrap_or(0.4);
    let mpg = s.mpg.unwrap_or(mpg_default);
    let value = ppg + 0.7 * rpg + 1.0 * apg + 1.6 * stk - 0.6 * s.tov.unwrap_or(1.8);
    (42.0 + value * 1.05 + (mpg - 24.0) * 0.25).clamp(35.0, 97.0)
}

impl League {
    /// Build a Player from an import entry, scaled to this league's era.
    pub fn player_from_entry(
        &mut self,
        e: &PlayerEntry,
        rng: &mut Rng,
        report: &mut ImportReport,
    ) -> Player {
        let age = e
            .age
            .unwrap_or_else(|| rng.range(22, 31) as i32)
            .clamp(16, 45);
        let height = e.height_in.unwrap_or(78.0).clamp(60.0, 94.0);
        let pos = e
            .position
            .as_deref()
            .and_then(Position::parse)
            .unwrap_or_else(|| {
                if e.position.is_some() {
                    report.warnings.push(format!(
                        "{}: unknown position '{}', guessed from height.",
                        e.name,
                        e.position.as_deref().unwrap_or("")
                    ));
                }
                if height >= 82.0 {
                    Position::C
                } else if height >= 80.0 {
                    Position::PF
                } else if height >= 78.0 {
                    Position::SF
                } else if height >= 75.0 {
                    Position::SG
                } else {
                    Position::PG
                }
            });
        let ovr = e
            .overall
            .unwrap_or_else(|| {
                e.stats
                    .as_ref()
                    .map(|s| overall_from_stats(s, 28.0))
                    .unwrap_or(48.0)
            })
            .clamp(25.0, 99.0);
        let pot = e
            .potential
            .unwrap_or(if age < 25 {
                ovr + (25 - age) as f64 * 1.8
            } else {
                ovr
            })
            .clamp(ovr, 99.0);
        let arch = e
            .archetype
            .clone()
            .unwrap_or_else(|| archetype_from_stats(e, pos, height).to_string());
        let arch = if self.content.archetypes.iter().any(|a| a.id == arch) {
            arch
        } else {
            report.warnings.push(format!(
                "{}: unknown archetype '{}', used a default.",
                e.name, arch
            ));
            archetype_from_stats(e, pos, height).to_string()
        };
        let mut spec = GenSpec::new(self.year, age, ovr, pot, OriginKind::College);
        spec.position = Some(pos);
        spec.height_in = Some(height.round() as u8);
        spec.archetype = Some(arch);
        spec.country = e
            .country
            .clone()
            .filter(|c| self.content.countries.iter().any(|x| &x.code == c) || c == "USA");
        let mut p = generate_player(&self.content, rng, 0, &spec, 1.0);
        // names
        let mut parts = e.name.split_whitespace();
        p.first = parts.next().unwrap_or("Unknown").to_string();
        p.last = parts.collect::<Vec<_>>().join(" ");
        if p.last.is_empty() {
            p.last = p.first.clone();
            p.first = "J.".into();
        }
        if let Some(w) = e.weight_lb {
            p.weight_lb = w;
        }
        if let Some(c) = &e.college {
            p.origin.school = c.clone();
        }
        if let Some(c) = &e.country {
            p.country = c.clone();
        }
        // stat-based nudges
        if let Some(s) = &e.stats {
            let mut nudge = |a: Attr, d: f64| p.attrs.add(a, d);
            if let (Some(pct), Some(att)) = (s.three_pct, s.three_pa) {
                if att >= 1.0 {
                    nudge(
                        Attr::ThreePoint,
                        (pct * if pct <= 1.0 { 100.0 } else { 1.0 } - 35.0) * 1.4
                            + att.min(8.0) * 1.0,
                    );
                }
            }
            if let Some(f) = s.ft_pct {
                let f = if f <= 1.0 { f * 100.0 } else { f };
                nudge(Attr::FreeThrow, (f - 75.0) * 1.2);
            }
            if let Some(a) = s.apg {
                nudge(Attr::PassAccuracy, (a - 3.0) * 2.2);
                nudge(Attr::PassVision, (a - 3.0) * 2.2);
                nudge(Attr::PassIq, (a - 3.0) * 1.6);
            }
            if let Some(r) = s.rpg {
                nudge(Attr::DefRebound, (r - 5.0) * 1.8);
                nudge(Attr::OffRebound, (r - 5.0) * 1.4);
            }
            if let Some(b) = s.bpg {
                nudge(Attr::Block, (b - 0.7) * 9.0);
                nudge(Attr::InteriorDef, (b - 0.7) * 3.5);
            }
            if let Some(sp) = s.spg {
                nudge(Attr::Steal, (sp - 1.0) * 14.0);
                nudge(Attr::PerimeterDef, (sp - 1.0) * 4.0);
            }
            if let Some(t) = s.tov {
                nudge(Attr::Hands, -(t - 2.0) * 2.5);
            }
        }
        // direct attribute overrides win
        for (k, v) in &e.attrs {
            match Attr::from_key(k) {
                Some(a) => p.attrs.set(a, *v),
                None => report
                    .warnings
                    .push(format!("{}: unknown attribute '{k}' ignored.", e.name)),
            }
        }
        p.recompute_ovr();
        // If the file gave an overall and no direct attributes, calibrate to match it.
        if e.attrs.is_empty() {
            for _ in 0..4 {
                let d = ovr - p.ovr as f64;
                if d.abs() < 0.8 {
                    break;
                }
                for a in Attr::ALL {
                    p.attrs.add(*a, d * 0.9);
                }
                p.recompute_ovr();
            }
        }
        p.potential = (pot as u8).max(p.ovr);
        p.badges = earned_badges(&p.attrs, &self.content.badges, 6);
        p.years_pro = (age - 20).max(0) as u8;
        p
    }

    fn contract_from_entry(&self, p: &Player, e: &PlayerEntry, rng: &mut Rng) -> Contract {
        let years = e.years.unwrap_or_else(|| rng.range(1, 4) as u8).max(1);
        match e.salary {
            Some(s) => Contract::rising(
                s.max(self.money.min_salary / 4),
                years,
                0.03,
                ContractKind::Standard,
                self.year,
            ),
            None => crate::generate::market_contract(&self.content, &self.money, p, self.year, rng),
        }
    }

    fn find_team_entry(&self, te: &TeamEntry) -> Option<TeamId> {
        let keys = [
            te.name.as_str(),
            te.abbr.as_str(),
            te.city.as_str(),
            te.nickname.as_str(),
        ];
        for k in keys {
            if k.is_empty() {
                continue;
            }
            if let Some(t) = self.find_team(k) {
                return Some(t);
            }
        }
        None
    }

    /// Import a roster file into the current league. `replace` swaps the named teams' rosters.
    pub fn import_rosters(&mut self, file: &RosterFile, replace: bool) -> ImportReport {
        let mut rep = ImportReport::default();
        let mut rng = self.rng.fork("import");
        for te in &file.teams {
            let Some(t) = self.find_team_entry(te) else {
                rep.errors.push(format!("Team '{}' doesn't exist in this league (use a city, nickname or abbreviation like BOS). Its {} players were skipped.", te.name, te.players.len()));
                continue;
            };
            rep.teams_touched += 1;
            if replace {
                for id in self.team(t).roster.clone() {
                    self.pm(id).contract = None;
                    self.release_to_fa(id, t, "roster import");
                    // dropped players leave the sport so the pool isn't flooded
                    self.retire_player(id, "replaced by import");
                }
            }
            let max = self.rules.roster_max as usize;
            if te.players.len()
                + if replace {
                    0
                } else {
                    self.team(t).roster.len()
                }
                > max
            {
                rep.warnings.push(format!("{}: {} players exceeds the roster limit of {max} for {}; extras go to free agency.", self.team(t).name(), te.players.len(), self.year));
            }
            for e in &te.players {
                let mut p = self.player_from_entry(e, &mut rng, &mut rep);
                let c = self.contract_from_entry(&p, e, &mut rng);
                p.contract = Some(c);
                let id = self.add_player(p);
                if self.team(t).roster.len() < max {
                    self.pm(id).affiliation = Affiliation::Nba(t);
                    self.team_mut(t).roster.push(id);
                    let tn = self.team(t).name();
                    let y = self.year;
                    self.ensure_record(id, t, &tn, y);
                } else {
                    self.pm(id).affiliation = Affiliation::FreeAgent;
                    self.pm(id).contract = None;
                    self.free_agents.push(id);
                }
                rep.players_created += 1;
            }
        }
        for e in &file.free_agents {
            let mut p = self.player_from_entry(e, &mut rng, &mut rep);
            p.affiliation = Affiliation::FreeAgent;
            let id = self.add_player(p);
            self.free_agents.push(id);
            rep.players_created += 1;
        }
        self.calibrate_engine();
        self.rng = rng;
        rep
    }

    pub fn import_roster_csv(&mut self, text: &str, replace: bool) -> Result<ImportReport, String> {
        let (file, warns) = parse_roster_csv(text)?;
        let mut rep = self.import_rosters(&file, replace);
        rep.warnings.splice(0..0, warns);
        Ok(rep)
    }

    pub fn import_roster_json(
        &mut self,
        text: &str,
        replace: bool,
    ) -> Result<ImportReport, String> {
        let file: RosterFile = serde_json::from_str(text)
            .map_err(|e| format!("Couldn't read the roster JSON: {e} (line {}).", e.line()))?;
        Ok(self.import_rosters(&file, replace))
    }

    /// Create a league whose teams and players come from a year pack.
    pub fn new_from_pack(
        pack: &YearPack,
        opts: &NewLeagueOptions,
    ) -> Result<(League, ImportReport), String> {
        if pack.teams.len() < 4 {
            return Err("A year pack needs at least 4 teams.".into());
        }
        let mut modpack = pack.r#mod.clone().unwrap_or_default();
        // franchises come from the pack
        let mut fr: Vec<Franchise> = vec![];
        for (i, t) in pack.teams.iter().enumerate() {
            let city = if t.city.is_empty() {
                t.name
                    .split_whitespace()
                    .next()
                    .unwrap_or("City")
                    .to_string()
            } else {
                t.city.clone()
            };
            let nick = if t.nickname.is_empty() {
                t.name
                    .split_whitespace()
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                t.nickname.clone()
            };
            let abbr = if t.abbr.is_empty() {
                city.chars()
                    .filter(|c| c.is_alphabetic())
                    .take(3)
                    .collect::<String>()
                    .to_uppercase()
            } else {
                t.abbr.clone()
            };
            let east = match t.conference.as_deref() {
                Some(c) if c.eq_ignore_ascii_case("east") || c == "0" => true,
                Some(c) if c.eq_ignore_ascii_case("west") || c == "1" => false,
                _ => i % 2 == 0,
            };
            let lon = t.lon.unwrap_or(if east {
                -78.0 + (i as f64 * 0.7) % 8.0
            } else {
                -105.0 - (i as f64 * 0.9) % 15.0
            });
            let lat = t.lat.unwrap_or(30.0 + (i as f64 * 1.3) % 15.0);
            fr.push(Franchise {
                key: format!("pack{i}"),
                eras: vec![(
                    pack.year.min(1946).min(opts.year),
                    Identity {
                        city,
                        nickname: if nick.is_empty() { "Team".into() } else { nick },
                        abbr,
                        lon,
                        lat,
                        market: t.market.unwrap_or(0.5),
                    },
                )],
                last_year: None,
                speculative: false,
            });
        }
        modpack.franchises = ListPatch {
            set: Some(fr),
            ..Default::default()
        };
        modpack.info = ModInfo {
            name: format!("Year pack: {}", pack.name),
            author: pack.author.clone(),
            version: "1".into(),
            description: pack.description.clone(),
        };
        let mut o = opts.clone();
        o.year = pack.year;
        o.mods
            .push(serde_json::to_string(&modpack).map_err(|e| e.to_string())?);
        for (k, v) in &pack.settings {
            o.overrides.insert(k.clone(), v.clone());
        }
        let mut league = League::new(o)?;
        if let Some(cap) = pack.salary_cap {
            league.cap_history.retain(|(y, _)| *y != pack.year);
            league.cap_history.push((pack.year, cap));
            league.refresh_season_context();
        }
        let file = RosterFile {
            teams: pack.teams.clone(),
            free_agents: pack.free_agents.clone(),
        };
        let mut rep = league.import_rosters(&file, true);
        // draft class: add as college players near the end of their eligibility
        if !pack.draft_class.is_empty() {
            let mut rng = league.rng.fork("pack-draft");
            for e in &pack.draft_class {
                let mut p = league.player_from_entry(e, &mut rng, &mut rep);
                p.affiliation = Affiliation::FreeAgent;
                let id = league.add_player(p);
                if let Some(c) = league
                    .colleges
                    .iter()
                    .min_by_key(|c| c.roster.len())
                    .map(|c| c.id)
                {
                    league.pm(id).affiliation = Affiliation::College(c);
                    crate::college::set_class(league.pm(id), 4);
                    league.colleges[c as usize].roster.push(id);
                }
                rep.players_created += 1;
            }
        }
        let fa: Vec<PlayerId> = league
            .free_agents
            .iter()
            .copied()
            .filter(|&id| !league.p(id).is_retired())
            .collect();
        league.free_agents = fa;
        // fill short rosters
        let mut rng = league.rng.fork("pack-fill");
        for t in league.active_team_ids() {
            let need =
                (league.rules.roster_min as usize).saturating_sub(league.team(t).roster.len());
            if need > 0 {
                rep.warnings.push(format!(
                    "{} had only {} players; added {} generated reserves.",
                    league.team(t).name(),
                    league.team(t).roster.len(),
                    need
                ));
            }
            for _ in 0..need {
                let spec = GenSpec::new(
                    league.year,
                    rng.range(22, 30) as i32,
                    42.0,
                    42.0,
                    OriginKind::College,
                );
                let mut p = generate_player(&league.content, &mut rng, 0, &spec, 1.0);
                p.affiliation = Affiliation::Nba(t);
                let c = Contract::rising(
                    league.money.min_salary,
                    1,
                    0.0,
                    ContractKind::Minimum,
                    league.year,
                );
                p.contract = Some(c);
                let id = league.add_player(p);
                league.team_mut(t).roster.push(id);
                let tn = league.team(t).name();
                let y = league.year;
                league.ensure_record(id, t, &tn, y);
            }
        }
        league.calibrate_engine();
        Ok((league, rep))
    }

    // ------------------------------------------------------------------ export

    pub fn export_rosters_csv(&self) -> String {
        let mut s = String::from(
            "team,name,age,position,height_in,overall,potential,salary,years,country,college",
        );
        for a in Attr::ALL {
            s += &format!(",{}", a.key());
        }
        s.push('\n');
        for t in self.active_team_ids() {
            for &id in &self.team(t).roster {
                let p = self.p(id);
                s += &format!(
                    "{},\"{}\",{},{},{},{},{},{},{},{},\"{}\"",
                    self.team(t).abbr,
                    p.name(),
                    self.year - p.birth_year,
                    p.position.name(),
                    p.height_in,
                    p.ovr,
                    p.potential,
                    p.current_salary(),
                    p.contract.as_ref().map(|c| c.years_left()).unwrap_or(0),
                    p.country,
                    p.origin.school
                );
                for a in Attr::ALL {
                    s += &format!(",{}", p.attrs.raw(*a));
                }
                s.push('\n');
            }
        }
        s
    }

    /// Export a year pack of the current league state (share your league as a pack!).
    pub fn export_year_pack(&self, name: &str) -> YearPack {
        let mut teams = vec![];
        for t in self.active_team_ids() {
            let tm = self.team(t);
            let players = tm
                .roster
                .iter()
                .map(|&id| {
                    let p = self.p(id);
                    let mut e = PlayerEntry {
                        name: p.name(),
                        age: Some(self.year - p.birth_year),
                        position: Some(p.position.name().into()),
                        height_in: Some(p.height_in as f64),
                        weight_lb: Some(p.weight_lb),
                        overall: Some(p.ovr as f64),
                        potential: Some(p.potential as f64),
                        salary: Some(p.current_salary()),
                        years: p.contract.as_ref().map(|c| c.years_left()),
                        country: Some(p.country.clone()),
                        archetype: Some(p.archetype.clone()),
                        college: Some(p.origin.school.clone()),
                        ..Default::default()
                    };
                    for a in Attr::ALL {
                        e.attrs.insert(a.key().into(), p.attrs.raw(*a) as f64);
                    }
                    e
                })
                .collect();
            teams.push(TeamEntry {
                name: tm.name(),
                abbr: tm.abbr.clone(),
                city: tm.city.clone(),
                nickname: tm.nickname.clone(),
                conference: Some(if tm.conf == 0 {
                    "East".into()
                } else {
                    "West".into()
                }),
                market: Some(tm.market),
                lon: Some(tm.lon),
                lat: Some(tm.lat),
                players,
            });
        }
        YearPack {
            name: name.into(),
            author: self.user.name.clone(),
            description: format!("Exported from {} in {}.", self.name, self.year),
            year: self.year,
            salary_cap: Some(self.money.cap),
            teams,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_parsing_handles_quotes_and_heights() {
        assert_eq!(split_csv_line("a,\"b,c\",d"), vec!["a", "b,c", "d"]);
        assert_eq!(parse_height("6'8\""), Some(80.0));
        assert_eq!(parse_height("6-9"), Some(81.0));
        assert_eq!(parse_height("203"), Some(203.0 / 2.54));
        let (f, w) = parse_roster_csv("team,name,age,position,height_in,overall\nBOS,Test Guy,25,PG,74,70\nBOS,Other Guy,30,C,84,66\nFA,Free Man,28,SF,79,55\n").unwrap();
        assert!(w.is_empty());
        assert_eq!(f.teams.len(), 1);
        assert_eq!(f.teams[0].players.len(), 2);
        assert_eq!(f.free_agents.len(), 1);
        assert!(parse_roster_csv("x,y\n1,2\n").is_err());
    }
}
