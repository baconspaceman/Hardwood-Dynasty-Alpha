//! Players: attributes, physical build, archetypes, badges, hidden traits and career history.
//!
//! The model is deliberately "NBA 2K style": ~36 attributes rated 0-99 roll up into an
//! Overall rating. On top of that sit hidden traits (potential, work ethic, injury proneness...),
//! 2K-style badges with Bronze/Silver/Gold/Hall-of-Fame tiers, and a full career record.
//!
//! Ratings are *era-relative*: a 70 means "a good starter in his own league", whether that
//! league is 1955 or 2025. The era profile (`era.rs`) decides how the league plays.

use crate::contract::Contract;
use crate::injury::{Injury, InjuryRecord};
use crate::life::LifeState;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Position {
    PG,
    SG,
    SF,
    PF,
    C,
}

impl Position {
    pub const ALL: [Position; 5] = [
        Position::PG,
        Position::SG,
        Position::SF,
        Position::PF,
        Position::C,
    ];
    pub fn idx(self) -> usize {
        self as usize
    }
    pub fn from_idx(i: usize) -> Position {
        Position::ALL[i.min(4)]
    }
    pub fn name(self) -> &'static str {
        match self {
            Position::PG => "PG",
            Position::SG => "SG",
            Position::SF => "SF",
            Position::PF => "PF",
            Position::C => "C",
        }
    }
    pub fn parse(s: &str) -> Option<Position> {
        match s.trim().to_uppercase().as_str() {
            "PG" | "G" | "POINT GUARD" => Some(Position::PG),
            "SG" | "SHOOTING GUARD" | "G-F" | "GF" => Some(Position::SG),
            "SF" | "F" | "SMALL FORWARD" | "F-G" => Some(Position::SF),
            "PF" | "POWER FORWARD" | "F-C" | "FC" => Some(Position::PF),
            "C" | "CENTER" | "C-F" => Some(Position::C),
            _ => None,
        }
    }
}

macro_rules! attrs {
    ($( $v:ident => ($key:expr, $name:expr, $fam:expr) ),* $(,)?) => {
        /// Every attribute a player can have.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum Attr { $( $v, )* }

        impl Attr {
            pub const ALL: &'static [Attr] = &[ $( Attr::$v, )* ];
            pub fn idx(self) -> usize { self as usize }
            /// Short stable id used in data files and mods.
            pub fn key(self) -> &'static str { match self { $( Attr::$v => $key, )* } }
            pub fn name(self) -> &'static str { match self { $( Attr::$v => $name, )* } }
            /// Which "skill family" the attribute belongs to.
            pub fn family(self) -> Family { match self { $( Attr::$v => $fam, )* } }
            pub fn from_key(k: &str) -> Option<Attr> { match k { $( $key => Some(Attr::$v), )* _ => None } }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Inside,
    Mid,
    Three,
    Playmaking,
    PerimeterD,
    InteriorD,
    Rebounding,
    Athletic,
    Mental,
}

impl Family {
    pub const ALL: [Family; 9] = [
        Family::Inside,
        Family::Mid,
        Family::Three,
        Family::Playmaking,
        Family::PerimeterD,
        Family::InteriorD,
        Family::Rebounding,
        Family::Athletic,
        Family::Mental,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Family::Inside => "inside",
            Family::Mid => "mid",
            Family::Three => "three",
            Family::Playmaking => "playmaking",
            Family::PerimeterD => "perimeter_d",
            Family::InteriorD => "interior_d",
            Family::Rebounding => "rebounding",
            Family::Athletic => "athletic",
            Family::Mental => "mental",
        }
    }
    pub fn from_key(k: &str) -> Option<Family> {
        Family::ALL.iter().copied().find(|f| f.key() == k)
    }
}

attrs! {
    CloseShot => ("close_shot", "Close Shot", Family::Inside),
    Layup => ("layup", "Layup", Family::Inside),
    DrivingDunk => ("driving_dunk", "Driving Dunk", Family::Inside),
    StandingDunk => ("standing_dunk", "Standing Dunk", Family::Inside),
    PostControl => ("post_control", "Post Control", Family::Inside),
    DrawFoul => ("draw_foul", "Draw Foul", Family::Inside),
    MidRange => ("mid_range", "Mid-Range Shot", Family::Mid),
    ShotIq => ("shot_iq", "Shot IQ", Family::Mid),
    OffConsistency => ("off_consistency", "Offensive Consistency", Family::Mid),
    FreeThrow => ("free_throw", "Free Throw", Family::Three),
    ThreePoint => ("three_point", "Three-Point Shot", Family::Three),
    BallHandle => ("ball_handle", "Ball Handle", Family::Playmaking),
    SpeedWithBall => ("speed_with_ball", "Speed With Ball", Family::Playmaking),
    PassAccuracy => ("pass_accuracy", "Pass Accuracy", Family::Playmaking),
    PassIq => ("pass_iq", "Pass IQ", Family::Playmaking),
    PassVision => ("pass_vision", "Pass Vision", Family::Playmaking),
    Hands => ("hands", "Hands", Family::Playmaking),
    PerimeterDef => ("perimeter_def", "Perimeter Defense", Family::PerimeterD),
    Steal => ("steal", "Steal", Family::PerimeterD),
    LateralQuickness => ("lateral_quickness", "Lateral Quickness", Family::PerimeterD),
    PassPerception => ("pass_perception", "Pass Perception", Family::PerimeterD),
    InteriorDef => ("interior_def", "Interior Defense", Family::InteriorD),
    Block => ("block", "Block", Family::InteriorD),
    HelpDefIq => ("help_def_iq", "Help Defense IQ", Family::InteriorD),
    OffRebound => ("off_rebound", "Offensive Rebound", Family::Rebounding),
    DefRebound => ("def_rebound", "Defensive Rebound", Family::Rebounding),
    Speed => ("speed", "Speed", Family::Athletic),
    Agility => ("agility", "Agility", Family::Athletic),
    Strength => ("strength", "Strength", Family::Athletic),
    Vertical => ("vertical", "Vertical", Family::Athletic),
    Stamina => ("stamina", "Stamina", Family::Athletic),
    Hustle => ("hustle", "Hustle", Family::Athletic),
    Durability => ("durability", "Overall Durability", Family::Athletic),
    Clutch => ("clutch", "Clutch", Family::Mental),
    Discipline => ("discipline", "Foul Discipline", Family::Mental),
    DefConsistency => ("def_consistency", "Defensive Consistency", Family::Mental),
}

pub const ATTR_COUNT: usize = Attr::ALL.len();

/// A player's 36 attribute ratings.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Attrs {
    v: Vec<u8>,
}

