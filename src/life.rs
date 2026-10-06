//! The life sim: everything about being a person, not just a basketball player.
//!
//! When you play as a created player you manage a month-by-month life: how you split your time
//! (study, skill work, gym, friends, family, rest, media, a job), your grades and eligibility,
//! family and friend relationships, personal money and taxes, reputation, stress and happiness.
//! Random life events (with choices!) fire from a data-driven library (`events.rs`).
//!
//! Almost nothing is hard-coded: the list of life stats, the time-allocation activities and the
//! events are all DATA (`LifeDefs`). A mod can add a new stat ("faith"), a new activity
//! ("coach youth camps") or 100 new events by dropping a JSON file in the mods folder.

use crate::events::*;
use crate::player::{Attr, Player};
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeStage {
    HighSchool,
    College,
    Pro,
    Overseas,
    Retired,
}

impl LifeStage {
    pub fn key(self) -> &'static str {
        match self {
            LifeStage::HighSchool => "high_school",
            LifeStage::College => "college",
            LifeStage::Pro => "pro",
            LifeStage::Overseas => "overseas",
            LifeStage::Retired => "retired",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LifeStage::HighSchool => "High school",
            LifeStage::College => "College",
            LifeStage::Pro => "Pro career",
            LifeStage::Overseas => "Playing overseas",
            LifeStage::Retired => "Retired",
        }
    }
}

// ----------------------------------------------------------------------------------------
// Definitions (data)
// ----------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LifeStatDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// Each month the stat drifts toward this value...
    pub drift_to: f64,
    /// ...by this fraction of the gap (0 = never drifts).
    pub drift_rate: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityDef {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Default share of your time (percent).
    pub default_pct: f64,
    /// Change per month at 100% of your time, per stat: (stat id, delta). Scaled by the share you choose.
    pub effects: Vec<(String, f64)>,
    /// Stages in which this activity is available (empty = all).
    #[serde(default)]
    pub stages: Vec<String>,
}

fn stat(
    id: &str,
    name: &str,
    desc: &str,
    min: f64,
    max: f64,
    default: f64,
    drift_to: f64,
    rate: f64,
) -> LifeStatDef {
    LifeStatDef {
        id: id.into(),
        name: name.into(),
        description: desc.into(),
        min,
        max,
        default,
        drift_to,
        drift_rate: rate,
    }
}

fn act(
    id: &str,
    name: &str,
    desc: &str,
    pct: f64,
    eff: &[(&str, f64)],
    stages: &[&str],
) -> ActivityDef {
    ActivityDef {
        id: id.into(),
        name: name.into(),
        description: desc.into(),
        default_pct: pct,
        effects: eff.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        stages: stages.iter().map(|s| s.to_string()).collect(),
    }
}

pub fn builtin_stats() -> Vec<LifeStatDef> {
    vec![
        stat("grades", "Grades (GPA)", "Your grade-point average (0-4). Below 2.0 you can lose eligibility.", 0.0, 4.0, 3.0, 3.0, 0.0),
        stat("happiness", "Happiness", "How good life feels. Low happiness hurts performance and decisions.", 0.0, 100.0, 62.0, 60.0, 0.10),
        stat("stress", "Stress", "Pressure from school, expectations and the spotlight. High stress hurts play and health.", 0.0, 100.0, 30.0, 30.0, 0.15),
        stat("energy", "Energy", "Rest vs. exhaustion. Low energy raises injury risk and lowers development.", 0.0, 100.0, 70.0, 70.0, 0.50),
        stat("reputation", "Reputation", "Your public image: coaches, brands and fans.", 0.0, 100.0, 35.0, 35.0, 0.02),
        stat("fame", "Fame", "How many people know who you are.", 0.0, 100.0, 4.0, 0.0, 0.02),
        stat("family", "Family bond", "How close and supportive your family is.", 0.0, 100.0, 60.0, 55.0, 0.05),
        stat("friends", "Social life", "Your friend group and sense of belonging.", 0.0, 100.0, 50.0, 40.0, 0.06),
        stat("romance", "Romance", "Your love life (0 = single / no one, 100 = a deep partnership).", 0.0, 100.0, 0.0, 0.0, 0.0),
        stat("discipline", "Discipline", "Habits and self-control. Helps grades, avoids trouble.", 0.0, 100.0, 55.0, 55.0, 0.01),
        stat("maturity", "Maturity", "Grows with age and experiences. Helps with money and media.", 0.0, 100.0, 40.0, 40.0, 0.0),
        stat("coach_trust", "Coach trust", "How much your coach believes in you.", 0.0, 100.0, 50.0, 50.0, 0.05),
        stat("dev_skill", "Skill momentum", "Short-term boost to skill development from recent skill work.", 0.0, 100.0, 0.0, 0.0, 0.60),
        stat("dev_phys", "Physical momentum", "Short-term boost to physical development from recent gym work.", 0.0, 100.0, 0.0, 0.0, 0.60),
    ]
}

pub fn builtin_activities() -> Vec<ActivityDef> {
    vec![
        act("study", "Study & classes", "Hit the books. Protects eligibility and raises grades.", 20.0,
            &[("grades", 0.45), ("stress", 8.0), ("energy", -18.0), ("discipline", 3.0), ("friends", -4.0)], &["high_school", "college"]),
        act("skill_work", "Skill work", "Individual drills, shooting, handles, film. The biggest boost to improving as a player.", 25.0,
            &[("dev_skill", 160.0), ("energy", -26.0), ("stress", 8.0), ("coach_trust", 7.0), ("discipline", 2.0)], &[]),
        act("gym", "Gym & conditioning", "Lifting, sprints, nutrition. Builds athleticism and durability.", 10.0,
            &[("dev_phys", 160.0), ("energy", -18.0), ("stress", 3.0), ("discipline", 2.0)], &[]),
        act("social", "Friends & fun", "Hang out, go out. Lifts mood but costs study time.", 15.0,
            &[("friends", 22.0), ("happiness", 14.0), ("reputation", 2.0), ("stress", -10.0), ("grades", -0.18), ("discipline", -3.0)], &[]),
        act("family", "Family time", "Time with parents, siblings, partner or kids.", 10.0,
            &[("family", 20.0), ("happiness", 9.0), ("stress", -8.0)], &[]),
        act("rest", "Rest & recovery", "Sleep, recovery, therapy, quiet time. Restores energy.", 15.0,
            &[("energy", 55.0), ("stress", -20.0), ("happiness", 6.0)], &[]),
        act("media", "Media & brand", "Social media, interviews, appearances. Grows fame and (sometimes) money.", 0.0,
            &[("fame", 3.5), ("reputation", 0.5), ("stress", 6.0), ("energy", -10.0), ("money", 600.0)], &[]),
        act("work", "Job / side hustle", "A part-time job or business. Earns money and builds maturity.", 5.0,
            &[("money", 900.0), ("maturity", 2.0), ("energy", -18.0), ("discipline", 2.0)], &["high_school", "college"]),
    ]
}

