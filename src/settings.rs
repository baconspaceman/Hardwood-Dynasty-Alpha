//! The settings system: "settings within settings" that stay easy to understand.
//!
//! Every setting is a *definition* (name, plain-English description, type, default, what the low
//! and high ends of a slider do) plus a *value*. Definitions are plain data (`SettingDef`), so
//! mods can add or change them. Presets are just bundles of values.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SettingKind {
    /// On / off switch.
    Toggle,
    /// Number between `min` and `max`, moved in steps of `step`.
    Slider { min: f64, max: f64, step: f64 },
    /// One of several named options.
    Choice { options: Vec<ChoiceOption> },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
    /// What picking this option does, in plain English.
    pub explain: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SettingValue {
    Bool(bool),
    Num(f64),
    Text(String),
}

impl SettingValue {
    pub fn show(&self) -> String {
        match self {
            SettingValue::Bool(b) => {
                if *b {
                    "ON".into()
                } else {
                    "OFF".into()
                }
            }
            SettingValue::Num(n) => {
                if (n - n.round()).abs() < 1e-9 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n:.2}")
                }
            }
            SettingValue::Text(t) => t.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingDef {
    /// Stable id like `injuries.frequency`. The part before the dot is the category.
    pub key: String,
    pub name: String,
    pub description: String,
    pub kind: SettingKind,
    pub default: SettingValue,
    /// Sliders: what the LOW end does.
    #[serde(default)]
    pub low_note: String,
    /// Sliders: what the HIGH end does. For toggles this describes what ON does.
    #[serde(default)]
    pub high_note: String,
    /// Hidden from the basic list until the player asks for "advanced".
    #[serde(default)]
    pub advanced: bool,
}

impl SettingDef {
    pub fn category(&self) -> &str {
        self.key.split('.').next().unwrap_or("misc")
    }
}

fn toggle(
    key: &str,
    name: &str,
    desc: &str,
    default: bool,
    on: &str,
    off: &str,
    advanced: bool,
) -> SettingDef {
    SettingDef {
        key: key.into(),
        name: name.into(),
        description: desc.into(),
        kind: SettingKind::Toggle,
        default: SettingValue::Bool(default),
        low_note: off.into(),
        high_note: on.into(),
        advanced,
    }
}

#[allow(clippy::too_many_arguments)]
fn slider(
    key: &str,
    name: &str,
    desc: &str,
    min: f64,
    max: f64,
    step: f64,
    default: f64,
    low: &str,
    high: &str,
    advanced: bool,
) -> SettingDef {
    SettingDef {
        key: key.into(),
        name: name.into(),
        description: desc.into(),
        kind: SettingKind::Slider { min, max, step },
        default: SettingValue::Num(default),
        low_note: low.into(),
        high_note: high.into(),
        advanced,
    }
}

fn choice(
    key: &str,
    name: &str,
    desc: &str,
    default: &str,
    opts: &[(&str, &str, &str)],
    advanced: bool,
) -> SettingDef {
    SettingDef {
        key: key.into(),
        name: name.into(),
        description: desc.into(),
        kind: SettingKind::Choice {
            options: opts
                .iter()
                .map(|(id, label, explain)| ChoiceOption {
                    id: id.to_string(),
                    label: label.to_string(),
                    explain: explain.to_string(),
                })
                .collect(),
        },
        default: SettingValue::Text(default.into()),
        low_note: String::new(),
        high_note: String::new(),
        advanced,
    }
}

/// Human names for categories, in menu order.
pub const CATEGORIES: &[(&str, &str)] = &[
    ("sim", "Game Simulation"),
    ("realism", "Era & Realism"),
    ("economy", "Money & Salary Cap"),
    ("injuries", "Injuries & Health"),
    ("progression", "Player Development"),
    ("draft", "Draft, Scouting & Recruiting"),
    ("ai", "Computer GMs"),
    ("story", "Storylines & News"),
    ("life", "Player Life Sim"),
    ("college", "College Basketball"),
    ("difficulty", "Difficulty"),
    ("display", "Display"),
];

/// All built-in setting definitions.
pub fn builtin_defs() -> Vec<SettingDef> {
    vec![
        // ---------------- Game simulation ----------------
        slider("sim.home_court", "Home-court advantage", "How much playing at home helps, in points per game.", 0.0, 6.0, 0.25, 2.5,
            "0 = a neutral gym: home and away teams are equal.", "6 = a roaring crowd: home teams get a huge edge.", false),
        slider("sim.randomness", "Game randomness", "Scales luck in every game. Lower means the better team wins more often.", 0.5, 1.6, 0.05, 1.0,
            "0.5 = chalk: favorites almost always win.", "1.6 = chaos: upsets and wild shooting nights are common.", false),
        slider("sim.parity", "League parity", "Pulls every team's strength toward the middle (or pushes them apart).", 0.0, 1.0, 0.05, 0.0,
            "0 = natural: dynasties and tank-jobs can form.", "1 = everyone is close; very few dominant teams.", false),
        slider("sim.star_power", "Star power", "How much the best player's talent bends the game. Higher makes superstars more dominant.", 0.5, 1.5, 0.05, 1.0,
            "0.5 = depth wins: stars matter less.", "1.5 = heliocentric: a superstar can carry a team alone.", false),
        slider("sim.fatigue", "Fatigue impact", "How quickly players tire and how much tiredness hurts their play.", 0.0, 2.0, 0.1, 1.0,
            "0 = nobody gets tired.", "2 = stamina is a real strategic problem.", false),
        slider("sim.foul_rate", "Foul frequency", "Scales how often fouls are called.", 0.5, 1.6, 0.05, 1.0,
            "0.5 = let them play: few whistles.", "1.6 = whistle-happy: lots of free throws and foul trouble.", false),
        toggle("sim.hot_hand", "Hot & cold streaks", "Players can get 'in the zone' or go cold within a game.", true,
            "ON: shooters ride streaks (small effect).", "OFF: every shot is independent.", false),
        toggle("sim.clutch", "Clutch factor", "Late-game close situations use players' Clutch rating.", true,
            "ON: clutch players matter in the last 5 minutes.", "OFF: the Clutch rating is ignored.", false),
        toggle("sim.play_by_play", "Record play-by-play", "Keep a full text log of every game. Uses more memory.", false,
            "ON: you can read any game's play-by-play.", "OFF: only box scores are stored (faster, smaller saves).", true),
        toggle("sim.coach_in_game", "Coaches adjust in-game", "Coaches make substitutions and tactical adjustments during games.", true,
            "ON: coach skill shows up in close games.", "OFF: fixed rotations.", true),
        slider("sim.possession_detail", "Possession detail", "How many special situations the engine models (late-game fouling, 2-for-1, press, hack-a-player).", 0.0, 1.0, 0.5, 1.0,
            "0 = simple: no late-game strategy.", "1 = full late-game chess.", true),

        // ---------------- Era & realism ----------------
        choice("realism.rules_mode", "Rule set", "Which rules the league plays by.", "historical",
            &[("historical", "Historical (era-correct)", "Rules change by year exactly as they did in real basketball: no 3-pointer before 1979, no shot clock before 1954, play-in from 2021, etc."),
              ("modern", "Modern everywhere", "Today's rules in every season, even 1950."),
              ("custom_year", "Frozen at a chosen year", "Use one year's rules for the whole game (see 'Frozen rules year').")], false),
        slider("realism.frozen_rules_year", "Frozen rules year", "Only used when Rule set is 'Frozen at a chosen year'.", 1946.0, 2040.0, 1.0, 1996.0,
            "Early years: low-scoring, slower game.", "Recent years: spacing and 3-pointers.", true),
        choice("realism.cap_mode", "Salary cap behaviour", "How the salary cap moves from year to year.", "historical",
            &[("historical", "Historical, then simulated", "Real cap numbers up to the latest known season, then it keeps growing based on simulated league revenue."),
              ("simulated", "Always simulated", "The cap starts at the era's level and then follows league revenue from your own simulation."),
              ("frozen", "Frozen", "The cap never changes. Salaries get relatively tighter over time.")], false),
        toggle("realism.cap_before_1984", "Salary cap before 1984", "Real basketball had no cap before 1984-85. Turn on to force a (scaled) soft cap in early years.", false,
            "ON: a soft cap exists in all years.", "OFF: no cap before 1984 (teams pay what they can afford).", false),
        toggle("realism.historical_events", "Historical league events", "Lockouts, a pandemic season, league mergers and expansion arrive at their real-world dates.", true,
            "ON: the league lives through the same kinds of events as history.", "OFF: the league is event-free unless random events are on.", false),
        slider("realism.international_flow", "International player flow", "How many players come from outside the US relative to history.", 0.0, 2.0, 0.1, 1.0,
            "0 = no international players.", "2 = double the historical share.", false),
        choice("realism.high_school_rule", "High-school players", "Whether players can jump from high school to the pros.", "historical",
            &[("historical", "Historical", "Allowed only in the years it was really possible (roughly 1995-2005, plus rare earlier hardship cases)."),
              ("always", "Always allowed", "Any 18-year-old may be drafted in any era."),
              ("never", "Never allowed", "Players must play college, overseas or in the G League first.")], false),
        toggle("realism.aba_merger", "Rival league merger", "A rival league (ABA-style) can exist from 1967 and merge into the league around 1976.", true,
            "ON: a rival league appears, and then merges.", "OFF: single league throughout.", true),

        // ---------------- Economy ----------------
        slider("economy.cap_growth", "Cap growth speed", "Multiplier on how fast the simulated salary cap grows.", 0.0, 2.0, 0.1, 1.0,
            "0 = cap barely moves.", "2 = cap booms (like the 2016 TV-money jump, every year).", false),
        slider("economy.revenue_volatility", "Revenue volatility", "How wildly league revenue swings (TV deals, recessions, pandemics).", 0.0, 2.0, 0.1, 1.0,
            "0 = stable growth.", "2 = booms and busts.", true),
        toggle("economy.luxury_tax", "Luxury tax", "Teams over the tax line pay extra (when the era has it).", true,
            "ON: era-correct luxury tax.", "OFF: no tax; spend freely.", false),
        toggle("economy.aprons", "Tax aprons", "Modern restrictions on teams far over the tax (2023-style first and second apron).", true,
            "ON: aprons limit trades and signings.", "OFF: only the tax applies.", false),
        toggle("economy.max_contracts", "Max contracts", "Cap on any one player's salary (as a % of the cap).", true,
            "ON: era-correct maximum.", "OFF: players can be paid anything.", false),
        slider("economy.player_greed", "Player greed", "How much money matters to players when choosing a team.", 0.0, 2.0, 0.1, 1.0,
            "0 = players sign for winning and role.", "2 = highest bidder wins.", false),
        slider("economy.ticket_sensitivity", "Fan price sensitivity", "How much attendance drops when you raise ticket prices.", 0.2, 2.0, 0.1, 1.0,
            "0.2 = fans pay anything.", "2 = fans punish price hikes.", false),
        toggle("economy.owner_chase_mode", "Owner chase mode", "Owners may deliberately overspend to chase a title.", true,
            "ON: the Chase Mode planner is available.", "OFF: hidden.", true),

        // ---------------- Injuries ----------------
        slider("injuries.frequency", "Injury frequency", "How often players get hurt.", 0.0, 3.0, 0.1, 1.0,
            "0 = nobody ever gets hurt.", "3 = injury-plagued.", false),
        slider("injuries.severity", "Injury severity", "How long injuries last.", 0.4, 2.5, 0.1, 1.0,
            "0.4 = quick recoveries.", "2.5 = long layoffs.", false),
        slider("injuries.career_ending", "Career-ending risk", "Chance that a serious injury ends a career.", 0.0, 5.0, 0.25, 1.0,
            "0 = never.", "5 = brutal.", false),
        slider("injuries.permanent_damage", "Lasting damage", "How much big injuries permanently reduce athleticism.", 0.0, 2.0, 0.1, 1.0,
            "0 = full recovery always.", "2 = knees and Achilles are never the same.", false),
        toggle("injuries.in_game", "In-game injuries", "Players can get hurt during games and leave mid-game.", true,
            "ON: injuries can happen mid-game.", "OFF: injuries are only found between games.", false),
        toggle("injuries.load_management", "Load management", "Coaches rest stars to reduce injury risk.", true,
            "ON: stars sit some back-to-backs.", "OFF: everyone plays.", false),
        toggle("injuries.rush_back", "Play through injuries", "Allow coaches (or you) to play injured players at reduced ability and higher re-injury risk.", true,
            "ON: you can rush a player back.", "OFF: players only return when fully healed.", true),
        toggle("injuries.hide_details", "Hide medical details", "Hide exact recovery timelines (shows only 'out' or 'questionable').", false,
            "ON: more mystery.", "OFF: full medical reports.", true),

        // ---------------- Progression ----------------
        slider("progression.speed", "Development speed", "How fast young players improve.", 0.5, 1.5, 0.05, 1.0,
            "0.5 = slow burns.", "1.5 = rapid growth.", false),
        slider("progression.variance", "Development randomness", "How unpredictable growth is.", 0.0, 2.0, 0.1, 1.0,
            "0 = ratings follow the projected curve.", "2 = busts and breakouts are common.", false),
        slider("progression.late_bloomers", "Late bloomers", "Chance that a player peaks much later than usual.", 0.0, 2.0, 0.1, 1.0,
            "0 = none.", "2 = many.", true),
        slider("progression.longevity", "Career length", "How long players stay good as they age.", 0.5, 1.6, 0.05, 1.0,
            "0.5 = players fall off at 30.", "1.6 = stars play into their 40s.", false),
        toggle("progression.sports_science", "Modern sports science", "Later eras extend careers because of better training and medicine.", true,
            "ON: longevity improves over time.", "OFF: same aging curve in every era.", true),
        toggle("progression.hidden_potential", "Hidden potential", "Hide each player's true potential (you only see scouting estimates).", true,
            "ON: potential is an estimate; you can be fooled.", "OFF: you see exact potential.", false),

        // ---------------- Draft / scouting ----------------
        slider("draft.scouting_fog", "Scouting uncertainty", "How noisy prospect ratings are before you scout them well.", 0.0, 2.0, 0.1, 1.0,
            "0 = you see true ratings.", "2 = prospects are a mystery.", false),
        slider("draft.generational_rate", "Generational talents", "How often a franchise-changing prospect appears.", 0.0, 3.0, 0.1, 1.0,
            "0 = never.", "3 = frequently.", false),
        slider("draft.class_variance", "Draft class strength swings", "How much draft classes differ in quality.", 0.0, 2.0, 0.1, 1.0,
            "0 = every class is average.", "2 = some classes are loaded, some are barren.", true),
        choice("draft.lottery_mode", "Draft order", "How the draft order is decided.", "historical",
            &[("historical", "Historical", "Coin flip, then the real lottery systems as they existed in each era."),
              ("worst_first", "Worst record first", "No lottery ever."),
              ("flat_lottery", "Flat lottery", "Every non-playoff team has equal odds."),
              ("random", "Pure random", "Complete lottery for all 30 teams.")], false),
        toggle("draft.draft_and_stash", "Draft-and-stash", "Teams may draft overseas players and leave them overseas.", true,
            "ON: era-correct stashing.", "OFF: drafted players must report.", true),

        // ---------------- AI ----------------
        slider("ai.trade_pickiness", "Trade AI pickiness", "How hard computer GMs are to fleece.", 0.5, 2.0, 0.1, 1.0,
            "0.5 = pushovers.", "2 = they only accept clearly winning trades.", false),
        slider("ai.trade_frequency", "AI-to-AI trades", "How often computer teams trade with each other.", 0.0, 2.0, 0.1, 1.0,
            "0 = none.", "2 = constant wheeling and dealing.", false),
        slider("ai.free_agency_aggression", "AI free-agency aggression", "How aggressively computer teams chase free agents.", 0.5, 1.5, 0.05, 1.0,
            "0.5 = cautious.", "1.5 = big spenders.", false),
        toggle("ai.smart_rebuilds", "Smart rebuilding", "AI teams recognise when to tank, retool or contend.", true,
            "ON: AI teams have a direction.", "OFF: AI teams just try to win.", true),
        toggle("ai.superteams", "Allow superteams", "Stars can team up by choice in free agency.", true,
            "ON: stars may join forces.", "OFF: stars tend to spread out.", false),

        // ---------------- Story ----------------
        slider("story.intensity", "Storyline intensity", "How much drama the story engine generates.", 0.0, 2.0, 0.1, 1.0,
            "0 = quiet: just the facts.", "2 = soap opera: feuds, comebacks and scandals.", false),
        toggle("story.random_events", "Random events", "Declarative random events (from content packs and mods) can fire.", true,
            "ON: surprises happen.", "OFF: only scheduled history.", false),
        toggle("story.media", "Media & headlines", "Generate news headlines and rumours.", true,
            "ON: news feed active.", "OFF: no news feed.", false),

        // ---------------- Life sim ----------------
        slider("life.intensity", "Life-sim depth", "How much off-court life management a Player career includes.", 0.0, 2.0, 0.5, 1.0,
            "0 = basketball only.", "2 = full life: school, family, money, friends, health.", false),
        toggle("life.school", "School & grades", "High school and college classes affect eligibility.", true,
            "ON: grades matter.", "OFF: no school stress.", false),
        toggle("life.relationships", "Family, friends & romance", "Relationships affect mood and decisions.", true,
            "ON: relationships matter.", "OFF: no relationships.", false),
        toggle("life.money", "Personal finances", "Track your player's money, taxes, bills and endorsements.", true,
            "ON: you manage personal money.", "OFF: money is automatic.", false),
        toggle("life.mental_health", "Mental health & stress", "Stress, confidence and burnout affect play.", true,
            "ON: you manage mental health.", "OFF: ignored.", false),
        toggle("life.social_media", "Social media & reputation", "Public image affects endorsements and fan support.", true,
            "ON: your image matters.", "OFF: no public image system.", false),
        toggle("life.auto_decisions", "Auto-resolve minor decisions", "Skip small life choices and let your agent/family handle them.", false,
            "ON: fewer prompts.", "OFF: you decide everything.", false),

        // ---------------- College ----------------
        slider("college.programs", "Number of college programs", "How many fictional college teams exist.", 16.0, 200.0, 8.0, 96.0,
            "16 = a tiny college world.", "200 = a full landscape (slower sims).", false),
        toggle("college.nil", "NIL money", "Players can be paid by boosters and brands (modern era).", true,
            "ON: NIL era-correct (2021+).", "OFF: amateurism strictly enforced.", false),
        toggle("college.transfer_portal", "Transfer portal", "College players can transfer freely (modern era).", true,
            "ON: free transfers in the modern era.", "OFF: transfers are rare.", false),
        toggle("college.tournament", "Postseason tournament", "A national tournament decides a college champion.", true,
            "ON: era-correct tournament field.", "OFF: no tournament.", false),

        // ---------------- Difficulty ----------------
        toggle("difficulty.can_be_fired", "Job security", "Owners can fire you as GM or coach.", true,
            "ON: results matter.", "OFF: you cannot be fired.", false),
        slider("difficulty.owner_patience", "Owner patience", "How long owners tolerate losing.", 0.5, 2.0, 0.1, 1.0,
            "0.5 = impatient.", "2 = patient.", false),
        slider("difficulty.budget_strictness", "Budget strictness", "How strictly owners punish overspending.", 0.0, 2.0, 0.1, 1.0,
            "0 = spend freely.", "2 = every dollar matters.", false),
        toggle("difficulty.trade_assist", "Trade assistant", "The game suggests ways to make a trade work under the cap.", true,
            "ON: helpful hints.", "OFF: you figure it out.", false),

        // ---------------- Display ----------------
        choice("display.rating_scale", "Rating scale", "How ratings are shown.", "2k",
            &[("2k", "0-99 (2K style)", "Video-game style overall ratings."),
              ("scout", "20-80 scouting grades", "Traditional scouting scale.")], false),
        toggle("display.imperial", "Imperial units", "Feet/inches and pounds (off = metric).", true,
            "ON: ft/in and lb.", "OFF: cm and kg.", false),
        toggle("display.explain", "Explain everything", "Show a short plain-English hint beside every screen and number.", true,
            "ON: beginner-friendly hints.", "OFF: clean, minimal screens.", false),
    ]
}

/// A named bundle of setting values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub values: BTreeMap<String, SettingValue>,
}

