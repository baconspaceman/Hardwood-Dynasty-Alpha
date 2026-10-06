//! Player development: how players grow, peak, decline and retire.
//!
//! Each attribute follows an age curve that depends on its family: athleticism peaks early and
//! fades first, shooting/rebounding/playmaking peak in the late 20s, and basketball IQ keeps
//! growing into the 30s. On top of the curve sit hidden traits (work ethic, late bloomer, bust),
//! coaching and facilities, playing time, practice focus, and random luck.

use crate::player::*;
use crate::rng::Rng;
use crate::types::*;

/// Expected yearly change (attribute points) for an attribute family going from `age` to `age+1`.
pub fn yearly_delta(fam: Family, age: f64) -> f64 {
    use Family::*;
    let curve = |pts: &[(f64, f64)]| -> f64 {
        if age <= pts[0].0 {
            return pts[0].1;
        }
        for w in pts.windows(2) {
            if age <= w[1].0 {
                let t = (age - w[0].0) / (w[1].0 - w[0].0);
                return w[0].1 + (w[1].1 - w[0].1) * t;
            }
        }
        pts[pts.len() - 1].1
    };
    match fam {
        Athletic => curve(&[
            (18.0, 4.5),
            (20.0, 3.4),
            (22.0, 2.0),
            (24.0, 0.4),
            (26.0, -0.4),
            (28.0, -1.3),
            (30.0, -2.3),
            (32.0, -3.4),
            (34.0, -4.4),
            (38.0, -5.5),
        ]),
        Inside | Mid | Three | Playmaking | Rebounding => curve(&[
            (18.0, 4.8),
            (20.0, 3.8),
            (22.0, 2.6),
            (24.0, 1.5),
            (26.0, 0.6),
            (28.0, 0.0),
            (30.0, -0.5),
            (32.0, -1.4),
            (34.0, -2.3),
            (38.0, -3.4),
        ]),
        PerimeterD => curve(&[
            (18.0, 4.0),
            (20.0, 3.4),
            (22.0, 2.4),
            (24.0, 1.2),
            (26.0, 0.2),
            (28.0, -0.6),
            (30.0, -1.3),
            (32.0, -2.2),
            (34.0, -3.2),
            (38.0, -4.2),
        ]),
        InteriorD => curve(&[
            (18.0, 4.0),
            (20.0, 3.4),
            (22.0, 2.6),
            (24.0, 1.8),
            (26.0, 0.9),
            (28.0, 0.2),
            (30.0, -0.5),
            (32.0, -1.4),
            (34.0, -2.4),
            (38.0, -3.4),
        ]),
        Mental => curve(&[
            (18.0, 4.5),
            (20.0, 4.0),
            (22.0, 3.2),
            (24.0, 2.4),
            (26.0, 1.6),
            (28.0, 0.8),
            (30.0, 0.3),
            (32.0, -0.4),
            (34.0, -1.4),
            (38.0, -2.6),
        ]),
    }
}