impl Default for Attrs {
    fn default() -> Self {
        Attrs {
            v: vec![50; ATTR_COUNT],
        }
    }
}

impl Attrs {
    pub fn new(fill: u8) -> Self {
        Attrs {
            v: vec![fill; ATTR_COUNT],
        }
    }
    #[inline]
    pub fn get(&self, a: Attr) -> f64 {
        self.v[a.idx()] as f64
    }
    #[inline]
    pub fn raw(&self, a: Attr) -> u8 {
        self.v[a.idx()]
    }
    pub fn set(&mut self, a: Attr, val: f64) {
        self.v[a.idx()] = val.round().clamp(1.0, 99.0) as u8;
    }
    pub fn add(&mut self, a: Attr, delta: f64) {
        let cur = self.get(a);
        self.set(a, cur + delta);
    }
    pub fn family_avg(&self, f: Family) -> f64 {
        let mut s = 0.0;
        let mut n = 0.0;
        for a in Attr::ALL {
            if a.family() == f {
                s += self.get(*a);
                n += 1.0;
            }
        }
        s / n
    }
    pub fn iter(&self) -> impl Iterator<Item = (Attr, u8)> + '_ {
        Attr::ALL.iter().map(move |a| (*a, self.v[a.idx()]))
    }
}

/// Compute the Overall rating (0-99) from attributes and height.
///
/// Offense counts the player's best scoring/creating skills (specialists are rewarded, but
/// not as much as all-around players), defense counts his best defensive family, then
/// rebounding, athleticism and mental traits.
pub fn compute_overall(a: &Attrs, height_in: u8) -> f64 {
    let f = |fam| a.family_avg(fam);
    let mut offs = [
        f(Family::Inside),
        f(Family::Mid),
        f(Family::Three),
        f(Family::Playmaking),
    ];
    offs.sort_by(|x, y| y.partial_cmp(x).unwrap());
    let offense = 0.42 * offs[0] + 0.28 * offs[1] + 0.18 * offs[2] + 0.12 * offs[3];
    let (p, i) = (f(Family::PerimeterD), f(Family::InteriorD));
    let defense = 0.62 * p.max(i) + 0.38 * p.min(i);
    // Bigs are expected to rebound; small guards get credit for the effort.
    let reb_w = if height_in >= 80 { 0.13 } else { 0.08 };
    let ath = f(Family::Athletic);
    let mental = f(Family::Mental);
    let raw = (0.42 - (reb_w - 0.08)) * offense
        + 0.27 * defense
        + reb_w * f(Family::Rebounding)
        + 0.17 * ath
        + 0.06 * mental;
    // Stretch the middle so stars separate from role players: raw 50 → 50, raw 75 → ~84.
    let ovr = 50.0 + (raw - 50.0) * 1.38;
    ovr.clamp(25.0, 99.0)
}