pub fn builtin_presets() -> Vec<Preset> {
    use SettingValue::*;
    let mk = |id: &str, name: &str, desc: &str, vals: Vec<(&str, SettingValue)>| Preset {
        id: id.into(),
        name: name.into(),
        description: desc.into(),
        values: vals.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
    };
    vec![
        mk(
            "balanced",
            "Balanced",
            "The default experience: realistic but forgiving.",
            vec![],
        ),
        mk(
            "arcade",
            "Arcade",
            "Fewer injuries, stars dominate, easier owner. Fun and fast.",
            vec![
                ("injuries.frequency", Num(0.4)),
                ("injuries.career_ending", Num(0.0)),
                ("sim.star_power", Num(1.3)),
                ("difficulty.owner_patience", Num(2.0)),
                ("difficulty.can_be_fired", Bool(false)),
                ("sim.randomness", Num(1.2)),
                ("life.intensity", Num(0.5)),
                ("progression.speed", Num(1.2)),
            ],
        ),
        mk(
            "sim",
            "Sim",
            "A serious simulation: scouting fog, real injuries, smart AI.",
            vec![
                ("injuries.frequency", Num(1.2)),
                ("injuries.career_ending", Num(1.5)),
                ("draft.scouting_fog", Num(1.3)),
                ("ai.trade_pickiness", Num(1.4)),
                ("progression.variance", Num(1.3)),
                ("difficulty.budget_strictness", Num(1.3)),
            ],
        ),
        mk(
            "hardcore",
            "Hardcore Realism",
            "Brutal injuries, picky AI, impatient owners, hidden potential, full life sim.",
            vec![
                ("injuries.frequency", Num(1.6)),
                ("injuries.severity", Num(1.4)),
                ("injuries.career_ending", Num(2.5)),
                ("injuries.permanent_damage", Num(1.5)),
                ("draft.scouting_fog", Num(1.6)),
                ("ai.trade_pickiness", Num(1.8)),
                ("difficulty.owner_patience", Num(0.7)),
                ("difficulty.budget_strictness", Num(1.6)),
                ("life.intensity", Num(2.0)),
                ("sim.randomness", Num(1.15)),
            ],
        ),
        mk(
            "sandbox",
            "Sandbox",
            "No fear: no injuries, no firing, no luxury tax. Build whatever you want.",
            vec![
                ("injuries.frequency", Num(0.0)),
                ("difficulty.can_be_fired", Bool(false)),
                ("economy.luxury_tax", Bool(false)),
                ("economy.aprons", Bool(false)),
                ("ai.trade_pickiness", Num(0.5)),
                ("difficulty.budget_strictness", Num(0.0)),
            ],
        ),
    ]
}

