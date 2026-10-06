//! Creating people: players with realistic builds, skills, hidden traits and origins.
//!
//! A player is built in layers:
//!   1. pick an *archetype* (Sharpshooter, Rim Protector, Floor General...) that fits the era,
//!   2. pick a position and height (taller on average for bigs, and by country),
//!   3. set every attribute from a talent level + the archetype's strengths + the body,
//!   4. nudge so the Overall rating hits the target talent,
//!   5. roll hidden traits (work ethic, injury-proneness, peak age...) and earn badges.

use crate::content::Content;
use crate::contract::Contract;
use crate::era;
use crate::names;
use crate::player::*;
use crate::rng::Rng;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A player archetype: a recognizable style of player. Data-driven, so mods can add new ones.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchetypeDef {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Relative likelihood of each position [PG, SG, SF, PF, C].
    pub positions: [f64; 5],
    /// Extra inches of height vs. the position average.
    #[serde(default)]
    pub height_shift: f64,
    /// Bonus/penalty by skill family key (see `Family::key`).
    #[serde(default)]
    pub family_bias: BTreeMap<String, f64>,
    /// Bonus/penalty for individual attributes by key.
    #[serde(default)]
    pub attr_bias: BTreeMap<String, f64>,
    /// How common this archetype is by year: (year, weight).
    pub era_weight: Vec<(i32, f64)>,
}

fn arch(id: &str, name: &str, desc: &str, pos: [f64; 5], hs: f64, fam: &[(&str, f64)], at: &[(&str, f64)], era: &[(i32, f64)]) -> ArchetypeDef {
    ArchetypeDef {
        id: id.into(),
        name: name.into(),
        description: desc.into(),
        positions: pos,
        height_shift: hs,
        family_bias: fam.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        attr_bias: at.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        era_weight: era.to_vec(),
    }
}

