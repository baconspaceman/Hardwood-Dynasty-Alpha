//! Injuries: from a rolled ankle to a career-ending spinal condition.
//!
//! Everything here is data-driven (`InjuryDef`) so mods can add injuries or retune them.
//! The model has several layers:
//!   * Exposure: players get hurt in games (more minutes = more risk) and in practice.
//!   * Risk factors: age, wear and tear, fatigue, durability rating, injury history,
//!     "injury-prone" hidden trait, medical staff quality, and the league's difficulty setting.
//!   * Type and severity: weighted by body part; durations are log-normal around a median.
//!   * Consequences: games missed, a "ramp-up" period of reduced play after returning,
//!     re-injury risk, permanent attribute loss for serious injuries and, rarely, retirement.

use crate::player::{Attr, Player};
use crate::rng::Rng;
use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    DayToDay,
    Minor,
    Moderate,
    Serious,
    Major,
    SeasonEnding,
    CareerThreatening,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Severity::DayToDay => "Day-to-day",
            Severity::Minor => "Minor",
            Severity::Moderate => "Moderate",
            Severity::Serious => "Serious",
            Severity::Major => "Major",
            Severity::SeasonEnding => "Season-ending",
            Severity::CareerThreatening => "Career-threatening",
        }
    }
}

/// One kind of injury.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InjuryDef {
    pub id: String,
    pub name: String,
    pub body_part: String,
    pub severity: Severity,
    /// Relative frequency among injuries.
    pub weight: f64,
    /// Typical (median) number of games missed.
    pub median_games: f64,
    pub max_games: u16,
    /// Chance (0-1) this injury ends a career, before setting/age/wear modifiers.
    #[serde(default)]
    pub career_ending: f64,
    /// Permanent attribute losses if the injury is serious: (attribute key, points lost).
    #[serde(default)]
    pub permanent: Vec<(String, f64)>,
    /// Extra weight when happening in games (contact) vs practice. 1.0 = same.
    #[serde(default = "one")]
    pub game_bias: f64,
    /// Earliest season this injury can occur/be recognised (e.g. concussion protocols).
    #[serde(default)]
    pub from_year: i32,
    /// Multiplier on re-injury chance of the same body part while recovering/after.
    #[serde(default = "one")]
    pub recurrence: f64,
}

fn one() -> f64 {
    1.0
}

#[allow(clippy::too_many_arguments)]
fn def(
    id: &str,
    name: &str,
    part: &str,
    sev: Severity,
    weight: f64,
    median: f64,
    max: u16,
    ce: f64,
    perm: &[(&str, f64)],
    game_bias: f64,
    recurrence: f64,
) -> InjuryDef {
    InjuryDef {
        id: id.into(),
        name: name.into(),
        body_part: part.into(),
        severity: sev,
        weight,
        median_games: median,
        max_games: max,
        career_ending: ce,
        permanent: perm.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        game_bias,
        from_year: 0,
        recurrence,
    }
}