fn cond(var: &str, op: &str, value: f64) -> Cond {
    Cond {
        var: var.into(),
        op: op.into(),
        value,
    }
}

#[allow(clippy::too_many_arguments)]
fn ev(
    id: &str,
    title: &str,
    text: &str,
    cat: &str,
    chance: f64,
    stages: &[&str],
    conds: Vec<Cond>,
    effects: Vec<Effect>,
) -> EventDef {
    EventDef {
        id: id.into(),
        title: title.into(),
        text: text.into(),
        category: cat.into(),
        chance,
        stages: stages.iter().map(|s| s.to_string()).collect(),
        min_year: 0,
        max_year: 9999,
        conditions: conds,
        once: false,
        cooldown: 0,
        effects,
        choices: vec![],
    }
}

fn ch(id: &str, label: &str, explain: &str, effects: Vec<Effect>, result: &str) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
        explain: explain.into(),
        conditions: vec![],
        effects,
        result: result.into(),
    }
}

fn with_choices(mut e: EventDef, choices: Vec<Choice>) -> EventDef {
    e.choices = choices;
    e
}

fn once(mut e: EventDef) -> EventDef {
    e.once = true;
    e
}

fn cooldown(mut e: EventDef, n: u32) -> EventDef {
    e.cooldown = n;
    e
}

fn years(mut e: EventDef, min: i32, max: i32) -> EventDef {
    e.min_year = min;
    e.max_year = max;
    e
}