pub fn builtin_archetypes() -> Vec<ArchetypeDef> {
    let all = [(1946, 1.0)];
    vec![
        arch("floor_general", "Floor General", "A pass-first point guard who runs the offense.", [1.0, 0.1, 0.0, 0.0, 0.0], -1.0,
            &[("playmaking", 12.0), ("inside", -4.0), ("interior_d", -12.0), ("rebounding", -8.0)], &[("pass_iq", 5.0), ("pass_vision", 6.0), ("steal", 3.0)], &all),
        arch("scoring_guard", "Scoring Guard", "A guard who creates and makes his own shot.", [0.5, 1.0, 0.1, 0.0, 0.0], 0.0,
            &[("mid", 8.0), ("three", 6.0), ("inside", 3.0), ("interior_d", -12.0), ("rebounding", -8.0)], &[("ball_handle", 4.0), ("off_consistency", 3.0)], &all),
        arch("sharpshooter", "Sharpshooter", "A shooter whose gravity bends defenses.", [0.2, 1.0, 0.6, 0.1, 0.0], 0.0,
            &[("three", 16.0), ("mid", 6.0), ("inside", -6.0), ("interior_d", -10.0), ("athletic", -3.0)], &[("free_throw", 6.0), ("shot_iq", 4.0)],
            &[(1946, 0.3), (1980, 0.7), (2005, 1.4), (2020, 1.6)]),
        arch("slasher", "Slasher", "A downhill athlete who lives at the rim.", [0.1, 0.8, 1.0, 0.2, 0.0], 0.0,
            &[("inside", 11.0), ("athletic", 8.0), ("three", -7.0), ("mid", -2.0), ("interior_d", -6.0)], &[("driving_dunk", 5.0), ("draw_foul", 5.0)], &all),
        arch("three_and_d", "3&D Wing", "A wing who defends the best scorer and hits open threes.", [0.0, 0.6, 1.0, 0.3, 0.0], 0.5,
            &[("three", 8.0), ("perimeter_d", 12.0), ("playmaking", -6.0), ("inside", -5.0), ("athletic", 3.0)], &[("lateral_quickness", 4.0)],
            &[(1946, 0.2), (1985, 0.5), (2008, 1.3), (2020, 1.8)]),
        arch("two_way_wing", "Two-Way Wing", "A complete wing who scores and defends.", [0.0, 0.5, 1.0, 0.4, 0.0], 0.5,
            &[("perimeter_d", 7.0), ("three", 4.0), ("mid", 4.0), ("inside", 4.0), ("athletic", 5.0), ("playmaking", 1.0)], &[], &all),
        arch("point_forward", "Point Forward", "A big wing who initiates the offense.", [0.1, 0.2, 1.0, 0.5, 0.0], 1.5,
            &[("playmaking", 10.0), ("inside", 3.0), ("mid", 3.0), ("rebounding", 2.0)], &[("pass_vision", 4.0)], &[(1946, 0.3), (1975, 0.7), (1995, 1.0), (2010, 1.2)]),
        arch("stretch_big", "Stretch Big", "A big man who spaces the floor.", [0.0, 0.0, 0.1, 0.8, 1.0], 0.0,
            &[("three", 11.0), ("mid", 8.0), ("athletic", -2.0), ("interior_d", -1.0), ("playmaking", -4.0)], &[("free_throw", 4.0)],
            &[(1946, 0.0), (1990, 0.2), (2005, 0.9), (2018, 1.6)]),
        arch("post_scorer", "Post Technician", "A back-to-the-basket scorer with footwork.", [0.0, 0.0, 0.0, 0.7, 1.0], 1.0,
            &[("inside", 14.0), ("three", -14.0), ("athletic", -3.0), ("playmaking", -7.0), ("perimeter_d", -12.0)], &[("post_control", 8.0), ("close_shot", 5.0)],
            &[(1946, 1.8), (1980, 1.4), (2000, 0.8), (2015, 0.3)]),
        arch("rim_protector", "Rim Protector", "A shot-blocking anchor who owns the paint.", [0.0, 0.0, 0.0, 0.3, 1.0], 1.5,
            &[("interior_d", 16.0), ("rebounding", 6.0), ("inside", -3.0), ("three", -14.0), ("playmaking", -10.0), ("perimeter_d", -12.0)], &[("block", 8.0)], &all),
        arch("glass_cleaner", "Glass Cleaner", "A rebounder and screen-setter who does the dirty work.", [0.0, 0.0, 0.0, 1.0, 0.6], 0.0,
            &[("rebounding", 14.0), ("athletic", 2.0), ("inside", 3.0), ("three", -10.0), ("playmaking", -8.0)], &[("strength", 6.0), ("hustle", 5.0)], &all),
        arch("athletic_finisher", "Athletic Finisher", "A lob-catching, rim-running big.", [0.0, 0.0, 0.1, 0.8, 1.0], 0.0,
            &[("athletic", 10.0), ("inside", 8.0), ("three", -12.0), ("playmaking", -8.0), ("interior_d", 4.0)], &[("vertical", 6.0), ("standing_dunk", 6.0)],
            &[(1946, 0.2), (1970, 0.7), (2000, 1.1), (2015, 1.4)]),
        arch("playmaking_big", "Point Center", "A big man who sees the floor like a guard.", [0.0, 0.0, 0.1, 0.6, 1.0], 1.0,
            &[("playmaking", 10.0), ("mid", 2.0), ("inside", 5.0), ("athletic", -5.0), ("perimeter_d", -8.0)], &[("pass_iq", 6.0), ("post_control", 4.0)],
            &[(1946, 0.05), (1975, 0.3), (2010, 0.9), (2020, 1.4)]),
        arch("lockdown_defender", "Lockdown Defender", "A defensive specialist who guards the best perimeter player.", [0.3, 0.8, 0.8, 0.1, 0.0], 0.0,
            &[("perimeter_d", 16.0), ("athletic", 5.0), ("inside", -7.0), ("mid", -6.0), ("three", -4.0), ("playmaking", -3.0)], &[("steal", 6.0), ("lateral_quickness", 5.0)], &all),
        arch("pure_scorer", "Pure Scorer", "A high-volume bucket-getter at every level.", [0.1, 0.8, 1.0, 0.3, 0.0], 0.5,
            &[("mid", 11.0), ("inside", 7.0), ("three", 5.0), ("perimeter_d", -6.0), ("interior_d", -8.0), ("rebounding", -4.0)], &[("off_consistency", 5.0), ("draw_foul", 4.0)], &all),
        arch("enforcer", "Enforcer", "A bruising big who sets the tone physically.", [0.0, 0.0, 0.0, 1.0, 0.8], 0.5,
            &[("athletic", 2.0), ("interior_d", 8.0), ("rebounding", 8.0), ("inside", -4.0), ("three", -14.0), ("playmaking", -10.0), ("mental", -6.0)], &[("strength", 10.0), ("hustle", 6.0), ("discipline", -10.0)],
            &[(1946, 1.5), (1985, 1.0), (2000, 0.4), (2015, 0.1)]),
        arch("unicorn", "Unicorn", "An enormous talent with guard skills in a giant's body.", [0.0, 0.0, 0.2, 0.7, 0.5], 3.0,
            &[("playmaking", 5.0), ("three", 5.0), ("mid", 4.0), ("inside", 5.0), ("interior_d", 4.0), ("athletic", 3.0)], &[("ball_handle", 4.0)],
            &[(1946, 0.02), (1990, 0.08), (2010, 0.25), (2022, 0.45)]),
        arch("combo_guard", "Combo Guard", "A guard who can run the offense or score.", [0.8, 0.8, 0.1, 0.0, 0.0], -0.5,
            &[("playmaking", 5.0), ("mid", 5.0), ("three", 4.0), ("interior_d", -12.0), ("rebounding", -8.0)], &[("speed_with_ball", 4.0)], &all),
        arch("energy_big", "Energy Big", "A high-motor big who defends and runs.", [0.0, 0.0, 0.0, 0.7, 1.0], 0.0,
            &[("athletic", 7.0), ("rebounding", 8.0), ("interior_d", 6.0), ("three", -12.0), ("playmaking", -9.0), ("mid", -6.0)], &[("hustle", 10.0), ("stamina", 6.0)], &all),
    ]
}