/// The live settings of a league: definitions + current values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(skip, default = "builtin_defs")]
    defs: Vec<SettingDef>,
    values: BTreeMap<String, SettingValue>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings::with_defs(builtin_defs())
    }
}

impl Settings {
    pub fn with_defs(defs: Vec<SettingDef>) -> Self {
        let values = defs
            .iter()
            .map(|d| (d.key.clone(), d.default.clone()))
            .collect();
        Settings { defs, values }
    }

    /// Re-attach definitions after loading a save (definitions are not saved; values are).
    pub fn set_defs(&mut self, defs: Vec<SettingDef>) {
        for d in &defs {
            self.values
                .entry(d.key.clone())
                .or_insert_with(|| d.default.clone());
        }
        self.defs = defs;
    }

    pub fn defs(&self) -> &[SettingDef] {
        &self.defs
    }

    pub fn def(&self, key: &str) -> Option<&SettingDef> {
        self.defs.iter().find(|d| d.key == key)
    }

    pub fn raw(&self, key: &str) -> Option<&SettingValue> {
        self.values.get(key)
    }

    pub fn bool(&self, key: &str) -> bool {
        match self.values.get(key) {
            Some(SettingValue::Bool(b)) => *b,
            _ => match self.def(key).map(|d| &d.default) {
                Some(SettingValue::Bool(b)) => *b,
                _ => false,
            },
        }
    }