pub fn builtin_injuries() -> Vec<InjuryDef> {
    use Severity::*;
    vec![
        // ---- Day to day ----
        def(
            "sore_ankle",
            "Sore ankle",
            "ankle",
            DayToDay,
            9.0,
            2.0,
            6,
            0.0,
            &[],
            1.0,
            1.5,
        ),
        def(
            "sore_knee",
            "Knee soreness",
            "knee",
            DayToDay,
            6.0,
            2.0,
            6,
            0.0,
            &[],
            0.7,
            1.4,
        ),
        def(
            "sore_back",
            "Back spasms",
            "back",
            DayToDay,
            6.0,
            2.0,
            7,
            0.0,
            &[],
            0.6,
            1.6,
        ),
        def(
            "illness",
            "Flu-like illness",
            "illness",
            DayToDay,
            6.0,
            2.0,
            6,
            0.0,
            &[],
            0.0,
            1.0,
        ),
        def(
            "bruise",
            "Thigh/hip contusion",
            "hip",
            DayToDay,
            5.0,
            2.0,
            5,
            0.0,
            &[],
            1.3,
            1.0,
        ),
        def(
            "finger_jam",
            "Jammed finger",
            "hand",
            DayToDay,
            4.0,
            2.0,
            6,
            0.0,
            &[],
            1.4,
            1.0,
        ),
        def(
            "eye_poke",
            "Eye injury",
            "face",
            DayToDay,
            1.5,
            2.0,
            5,
            0.0,
            &[],
            1.8,
            1.0,
        ),
        def(
            "tight_hamstring",
            "Tight hamstring",
            "hamstring",
            DayToDay,
            4.0,
            2.0,
            5,
            0.0,
            &[],
            0.8,
            1.8,
        ),
        // ---- Minor ----
        def(
            "ankle_sprain",
            "Sprained ankle",
            "ankle",
            Minor,
            9.0,
            6.0,
            20,
            0.0,
            &[],
            1.2,
            1.7,
        ),
        def(
            "hamstring_strain",
            "Hamstring strain",
            "hamstring",
            Minor,
            4.0,
            8.0,
            25,
            0.0,
            &[],
            0.8,
            2.0,
        ),
        def(
            "groin_strain",
            "Groin strain",
            "groin",
            Minor,
            3.5,
            7.0,
            22,
            0.0,
            &[],
            0.8,
            1.8,
        ),
        def(
            "calf_strain",
            "Calf strain",
            "calf",
            Minor,
            3.0,
            8.0,
            22,
            0.0,
            &[],
            0.8,
            1.8,
        ),
        def(
            "wrist_sprain",
            "Sprained wrist",
            "wrist",
            Minor,
            2.5,
            7.0,
            20,
            0.0,
            &[],
            1.4,
            1.2,
        ),
        def(
            "fractured_finger",
            "Fractured finger",
            "hand",
            Minor,
            2.5,
            9.0,
            25,
            0.0,
            &[],
            1.4,
            1.0,
        ),
        def(
            "rib_contusion",
            "Bruised ribs",
            "ribs",
            Minor,
            2.0,
            6.0,
            15,
            0.0,
            &[],
            1.5,
            1.0,
        ),
        def(
            "shoulder_sprain",
            "Shoulder sprain",
            "shoulder",
            Minor,
            2.5,
            8.0,
            25,
            0.0,
            &[],
            1.2,
            1.5,
        ),
        def(
            "back_strain",
            "Lower back strain",
            "back",
            Minor,
            4.0,
            8.0,
            25,
            0.0,
            &[],
            0.5,
            2.0,
        ),
        // ---- Moderate ----
        def(
            "high_ankle",
            "High ankle sprain",
            "ankle",
            Moderate,
            2.5,
            22.0,
            50,
            0.0,
            &[("lateral_quickness", 0.5)],
            1.2,
            2.0,
        ),
        def(
            "hamstring_tear",
            "Torn hamstring (partial)",
            "hamstring",
            Moderate,
            1.6,
            24.0,
            55,
            0.0,
            &[("speed", 0.8)],
            0.8,
            2.4,
        ),
        def(
            "sprained_knee",
            "Sprained knee (MCL)",
            "knee",
            Moderate,
            2.2,
            22.0,
            50,
            0.0,
            &[],
            1.1,
            1.8,
        ),
        def(
            "broken_hand",
            "Broken hand",
            "hand",
            Moderate,
            1.2,
            28.0,
            55,
            0.0,
            &[],
            1.2,
            1.0,
        ),
        def(
            "broken_nose",
            "Broken nose / facial fracture",
            "face",
            Moderate,
            0.8,
            12.0,
            30,
            0.0,
            &[],
            1.6,
            0.5,
        ),
        def(
            "sports_hernia",
            "Sports hernia",
            "groin",
            Moderate,
            0.8,
            24.0,
            55,
            0.0,
            &[],
            0.5,
            1.5,
        ),
        def(
            "concussion",
            "Concussion",
            "head",
            Moderate,
            2.2,
            8.0,
            40,
            0.0005,
            &[],
            1.4,
            2.2,
        ),
        def(
            "plantar_fasciitis",
            "Plantar fasciitis",
            "foot",
            Moderate,
            1.2,
            18.0,
            45,
            0.0,
            &[("speed", 0.3)],
            0.3,
            2.0,
        ),
        def(
            "tendinitis",
            "Patellar tendinitis",
            "knee",
            Moderate,
            1.4,
            20.0,
            50,
            0.0,
            &[("vertical", 0.4)],
            0.3,
            2.5,
        ),
        // ---- Serious ----
        def(
            "meniscus",
            "Torn meniscus",
            "knee",
            Serious,
            1.1,
            40.0,
            70,
            0.002,
            &[("lateral_quickness", 0.8), ("vertical", 0.8)],
            1.0,
            1.8,
        ),
        def(
            "broken_leg_tib",
            "Fractured tibia/fibula",
            "leg",
            Serious,
            0.35,
            60.0,
            90,
            0.01,
            &[("vertical", 1.0)],
            1.5,
            1.2,
        ),
        def(
            "fractured_wrist",
            "Fractured wrist",
            "wrist",
            Serious,
            0.6,
            38.0,
            65,
            0.0,
            &[("ball_handle", 0.5)],
            1.3,
            1.0,
        ),
        def(
            "fractured_foot",
            "Fractured foot (Jones)",
            "foot",
            Serious,
            0.7,
            55.0,
            90,
            0.004,
            &[("speed", 1.0), ("vertical", 0.6)],
            0.8,
            2.0,
        ),
        def(
            "shoulder_sublux",
            "Dislocated shoulder",
            "shoulder",
            Serious,
            0.5,
            45.0,
            75,
            0.002,
            &[("strength", 0.6)],
            1.4,
            2.4,
        ),
        def(
            "herniated_disc",
            "Herniated disc",
            "back",
            Serious,
            0.4,
            50.0,
            85,
            0.02,
            &[("agility", 1.2), ("vertical", 1.0)],
            0.4,
            2.5,
        ),
        def(
            "stress_fracture",
            "Stress fracture",
            "leg",
            Serious,
            0.6,
            50.0,
            85,
            0.006,
            &[("vertical", 0.7)],
            0.1,
            2.2,
        ),
        // ---- Major ----
        def(
            "acl",
            "Torn ACL",
            "knee",
            Major,
            0.55,
            150.0,
            260,
            0.035,
            &[
                ("speed", 2.0),
                ("agility", 2.0),
                ("vertical", 2.5),
                ("lateral_quickness", 2.0),
            ],
            1.5,
            1.8,
        ),
        def(
            "achilles",
            "Ruptured Achilles tendon",
            "achilles",
            Major,
            0.22,
            175.0,
            280,
            0.12,
            &[
                ("speed", 3.5),
                ("vertical", 4.0),
                ("agility", 2.5),
                ("lateral_quickness", 3.0),
            ],
            1.0,
            2.0,
        ),
        def(
            "patellar",
            "Ruptured patellar tendon",
            "knee",
            Major,
            0.12,
            170.0,
            280,
            0.12,
            &[("vertical", 4.0), ("speed", 2.5)],
            1.0,
            1.8,
        ),
        def(
            "microfracture",
            "Microfracture surgery (knee)",
            "knee",
            Major,
            0.18,
            140.0,
            240,
            0.10,
            &[("vertical", 3.0), ("speed", 2.0), ("agility", 2.0)],
            0.5,
            2.5,
        ),
        def(
            "broken_femur",
            "Broken femur",
            "leg",
            Major,
            0.05,
            140.0,
            260,
            0.08,
            &[("vertical", 3.0), ("speed", 2.5)],
            1.6,
            1.2,
        ),
        def(
            "multi_ligament",
            "Multi-ligament knee injury",
            "knee",
            SeasonEnding,
            0.04,
            220.0,
            380,
            0.30,
            &[("vertical", 4.5), ("speed", 4.0), ("agility", 4.0)],
            2.0,
            2.0,
        ),
        def(
            "labrum",
            "Torn labrum (shoulder)",
            "shoulder",
            Major,
            0.15,
            110.0,
            200,
            0.02,
            &[("strength", 1.0), ("mid_range", 1.0)],
            1.0,
            1.8,
        ),
        def(
            "lisfranc",
            "Lisfranc foot injury",
            "foot",
            Major,
            0.12,
            120.0,
            220,
            0.06,
            &[("speed", 2.0), ("vertical", 2.0)],
            1.0,
            1.8,
        ),
        def(
            "spinal_fusion",
            "Spinal fusion surgery",
            "back",
            SeasonEnding,
            0.06,
            200.0,
            360,
            0.55,
            &[("agility", 3.0), ("vertical", 3.0), ("speed", 2.0)],
            0.2,
            2.5,
        ),
        // ---- Career threatening ----
        def(
            "cardiac",
            "Cardiac condition found in screening",
            "heart",
            CareerThreatening,
            0.025,
            120.0,
            400,
            0.65,
            &[("stamina", 3.0)],
            0.0,
            1.0,
        ),
        def(
            "spinal_cord",
            "Spinal cord injury",
            "spine",
            CareerThreatening,
            0.01,
            300.0,
            600,
            0.90,
            &[("agility", 5.0), ("speed", 5.0), ("vertical", 5.0)],
            1.5,
            1.0,
        ),
        InjuryDef {
            from_year: 2010,
            ..def(
                "post_concussion",
                "Post-concussion syndrome",
                "head",
                CareerThreatening,
                0.04,
                180.0,
                500,
                0.45,
                &[("shot_iq", 2.0), ("def_consistency", 2.0)],
                0.5,
                2.0,
            )
        },
    ]
}