/// Pick the most natural position from build + skills.
pub fn infer_position(a: &Attrs, height_in: u8) -> Position {
    let f = |fam| a.family_avg(fam);
    let mut x = (height_in as f64 - 71.0) / 2.6;
    x += (f(Family::InteriorD) + f(Family::Rebounding)
        - f(Family::Playmaking)
        - f(Family::PerimeterD))
        / 60.0;
    x += (a.get(Attr::StandingDunk) + a.get(Attr::PostControl)
        - a.get(Attr::BallHandle)
        - a.get(Attr::ThreePoint))
        / 200.0;
    Position::from_idx(x.round().clamp(0.0, 4.0) as usize)
}

// -------------------------------------------------------------------------------------------
// Hidden traits
// -------------------------------------------------------------------------------------------

/// Things the player never sees on a rating card but that shape his whole career.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hidden {
    /// Work ethic 0-100: drives how much he improves from practice.
    pub work_ethic: u8,
    /// How well he responds to coaching 0-100.
    pub coachability: u8,
    /// Wants the ball and the spotlight. High ego = chemistry risk and contract demands.
    pub ego: u8,
    pub loyalty: u8,
    pub greed: u8,
    /// Cares about winning above all.
    pub winning: u8,
    /// Multiplier on injury chance (1.0 = average).
    pub injury_prone: f32,
    /// Age at which he peaks.
    pub peak_age: f32,
    /// Shift in development: positive = late bloomer, negative = early peaker.
    pub bloom: f32,
    /// 0 = will reach potential, 1 = a draft bust waiting to happen.
    pub bust: f32,
    pub charisma: u8,
}

impl Default for Hidden {
    fn default() -> Self {
        Hidden {
            work_ethic: 60,
            coachability: 60,
            ego: 50,
            loyalty: 50,
            greed: 50,
            winning: 50,
            injury_prone: 1.0,
            peak_age: 27.0,
            bloom: 0.0,
            bust: 0.0,
            charisma: 50,
        }
    }
}

// -------------------------------------------------------------------------------------------
// Badges
// -------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    Bronze = 1,
    Silver = 2,
    Gold = 3,
    HallOfFame = 4,
}

impl Tier {
    pub fn name(self) -> &'static str {
        match self {
            Tier::Bronze => "Bronze",
            Tier::Silver => "Silver",
            Tier::Gold => "Gold",
            Tier::HallOfFame => "Hall of Fame",
        }
    }
    pub fn from_level(n: u8) -> Option<Tier> {
        match n {
            1 => Some(Tier::Bronze),
            2 => Some(Tier::Silver),
            3 => Some(Tier::Gold),
            4 => Some(Tier::HallOfFame),
            _ => None,
        }
    }
}

/// A badge definition (data, so mods can add badges). A badge is earned when the player's
/// attributes meet *all* requirements; each tier needs `tier_step` more points on every one.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BadgeDef {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    /// (attribute key, minimum for Bronze)
    pub requires: Vec<(String, u8)>,
    #[serde(default = "default_step")]
    pub tier_step: u8,
    /// (effect key, strength added per tier level). Effect keys are listed in `game.rs`.
    pub effects: Vec<(String, f64)>,
}

fn default_step() -> u8 {
    6
}

fn bd(
    id: &str,
    name: &str,
    cat: &str,
    desc: &str,
    req: &[(&str, u8)],
    eff: &[(&str, f64)],
) -> BadgeDef {
    BadgeDef {
        id: id.into(),
        name: name.into(),
        category: cat.into(),
        description: desc.into(),
        requires: req.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        tier_step: default_step(),
        effects: eff.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
    }
}

