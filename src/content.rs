//! The content registry: every piece of game data in one place, loadable and modifiable.
//!
//! `Content::default()` is the built-in game. A *mod pack* is a JSON file that modifies it.
//! Every list in the game (injuries, badges, archetypes, awards, life events, eras...) can be
//! patched by a mod:
//!
//! ```json
//! {
//!   "mod": { "name": "Brutal Knees", "author": "you", "version": "1.0", "description": "ACLs everywhere" },
//!   "injuries": { "replace": [ { ...a full InjuryDef with id "acl"... } ] },
//!   "life_events": { "add": [ { ...an EventDef... } ] },
//!   "badges": { "remove": ["iron_man"] },
//!   "settings_defaults": { "injuries.frequency": 2.0 },
//!   "tuning": { "game.three_point_bonus": 0.02 }
//! }
//! ```
//!
//! Run `hwd mod export-defaults <folder>` to write every built-in list as an editable JSON
//! template, and `hwd mod check <file>` to validate a mod with plain-English error messages.

use crate::awards::{builtin_awards, AwardDef};
use crate::economy::EconomyTables;
use crate::era::*;
use crate::events::{validate, EventDef};
use crate::franchise::*;
use crate::generate::{builtin_archetypes, ArchetypeDef};
use crate::injury::{builtin_injuries, InjuryDef};
use crate::life::LifeDefs;
use crate::names::*;
use crate::player::{builtin_badges, Attr, BadgeDef};
use crate::settings::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Content {
    pub settings: Vec<SettingDef>,
    pub presets: Vec<Preset>,
    pub rule_timeline: Vec<RuleChange>,
    pub style_anchors: Vec<EraStyle>,
    pub economy: EconomyTables,
    pub franchises: Vec<Franchise>,
    pub pools: Vec<NamePool>,
    pub countries: Vec<Country>,
    pub archetypes: Vec<ArchetypeDef>,
    pub badges: Vec<BadgeDef>,
    pub injuries: Vec<InjuryDef>,
    pub life: LifeDefs,
    pub league_events: Vec<EventDef>,
    pub awards: Vec<AwardDef>,
    /// Overrides for named game-engine parameters (see `game::default_tuning`).
    pub tuning: BTreeMap<String, f64>,
    /// Names of mods applied, in order.
    pub mods_applied: Vec<String>,
}

impl Default for Content {
    fn default() -> Self {
        Content {
            settings: builtin_defs(),
            presets: builtin_presets(),
            rule_timeline: builtin_timeline(),
            style_anchors: builtin_style_anchors(),
            economy: EconomyTables::default(),
            franchises: builtin_franchises(),
            pools: builtin_pools(),
            countries: builtin_countries(),
            archetypes: builtin_archetypes(),
            badges: builtin_badges(),
            injuries: builtin_injuries(),
            life: LifeDefs::default(),
            league_events: crate::story::builtin_league_events(),
            awards: builtin_awards(),
            tuning: BTreeMap::new(),
            mods_applied: vec![],
        }
    }
}

impl Content {
    pub fn rules(&self, year: i32) -> Rules {
        rules_from(&self.rule_timeline, year)
    }
    pub fn style(&self, year: i32) -> EraStyle {
        style_from(&self.style_anchors, year)
    }
    /// Named engine parameter (falls back to the documented default).
    pub fn tune(&self, key: &str) -> f64 {
        if let Some(v) = self.tuning.get(key) {
            return *v;
        }
        crate::game::default_tuning().iter().find(|t| t.key == key).map(|t| t.default).unwrap_or(1.0)
    }
    pub fn economy_enabled_max(&self, year: i32) -> bool {
        self.rules(year).max_contract
    }
}

// -------------------------------------------------------------------------------------------
// Mod packs
// -------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModInfo {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
}