/// The injury a player currently has.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Injury {
    pub def_id: String,
    pub name: String,
    pub body_part: String,
    pub severity: Severity,
    pub games_total: u16,
    pub games_remaining: u16,
    pub season: Season,
    /// Ends the player's career when it happens.
    pub career_ending: bool,
    /// Permanent attribute losses still to apply when recovery finishes.
    pub permanent: Vec<(String, f64)>,
    /// After returning, he plays at a reduced level for this many games (ramp-up).
    pub ramp_games: u16,
    /// Playing through it? (reduced performance, higher re-injury risk)
    pub playing_through: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InjuryRecord {
    pub season: Season,
    pub name: String,
    pub body_part: String,
    pub games_missed: u16,
    pub career_ending: bool,
}

/// Knobs from settings, kept in one struct so the injury code doesn't need the whole settings object.
#[derive(Clone, Copy, Debug)]
pub struct InjuryParams {
    pub frequency: f64,
    pub severity: f64,
    pub career_ending: f64,
    pub permanent: f64,
    pub in_game: bool,
    /// Medical staff quality 0-100 (avg 50).
    pub medical: f64,
    pub year: Season,
}

impl InjuryParams {
    pub fn standard(year: Season) -> Self {
        InjuryParams {
            frequency: 1.0,
            severity: 1.0,
            career_ending: 1.0,
            permanent: 1.0,
            in_game: true,
            medical: 50.0,
            year,
        }
    }
}

