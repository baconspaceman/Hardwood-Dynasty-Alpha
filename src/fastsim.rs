//! A fast, team-level game model used for leagues we don't need possession-by-possession detail
//! for (college and overseas by default). It still produces believable box scores so every
//! player has real statistics, awards, and scouting reports.
//!
//! The *full* possession engine in `game.rs` can be used for these leagues too (see the
//! "College sim detail" setting); this one is simply ~100x faster.

use crate::player::*;
use crate::rng::Rng;
use crate::types::*;

#[derive(Clone, Debug)]
pub struct FastPlayer {
    pub id: PlayerId,
    pub ovr: f64,
    pub off: f64,
    pub reb: f64,
    pub pass: f64,
    pub def: f64,
    pub three: f64,
    pub height: f64,
    /// Expected minutes (out of 40).
    pub min: f64,
}

impl FastPlayer {
    pub fn from_player(p: &Player) -> FastPlayer {
        let a = &p.attrs;
        let f = |fam: Family| a.family_avg(fam);
        let mut o = [f(Family::Inside), f(Family::Mid), f(Family::Three)];
        o.sort_by(|x, y| y.partial_cmp(x).unwrap());
        FastPlayer {
            id: p.id,
            ovr: p.ovr as f64 * crate::injury::performance_penalty(p),
            off: 0.5 * o[0] + 0.3 * o[1] + 0.2 * f(Family::Playmaking),
            reb: f(Family::Rebounding),
            pass: f(Family::Playmaking),
            def: f(Family::PerimeterD).max(f(Family::InteriorD)),
            three: a.get(Attr::ThreePoint),
            height: p.height_in as f64,
            min: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FastTeam {
    pub players: Vec<FastPlayer>,
    /// 0..1 tempo preference (0.5 normal).
    pub tempo: f64,
}

impl FastTeam {
    /// Build a team from available players, assigning minutes by rank.
    pub fn build(mut players: Vec<FastPlayer>) -> FastTeam {
        players.sort_by(|a, b| b.ovr.partial_cmp(&a.ovr).unwrap());
        let mins = [32.0, 30.0, 28.0, 27.0, 25.0, 20.0, 16.0, 12.0, 6.0, 4.0];
        for (i, p) in players.iter_mut().enumerate() {
            p.min = mins.get(i).copied().unwrap_or(0.0);
        }
        // 200 minutes per 40-minute game
        let total: f64 = players.iter().map(|p| p.min).sum();
        if total > 0.0 {
            let k = 200.0 / total;
            for p in players.iter_mut() {
                p.min *= k;
            }
        }
        FastTeam {
            players,
            tempo: 0.5,
        }
    }

    pub fn rating(&self) -> f64 {
        let tot: f64 = self.players.iter().map(|p| p.min).sum::<f64>().max(1.0);
        self.players.iter().map(|p| p.ovr * p.min).sum::<f64>() / tot
    }
    fn off_rating(&self) -> f64 {
        let tot: f64 = self.players.iter().map(|p| p.min).sum::<f64>().max(1.0);
        self.players.iter().map(|p| p.off * p.min).sum::<f64>() / tot
    }
    fn def_rating(&self) -> f64 {
        let tot: f64 = self.players.iter().map(|p| p.min).sum::<f64>().max(1.0);
        self.players.iter().map(|p| p.def * p.min).sum::<f64>() / tot
    }
}

pub struct FastResult {
    pub pts: [u16; 2],
    pub overtimes: u8,
    pub lines: [Vec<(PlayerId, StatLine)>; 2],
}

/// Simulate a game. `base_ppg` is a typical team score for the league/era; `home_edge` in points.
pub fn simulate(
    a: &FastTeam,
    b: &FastTeam,
    base_ppg: f64,
    home_edge: f64,
    three_rate: f64,
    rng: &mut Rng,
) -> FastResult {
    let tempo = (a.tempo + b.tempo) / 2.0;
    let total_base = base_ppg * (0.96 + 0.08 * tempo);
    let ra = 0.5 * a.off_rating() + 0.5 * a.rating() - 0.25 * (b.def_rating() - 55.0);
    let rb = 0.5 * b.off_rating() + 0.5 * b.rating() - 0.25 * (a.def_rating() - 55.0);
    let margin = (ra - rb) * 0.55 + home_edge;
    let mut pa = total_base + margin / 2.0 + rng.gauss(0.0, 7.5);
    let mut pb = total_base - margin / 2.0 + rng.gauss(0.0, 7.5);
    pa = pa.max(25.0);
    pb = pb.max(25.0);
    let mut ia = pa.round() as u16;
    let mut ib = pb.round() as u16;
    let mut ot = 0;
    while ia == ib && ot < 5 {
        let extra_a = (base_ppg * 0.12 + (ra - rb) * 0.05 + rng.gauss(0.0, 3.0))
            .round()
            .max(2.0) as u16;
        let extra_b = (base_ppg * 0.12 - (ra - rb) * 0.05 + rng.gauss(0.0, 3.0))
            .round()
            .max(2.0) as u16;
        ia += extra_a;
        ib += extra_b;
        ot += 1;
    }
    if ia == ib {
        ia += 1;
    }
    let minutes_scale = 1.0 + ot as f64 * 0.12;
    let lines_a = allocate(a, ia, three_rate, minutes_scale, rng);
    let lines_b = allocate(b, ib, three_rate, minutes_scale, rng);
    FastResult {
        pts: [ia, ib],
        overtimes: ot,
        lines: [lines_a, lines_b],
    }
}

fn allocate(
    t: &FastTeam,
    pts: u16,
    three_rate: f64,
    mscale: f64,
    rng: &mut Rng,
) -> Vec<(PlayerId, StatLine)> {
    let n = t.players.len();
    if n == 0 {
        return vec![];
    }
    let w: Vec<f64> = t
        .players
        .iter()
        .map(|p| (p.min * (p.off / 55.0).powf(2.5)).max(0.0))
        .collect();
    let tw: f64 = w.iter().sum::<f64>().max(1.0);
    // distribute points by sampling each point-chunk
    let mut ppl = vec![0u32; n];
    let mut remaining = pts as i64;
    for i in 0..n {
        let share = w[i] / tw;
        let v = (pts as f64 * share + rng.gauss(0.0, 1.8 * share.sqrt() * 3.0))
            .round()
            .max(0.0) as i64;
        let v = v.min(remaining);
        ppl[i] = v as u32;
        remaining -= v;
    }
    // leftovers to the top scorer(s)
    let mut i = 0;
    while remaining > 0 {
        let k = rng.weighted(&w);
        ppl[k] += 1;
        remaining -= 1;
        i += 1;
        if i > 200 {
            break;
        }
    }
    let team_reb = (pts as f64 * 0.43 + rng.gauss(0.0, 3.0)).max(15.0);
    let team_ast = (pts as f64 * 0.22 + rng.gauss(0.0, 2.0)).max(5.0);
    let team_stl = (pts as f64 * 0.075 + rng.gauss(0.0, 1.5)).max(0.0);
    let team_blk = (pts as f64 * 0.045 + rng.gauss(0.0, 1.2)).max(0.0);
    let reb_w: Vec<f64> = t
        .players
        .iter()
        .map(|p| p.min * (p.reb / 55.0).powf(2.0) * (1.0 + (p.height - 78.0) * 0.04).max(0.3))
        .collect();
    let ast_w: Vec<f64> = t
        .players
        .iter()
        .map(|p| p.min * (p.pass / 55.0).powf(2.5))
        .collect();
    let def_w: Vec<f64> = t
        .players
        .iter()
        .map(|p| p.min * (p.def / 55.0).powf(2.0))
        .collect();
    let blk_w: Vec<f64> = t
        .players
        .iter()
        .map(|p| p.min * (p.def / 55.0).powf(2.0) * (1.0 + (p.height - 78.0) * 0.08).max(0.2))
        .collect();
    let sum = |v: &Vec<f64>| v.iter().sum::<f64>().max(1.0);
    let (sr, sa, sd, sb) = (sum(&reb_w), sum(&ast_w), sum(&def_w), sum(&blk_w));
    let mut out = vec![];
    for (i, p) in t.players.iter().enumerate() {
        let started = i < 5;
        let min = p.min * mscale;
        if min < 0.5 {
            continue;
        }
        let mut s = StatLine {
            g: 1,
            gs: if started { 1 } else { 0 },
            min,
            pts: ppl[i],
            ..Default::default()
        };
        // shooting split
        let pt = ppl[i] as f64;
        let ft_pts = (pt * 0.17).round();
        let ftm = ft_pts as u32;
        s.ftm = ftm;
        s.fta = (ft_pts / 0.74 + rng.gauss(0.0, 0.8)).round().max(ft_pts) as u32;
        let field = pt - ft_pts;
        let tpa_share = (three_rate * (0.6 + p.three / 90.0)).clamp(0.0, 0.7);
        let threes_made = if three_rate > 0.0 {
            ((field * tpa_share * 0.33) / 3.0 * 3.0 / 3.0)
                .round()
                .min(field / 3.0)
        } else {
            0.0
        };
        s.tpm = threes_made as u32;
        let two_pts = field - threes_made * 3.0;
        let twos = (two_pts / 2.0).round().max(0.0);
        s.fgm = (twos + threes_made) as u32;
        let fg_pct = (0.44 + (p.off - 55.0) * 0.003).clamp(0.3, 0.62);
        s.fga = ((s.fgm as f64 / fg_pct) + rng.gauss(0.0, 0.7))
            .round()
            .max(s.fgm as f64) as u32;
        s.tpa = ((s.tpm as f64 / 0.34) + rng.gauss(0.0, 0.5))
            .round()
            .max(s.tpm as f64) as u32;
        s.tpa = if three_rate > 0.0 { s.tpa.min(s.fga) } else { 0 };
        let mut m = |w: f64, tot: f64, sumw: f64| {
            ((tot * w / sumw) + rng.gauss(0.0, 0.5)).round().max(0.0) as u32
        };
        let r = m(reb_w[i], team_reb, sr);
        s.orb = (r as f64 * 0.28).round() as u32;
        s.drb = r - s.orb;
        s.ast = m(ast_w[i], team_ast, sa);
        s.stl = m(def_w[i], team_stl, sd);
        s.blk = m(blk_w[i], team_blk, sb);
        s.tov = ((min / 40.0) * (2.2 - (p.pass - 55.0) * 0.01) + rng.gauss(0.0, 0.7))
            .round()
            .max(0.0) as u32;
        s.pf = ((min / 40.0) * 2.4 + rng.gauss(0.0, 1.0))
            .round()
            .clamp(0.0, 5.0) as u32;
        out.push((p.id, s));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::generate::*;

    fn team(c: &Content, rng: &mut Rng, ovr: f64) -> FastTeam {
        let ps: Vec<FastPlayer> = (0..10)
            .map(|i| {
                let spec = GenSpec::new(
                    2000,
                    20,
                    ovr - i as f64,
                    ovr - i as f64,
                    OriginKind::College,
                );
                FastPlayer::from_player(&generate_player(c, rng, i, &spec, 1.0))
            })
            .collect();
        FastTeam::build(ps)
    }

    #[test]
    fn better_teams_win_more_and_box_adds_up() {
        let c = Content::default();
        let mut rng = Rng::new(1);
        let a = team(&c, &mut rng, 70.0);
        let b = team(&c, &mut rng, 56.0);
        let mut wins = 0;
        for _ in 0..200 {
            let r = simulate(&a, &b, 72.0, 3.0, 0.2, &mut rng);
            if r.pts[0] > r.pts[1] {
                wins += 1;
            }
            let sum: u32 = r.lines[0].iter().map(|(_, s)| s.pts).sum();
            assert_eq!(sum, r.pts[0] as u32);
        }
        assert!(wins > 130, "strong team won only {wins}/200");
    }
}