/// What kind of player to generate.
#[derive(Clone, Debug)]
pub struct GenSpec {
    pub season: Season,
    pub age: i32,
    /// Target Overall (current ability) in era-relative terms.
    pub ovr: f64,
    /// Target potential (final ceiling). If lower than `ovr`, equals `ovr`.
    pub potential: f64,
    pub origin: OriginKind,
    pub country: Option<String>,
    pub archetype: Option<String>,
    pub position: Option<Position>,
    /// Fixed height in inches.
    pub height_in: Option<u8>,
}

impl GenSpec {
    pub fn new(season: Season, age: i32, ovr: f64, potential: f64, origin: OriginKind) -> Self {
        GenSpec { season, age, ovr, potential, origin, country: None, archetype: None, position: None, height_in: None }
    }
}

fn avg_height(pos: Position, year: i32) -> f64 {
    let base = match pos {
        Position::PG => 74.0,
        Position::SG => 76.5,
        Position::SF => 79.0,
        Position::PF => 81.0,
        Position::C => 83.0,
    };
    // Early-era players were shorter.
    base + era::interp(&[(1946, -3.5), (1970, -1.5), (1990, 0.0), (2024, 0.4)], year as f64)
}

/// How much each attribute shifts per inch above 6'6" (78in).
fn height_coef(a: Attr) -> f64 {
    match a {
        Attr::Block => 1.5,
        Attr::InteriorDef => 1.2,
        Attr::StandingDunk => 1.4,
        Attr::OffRebound => 1.1,
        Attr::DefRebound => 1.2,
        Attr::Strength => 1.0,
        Attr::PostControl => 0.9,
        Attr::CloseShot => 0.3,
        Attr::Speed => -1.5,
        Attr::Agility => -1.2,
        Attr::BallHandle => -1.7,
        Attr::SpeedWithBall => -1.7,
        Attr::PassAccuracy => -0.4,
        Attr::Steal => -0.7,
        Attr::LateralQuickness => -1.4,
        Attr::PerimeterDef => -0.9,
        Attr::ThreePoint => -0.35,
        Attr::DrivingDunk => 0.0,
        _ => 0.0,
    }
}

pub fn pick_archetype<'a>(content: &'a Content, rng: &mut Rng, year: Season, pos_hint: Option<Position>) -> &'a ArchetypeDef {
    let w: Vec<f64> = content
        .archetypes
        .iter()
        .map(|a| {
            let e = era::interp(&a.era_weight, year as f64).max(0.0);
            let p = match pos_hint {
                Some(p) => a.positions[p.idx()],
                None => a.positions.iter().sum::<f64>(),
            };
            e * p
        })
        .collect();
    &content.archetypes[rng.weighted(&w)]
}

fn pick_country<'a>(content: &'a Content, rng: &mut Rng, year: Season, international: bool, flow: f64) -> Option<&'a names::Country> {
    if !international {
        return None;
    }
    let w: Vec<f64> = content.countries.iter().map(|c| names::country_weight(c, year) * flow.max(0.05)).collect();
    if w.iter().sum::<f64>() <= 0.0 {
        return None;
    }
    Some(&content.countries[rng.weighted(&w)])
}