/// Rough overall-rating shift relative to a player's peak at a given age (used to create
/// realistic rosters: a 20-year-old with a 78 peak starts around 65).
pub fn age_shift(age: i32) -> f64 {
    match age {
        ..=18 => -19.0,
        19 => -16.0,
        20 => -12.5,
        21 => -9.0,
        22 => -6.0,
        23 => -3.5,
        24 => -1.8,
        25 => -0.7,
        26..=29 => 0.0,
        30 => -1.0,
        31 => -2.5,
        32 => -4.5,
        33 => -7.0,
        34 => -9.5,
        35 => -12.5,
        36 => -15.5,
        _ => -19.0,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProgParams {
    pub speed: f64,
    pub variance: f64,
    pub late_bloomers: f64,
    pub longevity: f64,
    /// Modern sports science: extends careers in later eras (1.0 = none).
    pub science: f64,
    pub hidden_potential: bool,
}

impl ProgParams {
    pub fn standard() -> Self {
        ProgParams {
            speed: 1.0,
            variance: 1.0,
            late_bloomers: 1.0,
            longevity: 1.0,
            science: 1.0,
            hidden_potential: true,
        }
    }
}

/// Era longevity multiplier: careers got longer as training and medicine improved.
pub fn sports_science_factor(year: Season) -> f64 {
    crate::era::interp(
        &[
            (1946, 0.92),
            (1980, 1.0),
            (2000, 1.08),
            (2024, 1.2),
            (2060, 1.3),
        ],
        year as f64,
    )
}

/// Context for one player's yearly development.
pub struct DevContext {
    /// Coach development rating 0-100 (head coach + assistants).
    pub coaching: f64,
    /// Facilities 0-100.
    pub facilities: f64,
    /// Share of available minutes played last season (0-1). 0.5 = average starter.
    pub minutes_share: f64,
    /// Life-sim momentum multipliers (skill, physical); 1.0 when not applicable.
    pub life_skill: f64,
    pub life_phys: f64,
}

impl Default for DevContext {
    fn default() -> Self {
        DevContext {
            coaching: 50.0,
            facilities: 50.0,
            minutes_share: 0.5,
            life_skill: 1.0,
            life_phys: 1.0,
        }
    }
}

/// Advance a player by one year of development (called after the season; `next_age` is his age
/// at the start of the next season).
pub fn develop(p: &mut Player, next_age: i32, ctx: &DevContext, prm: &ProgParams, rng: &mut Rng) {
    let age_from = (next_age - 1) as f64;
    let h = p.hidden.clone();
    // Late bloomers/early peakers shift the player's age curve.
    let shift = (h.peak_age as f64 - 27.0) + h.bloom as f64 * prm.late_bloomers;
    let eff_age = age_from - shift;
    let we = 0.8 + 0.4 * h.work_ethic as f64 / 100.0;
    let coach = 0.9 + 0.2 * ctx.coaching / 100.0;
    let fac = 0.97 + 0.06 * ctx.facilities / 100.0;
    let pt = 0.8 + 0.2 * (ctx.minutes_share * 2.0).clamp(0.0, 1.0);
    let gap = (p.potential as f64 - p.ovr as f64 + 6.0) / 10.0;
    let young_growth = gap.clamp(0.25, 2.0);
    let ps = p.dev_focus;
    let mut shock = rng.gauss(0.0, 1.0 * prm.variance);
    // Busts stall; breakouts surge.
    if h.bust > 0.0 && age_from <= 23.0 {
        shock -= h.bust as f64 * 1.5;
    }
    // coachability and mood matter slightly
    let coachable = 0.9 + 0.2 * h.coachability as f64 / 100.0;
    for a in Attr::ALL {
        let fam = a.family();
        let exp = yearly_delta(fam, eff_age);
        let mut d = if exp > 0.0 {
            // growth
            let base_scale = prm.speed * we * coach * fac * pt * young_growth * coachable;
            let life = if fam == Family::Athletic {
                ctx.life_phys
            } else {
                ctx.life_skill
            };
            exp * base_scale * life
        } else {
            // decline: slowed by longevity setting/sports science, accelerated by wear
            let slow = prm.longevity * prm.science;
            exp / slow.max(0.3) * (1.0 + p.wear as f64 / 250.0)
        };
        if ps.targets().contains(a) {
            d += if exp > 0.0 { 0.9 } else { 0.6 } * we;
        }
        if ps == DevFocus::Conditioning && matches!(a, Attr::Stamina | Attr::Durability) {
            d += 0.6;
        }
        // durability slowly erodes with age & wear regardless
        let noise = rng.gauss(0.0, 1.1 * prm.variance);
        p.attrs.add(*a, d + noise + shock * 0.8);
    }
    p.recompute_ovr();
    // Potential drifts: revealed over time; converges on current ability by the late 20s.
    let pot = p.potential as f64;
    let ovr = p.ovr as f64;
    let drift = if next_age <= 24 {
        rng.gauss(0.0, 1.6 * prm.variance) - h.bust as f64 * 1.2
    } else {
        (ovr - pot) * 0.5
    };
    let new_pot = (pot + drift).max(ovr).min(99.0);
    p.potential = if next_age >= 27 {
        ovr.max(p.potential.min(ovr as u8 + 2) as f64) as u8
    } else {
        new_pot.round() as u8
    };
    // Wear recovers a little each offseason.
    p.wear = (p.wear - 3.0).max(0.0);
}

/// Chance (0-1) that a player retires this offseason (before contract/free-agent status is considered).
pub fn retirement_chance(
    p: &Player,
    next_age: i32,
    replacement_ovr: f64,
    wealth_bonus: f64,
    longevity: f64,
) -> f64 {
    if p.flags.contains("final_season") {
        return 1.0;
    }
    let age = next_age as f64;
    let decline = (replacement_ovr - p.ovr as f64).max(0.0);
    let x = (age - 36.0 * longevity.max(0.5)) * 0.55
        + decline * 0.09
        + wealth_bonus
        + (p.wear as f64 - 40.0) / 60.0;
    let base = 1.0 / (1.0 + (-x).exp());
    if age < 31.0 {
        // Early retirement only after catastrophic wear or injuries.
        return (base * 0.02).min(0.05);
    }
    base.clamp(0.0, 0.97)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::generate::*;

    #[test]
    fn young_players_grow_and_old_players_decline() {
        let content = Content::default();
        let mut rng = Rng::new(11);
        let prm = ProgParams::standard();
        let mut young_gain = 0.0;
        let mut old_loss = 0.0;
        for i in 0..150 {
            let spec = GenSpec::new(2000, 20, 55.0, 75.0, OriginKind::College);
            let mut p = generate_player(&content, &mut rng, i, &spec, 1.0);
            let before = p.ovr as f64;
            develop(&mut p, 21, &DevContext::default(), &prm, &mut rng);
            young_gain += p.ovr as f64 - before;
            let spec = GenSpec::new(2000, 34, 70.0, 70.0, OriginKind::College);
            let mut o = generate_player(&content, &mut rng, 1000 + i, &spec, 1.0);
            let before = o.ovr as f64;
            develop(&mut o, 35, &DevContext::default(), &prm, &mut rng);
            old_loss += before - o.ovr as f64;
        }
        assert!(
            young_gain / 150.0 > 2.0,
            "young gain {}",
            young_gain / 150.0
        );
        assert!(old_loss / 150.0 > 1.5, "old loss {}", old_loss / 150.0);
    }

    #[test]
    fn retirement_rises_with_age() {
        let content = Content::default();
        let mut rng = Rng::new(2);
        let spec = GenSpec::new(2000, 30, 50.0, 50.0, OriginKind::College);
        let p = generate_player(&content, &mut rng, 1, &spec, 1.0);
        assert!(
            retirement_chance(&p, 31, 45.0, 0.0, 1.0) < retirement_chance(&p, 38, 45.0, 0.0, 1.0)
        );
        assert!(retirement_chance(&p, 38, 45.0, 0.0, 1.0) > 0.5);
    }
}
