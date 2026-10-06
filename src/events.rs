//! The declarative event engine.
//!
//! An *event* is described entirely in data: when it can happen (stage, years, conditions),
//! how likely it is, what text to show, and what it does (effects). Some events present the
//! player with *choices* (each choice has its own effects). Because nothing here is hard-coded,
//! a modder can add hundreds of life events, league events or story beats without writing Rust.
//!
//! Example (JSON):
//! ```json
//! { "id": "pickup_game", "title": "Pickup game at the park", "chance": 0.08,
//!   "stages": ["high_school"],
//!   "conditions": [{ "var": "life.energy", "op": ">=", "value": 40 }],
//!   "text": "{name} runs into some older guys at the park and holds his own.",
//!   "effects": [{ "target": "attr.hustle", "op": "add", "value": 1 },
//!               { "target": "life.reputation", "op": "add", "value": 2 }] }
//! ```

use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cond {
    /// Variable name such as `life.grades`, `player.ovr`, `age`, `flag:scandal`.
    pub var: String,
    /// One of `>=`, `<=`, `>`, `<`, `==`, `!=`.
    pub op: String,
    #[serde(default)]
    pub value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Effect {
    /// What to change: `life.<stat>`, `attr.<key>`, `mood`, `money`, `flag`, `rel.<kind>`, `custom.<key>`, `news`, `injury`, `ovr`...
    pub target: String,
    /// `add`, `set`, `mul`, `flag_add`, `flag_remove`.
    #[serde(default = "add")]
    pub op: String,
    #[serde(default)]
    pub value: f64,
    /// Used by flag/news/relationship effects.
    #[serde(default)]
    pub text: String,
}

fn add() -> String {
    "add".into()
}

impl Effect {
    pub fn add(target: &str, value: f64) -> Effect {
        Effect {
            target: target.into(),
            op: "add".into(),
            value,
            text: String::new(),
        }
    }
    pub fn set(target: &str, value: f64) -> Effect {
        Effect {
            target: target.into(),
            op: "set".into(),
            value,
            text: String::new(),
        }
    }
    pub fn flag(name: &str) -> Effect {
        Effect {
            target: "flag".into(),
            op: "flag_add".into(),
            value: 0.0,
            text: name.into(),
        }
    }
    pub fn unflag(name: &str) -> Effect {
        Effect {
            target: "flag".into(),
            op: "flag_remove".into(),
            value: 0.0,
            text: name.into(),
        }
    }
    pub fn news(text: &str) -> Effect {
        Effect {
            target: "news".into(),
            op: "add".into(),
            value: 0.0,
            text: text.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
    /// Plain-English explanation of what this choice leads to.
    pub explain: String,
    #[serde(default)]
    pub conditions: Vec<Cond>,
    #[serde(default)]
    pub effects: Vec<Effect>,
    /// Text shown after choosing.
    #[serde(default)]
    pub result: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventDef {
    pub id: String,
    pub title: String,
    /// Template; `{name}` is replaced with the subject's name.
    pub text: String,
    #[serde(default)]
    pub category: String,
    /// Chance per tick (0-1) before the intensity setting.
    pub chance: f64,
    /// Life stages or scopes this event applies to (`high_school`, `college`, `pro`, `overseas`, `retired`, or `any`).
    #[serde(default)]
    pub stages: Vec<String>,
    #[serde(default)]
    pub min_year: i32,
    #[serde(default = "far")]
    pub max_year: i32,
    #[serde(default)]
    pub conditions: Vec<Cond>,
    /// Fires at most once per career/league.
    #[serde(default)]
    pub once: bool,
    /// Minimum number of ticks between firings.
    #[serde(default)]
    pub cooldown: u32,
    /// Effects applied automatically (events with no choices).
    #[serde(default)]
    pub effects: Vec<Effect>,
    /// If non-empty, the human player must pick one.
    #[serde(default)]
    pub choices: Vec<Choice>,
}

fn far() -> i32 {
    9999
}

/// What the engine needs from whatever the event is happening *to*.
pub trait EventContext {
    fn var(&self, name: &str) -> f64;
    fn has_flag(&self, name: &str) -> bool;
    fn stage(&self) -> String;
    fn year(&self) -> i32;
    fn subject_name(&self) -> String;
    /// Ticks since this event last fired (u32::MAX if never).
    fn since_fired(&self, id: &str) -> u32;
    fn apply(&mut self, effect: &Effect, log: &mut Vec<String>);
    fn mark_fired(&mut self, id: &str);
}

pub fn check(c: &Cond, ctx: &dyn EventContext) -> bool {
    if let Some(flag) = c.var.strip_prefix("flag:") {
        let has = ctx.has_flag(flag);
        return match c.op.as_str() {
            "!=" | "no_flag" => !has,
            _ => has,
        };
    }
    let v = ctx.var(&c.var);
    match c.op.as_str() {
        ">=" => v >= c.value,
        "<=" => v <= c.value,
        ">" => v > c.value,
        "<" => v < c.value,
        "==" => (v - c.value).abs() < 1e-9,
        "!=" => (v - c.value).abs() >= 1e-9,
        _ => false,
    }
}

pub fn eligible(e: &EventDef, ctx: &dyn EventContext) -> bool {
    let y = ctx.year();
    if y < e.min_year || y > e.max_year {
        return false;
    }
    if !e.stages.is_empty() && !e.stages.iter().any(|s| s == "any" || *s == ctx.stage()) {
        return false;
    }
    if e.once && ctx.since_fired(&e.id) != u32::MAX {
        return false;
    }
    if e.cooldown > 0 && ctx.since_fired(&e.id) < e.cooldown {
        return false;
    }
    e.conditions.iter().all(|c| check(c, ctx))
}

pub fn render(text: &str, ctx: &dyn EventContext) -> String {
    text.replace("{name}", &ctx.subject_name())
        .replace("{year}", &ctx.year().to_string())
}

/// An event waiting for the human to choose.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingEvent {
    pub event_id: String,
    pub title: String,
    pub text: String,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventOutcome {
    /// Lines describing what happened automatically (title, text, effects).
    pub log: Vec<String>,
    pub pending: Vec<PendingEvent>,
}

/// Roll all events once for a context. `intensity` scales every chance. `max_events` caps how
/// many fire in one tick so the player isn't flooded.
pub fn roll(
    defs: &[EventDef],
    ctx: &mut dyn EventContext,
    rng: &mut Rng,
    intensity: f64,
    max_events: usize,
    auto_choice: bool,
) -> EventOutcome {
    let mut out = EventOutcome::default();
    let mut fired = 0;
    // Random order so no event is always checked first.
    let mut order: Vec<usize> = (0..defs.len()).collect();
    rng.shuffle(&mut order);
    for i in order {
        if fired >= max_events {
            break;
        }
        let e = &defs[i];
        if !eligible(e, ctx) {
            continue;
        }
        if !rng.chance((e.chance * intensity).clamp(0.0, 1.0)) {
            continue;
        }
        fired += 1;
        ctx.mark_fired(&e.id);
        let text = render(&e.text, ctx);
        if e.choices.is_empty() {
            out.log.push(format!("{}: {}", e.title, text));
            for ef in &e.effects {
                ctx.apply(ef, &mut out.log);
            }
        } else {
            let avail: Vec<Choice> = e
                .choices
                .iter()
                .filter(|c| c.conditions.iter().all(|k| check(k, ctx)))
                .cloned()
                .collect();
            if avail.is_empty() {
                continue;
            }
            // Apply automatic effects first.
            for ef in &e.effects {
                ctx.apply(ef, &mut out.log);
            }
            if auto_choice {
                let c = &avail[rng.range_usize(avail.len())];
                out.log.push(format!(
                    "{}: {} (handled automatically: {})",
                    e.title, text, c.label
                ));
                for ef in &c.effects {
                    ctx.apply(ef, &mut out.log);
                }
            } else {
                out.pending.push(PendingEvent {
                    event_id: e.id.clone(),
                    title: e.title.clone(),
                    text,
                    choices: avail,
                });
            }
        }
    }
    out
}

/// Apply a chosen option of a pending event.
pub fn resolve(
    p: &PendingEvent,
    choice_id: &str,
    ctx: &mut dyn EventContext,
) -> Result<Vec<String>, String> {
    let c = p
        .choices
        .iter()
        .find(|c| c.id == choice_id)
        .ok_or_else(|| {
            format!(
                "'{}' is not one of the options for '{}'.",
                choice_id, p.title
            )
        })?;
    let mut log = vec![];
    if !c.result.is_empty() {
        log.push(render(&c.result, ctx));
    }
    for ef in &c.effects {
        ctx.apply(ef, &mut log);
    }
    Ok(log)
}

/// Validate event definitions and return plain-English problems (used by the mod checker).
pub fn validate(defs: &[EventDef]) -> Vec<String> {
    let mut errs = vec![];
    let mut seen = std::collections::HashSet::new();
    for e in defs {
        if !seen.insert(&e.id) {
            errs.push(format!("Event id '{}' is used twice.", e.id));
        }
        if e.chance < 0.0 || e.chance > 1.0 {
            errs.push(format!(
                "Event '{}' has chance {} but chance must be between 0 and 1.",
                e.id, e.chance
            ));
        }
        if e.text.trim().is_empty() {
            errs.push(format!("Event '{}' has no text.", e.id));
        }
        for c in &e.conditions {
            if ![">=", "<=", ">", "<", "==", "!=", "no_flag", "has_flag"].contains(&c.op.as_str()) {
                errs.push(format!(
                    "Event '{}' has a condition with unknown operator '{}'.",
                    e.id, c.op
                ));
            }
        }
        for ef in e
            .effects
            .iter()
            .chain(e.choices.iter().flat_map(|c| c.effects.iter()))
        {
            if !["add", "set", "mul", "flag_add", "flag_remove"].contains(&ef.op.as_str()) {
                errs.push(format!(
                    "Event '{}' has an effect with unknown op '{}'.",
                    e.id, ef.op
                ));
            }
        }
    }
    errs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    struct Ctx {
        vars: BTreeMap<String, f64>,
        flags: BTreeSet<String>,
        fired: BTreeMap<String, u32>,
    }
    impl EventContext for Ctx {
        fn var(&self, n: &str) -> f64 {
            *self.vars.get(n).unwrap_or(&0.0)
        }
        fn has_flag(&self, n: &str) -> bool {
            self.flags.contains(n)
        }
        fn stage(&self) -> String {
            "high_school".into()
        }
        fn year(&self) -> i32 {
            2000
        }
        fn subject_name(&self) -> String {
            "Test".into()
        }
        fn since_fired(&self, id: &str) -> u32 {
            *self.fired.get(id).unwrap_or(&u32::MAX)
        }
        fn apply(&mut self, e: &Effect, _log: &mut Vec<String>) {
            if e.op == "add" {
                *self.vars.entry(e.target.clone()).or_insert(0.0) += e.value;
            }
        }
        fn mark_fired(&mut self, id: &str) {
            self.fired.insert(id.into(), 0);
        }
    }

    #[test]
    fn event_fires_and_applies() {
        let defs = vec![EventDef {
            id: "e".into(),
            title: "T".into(),
            text: "{name} does a thing".into(),
            category: "x".into(),
            chance: 1.0,
            stages: vec!["high_school".into()],
            min_year: 0,
            max_year: 9999,
            conditions: vec![Cond {
                var: "life.energy".into(),
                op: ">=".into(),
                value: 10.0,
            }],
            once: true,
            cooldown: 0,
            effects: vec![Effect::add("life.rep", 5.0)],
            choices: vec![],
        }];
        let mut ctx = Ctx {
            vars: BTreeMap::new(),
            flags: BTreeSet::new(),
            fired: BTreeMap::new(),
        };
        ctx.vars.insert("life.energy".into(), 50.0);
        let mut rng = Rng::new(1);
        let o = roll(&defs, &mut ctx, &mut rng, 1.0, 3, false);
        assert_eq!(o.log.len(), 1);
        assert_eq!(ctx.var("life.rep"), 5.0);
        // once: second roll does nothing
        let o2 = roll(&defs, &mut ctx, &mut rng, 1.0, 3, false);
        assert!(o2.log.is_empty());
        assert!(validate(&defs).is_empty());
    }
}