/// Create a player. `next_id` supplies the new id.
pub fn generate_player(content: &Content, rng: &mut Rng, id: PlayerId, spec: &GenSpec, intl_flow: f64) -> Player {
    let year = spec.season;

    // Nationality / origin
    let intl = match spec.origin {
        OriginKind::International => true,
        OriginKind::HighSchool | OriginKind::College | OriginKind::GLeague | OriginKind::Undrafted => rng.chance(era::international_share(year) * intl_flow * 0.6),
    };
    let country: Option<names::Country> = match &spec.country {
        Some(code) if code != "USA" => content.countries.iter().find(|c| &c.code == code).cloned(),
        Some(_) => None,
        None => pick_country(content, rng, year, intl, intl_flow).cloned(),
    };
    let (country_code, country_name, pool_id, towns, hbonus) = match &country {
        Some(c) => (c.code.clone(), c.name.clone(), c.pool.clone(), c.towns.clone(), c.height_bonus),
        None => ("USA".to_string(), "United States".to_string(), "us".to_string(), names::us_towns(), 0.0),
    };
    let pool = content.pools.iter().find(|p| p.id == pool_id).unwrap_or(&content.pools[0]);
    let (first, last) = names::random_name(rng, pool);
    let hometown = rng.pick(&towns).clone();

    // Archetype and position
    let arch = match &spec.archetype {
        Some(id) => content.archetypes.iter().find(|a| &a.id == id).unwrap_or(&content.archetypes[0]).clone(),
        None => pick_archetype(content, rng, year, spec.position).clone(),
    };
    let position = spec.position.unwrap_or_else(|| Position::from_idx(rng.weighted(&arch.positions)));
    let height = match spec.height_in {
        Some(h) => h as f64,
        None => (avg_height(position, year) + arch.height_shift + hbonus * 0.5 + rng.gauss(0.0, 1.9)).clamp(66.0, 91.0),
    };
    let height_in = height.round() as u8;

    // Attributes
    let mut attrs = Attrs::new(50);
    let base = spec.ovr;
    let h_off = height - 78.0;
    for a in Attr::ALL {
        let fam_b = arch.family_bias.get(a.family().key()).copied().unwrap_or(0.0);
        let at_b = arch.attr_bias.get(a.key()).copied().unwrap_or(0.0);
        let mut v = base + fam_b + at_b + h_off * height_coef(*a) + rng.gauss(0.0, 5.5);
        // Physical durability, stamina and similar are less tied to talent.
        if matches!(a, Attr::Durability | Attr::Stamina | Attr::Hustle | Attr::Discipline) {
            v = 0.55 * v + 0.45 * rng.gauss(62.0, 10.0);
        }
        attrs.set(*a, v);
    }
    // Calibrate attributes so the computed Overall lands on the target.
    for _ in 0..4 {
        let cur = compute_overall(&attrs, height_in);
        let delta = spec.ovr - cur;
        if delta.abs() < 0.6 {
            break;
        }
        for a in Attr::ALL {
            // Skills move fully with the correction; athletic/mental traits only partially.
            let k = match a.family() {
                Family::Athletic | Family::Mental => 0.6,
                _ => 1.0,
            };
            attrs.add(*a, delta * k * 0.9);
        }
    }

    // Hidden traits
    let age = spec.age;
    let hidden = Hidden {
        work_ethic: rng.gauss(60.0, 18.0).clamp(5.0, 99.0) as u8,
        coachability: rng.gauss(60.0, 17.0).clamp(5.0, 99.0) as u8,
        ego: rng.gauss(50.0, 20.0).clamp(0.0, 99.0) as u8,
        loyalty: rng.gauss(50.0, 22.0).clamp(0.0, 99.0) as u8,
        greed: rng.gauss(50.0, 20.0).clamp(0.0, 99.0) as u8,
        winning: rng.gauss(55.0, 20.0).clamp(0.0, 99.0) as u8,
        injury_prone: rng.gauss(1.0, 0.25).clamp(0.45, 2.2) as f32,
        peak_age: (rng.gauss(27.0, 1.6) + if arch.id.contains("athletic") || arch.id == "slasher" { -1.0 } else { 0.0 } + if arch.id == "sharpshooter" || arch.id == "post_scorer" { 1.0 } else { 0.0 }).clamp(23.0, 32.0) as f32,
        bloom: if rng.chance(0.08) { rng.uniform(1.0, 3.0) as f32 } else if rng.chance(0.08) { -rng.uniform(1.0, 2.0) as f32 } else { 0.0 },
        bust: if rng.chance(0.10) { rng.uniform(0.3, 1.0) as f32 } else { 0.0 },
        charisma: rng.gauss(50.0, 22.0).clamp(0.0, 99.0) as u8,
    };

    let weight = (height * 2.35 + attrs.get(Attr::Strength) * 0.55 - 10.0 + rng.gauss(0.0, 9.0)).clamp(135.0, 340.0) as u16;
    let wingspan = (height + rng.gauss(2.8, 1.8)).clamp(height - 1.0, height + 9.0).round() as u8;
    let badges = earned_badges(&attrs, &content.badges, 6);
    let mut p = Player {
        id,
        first,
        last,
        birth_year: year - age,
        country: country_code,
        hometown: if country.is_some() { format!("{}, {}", hometown, country_name) } else { hometown },
        height_in,
        weight_lb: weight,
        wingspan_in: wingspan,
        left_handed: rng.chance(0.11),
        position,
        archetype: arch.id.clone(),
        attrs,
        potential: spec.potential.max(spec.ovr).round().clamp(30.0, 99.0) as u8,
        ovr: 0,
        peak_ovr: 0,
        hidden,
        origin: Origin { kind: spec.origin, school: String::new() },
        affiliation: Affiliation::FreeAgent,
        contract: None,
        draft_rights: None,
        draft: None,
        injury: None,
        injury_history: vec![],
        wear: 0.0,
        concussions: 0,
        fitness: 90.0,
        mood: Mood::default(),
        badges,
        seasons: vec![],
        awards: vec![],
        years_pro: 0,
        dev_focus: DevFocus::Balanced,
        retired: None,
        hall_of_fame: None,
        user_controlled: false,
        life: None,
        custom: BTreeMap::new(),
        flags: BTreeSet::new(),
        minutes_target: None,
    };
    p.recompute_ovr();
    p.potential = p.potential.max(p.ovr);
    p
}