pub fn builtin_badges() -> Vec<BadgeDef> {
    vec![
        // Shooting
        bd(
            "deadeye",
            "Deadeye",
            "Shooting",
            "Contested jumpers hurt less.",
            &[("three_point", 72), ("shot_iq", 65)],
            &[("contest_resist", 0.12)],
        ),
        bd(
            "catch_shoot",
            "Catch & Shoot",
            "Shooting",
            "Better on assisted jump shots.",
            &[("three_point", 70)],
            &[("assisted_three", 0.012)],
        ),
        bd(
            "limitless_range",
            "Limitless Range",
            "Shooting",
            "Deep threes fall more often.",
            &[("three_point", 80)],
            &[("three", 0.010)],
        ),
        bd(
            "mid_maestro",
            "Mid-Range Maestro",
            "Shooting",
            "A lethal mid-range game.",
            &[("mid_range", 76), ("shot_iq", 66)],
            &[("mid", 0.012)],
        ),
        bd(
            "clutch_shooter",
            "Clutch Shooter",
            "Shooting",
            "Hits shots in crunch time.",
            &[("clutch", 78), ("off_consistency", 65)],
            &[("clutch", 0.015)],
        ),
        bd(
            "free_throw_ace",
            "Free Throw Ace",
            "Shooting",
            "Rarely misses at the line.",
            &[("free_throw", 84)],
            &[("ft", 0.012)],
        ),
        // Finishing
        bd(
            "posterizer",
            "Posterizer",
            "Finishing",
            "Dunks over defenders.",
            &[("driving_dunk", 82), ("vertical", 78)],
            &[("rim", 0.012), ("draw_foul", 0.01)],
        ),
        bd(
            "acrobat",
            "Acrobat",
            "Finishing",
            "Contorts around rim protectors.",
            &[("layup", 80), ("agility", 72)],
            &[("rim", 0.010)],
        ),
        bd(
            "pro_touch",
            "Pro Touch",
            "Finishing",
            "Soft touch on layups and floaters.",
            &[("close_shot", 78), ("layup", 74)],
            &[("rim", 0.008), ("mid", 0.004)],
        ),
        bd(
            "post_scorer",
            "Post Technician",
            "Finishing",
            "Crafty back-to-the-basket scoring.",
            &[("post_control", 78), ("close_shot", 70)],
            &[("post", 0.015)],
        ),
        bd(
            "foul_magnet",
            "Foul Magnet",
            "Finishing",
            "Draws fouls constantly.",
            &[("draw_foul", 78)],
            &[("draw_foul", 0.025)],
        ),
        // Playmaking
        bd(
            "dimer",
            "Dimer",
            "Playmaking",
            "Teammates shoot better off his passes.",
            &[("pass_accuracy", 78), ("pass_vision", 72)],
            &[("assist_boost", 0.012)],
        ),
        bd(
            "floor_general",
            "Floor General",
            "Playmaking",
            "Raises the whole lineup.",
            &[("pass_iq", 80), ("pass_vision", 76), ("ball_handle", 70)],
            &[("team_boost", 0.006)],
        ),
        bd(
            "handles_for_days",
            "Handles for Days",
            "Playmaking",
            "Rarely tires dribbling; rarely turns it over.",
            &[("ball_handle", 84), ("speed_with_ball", 78)],
            &[("tov_reduce", 0.012)],
        ),
        bd(
            "ankle_breaker",
            "Ankle Breaker",
            "Playmaking",
            "Breaks defenders down off the bounce.",
            &[("ball_handle", 86), ("agility", 80)],
            &[("iso", 0.012)],
        ),
        bd(
            "unpluckable",
            "Unpluckable",
            "Playmaking",
            "Hard to steal from.",
            &[("hands", 78), ("ball_handle", 70)],
            &[("tov_reduce", 0.010)],
        ),
        // Defense
        bd(
            "clamps",
            "Clamps",
            "Defense",
            "Sticks to ball handlers like glue.",
            &[("perimeter_def", 82), ("lateral_quickness", 78)],
            &[("perim_d", 0.020)],
        ),
        bd(
            "rim_protector",
            "Rim Protector",
            "Defense",
            "Blocks and alters shots at the rim.",
            &[("block", 80), ("interior_def", 76)],
            &[("block", 0.020), ("rim_d", 0.015)],
        ),
        bd(
            "intimidator",
            "Intimidator",
            "Defense",
            "Shooters in his area miss more.",
            &[("interior_def", 82), ("strength", 70)],
            &[("rim_d", 0.012)],
        ),
        bd(
            "interceptor",
            "Interceptor",
            "Defense",
            "Jumps passing lanes.",
            &[("steal", 80), ("pass_perception", 76)],
            &[("steal", 0.020)],
        ),
        bd(
            "pick_dodger",
            "Pick Dodger",
            "Defense",
            "Fights through screens.",
            &[("lateral_quickness", 74), ("perimeter_def", 70)],
            &[("perim_d", 0.008)],
        ),
        bd(
            "chase_down",
            "Chase Down Artist",
            "Defense",
            "Hunts down transition layups.",
            &[("speed", 82), ("vertical", 76), ("block", 65)],
            &[("block", 0.010)],
        ),
        bd(
            "brick_wall",
            "Brick Wall",
            "Defense",
            "Immovable screens and box-outs.",
            &[("strength", 84), ("interior_def", 70)],
            &[("dreb", 0.010)],
        ),
        // Rebounding
        bd(
            "rebound_chaser",
            "Rebound Chaser",
            "Rebounding",
            "Tracks down boards from anywhere.",
            &[("off_rebound", 78), ("hustle", 75)],
            &[("oreb", 0.015)],
        ),
        bd(
            "glass_cleaner",
            "Glass Cleaner",
            "Rebounding",
            "Cleans the defensive glass.",
            &[("def_rebound", 80), ("strength", 70)],
            &[("dreb", 0.020)],
        ),
        // General
        bd(
            "tireless",
            "Tireless Worker",
            "General",
            "Slow to tire.",
            &[("stamina", 85)],
            &[("stamina", 0.10)],
        ),
        bd(
            "iron_man",
            "Iron Man",
            "General",
            "Rarely hurt.",
            &[("durability", 88)],
            &[("injury", -0.10)],
        ),
        bd(
            "clean_hands",
            "Disciplined",
            "General",
            "Avoids silly fouls.",
            &[("discipline", 82)],
            &[("foul_avoid", 0.12)],
        ),
        bd(
            "heart",
            "Heart of a Champion",
            "General",
            "Elevates in big moments.",
            &[("clutch", 82), ("hustle", 78)],
            &[("clutch", 0.010), ("team_boost", 0.004)],
        ),
    ]
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlayerBadge {
    pub id: String,
    pub tier: Tier,
}

// -------------------------------------------------------------------------------------------
// Statistics
// -------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(from = "StatLineCompact", into = "StatLineCompact")]
pub struct StatLine {
    pub g: u16,
    pub gs: u16,
    pub min: f64,
    pub fgm: u32,
    pub fga: u32,
    pub tpm: u32,
    pub tpa: u32,
    pub ftm: u32,
    pub fta: u32,
    pub orb: u32,
    pub drb: u32,
    pub ast: u32,
    pub stl: u32,
    pub blk: u32,
    pub tov: u32,
    pub pf: u32,
    pub pts: u32,
    pub plus_minus: i32,
}

/// Saves store a stat line as a short array instead of a labeled object (much smaller files).
#[derive(Serialize, Deserialize)]
struct StatLineCompact(u16, u16, f64, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, i32);

impl From<StatLineCompact> for StatLine {
    fn from(c: StatLineCompact) -> Self {
        StatLine { g: c.0, gs: c.1, min: c.2, fgm: c.3, fga: c.4, tpm: c.5, tpa: c.6, ftm: c.7, fta: c.8, orb: c.9, drb: c.10, ast: c.11, stl: c.12, blk: c.13, tov: c.14, pf: c.15, pts: c.16, plus_minus: c.17 }
    }
}

impl From<StatLine> for StatLineCompact {
    fn from(s: StatLine) -> Self {
        StatLineCompact(s.g, s.gs, s.min, s.fgm, s.fga, s.tpm, s.tpa, s.ftm, s.fta, s.orb, s.drb, s.ast, s.stl, s.blk, s.tov, s.pf, s.pts, s.plus_minus)
    }
}

impl StatLine {
    pub fn add(&mut self, o: &StatLine) {
        self.g += o.g;
        self.gs += o.gs;
        self.min += o.min;
        self.fgm += o.fgm;
        self.fga += o.fga;
        self.tpm += o.tpm;
        self.tpa += o.tpa;
        self.ftm += o.ftm;
        self.fta += o.fta;
        self.orb += o.orb;
        self.drb += o.drb;
        self.ast += o.ast;
        self.stl += o.stl;
        self.blk += o.blk;
        self.tov += o.tov;
        self.pf += o.pf;
        self.pts += o.pts;
        self.plus_minus += o.plus_minus;
    }
    pub fn reb(&self) -> u32 {
        self.orb + self.drb
    }
    fn per(&self, x: f64) -> f64 {
        if self.g == 0 {
            0.0
        } else {
            x / self.g as f64
        }
    }
    pub fn ppg(&self) -> f64 {
        self.per(self.pts as f64)
    }
    pub fn rpg(&self) -> f64 {
        self.per(self.reb() as f64)
    }
    pub fn apg(&self) -> f64 {
        self.per(self.ast as f64)
    }
    pub fn spg(&self) -> f64 {
        self.per(self.stl as f64)
    }
    pub fn bpg(&self) -> f64 {
        self.per(self.blk as f64)
    }
    pub fn mpg(&self) -> f64 {
        self.per(self.min)
    }
    pub fn fg_pct(&self) -> f64 {
        if self.fga == 0 {
            0.0
        } else {
            self.fgm as f64 / self.fga as f64
        }
    }
    pub fn tp_pct(&self) -> f64 {
        if self.tpa == 0 {
            0.0
        } else {
            self.tpm as f64 / self.tpa as f64
        }
    }
    pub fn ft_pct(&self) -> f64 {
        if self.fta == 0 {
            0.0
        } else {
            self.ftm as f64 / self.fta as f64
        }
    }
    /// True shooting %.
    pub fn ts_pct(&self) -> f64 {
        let d = 2.0 * (self.fga as f64 + 0.44 * self.fta as f64);
        if d == 0.0 {
            0.0
        } else {
            self.pts as f64 / d
        }
    }
    /// A simple, transparent box-score value rating (similar to "Game Score" per game).
    pub fn game_score_pg(&self) -> f64 {
        if self.g == 0 {
            return 0.0;
        }
        let t = self.pts as f64 + 0.4 * self.fgm as f64
            - 0.7 * self.fga as f64
            - 0.4 * (self.fta - self.ftm) as f64
            + 0.7 * self.orb as f64
            + 0.3 * self.drb as f64
            + self.stl as f64
            + 0.7 * self.ast as f64
            + 0.7 * self.blk as f64
            - 0.4 * self.pf as f64
            - self.tov as f64;
        t / self.g as f64
    }
}

/// Where the season was played.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    HighSchool,
    College,
    Overseas,
    GLeague,
    Pro,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeasonRecord {
    pub season: Season,
    pub level: Level,
    /// Team name at the time (kept as text so history survives relocations/renames).
    pub team: String,
    pub team_id: Option<TeamId>,
    pub age: u8,
    pub ovr: u8,
    pub stats: StatLine,
    pub playoffs: StatLine,
    pub salary: Money,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwardRecord {
    pub season: Season,
    pub award: String,
    pub team: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DraftInfo {
    pub year: Season,
    pub round: u8,
    pub pick: u16,
    pub team: String,
    pub team_id: Option<TeamId>,
}

/// Where a player came from.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Origin {
    pub kind: OriginKind,
    /// School or club name ("Lincoln Prep", "Belgrade Stars").
    pub school: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginKind {
    HighSchool,
    College,
    International,
    GLeague,
    Undrafted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Affiliation {
    /// Not currently signed anywhere (pro free agent).
    FreeAgent,
    Nba(TeamId),
    GLeague(TeamId),
    College(CollegeId),
    HighSchool,
    Overseas(ClubId),
    Retired,
}

/// What a player works on in practice. Chosen by you (for your players) or by the AI coach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevFocus {
    Balanced,
    Shooting,
    Finishing,
    Playmaking,
    Defense,
    Rebounding,
    Athleticism,
    Conditioning,
    FilmStudy,
}

impl DevFocus {
    pub const ALL: [DevFocus; 9] = [
        DevFocus::Balanced,
        DevFocus::Shooting,
        DevFocus::Finishing,
        DevFocus::Playmaking,
        DevFocus::Defense,
        DevFocus::Rebounding,
        DevFocus::Athleticism,
        DevFocus::Conditioning,
        DevFocus::FilmStudy,
    ];
    pub fn name(self) -> &'static str {
        match self {
            DevFocus::Balanced => "Balanced",
            DevFocus::Shooting => "Shooting",
            DevFocus::Finishing => "Finishing",
            DevFocus::Playmaking => "Playmaking",
            DevFocus::Defense => "Defense",
            DevFocus::Rebounding => "Rebounding",
            DevFocus::Athleticism => "Athleticism",
            DevFocus::Conditioning => "Conditioning & health",
            DevFocus::FilmStudy => "Film study (IQ)",
        }
    }
    pub fn parse(s: &str) -> Option<DevFocus> {
        let s = s.to_lowercase();
        DevFocus::ALL.iter().copied().find(|f| {
            f.name().to_lowercase().starts_with(&s) || format!("{f:?}").to_lowercase() == s
        })
    }
    /// Attributes this focus pushes.
    pub fn targets(self) -> &'static [Attr] {
        match self {
            DevFocus::Balanced => &[],
            DevFocus::Shooting => &[
                Attr::ThreePoint,
                Attr::MidRange,
                Attr::FreeThrow,
                Attr::CloseShot,
            ],
            DevFocus::Finishing => &[
                Attr::Layup,
                Attr::DrivingDunk,
                Attr::PostControl,
                Attr::DrawFoul,
            ],
            DevFocus::Playmaking => &[
                Attr::BallHandle,
                Attr::PassAccuracy,
                Attr::PassIq,
                Attr::PassVision,
            ],
            DevFocus::Defense => &[
                Attr::PerimeterDef,
                Attr::InteriorDef,
                Attr::Steal,
                Attr::Block,
                Attr::HelpDefIq,
            ],
            DevFocus::Rebounding => &[Attr::OffRebound, Attr::DefRebound, Attr::Hustle],
            DevFocus::Athleticism => &[Attr::Speed, Attr::Agility, Attr::Strength, Attr::Vertical],
            DevFocus::Conditioning => &[Attr::Stamina, Attr::Durability],
            DevFocus::FilmStudy => &[
                Attr::ShotIq,
                Attr::PassIq,
                Attr::HelpDefIq,
                Attr::Discipline,
            ],
        }
    }
}

/// How happy the player is, broken down so decisions can be explained.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Mood {
    /// 0-100 overall.
    pub overall: f32,
    pub playing_time: f32,
    pub winning: f32,
    pub contract: f32,
    pub role: f32,
    pub team_chemistry: f32,
    /// True when he has asked to be traded.
    pub wants_trade: bool,
}