/// Risk multiplier from a player's personal situation (1.0 = average).
pub fn risk_multiplier(p: &Player, season: Season, params: &InjuryParams) -> f64 {
    let age = (season - p.birth_year) as f64;
    let age_f = 1.0 + ((age - 26.0) * 0.03).max(-0.15);
    let wear_f = 1.0 + p.wear as f64 / 120.0;
    let dur_f = 1.7 - p.attrs.get(Attr::Durability) / 70.0; // dur 70 → 0.7, dur 40 → 1.13, dur 90 → 0.41
    let tired_f = 1.0 + ((60.0 - p.fitness as f64) / 100.0).max(0.0);
    let hist_f = 1.0 + (p.injury_history.len() as f64 * 0.015).min(0.3);
    let med_f = 1.25 - params.medical / 200.0; // medical 50 → 1.0, 90 → 0.8
    let badge_f = 1.0
        + p.badges
            .iter()
            .filter(|b| b.id == "iron_man")
            .map(|b| -0.08 * b.tier as i32 as f64)
            .sum::<f64>();
    (age_f
        * wear_f
        * dur_f.max(0.3)
        * tired_f
        * hist_f
        * med_f
        * badge_f
        * p.hidden.injury_prone as f64)
        .clamp(0.1, 6.0)
}