pub fn builtin_life_events() -> Vec<EventDef> {
    let a = Effect::add;
    vec![
        // ------------------------------------------------------------ High school
        ev("pickup_game", "Pickup game at the park", "{name} runs into some older guys at the park and more than holds his own.", "basketball", 0.06, &["high_school", "college"],
            vec![cond("life.energy", ">=", 40.0)], vec![a("attr.hustle", 1.0), a("life.reputation", 2.0), a("life.happiness", 3.0)]),
        once(ev("growth_spurt", "Growth spurt", "{name} wakes up taller than he went to sleep. His pants don't fit and his coordination is a work in progress.", "body", 0.02, &["high_school"],
            vec![cond("age", "<=", 17.0), cond("height", "<=", 82.0)], vec![a("height", 1.0), a("attr.agility", -1.0), a("life.stress", 3.0)])),
        with_choices(cooldown(ev("failing_class", "Struggling in class", "{name} is slipping in a core class. A teacher pulls him aside after school.", "school", 0.20, &["high_school", "college"],
            vec![cond("life.grades", "<", 2.4)], vec![]), 4), vec![
            ch("tutor", "Hire a tutor", "Costs money, takes time, but your grades recover quickly.", vec![a("money", -400.0), a("life.grades", 0.35), a("life.stress", -4.0)], "A tutor turns things around."),
            ch("cram", "Cram on your own", "Free, but exhausting and only partly effective.", vec![a("life.grades", 0.15), a("life.energy", -15.0), a("life.stress", 8.0)], "Late nights help a little."),
            ch("ignore", "Ignore it, focus on basketball", "More time to hoop. Your grades keep falling and eligibility is at risk.", vec![a("life.grades", -0.2), a("life.coach_trust", -3.0)], "The grades slide continues."),
        ]),
        with_choices(cooldown(ev("aau_invite", "AAU circuit invitation", "A top travel-ball program wants {name} on its roster for the summer circuit.", "basketball", 0.08, &["high_school"],
            vec![cond("player.ovr", ">=", 45.0)], vec![]), 12), vec![
            ch("join", "Join the circuit", "Big exposure and tough competition. Costs travel money and sleep.", vec![a("life.fame", 6.0), a("attr.hustle", 1.0), a("life.energy", -12.0), a("money", -600.0), a("flag", 0.0), Effect::flag("aau_circuit")], "The summer circuit puts you on the map."),
            ch("stay", "Stay with the school team", "Safe and cheap. Fewer eyes on you.", vec![a("life.coach_trust", 4.0), a("life.family", 2.0)], "You stay loyal to your high school team."),
        ]),
        ev("viral_highlight", "Viral highlight", "A clip of {name} going viral spreads across the internet. Strangers know his name overnight.", "fame", 0.015, &["high_school", "college", "pro"],
            vec![cond("player.ovr", ">=", 55.0), cond("year", ">=", 2008.0)], vec![a("life.fame", 8.0), a("life.reputation", 3.0), a("life.stress", 4.0)]),
        with_choices(cooldown(ev("party_invite", "Big party invitation", "Everyone is going to the party this weekend. {name} has a game on Saturday.", "social", 0.10, &["high_school", "college"], vec![], vec![]), 6), vec![
            ch("go", "Go to the party", "A great night, but risky for sleep, discipline and reputation.", vec![a("life.friends", 10.0), a("life.happiness", 8.0), a("life.discipline", -4.0), a("life.energy", -15.0), a("wear", 0.5)], "A night to remember (mostly)."),
            ch("skip", "Stay in and rest", "You miss out socially, but you're sharp for the game.", vec![a("life.energy", 10.0), a("life.discipline", 3.0), a("life.friends", -3.0)], "You stay home and recharge."),
        ]),
        with_choices(cooldown(ev("family_money_trouble", "Money trouble at home", "Bills are piling up at home. {name}'s parents are stressed.", "family", 0.04, &["high_school", "college"], vec![cond("life.family", ">=", 20.0)], vec![a("life.stress", 5.0)]), 12), vec![
            ch("job", "Get a part-time job", "Helps the family and builds maturity but eats into practice and study.", vec![a("money", 1500.0), a("life.maturity", 5.0), a("life.energy", -10.0), a("life.family", 8.0)], "Your paycheck keeps the lights on."),
            ch("focus", "Focus on basketball", "Your future earnings are the family plan. Pressure builds.", vec![a("life.stress", 6.0), a("life.family", -3.0), a("attr.clutch", 1.0)], "You carry the weight silently."),
        ]),
        with_choices(cooldown(ev("coach_conflict", "Clash with your coach", "{name} and his coach disagree over playing time and role.", "basketball", 0.07, &["high_school", "college"],
            vec![cond("life.coach_trust", "<=", 55.0)], vec![]), 8), vec![
            ch("talk", "Talk it out respectfully", "A mature conversation repairs trust.", vec![a("life.coach_trust", 10.0), a("life.maturity", 3.0)], "You and coach clear the air."),
            ch("sulk", "Sulk and coast", "Easy now, costly later.", vec![a("life.coach_trust", -10.0), a("life.happiness", -4.0), a("mood", -4.0)], "Things stay frosty."),
            ch("transfer", "Start looking for another program", "Opens options (and drama). Flags a possible transfer.", vec![Effect::flag("wants_transfer"), a("life.coach_trust", -8.0), a("life.stress", 4.0)], "Your phone starts buzzing with other offers."),
        ]),
        once(ev("first_love", "First love", "{name} starts seeing someone special. Life feels lighter, but time gets tighter.", "romance", 0.025, &["high_school", "college"],
            vec![cond("life.romance", "<=", 5.0), cond("age", ">=", 16.0)], vec![a("life.romance", 45.0), a("life.happiness", 10.0), a("life.stress", -3.0), Effect::flag("in_relationship")])),
        with_choices(cooldown(ev("runner_approach", "A 'friend of the family' offers help", "A stranger offers {name}'s family cash and 'connections' in exchange for a future promise. It smells like an agent runner.", "recruiting", 0.025, &["high_school", "college"],
            vec![cond("player.ovr", ">=", 55.0), cond("life.family", "<=", 90.0)], vec![]), 24), vec![
            ch("accept", "Accept the money", "Cash now. If it comes out you could lose eligibility and your reputation.", vec![a("money", 8000.0), a("life.reputation", -2.0), Effect::flag("took_improper_benefits")], "The money arrives quietly."),
            ch("decline", "Say no thanks", "Clean conscience, clean record.", vec![a("life.reputation", 3.0), a("life.maturity", 3.0)], "You walk away."),
        ]),
        with_choices(cooldown(ev("overuse_soreness", "Persistent soreness", "{name}'s legs feel heavy after a stretch of games. Something nags.", "health", 0.08, &["high_school", "college", "pro", "overseas"],
            vec![cond("life.energy", "<=", 40.0)], vec![]), 4), vec![
            ch("rest", "Take a few days off", "Recovery now prevents a real injury.", vec![a("life.energy", 25.0), a("wear", -2.0), a("fitness", 10.0)], "You rest up and bounce back."),
            ch("play", "Play through it", "Show toughness, risk a real injury.", vec![a("wear", 3.0), a("life.coach_trust", 4.0), a("fitness", -8.0)], "You grit your teeth."),
        ]),
        with_choices(once(ev("prep_school_offer", "Prep school offer", "A national prep powerhouse offers {name} a spot. Better competition and exposure, tougher academics, away from home.", "school", 0.05, &["high_school"],
            vec![cond("player.ovr", ">=", 52.0), cond("age", "<=", 17.0)], vec![])), vec![
            ch("transfer", "Transfer to the prep school", "More exposure and development. Harder classes, homesick.", vec![Effect::flag("prep_school"), a("life.fame", 8.0), a("life.dev_skill", 10.0), a("life.family", -8.0), a("life.grades", -0.2), a("life.stress", 6.0)], "New school, bigger stage."),
            ch("stay", "Stay home", "Keep your people close.", vec![a("life.family", 6.0), a("life.happiness", 4.0)], "You stay put."),
        ]),
        ev("mentor_offer", "A mentor steps in", "A former pro takes {name} under his wing for the summer, teaching film study and pro habits.", "basketball", 0.03, &["high_school", "college"],
            vec![cond("life.reputation", ">=", 40.0)], vec![a("attr.shot_iq", 2.0), a("life.maturity", 4.0), a("life.dev_skill", 10.0)]),
        ev("scholarship_letters", "Letters pile up", "Recruiting mail floods in for {name}. Programs across the country want to know him.", "recruiting", 0.06, &["high_school"],
            vec![cond("player.ovr", ">=", 50.0), cond("life.grades", ">=", 2.0)], vec![a("life.fame", 3.0), a("life.happiness", 4.0)]),
        // ------------------------------------------------------------ College
        with_choices(cooldown(ev("nil_deal", "NIL deal offered", "A local brand wants {name} for an endorsement deal.", "money", 0.10, &["college"],
            vec![cond("life.fame", ">=", 15.0)], vec![]), 6), vec![
            ch("sign", "Sign the deal", "Real money while you're in school. Takes time and attention.", vec![a("money", 25000.0), a("life.reputation", 2.0), a("life.energy", -6.0)], "You sign and cash in."),
            ch("hold", "Hold out for a bigger one", "Might price yourself out.", vec![a("life.stress", 2.0)], "You keep waiting."),
        ]),
        ev("probation", "Academic warning", "{name} receives a warning from the academic office: one more bad term and he's ineligible.", "school", 0.20, &["college"],
            vec![cond("life.grades", "<", 2.0)], vec![a("life.stress", 8.0), a("life.happiness", -6.0), a("life.coach_trust", -4.0)]),
        ev("freshman_hype", "Freshman phenom", "Analysts are calling {name} one of the most exciting freshmen in the country.", "fame", 0.05, &["college"],
            vec![cond("player.ovr", ">=", 62.0)], vec![a("life.fame", 7.0), a("life.stress", 5.0), a("life.reputation", 2.0)]),
        with_choices(cooldown(ev("social_media_post", "Controversial post", "Something {name} posted late at night is blowing up for the wrong reasons.", "media", 0.03, &["college", "pro"],
            vec![cond("year", ">=", 2009.0), cond("life.fame", ">=", 10.0)], vec![]), 12), vec![
            ch("apologize", "Apologize sincerely", "Take the hit and move on.", vec![a("life.reputation", -2.0), a("life.maturity", 3.0)], "The storm passes."),
            ch("double_down", "Double down", "Your fans love it. Brands and coaches do not.", vec![a("life.fame", 4.0), a("life.reputation", -8.0), a("life.stress", 5.0)], "You dig in."),
            ch("delete", "Delete and stay quiet", "Mostly works.", vec![a("life.reputation", -1.0)], "It blows over."),
        ]),
        with_choices(ev("declare_pressure", "Pressure to go pro", "People close to {name} are urging him to turn pro early while his stock is high.", "career", 0.10, &["college"],
            vec![cond("player.ovr", ">=", 66.0), cond("life.family", ">=", 20.0)], vec![]), vec![
            ch("listen", "Weigh the family's advice", "Leaning toward the draft raises family closeness and stress.", vec![a("life.family", 4.0), a("life.stress", 5.0), Effect::flag("considering_draft")], "You start thinking seriously about the draft."),
            ch("stay", "Stay focused on this season", "Keep your head down.", vec![a("life.discipline", 2.0)], "You stay in the moment."),
        ]),
        // ------------------------------------------------------------ Pro / Overseas
        with_choices(cooldown(ev("endorsement_offer", "Endorsement offer", "A shoe company is courting {name}.", "money", 0.07, &["pro", "overseas"],
            vec![cond("life.fame", ">=", 20.0), cond("life.reputation", ">=", 35.0)], vec![]), 8), vec![
            ch("sign", "Sign it", "Steady income each year.", vec![a("endorsement", 250000.0), a("life.reputation", 1.0)], "Ink on paper."),
            ch("shop", "Shop it to rivals", "Could raise the price... or lose the deal.", vec![a("endorsement", 120000.0), a("life.stress", 3.0)], "You negotiate hard."),
        ]),
        with_choices(cooldown(ev("family_request", "Family asks for money", "Relatives need help and look to {name}.", "family", 0.10, &["pro", "overseas"],
            vec![cond("money", ">=", 20000.0)], vec![]), 6), vec![
            ch("give", "Help them out", "Closer family, lighter wallet.", vec![a("money", -50000.0), a("life.family", 8.0), a("life.happiness", 4.0)], "You help generously."),
            ch("limit", "Set a budget", "A fair compromise.", vec![a("money", -10000.0), a("life.family", 2.0), a("life.stress", 2.0)], "You help within limits."),
            ch("no", "Say no", "Protect your savings. Feelings get hurt.", vec![a("life.family", -8.0), a("life.stress", 4.0)], "Tension at the next reunion."),
        ]),
        with_choices(cooldown(ev("investment_pitch", "Investment pitch", "Someone pitches {name} on a can't-miss business.", "money", 0.06, &["pro", "overseas"],
            vec![cond("money", ">=", 50000.0)], vec![]), 8), vec![
            ch("invest", "Invest big", "High risk, high reward.", vec![a("money_gamble", 0.0)], "Fingers crossed."),
            ch("index", "Put it in safe index funds", "Boring wins.", vec![a("money", 5000.0), a("life.maturity", 2.0)], "Slow and steady."),
            ch("pass", "Pass", "Keep your cash.", vec![], "You pass."),
        ]),
        with_choices(cooldown(ev("locker_room_rift", "Locker-room rift", "A feud is brewing in the locker room and {name} is in the middle of it.", "team", 0.06, &["pro", "overseas", "college"],
            vec![cond("mood", "<=", 60.0)], vec![]), 8), vec![
            ch("mediate", "Mediate", "Take the leadership role.", vec![a("life.maturity", 4.0), a("mood", 4.0), a("life.stress", 4.0)], "You calm things down."),
            ch("side", "Pick a side", "Loyalty has a price.", vec![a("life.friends", 4.0), a("mood", -4.0)], "You choose your teammate."),
            ch("stay_out", "Stay out of it", "Safe, not heroic.", vec![], "You keep your head down."),
        ]),
        with_choices(cooldown(ev("burnout_warning", "Running on empty", "{name} is mentally drained. Friends and family are worried.", "health", 0.25, &["pro", "overseas", "college", "high_school"],
            vec![cond("life.stress", ">=", 80.0)], vec![a("life.happiness", -5.0)]), 3), vec![
            ch("therapy", "See a therapist", "Costs a little, helps a lot.", vec![a("money", -1500.0), a("life.stress", -25.0), a("life.happiness", 10.0), a("life.maturity", 3.0)], "Talking helps."),
            ch("vacation", "Take a break", "Time off.", vec![a("life.stress", -20.0), a("life.energy", 25.0), a("life.dev_skill", -10.0)], "You recharge."),
            ch("push", "Push through", "Short-term toughness, long-term risk.", vec![a("life.stress", 5.0), a("wear", 1.5), a("life.happiness", -6.0)], "You keep grinding."),
        ]),
        with_choices(once(ev("marriage", "Wedding bells", "{name} and his partner decide to get married.", "romance", 0.04, &["pro", "overseas", "retired"],
            vec![cond("life.romance", ">=", 70.0), cond("age", ">=", 22.0)], vec![])), vec![
            ch("big", "A lavish wedding", "Unforgettable, and expensive.", vec![a("money", -80000.0), a("life.happiness", 12.0), a("life.romance", 15.0), a("life.fame", 2.0), Effect::flag("married")], "You throw a party for the ages."),
            ch("small", "A small ceremony", "Intimate and sensible.", vec![a("money", -8000.0), a("life.happiness", 9.0), a("life.romance", 12.0), Effect::flag("married")], "You keep it simple."),
        ]),
        once(ev("child_born", "A new addition", "{name} becomes a parent. Life changes overnight.", "family", 0.03, &["pro", "overseas", "retired"],
            vec![cond("flag:married", "==", 1.0)], vec![a("life.family", 12.0), a("life.happiness", 10.0), a("life.maturity", 10.0), a("life.energy", -10.0), a("money", -20000.0), Effect::flag("parent")])),
        with_choices(cooldown(ev("buy_home", "House hunting", "{name} is thinking about buying a home.", "money", 0.05, &["pro"],
            vec![cond("money", ">=", 150000.0), cond("life.maturity", ">=", 40.0)], vec![]), 24), vec![
            ch("buy", "Buy a home", "A big purchase that holds value.", vec![a("money", -120000.0), a("asset", 150000.0), a("life.happiness", 5.0)], "You get the keys."),
            ch("rent", "Keep renting", "Flexible.", vec![], "You stay flexible."),
        ]),
        with_choices(cooldown(ev("charity", "Start a foundation", "{name} is invited to start a charitable foundation in his hometown.", "reputation", 0.03, &["pro", "overseas", "retired"],
            vec![cond("money", ">=", 100000.0)], vec![]), 36), vec![
            ch("yes", "Launch it", "Costs money and time. Builds reputation.", vec![a("money", -75000.0), a("life.reputation", 10.0), a("life.fame", 3.0), a("life.happiness", 6.0)], "Your foundation opens its doors."),
            ch("no", "Not now", "Maybe later.", vec![], "Maybe later."),
        ]),
        once(years(ev("content_channel", "Starting a channel", "{name} launches a podcast and video channel about his life in the league.", "media", 0.015, &["pro", "overseas", "retired"],
            vec![cond("life.fame", ">=", 25.0)], vec![a("life.fame", 6.0), a("endorsement", 120000.0), a("life.stress", 3.0)]), 2012, 9999)),
        with_choices(cooldown(ev("scandal", "Off-court trouble", "A story about {name} is about to break.", "scandal", 0.012, &["pro", "college", "overseas"],
            vec![cond("life.discipline", "<=", 45.0)], vec![]), 36), vec![
            ch("own_it", "Own it publicly", "Painful now, respected later.", vec![a("life.reputation", -8.0), a("life.maturity", 6.0), a("mood", -4.0)], "You take responsibility."),
            ch("lawyer_up", "Lawyer up and deny", "Might work. Might not.", vec![a("money", -40000.0), a("life.reputation", -12.0), a("life.stress", 10.0)], "A messy fight in the press."),
        ]),
        ev("hometown_camp", "Hometown camp", "{name} hosts a youth camp back home. The kids love it.", "reputation", 0.03, &["pro", "retired"],
            vec![cond("life.reputation", ">=", 45.0)], vec![a("life.reputation", 4.0), a("life.happiness", 5.0), a("money", -5000.0)]),
        ev("injury_comeback_story", "Back on the court", "{name}'s grind through rehab inspires fans everywhere.", "comeback", 0.2, &["pro", "overseas", "college"],
            vec![cond("flag:recent_return", "==", 1.0)], vec![a("life.reputation", 5.0), a("life.fame", 3.0), a("life.happiness", 6.0), Effect::unflag("recent_return")]),
        ev("rookie_hazing", "Rookie initiation", "The vets give {name} a rookie initiation. Carrying bags, a silly song, the usual.", "team", 0.08, &["pro"],
            vec![cond("years_pro", "<=", 1.0)], vec![a("life.friends", 3.0), a("life.happiness", 2.0), a("mood", 2.0)]),
        ev("vet_mentor", "A veteran takes you in", "A respected veteran teammate shows {name} how to handle the business of the league.", "team", 0.05, &["pro"],
            vec![cond("years_pro", "<=", 3.0)], vec![a("life.maturity", 4.0), a("attr.shot_iq", 1.0), a("life.dev_skill", 8.0)]),
        with_choices(cooldown(ev("retirement_thoughts", "Thinking about the end", "{name} wonders how much longer his body will let him do this.", "career", 0.10, &["pro", "overseas"],
            vec![cond("age", ">=", 34.0)], vec![]), 12), vec![
            ch("keep_going", "Keep playing", "One more run.", vec![a("life.happiness", 2.0)], "You're not done yet."),
            ch("last_season", "Announce this is the last season", "Fans will celebrate you. Your story gets a proper ending.", vec![Effect::flag("final_season"), a("life.fame", 3.0), a("life.happiness", 4.0)], "A farewell tour begins."),
        ]),
        ev("overseas_homesick", "Homesick", "Life overseas is hard: language, food, distance. {name} misses home.", "life", 0.08, &["overseas"],
            vec![], vec![a("life.happiness", -6.0), a("life.stress", 5.0), a("life.family", -2.0)]),
        ev("overseas_fans", "Fans embrace you", "The supporters' section chants {name}'s name every game.", "fame", 0.05, &["overseas"],
            vec![cond("player.ovr", ">=", 58.0)], vec![a("life.happiness", 6.0), a("life.fame", 3.0), a("life.stress", -4.0)]),
    ]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LifeDefs {
    pub stats: Vec<LifeStatDef>,
    pub activities: Vec<ActivityDef>,
    pub events: Vec<EventDef>,
}

impl Default for LifeDefs {
    fn default() -> Self {
        LifeDefs {
            stats: builtin_stats(),
            activities: builtin_activities(),
            events: builtin_life_events(),
        }
    }
}

// ----------------------------------------------------------------------------------------
// State
// ----------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Relationship {
    pub name: String,
    /// `parent`, `sibling`, `friend`, `coach`, `mentor`, `partner`, `agent`, `teammate`, `rival`...
    pub kind: String,
    pub closeness: f64,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PersonalFinance {
    pub cash: Money,
    pub investments: Money,
    pub assets: Money,
    pub debt: Money,
    /// Annual endorsement income.
    pub endorsements: Money,
    /// Lifestyle spending level 0 (frugal) - 100 (lavish).
    pub lifestyle: f64,
    pub income_this_year: Money,
    pub taxes_this_year: Money,
    pub lifetime_earnings: Money,
    pub lifetime_taxes: Money,
}

impl PersonalFinance {
    pub fn net_worth(&self) -> Money {
        self.cash + self.investments + self.assets - self.debt
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    /// Skill 0-100: better agents negotiate more money but charge the same % fee.
    pub skill: f64,
    pub fee_pct: f64,
    pub trust: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LifeEntry {
    pub season: Season,
    pub age: i32,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LifeState {
    pub stage: LifeStage,
    pub stats: BTreeMap<String, f64>,
    /// Time allocation: activity id → percent (sums to ~100).
    pub allocation: BTreeMap<String, f64>,
    pub relationships: Vec<Relationship>,
    pub finance: PersonalFinance,
    pub agent: Option<AgentInfo>,
    pub traits: Vec<String>,
    pub timeline: Vec<LifeEntry>,
    pub pending: Vec<PendingEvent>,
    /// Ticks since each event last fired.
    pub fired: BTreeMap<String, u32>,
    pub flags: std::collections::BTreeSet<String>,
    pub school: String,
    /// 9-12 in high school, 1-4 in college.
    pub class_year: u8,
    pub months: u32,
    /// Is the player currently academically eligible to play?
    pub eligible: bool,
}

impl LifeState {
    pub fn new(defs: &LifeDefs, stage: LifeStage, school: &str, class_year: u8) -> LifeState {
        let stats = defs
            .stats
            .iter()
            .map(|s| (s.id.clone(), s.default))
            .collect();
        let allocation = defs
            .activities
            .iter()
            .map(|a| (a.id.clone(), a.default_pct))
            .collect();
        LifeState {
            stage,
            stats,
            allocation,
            relationships: vec![],
            finance: PersonalFinance {
                lifestyle: 35.0,
                ..Default::default()
            },
            agent: None,
            traits: vec![],
            timeline: vec![],
            pending: vec![],
            fired: BTreeMap::new(),
            flags: Default::default(),
            school: school.into(),
            class_year,
            months: 0,
            eligible: true,
        }
    }

    pub fn stat(&self, id: &str) -> f64 {
        self.stats.get(id).copied().unwrap_or(0.0)
    }

    pub fn log(&mut self, season: Season, age: i32, text: impl Into<String>) {
        self.timeline.push(LifeEntry {
            season,
            age,
            text: text.into(),
        });
        if self.timeline.len() > 400 {
            self.timeline.remove(0);
        }
    }

    /// Set the time allocation. Values are rescaled so they sum to 100. Returns an error for unknown activities.
    pub fn set_allocation(
        &mut self,
        defs: &LifeDefs,
        wanted: &[(String, f64)],
    ) -> Result<(), String> {
        let mut alloc: BTreeMap<String, f64> = defs
            .activities
            .iter()
            .map(|a| (a.id.clone(), 0.0))
            .collect();
        for (k, v) in wanted {
            if !alloc.contains_key(k) {
                return Err(format!(
                    "Unknown activity '{k}'. Options: {}.",
                    defs.activities
                        .iter()
                        .map(|a| a.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if *v < 0.0 {
                return Err("Time shares can't be negative.".into());
            }
            alloc.insert(k.clone(), *v);
        }
        let total: f64 = alloc.values().sum();
        if total <= 0.0 {
            return Err("Give at least one activity some time.".into());
        }
        for v in alloc.values_mut() {
            *v = *v / total * 100.0;
        }
        self.allocation = alloc;
        Ok(())
    }

    /// Performance multiplier from mental state: happiness helps a little, stress and exhaustion hurt.
    pub fn performance_mod(&self) -> f64 {
        let h = (self.stat("happiness") - 60.0) / 1800.0;
        let s = ((self.stat("stress") - 65.0).max(0.0)) / 500.0;
        let e = ((35.0 - self.stat("energy")).max(0.0)) / 500.0;
        (1.0 + h - s - e).clamp(0.85, 1.03)
    }

    /// Multiplier on development from momentum stats.
    pub fn dev_bonus(&self) -> (f64, f64) {
        (
            1.0 + self.stat("dev_skill") / 100.0 * 0.6,
            1.0 + self.stat("dev_phys") / 100.0 * 0.6,
        )
    }
}

/// Cost-of-living index relative to 2024 (=1.0). Used to scale money events across eras.
pub fn cpi(year: Season) -> f64 {
    crate::era::interp(
        &[
            (1946, 0.095),
            (1960, 0.13),
            (1970, 0.19),
            (1980, 0.38),
            (1990, 0.6),
            (2000, 0.74),
            (2010, 0.88),
            (2024, 1.0),
            (2060, 2.4),
        ],
        year as f64,
    )
}

/// Effective personal income tax rate by income relative to the era's average pro salary.
pub fn tax_rate(year: Season, income: Money, avg_salary: Money) -> f64 {
    let r = income as f64 / avg_salary.max(1) as f64;
    let base = if r < 0.3 {
        0.10
    } else if r < 1.0 {
        0.20
    } else if r < 3.0 {
        0.30
    } else if r < 10.0 {
        0.37
    } else {
        0.42
    };
    // High-tax mid-century and 1970s: top rates were far higher.
    let era = crate::era::interp(
        &[
            (1946, 0.18),
            (1963, 0.20),
            (1970, 0.12),
            (1981, 0.05),
            (1987, -0.05),
            (2000, -0.02),
            (2024, 0.0),
        ],
        year as f64,
    );
    (base + if r >= 1.0 { era } else { era * 0.4 }).clamp(0.05, 0.75)
}

// ----------------------------------------------------------------------------------------
// Monthly tick
// ----------------------------------------------------------------------------------------

/// Inputs from the league needed for a monthly life tick.
pub struct LifeInputs<'a> {
    pub defs: &'a LifeDefs,
    pub year: Season,
    pub team_win_pct: f64,
    pub avg_salary: Money,
    /// Annual pay (pro salary, overseas contract, stipend...). Paid monthly.
    pub annual_income: Money,
    /// Is it the school year / season?
    pub intensity: f64,
    pub auto_decisions: bool,
    /// Life-sim switches from settings.
    pub school: bool,
    pub money: bool,
    pub relationships: bool,
    pub mental_health: bool,
    pub social_media: bool,
}

/// Apply one month of life: time allocation → stats, finances, then roll life events.
/// Returns log lines to show.
pub fn monthly_tick(p: &mut Player, inp: &LifeInputs, rng: &mut crate::rng::Rng) -> Vec<String> {
    let mut life = match p.life.take() {
        Some(l) => l,
        None => return vec![],
    };
    let mut log = vec![];
    let year = inp.year;
    let age = p.age(year);

    // 1. Time allocation → stat changes.
    let stage = life.stage.key().to_string();
    let mut money_gain = 0.0;
    for a in &inp.defs.activities {
        if !a.stages.is_empty() && !a.stages.contains(&stage) {
            continue;
        }
        let share = life.allocation.get(&a.id).copied().unwrap_or(0.0) / 100.0;
        if share <= 0.0 {
            continue;
        }
        for (k, d) in &a.effects {
            if k == "money" {
                if inp.money {
                    money_gain += d * share * cpi(year);
                }
            } else if k == "grades" && !inp.school {
                // school is switched off: grades never change
            } else {
                let v = life.stats.entry(k.clone()).or_insert(0.0);
                *v += d * share;
            }
        }
    }
    // 2. Drift + clamp.
    for s in &inp.defs.stats {
        let v = life.stats.entry(s.id.clone()).or_insert(s.default);
        if s.drift_rate > 0.0 {
            *v += (s.drift_to - *v) * s.drift_rate;
        }
        *v = v.clamp(s.min, s.max);
    }
    // Maturity grows with age; fame decays slowly if you aren't playing.
    {
        let m = life.stats.entry("maturity".into()).or_insert(40.0);
        *m = (*m + 0.35).min(100.0);
    }
    // 3. Skill momentum feeds mood/energy → fitness
    p.fitness = (p.fitness as f64 * 0.7 + life.stat("energy") * 0.3).clamp(0.0, 100.0) as f32;
    p.mood.overall = (p.mood.overall as f64 * 0.8
        + (life.stat("happiness") - life.stat("stress") * 0.25 + 18.0) * 0.2)
        .clamp(0.0, 100.0) as f32;

    // 4. Money.
    if inp.money {
        let f = &mut life.finance;
        let monthly_income =
            inp.annual_income as f64 / 12.0 + f.endorsements as f64 / 12.0 + money_gain;
        let tax = monthly_income
            * tax_rate(
                year,
                (inp.annual_income + f.endorsements).max(1),
                inp.avg_salary,
            );
        let cost = cpi(year) * (1200.0 + 4800.0 * f.lifestyle / 100.0)
            + if life.stage == LifeStage::Pro || life.stage == LifeStage::Overseas {
                cpi(year) * 3000.0
            } else {
                0.0
            };
        // Agents take a cut of salary + endorsements
        let agent_fee = life
            .agent
            .as_ref()
            .map(|a| a.fee_pct / 100.0 * monthly_income)
            .unwrap_or(0.0);
        let f = &mut life.finance;
        f.income_this_year += monthly_income as i64;
        f.taxes_this_year += tax as i64;
        f.lifetime_earnings += monthly_income as i64;
        f.lifetime_taxes += tax as i64;
        let net = monthly_income - tax - cost - agent_fee;
        f.cash += net as i64;
        // Investments return ~0.5%/month ± noise; cash shortage becomes debt (and costs interest).
        f.investments = (f.investments as f64 * (1.0 + rng.gauss(0.005, 0.03))) as i64;
        if f.cash < 0 {
            f.debt += -f.cash;
            f.cash = 0;
        } else if f.debt > 0 && f.cash > f.debt {
            f.cash -= f.debt;
            f.debt = 0;
        }
        f.debt = (f.debt as f64 * 1.01) as i64;
        // Excess cash goes into investments for mature players.
        if f.cash > (cpi(year) * 400_000.0) as i64
            && life.stats.get("maturity").copied().unwrap_or(0.0) > 55.0
        {
            let move_amt = f.cash / 2;
            f.cash -= move_amt;
            f.investments += move_amt;
        }
    }

    // 5. Academic eligibility.
    if matches!(life.stage, LifeStage::HighSchool | LifeStage::College) {
        let g = life.stat("grades");
        let was = life.eligible;
        life.eligible = !inp.school || g >= 2.0;
        if was && !life.eligible {
            log.push(format!(
                "{} is now academically INELIGIBLE (GPA {:.2}). Raise your grades to play again.",
                p.name(),
                g
            ));
        } else if !was && life.eligible {
            log.push(format!("{} regained academic eligibility.", p.name()));
        }
    }

    // 6. Events.
    life.months += 1;
    for v in life.fired.values_mut() {
        *v = v.saturating_add(1);
    }
    let intensity = inp.intensity;
    if intensity > 0.0 {
        let mut off: Vec<&str> = vec![];
        if !inp.relationships {
            off.extend(["romance", "family", "social"]);
        }
        if !inp.mental_health {
            off.push("health");
        }
        if !inp.social_media {
            off.extend(["media", "fame"]);
        }
        if !inp.school {
            off.push("school");
        }
        if !inp.money {
            off.push("money");
        }
        let filtered: Vec<EventDef> = inp
            .defs
            .events
            .iter()
            .filter(|e| !off.contains(&e.category.as_str()))
            .cloned()
            .collect();
        let defs_events = &filtered;
        let mut news = vec![];
        let pending_before = life.pending.len();
        let mut ctx = LifeCtx {
            player: p,
            life: &mut life,
            year,
            team_win_pct: inp.team_win_pct,
            news: &mut news,
            cpi: cpi(year),
        };
        let out = roll(defs_events, &mut ctx, rng, intensity, 2, inp.auto_decisions);
        log.extend(out.log);
        ctx.life.pending.extend(out.pending);
        let _ = pending_before;
        for n in news {
            log.push(n);
        }
    }
    for l in log.clone() {
        life.log(year, age, l);
    }
    // Keep timeline bounded: pending events cap
    life.pending.truncate(6);
    p.life = Some(life);
    log
}

/// Resolve a pending life event for the user's player.
pub fn resolve_event(
    p: &mut Player,
    defs: &LifeDefs,
    year: Season,
    event_id: &str,
    choice_id: &str,
) -> Result<Vec<String>, String> {
    let mut life = p.life.take().ok_or("This player has no life sim.")?;
    let idx = life.pending.iter().position(|e| e.event_id == event_id);
    let idx = match idx {
        Some(i) => i,
        None => {
            p.life = Some(life);
            return Err(format!("There is no pending event '{event_id}'."));
        }
    };
    let pe = life.pending[idx].clone();
    let mut news = vec![];
    let res;
    {
        let mut ctx = LifeCtx {
            player: p,
            life: &mut life,
            year,
            team_win_pct: 0.5,
            news: &mut news,
            cpi: cpi(year),
        };
        res = resolve(&pe, choice_id, &mut ctx);
    }
    let _ = defs;
    match res {
        Ok(mut log) => {
            life.pending.remove(idx);
            log.extend(news);
            let age = p.age(year);
            for l in &log {
                life.log(year, age, l.clone());
            }
            p.life = Some(life);
            Ok(log)
        }
        Err(e) => {
            p.life = Some(life);
            Err(e)
        }
    }
}

/// Bridges the event engine to a player + life state.
pub struct LifeCtx<'a> {
    pub player: &'a mut Player,
    pub life: &'a mut LifeState,
    pub year: Season,
    pub team_win_pct: f64,
    pub news: &'a mut Vec<String>,
    pub cpi: f64,
}

impl<'a> EventContext for LifeCtx<'a> {
    fn var(&self, name: &str) -> f64 {
        let p = &*self.player;
        if let Some(k) = name.strip_prefix("life.") {
            return self.life.stat(k);
        }
        if let Some(k) = name.strip_prefix("attr.") {
            return Attr::from_key(k).map(|a| p.attrs.get(a)).unwrap_or(0.0);
        }
        if let Some(k) = name.strip_prefix("custom.") {
            return p.custom.get(k).copied().unwrap_or(0.0);
        }
        match name {
            "age" => p.age(self.year) as f64,
            "year" => self.year as f64,
            "player.ovr" | "ovr" => p.ovr as f64,
            "player.potential" | "potential" => p.potential as f64,
            "height" => p.height_in as f64,
            "money" => self.life.finance.cash as f64 / self.cpi,
            "net_worth" => self.life.finance.net_worth() as f64 / self.cpi,
            "mood" => p.mood.overall as f64,
            "wear" => p.wear as f64,
            "years_pro" => p.years_pro as f64,
            "team.win_pct" => self.team_win_pct,
            "class_year" => self.life.class_year as f64,
            "injured" => {
                if p.is_injured() {
                    1.0
                } else {
                    0.0
                }
            }
            "eligible" if self.life.eligible => 1.0,
            _ => 0.0,
        }
    }
    fn has_flag(&self, name: &str) -> bool {
        self.life.flags.contains(name) || self.player.flags.contains(name)
    }
    fn stage(&self) -> String {
        self.life.stage.key().to_string()
    }
    fn year(&self) -> i32 {
        self.year
    }
    fn subject_name(&self) -> String {
        self.player.name()
    }
    fn since_fired(&self, id: &str) -> u32 {
        self.life.fired.get(id).copied().unwrap_or(u32::MAX)
    }
    fn mark_fired(&mut self, id: &str) {
        self.life.fired.insert(id.to_string(), 0);
    }
    fn apply(&mut self, e: &Effect, log: &mut Vec<String>) {
        let v = e.value;
        let target = e.target.as_str();
        let apply_num = |cur: f64, op: &str, v: f64| match op {
            "set" => v,
            "mul" => cur * v,
            _ => cur + v,
        };
        if let Some(k) = target.strip_prefix("life.") {
            let cur = self.life.stats.get(k).copied().unwrap_or(0.0);
            self.life
                .stats
                .insert(k.to_string(), apply_num(cur, &e.op, v));
            // clamp is applied next tick by the drift step; clamp roughly now for display
            let nv = self.life.stats[k];
            let clamped = if k == "grades" {
                nv.clamp(0.0, 4.0)
            } else {
                nv.clamp(0.0, 100.0)
            };
            self.life.stats.insert(k.to_string(), clamped);
        } else if let Some(k) = target.strip_prefix("attr.") {
            if let Some(a) = Attr::from_key(k) {
                let cur = self.player.attrs.get(a);
                self.player.attrs.set(a, apply_num(cur, &e.op, v));
                self.player.recompute_ovr();
            }
        } else if let Some(k) = target.strip_prefix("custom.") {
            let cur = self.player.custom.get(k).copied().unwrap_or(0.0);
            self.player
                .custom
                .insert(k.to_string(), apply_num(cur, &e.op, v));
        } else if let Some(k) = target.strip_prefix("rel.") {
            for r in self.life.relationships.iter_mut().filter(|r| r.kind == k) {
                r.closeness = apply_num(r.closeness, &e.op, v).clamp(0.0, 100.0);
            }
        } else {
            match target {
                "money" => {
                    let amt = (v * self.cpi) as i64;
                    if e.op == "set" {
                        self.life.finance.cash = amt;
                    } else {
                        self.life.finance.cash += amt;
                    }
                    if v > 0.0 {
                        self.life.finance.lifetime_earnings += amt;
                    }
                }
                "money_gamble" => {
                    // Big risky investment: 40% lose half the cash, 60% gain 40%... resolved deterministically from the year.
                    let stake = self.life.finance.cash / 3;
                    let roll = ((self.player.id as i64 * 7919
                        + self.year as i64 * 104729
                        + self.life.months as i64 * 31)
                        % 100) as f64
                        / 100.0;
                    if roll < 0.42 {
                        self.life.finance.cash -= stake / 2;
                        self.news.push(format!(
                            "The investment went badly: -{}.",
                            fmt_money(stake / 2)
                        ));
                    } else {
                        self.life.finance.cash += stake * 4 / 10;
                        self.news.push(format!(
                            "The investment paid off: +{}.",
                            fmt_money(stake * 4 / 10)
                        ));
                    }
                }
                "endorsement" => self.life.finance.endorsements += (v * self.cpi) as i64,
                "asset" => self.life.finance.assets += (v * self.cpi) as i64,
                "height" => {
                    self.player.height_in =
                        (self.player.height_in as f64 + v).clamp(60.0, 94.0) as u8
                }
                "mood" => {
                    self.player.mood.overall =
                        (apply_num(self.player.mood.overall as f64, &e.op, v)).clamp(0.0, 100.0)
                            as f32
                }
                "wear" => self.player.wear = (self.player.wear as f64 + v).clamp(0.0, 100.0) as f32,
                "fitness" => {
                    self.player.fitness = (self.player.fitness as f64 + v).clamp(0.0, 100.0) as f32
                }
                "potential" => {
                    self.player.potential =
                        (self.player.potential as f64 + v).clamp(30.0, 99.0) as u8
                }
                "flag" => match e.op.as_str() {
                    "flag_remove" => {
                        self.life.flags.remove(&e.text);
                        self.player.flags.remove(&e.text);
                    }
                    _ => {
                        if !e.text.is_empty() {
                            self.life.flags.insert(e.text.clone());
                        }
                    }
                },
                "news" => self.news.push(e.text.clone()),
                _ => {}
            }
        }
        let _ = log;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_defs_validate() {
        let d = LifeDefs::default();
        let errs = validate(&d.events);
        assert!(errs.is_empty(), "{errs:?}");
        let total: f64 = d.activities.iter().map(|a| a.default_pct).sum();
        assert!((total - 100.0).abs() < 1e-9);
        // every stat an activity touches exists (or is the special 'money')
        let ids: Vec<&str> = d.stats.iter().map(|s| s.id.as_str()).collect();
        for a in &d.activities {
            for (k, _) in &a.effects {
                assert!(
                    k == "money" || ids.contains(&k.as_str()),
                    "activity {} touches unknown stat {k}",
                    a.id
                );
            }
        }
        assert!(d.events.len() >= 30);
    }

    #[test]
    fn tax_and_cpi_behave() {
        assert!(cpi(1950) < cpi(2000) && cpi(2000) < cpi(2024));
        assert!(tax_rate(1965, 1_000_000, 20_000) > tax_rate(2020, 1_000_000, 20_000));
    }
}