impl Default for Mood {
    fn default() -> Self {
        Mood {
            overall: 65.0,
            playing_time: 60.0,
            winning: 55.0,
            contract: 60.0,
            role: 60.0,
            team_chemistry: 60.0,
            wants_trade: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub first: String,
    pub last: String,
    pub birth_year: i32,
    pub country: String,
    pub hometown: String,
    pub height_in: u8,
    pub weight_lb: u16,
    pub wingspan_in: u8,
    pub left_handed: bool,
    pub position: Position,
    /// Archetype id (see `generate::ArchetypeDef`).
    pub archetype: String,
    pub attrs: Attrs,
    /// True ceiling for Overall (hidden from the user when "hidden potential" is on).
    pub potential: u8,
    pub ovr: u8,
    pub peak_ovr: u8,
    pub hidden: Hidden,
    pub origin: Origin,
    pub affiliation: Affiliation,
    pub contract: Option<Contract>,
    /// NBA team that owns this player's draft rights while he plays elsewhere (draft-and-stash).
    pub draft_rights: Option<TeamId>,
    pub draft: Option<DraftInfo>,
    pub injury: Option<Injury>,
    pub injury_history: Vec<InjuryRecord>,
    /// Cumulative wear and tear 0-100. High wear raises injury risk and speeds decline.
    pub wear: f32,
    pub concussions: u8,
    /// Day-to-day fitness 0-100 (energy carried between games).
    pub fitness: f32,
    pub mood: Mood,
    pub badges: Vec<PlayerBadge>,
    pub seasons: Vec<SeasonRecord>,
    pub awards: Vec<AwardRecord>,
    pub years_pro: u8,
    pub dev_focus: DevFocus,
    pub retired: Option<Season>,
    pub hall_of_fame: Option<Season>,
    /// Is this the human's own created player?
    pub user_controlled: bool,
    pub life: Option<Box<LifeState>>,
    /// Free-form numbers for mods/events (e.g. "fan_love").
    #[serde(default)]
    pub custom: BTreeMap<String, f64>,
    /// Free-form tags for mods/events (e.g. "scandal_2031").
    #[serde(default)]
    pub flags: BTreeSet<String>,
    /// Minutes the coach wants for him this season (set by AI or user). `None` = automatic.
    pub minutes_target: Option<f32>,
}

impl Player {
    pub fn name(&self) -> String {
        format!("{} {}", self.first, self.last)
    }
    pub fn short_name(&self) -> String {
        format!(
            "{}. {}",
            self.first.chars().next().unwrap_or('?'),
            self.last
        )
    }
    pub fn age(&self, season: Season) -> i32 {
        season - self.birth_year
    }
    pub fn recompute_ovr(&mut self) {
        let o = compute_overall(&self.attrs, self.height_in).round() as u8;
        self.ovr = o;
        if o > self.peak_ovr {
            self.peak_ovr = o;
        }
    }
    pub fn is_active_pro(&self) -> bool {
        matches!(
            self.affiliation,
            Affiliation::Nba(_) | Affiliation::GLeague(_)
        )
    }
    pub fn team_id(&self) -> Option<TeamId> {
        match self.affiliation {
            Affiliation::Nba(t) | Affiliation::GLeague(t) => Some(t),
            _ => None,
        }
    }
    pub fn is_retired(&self) -> bool {
        self.retired.is_some() || self.affiliation == Affiliation::Retired
    }
    pub fn height_str(&self) -> String {
        format!("{}'{}\"", self.height_in / 12, self.height_in % 12)
    }
    pub fn is_injured(&self) -> bool {
        self.injury
            .as_ref()
            .map(|i| i.games_remaining > 0)
            .unwrap_or(false)
    }
    pub fn badge_tier(&self, id: &str) -> Option<Tier> {
        self.badges.iter().find(|b| b.id == id).map(|b| b.tier)
    }
    /// Career totals across all pro seasons.
    pub fn career_pro(&self) -> StatLine {
        let mut t = StatLine::default();
        for s in self.seasons.iter().filter(|s| s.level == Level::Pro) {
            t.add(&s.stats);
        }
        t
    }
    pub fn current_salary(&self) -> Money {
        self.contract.as_ref().map(|c| c.salary()).unwrap_or(0)
    }
    pub fn pro_seasons(&self) -> usize {
        self.seasons
            .iter()
            .filter(|s| s.level == Level::Pro)
            .count()
    }
}

/// Earn badges from current attributes. Returns the list of badges the player qualifies for.
/// `limit` caps how many badges one player can hold (best ones win).
pub fn earned_badges(attrs: &Attrs, defs: &[BadgeDef], limit: usize) -> Vec<PlayerBadge> {
    let mut found: Vec<(f64, PlayerBadge)> = vec![];
    for d in defs {
        // How far above the Bronze threshold is the *weakest* requirement?
        let mut margin = f64::MAX;
        let mut ok = true;
        for (k, min) in &d.requires {
            match Attr::from_key(k) {
                Some(a) => {
                    let m = attrs.get(a) - *min as f64;
                    margin = margin.min(m);
                    if m < 0.0 {
                        ok = false;
                    }
                }
                None => ok = false,
            }
        }
        if !ok {
            continue;
        }
        let step = d.tier_step.max(1) as f64;
        let level = ((margin / step).floor() as i64 + 1).clamp(1, 4) as u8;
        found.push((
            margin,
            PlayerBadge {
                id: d.id.clone(),
                tier: Tier::from_level(level).unwrap(),
            },
        ));
    }
    found.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    found.into_iter().take(limit).map(|x| x.1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overall_orders_sensibly() {
        let mut star = Attrs::new(55);
        for a in [
            Attr::ThreePoint,
            Attr::MidRange,
            Attr::BallHandle,
            Attr::Layup,
            Attr::Speed,
            Attr::PerimeterDef,
        ] {
            star.set(a, 88.0);
        }
        let avg = Attrs::new(55);
        assert!(compute_overall(&star, 77) > compute_overall(&avg, 77) + 8.0);
        assert!(compute_overall(&Attrs::new(30), 77) < 40.0);
    }

    #[test]
    fn attr_keys_round_trip() {
        for a in Attr::ALL {
            assert_eq!(Attr::from_key(a.key()), Some(*a));
        }
        assert_eq!(ATTR_COUNT, 36);
    }

    #[test]
    fn badges_follow_attributes() {
        let mut a = Attrs::new(50);
        a.set(Attr::ThreePoint, 90.0);
        a.set(Attr::ShotIq, 80.0);
        let b = earned_badges(&a, &builtin_badges(), 10);
        assert!(b.iter().any(|x| x.id == "deadeye" && x.tier >= Tier::Gold));
        assert!(!b.iter().any(|x| x.id == "clamps"));
    }
}