/// Base chance of an injury per 100 minutes on the court (injuries that cost at least a game).
pub const BASE_PER_100_MIN: f64 = 0.0175;
/// Base chance of an injury per day away from games (practice, travel, accidents).
pub const BASE_PER_DAY: f64 = 0.00085;

/// Roll a specific injury given that one has happened.
fn choose_def<'a>(
    rng: &mut Rng,
    catalog: &'a [InjuryDef],
    in_game: bool,
    year: Season,
    p: &Player,
) -> &'a InjuryDef {
    let w: Vec<f64> = catalog
        .iter()
        .map(|d| {
            if d.from_year > year {
                return 0.0;
            }
            let mut w = d.weight
                * if in_game {
                    d.game_bias
                } else {
                    (2.0 - d.game_bias).max(0.0)
                };
            // repeat injuries to the same body part are more likely
            let repeats = p
                .injury_history
                .iter()
                .filter(|h| h.body_part == d.body_part)
                .count() as f64;
            w *= 1.0 + repeats * 0.15 * (d.recurrence - 1.0).max(0.0);
            w
        })
        .collect();
    &catalog[rng.weighted(&w)]
}

/// Create an injury from a definition for this player.
pub fn make_injury(
    rng: &mut Rng,
    d: &InjuryDef,
    p: &Player,
    season: Season,
    params: &InjuryParams,
) -> Injury {
    let age = (season - p.birth_year) as f64;
    let sigma = 0.55;
    let med = d.median_games.max(1.0);
    let mut games = rng.gauss(med.ln(), sigma).exp() * params.severity;
    // Older players heal slower; good medical staff heals faster.
    games *= 1.0 + ((age - 28.0) * 0.02).max(0.0);
    games *= 1.35 - params.medical / 150.0;
    let games = games
        .round()
        .clamp(1.0, d.max_games as f64 * params.severity.max(1.0)) as u16;

    // Career-ending check.
    let ce_base = d.career_ending * params.career_ending;
    let age_mod = 1.0 + ((age - 28.0) * 0.06).max(0.0);
    let wear_mod = 1.0 + p.wear as f64 / 100.0;
    let history_mod = 1.0
        + p.injury_history
            .iter()
            .filter(|h| h.body_part == d.body_part)
            .count() as f64
            * 0.35;
    let ce = rng.chance((ce_base * age_mod * wear_mod * history_mod).min(0.95));

    // Permanent loss: scaled by setting, age, and whether it happened to a body part before.
    let permanent: Vec<(String, f64)> = d
        .permanent
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                (v * params.permanent
                    * (0.7 + 0.04 * (age - 20.0).max(0.0))
                    * rng.uniform(0.6, 1.3))
                .max(0.0),
            )
        })
        .collect();

    let ramp = match d.severity {
        Severity::DayToDay | Severity::Minor => 0,
        Severity::Moderate => 3,
        Severity::Serious => 8,
        _ => 20,
    };
    Injury {
        def_id: d.id.clone(),
        name: d.name.clone(),
        body_part: d.body_part.clone(),
        severity: d.severity,
        games_total: games,
        games_remaining: games,
        season,
        career_ending: ce,
        permanent,
        ramp_games: ramp,
        playing_through: false,
    }
}

/// Decide whether an injury happens for `exposure` units. For in-game use `exposure = minutes/100`;
/// for off-day checks use `exposure = days` with `in_game = false`.
pub fn roll_injury(
    rng: &mut Rng,
    catalog: &[InjuryDef],
    p: &Player,
    season: Season,
    exposure: f64,
    in_game: bool,
    params: &InjuryParams,
) -> Option<Injury> {
    if params.frequency <= 0.0 || p.injury.is_some() {
        return None;
    }
    let base = if in_game {
        BASE_PER_100_MIN
    } else {
        BASE_PER_DAY
    };
    let chance = base * exposure * risk_multiplier(p, season, params) * params.frequency;
    if !rng.chance(chance) {
        return None;
    }
    let d = choose_def(rng, catalog, in_game, params.year, p);
    Some(make_injury(rng, d, p, season, params))
}