    pub fn num(&self, key: &str) -> f64 {
        match self.values.get(key) {
            Some(SettingValue::Num(n)) => *n,
            _ => match self.def(key).map(|d| &d.default) {
                Some(SettingValue::Num(n)) => *n,
                _ => 1.0,
            },
        }
    }

    pub fn text(&self, key: &str) -> String {
        match self.values.get(key) {
            Some(SettingValue::Text(t)) => t.clone(),
            _ => match self.def(key).map(|d| &d.default) {
                Some(SettingValue::Text(t)) => t.clone(),
                _ => String::new(),
            },
        }
    }

    /// Set a value, validating it against the definition. Errors are written for humans.
    pub fn set(&mut self, key: &str, value: SettingValue) -> Result<(), String> {
        let def = self
            .def(key)
            .ok_or_else(|| {
                format!("There is no setting called '{key}'. Try 'settings search <word>'.")
            })?
            .clone();
        let fixed = match (&def.kind, value) {
            (SettingKind::Toggle, SettingValue::Bool(b)) => SettingValue::Bool(b),
            (SettingKind::Toggle, SettingValue::Text(t)) => match t.to_lowercase().as_str() {
                "on" | "true" | "yes" | "1" => SettingValue::Bool(true),
                "off" | "false" | "no" | "0" => SettingValue::Bool(false),
                _ => return Err(format!("'{}' is a toggle. Use on or off.", def.name)),
            },
            (SettingKind::Slider { min, max, step }, v) => {
                let n = match v {
                    SettingValue::Num(n) => n,
                    SettingValue::Text(t) => t.parse::<f64>().map_err(|_| {
                        format!("'{}' needs a number between {min} and {max}.", def.name)
                    })?,
                    SettingValue::Bool(_) => {
                        return Err(format!(
                            "'{}' needs a number between {min} and {max}.",
                            def.name
                        ))
                    }
                };
                if n < *min - 1e-9 || n > *max + 1e-9 {
                    return Err(format!(
                        "'{}' must be between {min} and {max} (you gave {n}).",
                        def.name
                    ));
                }
                let snapped = if *step > 0.0 {
                    ((n / step).round() * step * 1e6).round() / 1e6
                } else {
                    n
                };
                SettingValue::Num(snapped.clamp(*min, *max))
            }
            (SettingKind::Choice { options }, v) => {
                let t = match v {
                    SettingValue::Text(t) => t,
                    _ => {
                        return Err(format!(
                            "'{}' needs one of: {}.",
                            def.name,
                            options
                                .iter()
                                .map(|o| o.id.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                    }
                };
                if options.iter().any(|o| o.id == t) {
                    SettingValue::Text(t)
                } else {
                    return Err(format!(
                        "'{}' has no option '{}'. Options: {}.",
                        def.name,
                        t,
                        options
                            .iter()
                            .map(|o| o.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            (SettingKind::Toggle, SettingValue::Num(n)) => SettingValue::Bool(n != 0.0),
        };
        self.values.insert(key.to_string(), fixed);
        Ok(())
    }

    pub fn reset(&mut self, key: &str) {
        if let Some(d) = self.def(key).cloned() {
            self.values.insert(d.key, d.default);
        }
    }

    pub fn reset_category(&mut self, cat: &str) {
        let keys: Vec<String> = self
            .defs
            .iter()
            .filter(|d| d.category() == cat)
            .map(|d| d.key.clone())
            .collect();
        for k in keys {
            self.reset(&k);
        }
    }

    pub fn apply_preset(&mut self, preset: &Preset) {
        let defaults: Vec<(String, SettingValue)> = self
            .defs
            .iter()
            .map(|d| (d.key.clone(), d.default.clone()))
            .collect();
        for (k, v) in defaults {
            self.values.insert(k, v);
        }
        for (k, v) in &preset.values {
            let _ = self.set(k, v.clone());
        }
    }

    /// Which settings differ from their default (for a "what did I change?" screen).
    pub fn changed(&self) -> Vec<(&SettingDef, &SettingValue)> {
        self.defs
            .iter()
            .filter_map(|d| {
                self.values
                    .get(&d.key)
                    .filter(|v| **v != d.default)
                    .map(|v| (d, v))
            })
            .collect()
    }

    pub fn search(&self, word: &str) -> Vec<&SettingDef> {
        let w = word.to_lowercase();
        self.defs
            .iter()
            .filter(|d| {
                d.key.to_lowercase().contains(&w)
                    || d.name.to_lowercase().contains(&w)
                    || d.description.to_lowercase().contains(&w)
            })
            .collect()
    }

    /// Plain-English explanation of one setting and its current value.
    pub fn explain(&self, key: &str) -> Option<String> {
        let d = self.def(key)?;
        let cur = self.values.get(key).map(|v| v.show()).unwrap_or_default();
        let mut s = format!(
            "{}  [{}]\n  {}\n  Now: {}   Default: {}\n",
            d.name,
            d.key,
            d.description,
            cur,
            d.default.show()
        );
        match &d.kind {
            SettingKind::Toggle => {
                s += &format!("  ON : {}\n  OFF: {}\n", d.high_note, d.low_note);
            }
            SettingKind::Slider { min, max, .. } => {
                s += &format!(
                    "  Range {min} to {max}\n  Low : {}\n  High: {}\n",
                    d.low_note, d.high_note
                );
            }
            SettingKind::Choice { options } => {
                for o in options {
                    s += &format!("  - {} ({}): {}\n", o.id, o.label, o.explain);
                }
            }
        }
        Some(s)
    }

    /// Export only the values that were changed, for sharing a settings file.
    pub fn export_changed(&self) -> BTreeMap<String, SettingValue> {
        self.changed()
            .into_iter()
            .map(|(d, v)| (d.key.clone(), v.clone()))
            .collect()
    }

    pub fn import_values(&mut self, vals: &BTreeMap<String, SettingValue>) -> Vec<String> {
        let mut errs = vec![];
        for (k, v) in vals {
            if let Err(e) = self.set(k, v.clone()) {
                errs.push(e);
            }
        }
        errs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_is_documented_and_in_range() {
        let defs = builtin_defs();
        let mut keys = std::collections::HashSet::new();
        for d in &defs {
            assert!(keys.insert(d.key.clone()), "duplicate key {}", d.key);
            assert!(d.description.len() > 10, "{} lacks a description", d.key);
            assert!(
                CATEGORIES.iter().any(|(c, _)| *c == d.category()),
                "unknown category for {}",
                d.key
            );
            match (&d.kind, &d.default) {
                (SettingKind::Slider { min, max, .. }, SettingValue::Num(n)) => {
                    assert!(n >= min && n <= max, "{} default out of range", d.key);
                    assert!(
                        !d.low_note.is_empty() && !d.high_note.is_empty(),
                        "{} needs low/high notes",
                        d.key
                    );
                }
                (SettingKind::Toggle, SettingValue::Bool(_)) => {}
                (SettingKind::Choice { options }, SettingValue::Text(t)) => {
                    assert!(options.iter().any(|o| &o.id == t))
                }
                _ => panic!("{} has mismatched kind/default", d.key),
            }
        }
    }

    #[test]
    fn presets_are_valid() {
        for p in builtin_presets() {
            let mut s = Settings::default();
            s.apply_preset(&p);
            for (k, v) in &p.values {
                assert_eq!(s.raw(k), Some(v), "preset {} key {}", p.id, k);
            }
        }
    }

    #[test]
    fn set_validates() {
        let mut s = Settings::default();
        assert!(s
            .set("injuries.frequency", SettingValue::Num(99.0))
            .is_err());
        assert!(s.set("injuries.frequency", SettingValue::Num(2.0)).is_ok());
        assert!(s
            .set("sim.hot_hand", SettingValue::Text("off".into()))
            .is_ok());
        assert!(!s.bool("sim.hot_hand"));
        assert!(s.set("nope", SettingValue::Num(1.0)).is_err());
    }
}