/// Helper to attach an initial contract at market value (used when building the starting league).
pub fn market_contract(content: &Content, money: &crate::economy::SeasonMoney, p: &Player, season: Season, rng: &mut Rng) -> Contract {
    use crate::economy::market_value_pct;
    let age = p.age(season) as f64;
    let pct = market_value_pct(p.ovr as f64, age, p.potential as f64);
    let mut salary = (money.cap as f64 * pct * rng.uniform(0.85, 1.15)) as i64;
    let years_service = p.years_pro;
    if content.economy_enabled_max(season) {
        salary = salary.min(content.economy.max_salary(money, years_service));
    }
    salary = salary.max(money.min_salary);
    let years = if p.ovr >= 72 { rng.range(3, 5) } else if p.ovr >= 60 { rng.range(2, 4) } else { rng.range(1, 3) } as u8;
    let kind = if salary <= money.min_salary { crate::contract::ContractKind::Minimum } else { crate::contract::ContractKind::Standard };
    let mut c = Contract::rising(salary, years, 0.05, kind, season);
    // Random remaining time: not everyone just signed.
    let elapsed = rng.range(0, years as i64 - 1) as usize;
    for _ in 0..elapsed {
        c.tick();
    }
    c.total_years = years;
    c
}

pub fn height_label(h: u8, imperial: bool) -> String {
    if imperial {
        format!("{}'{}\"", h / 12, h % 12)
    } else {
        format!("{} cm", (h as f64 * 2.54).round())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_players_hit_target_overall() {
        let content = Content::default();
        let mut rng = Rng::new(5);
        let mut total_err = 0.0;
        for i in 0..300 {
            let target = 45.0 + (i % 40) as f64;
            let spec = GenSpec::new(2000, 25, target, target, OriginKind::College);
            let p = generate_player(&content, &mut rng, i as u32, &spec, 1.0);
            total_err += (p.ovr as f64 - target).abs();
            assert!(p.height_in >= 66 && p.height_in <= 91);
            assert!(p.potential >= p.ovr);
        }
        assert!(total_err / 300.0 < 2.0, "average miss {}", total_err / 300.0);
    }

    #[test]
    fn era_changes_player_types() {
        let content = Content::default();
        let mut rng = Rng::new(9);
        let count = |year: i32, rng: &mut Rng| {
            (0..400)
                .filter(|i| {
                    let spec = GenSpec::new(year, 25, 60.0, 60.0, OriginKind::College);
                    generate_player(&content, rng, *i, &spec, 1.0).archetype == "stretch_big"
                })
                .count()
        };
        assert!(count(2022, &mut rng) > count(1960, &mut rng) + 10);
    }
}