/// Create an injury that is *known* to have happened (e.g. a player was hurt in a game).
pub fn random_injury(
    rng: &mut Rng,
    catalog: &[InjuryDef],
    p: &Player,
    season: Season,
    in_game: bool,
    params: &InjuryParams,
) -> Injury {
    let d = choose_def(rng, catalog, in_game, params.year, p);
    make_injury(rng, d, p, season, params)
}

/// Apply an injury to a player. If it is career-ending he is retired by the caller.
pub fn start_injury(p: &mut Player, inj: Injury) {
    p.fitness = (p.fitness - 15.0).max(0.0);
    p.wear = (p.wear
        + match inj.severity {
            Severity::DayToDay => 0.2,
            Severity::Minor => 1.0,
            Severity::Moderate => 2.5,
            Severity::Serious => 5.0,
            _ => 9.0,
        })
    .min(100.0);
    if inj.def_id == "concussion" || inj.def_id == "post_concussion" {
        p.concussions = p.concussions.saturating_add(1);
    }
    p.injury = Some(inj);
}

/// Called after one game (or rest day) has passed for an injured player. Returns `Some(record)`
/// when he has just recovered.
pub fn heal_one_game(p: &mut Player, medical: f64, rng: &mut Rng) -> Option<InjuryRecord> {
    let inj = p.injury.as_mut()?;
    if inj.career_ending {
        return None;
    }
    // Good medical staff occasionally shave a game off.
    let mut step = 1u16;
    if rng.chance(((medical - 50.0) / 400.0).max(0.0)) {
        step = 2;
    }
    inj.games_remaining = inj.games_remaining.saturating_sub(step);
    if inj.games_remaining == 0 {
        let rec = InjuryRecord {
            season: inj.season,
            name: inj.name.clone(),
            body_part: inj.body_part.clone(),
            games_missed: inj.games_total,
            career_ending: false,
        };
        // apply permanent damage
        let perms = inj.permanent.clone();
        let ramp = inj.ramp_games;
        for (k, loss) in perms {
            if let Some(a) = Attr::from_key(&k) {
                p.attrs.add(a, -loss);
            }
        }
        p.recompute_ovr();
        p.custom.insert("ramp_games".into(), ramp as f64);
        p.injury_history.push(rec.clone());
        p.injury = None;
        return Some(rec);
    }
    None
}

/// Performance multiplier (≤ 1) for a player who is playing through an injury or still
/// ramping up after one.
pub fn performance_penalty(p: &Player) -> f64 {
    let mut pen = 1.0;
    if let Some(i) = &p.injury {
        if i.playing_through {
            pen *= match i.severity {
                Severity::DayToDay => 0.97,
                Severity::Minor => 0.93,
                Severity::Moderate => 0.88,
                _ => 0.80,
            };
        }
    }
    if let Some(r) = p.custom.get("ramp_games") {
        if *r > 0.0 {
            pen *= 0.94 + 0.06 * (1.0 - (*r / 20.0).min(1.0));
        }
    }
    pen
}

/// Convert an injury to a short, human report ("Torn ACL — out ~75 games (est.)").
pub fn describe(i: &Injury, hide_details: bool) -> String {
    if hide_details {
        return format!(
            "{} ({})",
            i.name,
            if i.games_remaining > 20 {
                "out for an extended period"
            } else {
                "questionable"
            }
        );
    }
    if i.career_ending {
        return format!("{} — career in jeopardy", i.name);
    }
    format!(
        "{} — {} games remaining ({})",
        i.name,
        i.games_remaining,
        i.severity.name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_valid() {
        let c = builtin_injuries();
        let mut ids = std::collections::HashSet::new();
        for d in &c {
            assert!(ids.insert(d.id.clone()));
            assert!(
                d.weight > 0.0 && d.median_games >= 1.0 && d.max_games as f64 >= d.median_games
            );
            for (k, _) in &d.permanent {
                assert!(Attr::from_key(k).is_some(), "bad attr {k} in {}", d.id);
            }
        }
        assert!(c.iter().any(|d| d.severity == Severity::CareerThreatening));
    }
}