/// How a mod changes one list. All parts are optional.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ListPatch<T> {
    /// Replace the entire list with this one.
    pub set: Option<Vec<T>>,
    /// Add new entries (an entry with an existing id replaces it).
    pub add: Vec<T>,
    /// Replace existing entries by id (an unknown id is added).
    pub replace: Vec<T>,
    /// Remove entries by id.
    pub remove: Vec<String>,
}

impl<T> Default for ListPatch<T> {
    fn default() -> Self {
        ListPatch { set: None, add: vec![], replace: vec![], remove: vec![] }
    }
}

impl<T: Clone> ListPatch<T> {
    pub fn apply(&self, list: &mut Vec<T>, key: impl Fn(&T) -> String) {
        if let Some(s) = &self.set {
            *list = s.clone();
        }
        for item in self.add.iter().chain(self.replace.iter()) {
            let k = key(item);
            if let Some(slot) = list.iter_mut().find(|x| key(x) == k) {
                *slot = item.clone();
            } else {
                list.push(item.clone());
            }
        }
        list.retain(|x| !self.remove.contains(&key(x)));
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModPack {
    #[serde(rename = "mod")]
    pub info: ModInfo,
    pub settings: ListPatch<SettingDef>,
    pub presets: ListPatch<Preset>,
    pub rule_changes: ListPatch<RuleChange>,
    pub style_anchors: ListPatch<EraStyle>,
    pub franchises: ListPatch<Franchise>,
    pub name_pools: ListPatch<NamePool>,
    pub countries: ListPatch<Country>,
    pub archetypes: ListPatch<ArchetypeDef>,
    pub badges: ListPatch<BadgeDef>,
    pub injuries: ListPatch<InjuryDef>,
    pub life_events: ListPatch<EventDef>,
    pub life_stats: ListPatch<crate::life::LifeStatDef>,
    pub life_activities: ListPatch<crate::life::ActivityDef>,
    pub league_events: ListPatch<EventDef>,
    pub awards: ListPatch<AwardDef>,
    /// Replace pieces of the economy tables, e.g. a whole new cap table.
    pub economy: Option<EconomyTables>,
    /// New default values for settings.
    pub settings_defaults: BTreeMap<String, SettingValue>,
    /// Engine tuning parameters.
    pub tuning: BTreeMap<String, f64>,
}

impl Content {
    /// Apply a mod. Returns plain-English warnings (the mod is still applied).
    pub fn apply_mod(&mut self, m: &ModPack) -> Vec<String> {
        let mut warnings = vec![];
        m.settings.apply(&mut self.settings, |x| x.key.clone());
        m.presets.apply(&mut self.presets, |x| x.id.clone());
        m.rule_changes.apply(&mut self.rule_timeline, |x| x.id.clone());
        m.style_anchors.apply(&mut self.style_anchors, |x| x.year.to_string());
        m.franchises.apply(&mut self.franchises, |x| x.key.clone());
        m.name_pools.apply(&mut self.pools, |x| x.id.clone());
        m.countries.apply(&mut self.countries, |x| x.code.clone());
        m.archetypes.apply(&mut self.archetypes, |x| x.id.clone());
        m.badges.apply(&mut self.badges, |x| x.id.clone());
        m.injuries.apply(&mut self.injuries, |x| x.id.clone());
        m.life_events.apply(&mut self.life.events, |x| x.id.clone());
        m.life_stats.apply(&mut self.life.stats, |x| x.id.clone());
        m.life_activities.apply(&mut self.life.activities, |x| x.id.clone());
        m.league_events.apply(&mut self.league_events, |x| x.id.clone());
        m.awards.apply(&mut self.awards, |x| x.id.clone());
        if let Some(e) = &m.economy {
            self.economy = e.clone();
        }
        for (k, v) in &m.settings_defaults {
            match self.settings.iter_mut().find(|d| &d.key == k) {
                Some(d) => d.default = v.clone(),
                None => warnings.push(format!("settings_defaults mentions unknown setting '{k}'.")),
            }
        }
        let known = crate::game::default_tuning();
        for (k, v) in &m.tuning {
            if !known.iter().any(|t| &t.key == k) {
                warnings.push(format!("tuning parameter '{k}' is not used by the engine (see 'hwd mod tuning' for the list)."));
            }
            self.tuning.insert(k.clone(), *v);
        }
        warnings.extend(self.validate());
        self.mods_applied.push(if m.info.name.is_empty() { "(unnamed mod)".into() } else { m.info.name.clone() });
        warnings
    }

    pub fn apply_mod_json(&mut self, json: &str) -> Result<Vec<String>, String> {
        let m: ModPack = serde_json::from_str(json).map_err(|e| format!("Could not read the mod file: {e}. Check commas, quotes and brackets near line {}.", e.line()))?;
        Ok(self.apply_mod(&m))
    }

    /// Check the whole content set for problems a modder could introduce.
    pub fn validate(&self) -> Vec<String> {
        let mut errs = vec![];
        errs.extend(validate(&self.life.events));
        errs.extend(validate(&self.league_events));
        for d in &self.injuries {
            for (k, _) in &d.permanent {
                if Attr::from_key(k).is_none() {
                    errs.push(format!("Injury '{}' lowers unknown attribute '{k}'. Valid keys: {}.", d.id, Attr::ALL.iter().map(|a| a.key()).collect::<Vec<_>>().join(", ")));
                }
            }
            if d.weight <= 0.0 {
                errs.push(format!("Injury '{}' has weight {} but must be above 0.", d.id, d.weight));
            }
        }
        for b in &self.badges {
            for (k, _) in &b.requires {
                if Attr::from_key(k).is_none() {
                    errs.push(format!("Badge '{}' requires unknown attribute '{k}'.", b.id));
                }
            }
        }
        for a in &self.archetypes {
            for k in a.attr_bias.keys() {
                if Attr::from_key(k).is_none() {
                    errs.push(format!("Archetype '{}' biases unknown attribute '{k}'.", a.id));
                }
            }
            for k in a.family_bias.keys() {
                if crate::player::Family::from_key(k).is_none() {
                    errs.push(format!("Archetype '{}' biases unknown skill family '{k}'.", a.id));
                }
            }
            if a.positions.iter().sum::<f64>() <= 0.0 {
                errs.push(format!("Archetype '{}' can't play any position.", a.id));
            }
        }
        for c in &self.countries {
            if !self.pools.iter().any(|p| p.id == c.pool) {
                errs.push(format!("Country '{}' uses name pool '{}' which doesn't exist.", c.code, c.pool));
            }
        }
        for s in &self.life.stats {
            if s.min > s.max {
                errs.push(format!("Life stat '{}' has min above max.", s.id));
            }
        }
        for a in &self.life.activities {
            for (k, _) in &a.effects {
                if k != "money" && !self.life.stats.iter().any(|s| &s.id == k) {
                    errs.push(format!("Activity '{}' changes unknown life stat '{k}'.", a.id));
                }
            }
        }
        for f in &self.franchises {
            if f.eras.is_empty() {
                errs.push(format!("Franchise '{}' has no identity.", f.key));
            }
        }
        if self.rule_timeline.is_empty() {
            errs.push("There are no rule changes at all, so no base rules can apply.".into());
        }
        errs
    }
}

/// Write each list as its own JSON file (editable templates for modders).
pub fn export_defaults(dir: &std::path::Path) -> Result<Vec<String>, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Can't create {}: {e}", dir.display()))?;
    let c = Content::default();
    let mut written = vec![];
    let mut w = |name: &str, json: String| -> Result<(), String> {
        let path = dir.join(name);
        std::fs::write(&path, json).map_err(|e| format!("Can't write {}: {e}", path.display()))?;
        written.push(path.display().to_string());
        Ok(())
    };
    let pretty = |v: &dyn erased::Ser| v.to_pretty();
    w("settings.json", pretty(&c.settings))?;
    w("presets.json", pretty(&c.presets))?;
    w("rule_changes.json", pretty(&c.rule_timeline))?;
    w("style_anchors.json", pretty(&c.style_anchors))?;
    w("economy.json", pretty(&c.economy))?;
    w("franchises.json", pretty(&c.franchises))?;
    w("name_pools.json", pretty(&c.pools))?;
    w("countries.json", pretty(&c.countries))?;
    w("archetypes.json", pretty(&c.archetypes))?;
    w("badges.json", pretty(&c.badges))?;
    w("injuries.json", pretty(&c.injuries))?;
    w("life_stats.json", pretty(&c.life.stats))?;
    w("life_activities.json", pretty(&c.life.activities))?;
    w("life_events.json", pretty(&c.life.events))?;
    w("league_events.json", pretty(&c.league_events))?;
    w("awards.json", pretty(&c.awards))?;
    w("tuning.json", pretty(&crate::game::default_tuning()))?;
    let example = ModPack {
        info: ModInfo { name: "Example mod".into(), author: "you".into(), version: "1.0".into(), description: "Copy entries from the other files here to change them. Delete what you don't need.".into() },
        settings_defaults: [("injuries.frequency".to_string(), SettingValue::Num(1.5))].into_iter().collect(),
        tuning: [("game.fatigue_rate".to_string(), 1.2)].into_iter().collect(),
        ..Default::default()
    };
    w("example_mod.json", serde_json::to_string_pretty(&example).unwrap())?;
    Ok(written)
}

mod erased {
    pub trait Ser {
        fn to_pretty(&self) -> String;
    }
    impl<T: serde::Serialize> Ser for T {
        fn to_pretty(&self) -> String {
            serde_json::to_string_pretty(self).unwrap_or_default()
        }
    }
}

/// Load every `*.json` mod pack from a directory (sorted by file name).
pub fn load_mods_from_dir(content: &mut Content, dir: &std::path::Path) -> Vec<String> {
    let mut msgs = vec![];
    let mut files: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).collect(),
        Err(_) => return msgs,
    };
    files.sort();
    for f in files {
        match std::fs::read_to_string(&f) {
            Ok(s) => match content.apply_mod_json(&s) {
                Ok(w) => {
                    msgs.push(format!("Loaded mod {}", f.display()));
                    msgs.extend(w.into_iter().map(|x| format!("  warning: {x}")));
                }
                Err(e) => msgs.push(format!("Skipped {}: {e}", f.display())),
            },
            Err(e) => msgs.push(format!("Could not read {}: {e}", f.display())),
        }
    }
    msgs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_content_is_valid() {
        let c = Content::default();
        let errs = c.validate();
        assert!(errs.is_empty(), "{errs:?}");
    }

    #[test]
    fn mods_can_add_replace_and_remove() {
        let mut c = Content::default();
        let before = c.badges.len();
        let m = r#"{ "mod": {"name":"t"}, "badges": { "remove": ["iron_man"] }, "tuning": {"game.fatigue_rate": 2.0},
                     "settings_defaults": {"injuries.frequency": 2.0} }"#;
        let w = c.apply_mod_json(m).unwrap();
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c.badges.len(), before - 1);
        assert_eq!(c.tune("game.fatigue_rate"), 2.0);
        assert_eq!(c.settings.iter().find(|d| d.key == "injuries.frequency").unwrap().default, SettingValue::Num(2.0));
        assert!(c.apply_mod_json("{ not json").is_err());
    }

    #[test]
    fn bad_mod_gives_plain_english() {
        let mut c = Content::default();
        let m = r#"{ "badges": { "add": [ { "id":"x","name":"X","category":"c","description":"d","requires":[["nope",50]],"effects":[] } ] } }"#;
        let w = c.apply_mod_json(m).unwrap();
        assert!(w.iter().any(|s| s.contains("unknown attribute 'nope'")));
    }
}
