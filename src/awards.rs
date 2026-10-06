//! Awards: MVP, Defensive Player of the Year, All-League teams and more.
//!
//! Each award is data (`AwardDef`): the year it was introduced, which stats matter and how
//! much, and who is eligible. A mod can add "Best Dunker" or retune MVP in a few lines.
//! The *voting* is a transparent score, `Σ weight × per-game stat`, plus a little noise for
//! "voter bias", so results feel human but explainable.

use crate::rng::Rng;
use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AwardKind {
    /// One winner.
    Single,
    /// A set of teams of 5 (All-League 1st/2nd/3rd...).
    AllTeam,
    /// Picks the best players regardless of position, N of them (All-Star selections).
    Roster,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwardDef {
    pub id: String,
    pub name: String,
    pub first_year: i32,
    #[serde(default = "far")]
    pub last_year: i32,
    pub kind: AwardKind,
    /// (stat key, weight). Keys: ppg rpg apg spg bpg mpg ts ovr win_pct net tov_pg fg_pct ovr_gain def_rating
    pub metric: Vec<(String, f64)>,
    /// Fraction of the schedule a player must have played (0-1).
    #[serde(default)]
    pub min_games_pct: f64,
    #[serde(default)]
    pub min_mpg: f64,
    #[serde(default)]
    pub rookies_only: bool,
    /// Mostly-bench players only (started fewer than a third of games).
    #[serde(default)]
    pub bench_only: bool,
    /// For team awards: number of players per team (usually 5).
    #[serde(default)]
    pub team_size: u8,
    /// For `AllTeam`: number of teams. For `Roster`: roster size.
    #[serde(default)]
    pub count: u8,
    /// Share of votes that is random noise (0 = pure formula).
    #[serde(default)]
    pub noise: f64,
}

fn far() -> i32 {
    9999
}

#[allow(clippy::too_many_arguments)]
fn aw(
    id: &str,
    name: &str,
    first: i32,
    kind: AwardKind,
    metric: &[(&str, f64)],
    min_g: f64,
    min_mpg: f64,
    noise: f64,
) -> AwardDef {
    AwardDef {
        id: id.into(),
        name: name.into(),
        first_year: first,
        last_year: far(),
        kind,
        metric: metric.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        min_games_pct: min_g,
        min_mpg,
        rookies_only: false,
        bench_only: false,
        team_size: 0,
        count: 0,
        noise,
    }
}

pub fn builtin_awards() -> Vec<AwardDef> {
    let off = &[
        ("ppg", 1.0),
        ("rpg", 0.55),
        ("apg", 0.75),
        ("spg", 0.9),
        ("bpg", 0.7),
        ("ts", 18.0),
        ("win_pct", 28.0),
        ("ovr", 0.15),
    ];
    vec![
        aw(
            "mvp",
            "Most Valuable Player",
            1955,
            AwardKind::Single,
            off,
            0.6,
            25.0,
            0.05,
        ),
        aw(
            "dpoy",
            "Defensive Player of the Year",
            1982,
            AwardKind::Single,
            &[
                ("spg", 3.0),
                ("bpg", 3.0),
                ("rpg", 0.6),
                ("def_rating", 0.08),
                ("win_pct", 6.0),
            ],
            0.6,
            22.0,
            0.06,
        ),
        AwardDef {
            rookies_only: true,
            ..aw(
                "roy",
                "Rookie of the Year",
                1952,
                AwardKind::Single,
                &[
                    ("ppg", 1.0),
                    ("rpg", 0.6),
                    ("apg", 0.7),
                    ("spg", 0.6),
                    ("bpg", 0.5),
                    ("ts", 10.0),
                    ("win_pct", 6.0),
                ],
                0.5,
                15.0,
                0.04,
            )
        },
        AwardDef {
            bench_only: true,
            ..aw(
                "sixth",
                "Sixth Man of the Year",
                1982,
                AwardKind::Single,
                &[
                    ("ppg", 1.0),
                    ("rpg", 0.5),
                    ("apg", 0.6),
                    ("ts", 12.0),
                    ("win_pct", 5.0),
                ],
                0.5,
                15.0,
                0.04,
            )
        },
        aw(
            "mip",
            "Most Improved Player",
            1985,
            AwardKind::Single,
            &[("ovr_gain", 2.5), ("ppg", 0.35), ("win_pct", 3.0)],
            0.5,
            15.0,
            0.06,
        ),
        AwardDef {
            team_size: 5,
            count: 3,
            ..aw(
                "all_league",
                "All-League Team",
                1946,
                AwardKind::AllTeam,
                off,
                0.5,
                20.0,
                0.03,
            )
        },
        AwardDef {
            team_size: 5,
            count: 2,
            ..aw(
                "all_defense",
                "All-Defensive Team",
                1968,
                AwardKind::AllTeam,
                &[
                    ("spg", 3.0),
                    ("bpg", 3.0),
                    ("rpg", 0.6),
                    ("def_rating", 0.08),
                    ("win_pct", 6.0),
                ],
                0.5,
                20.0,
                0.05,
            )
        },
        AwardDef {
            team_size: 5,
            count: 2,
            rookies_only: true,
            ..aw(
                "all_rookie",
                "All-Rookie Team",
                1962,
                AwardKind::AllTeam,
                &[("ppg", 1.0), ("rpg", 0.6), ("apg", 0.7), ("ts", 10.0)],
                0.4,
                12.0,
                0.04,
            )
        },
        AwardDef {
            count: 24,
            ..aw(
                "all_star",
                "All-Star Selection",
                1950,
                AwardKind::Roster,
                off,
                0.4,
                20.0,
                0.08,
            )
        },
    ]
}

/// A player's numbers for the season, as the award system sees them.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: PlayerId,
    pub team: TeamId,
    pub games: u16,
    pub started: u16,
    pub mpg: f64,
    pub ppg: f64,
    pub rpg: f64,
    pub apg: f64,
    pub spg: f64,
    pub bpg: f64,
    pub ts: f64,
    pub fg_pct: f64,
    pub tov_pg: f64,
    pub team_win_pct: f64,
    pub ovr: f64,
    pub ovr_gain: f64,
    pub def_rating: f64,
    pub rookie: bool,
    pub position_group: u8,
}

impl Candidate {
    fn stat(&self, key: &str) -> f64 {
        match key {
            "ppg" => self.ppg,
            "rpg" => self.rpg,
            "apg" => self.apg,
            "spg" => self.spg,
            "bpg" => self.bpg,
            "mpg" => self.mpg,
            "ts" => self.ts,
            "fg_pct" => self.fg_pct,
            "tov_pg" => self.tov_pg,
            "win_pct" => self.team_win_pct,
            "ovr" => self.ovr,
            "ovr_gain" => self.ovr_gain,
            "def_rating" => self.def_rating,
            _ => 0.0,
        }
    }
}

pub fn score(def: &AwardDef, c: &Candidate, rng: &mut Rng) -> f64 {
    let base: f64 = def.metric.iter().map(|(k, w)| c.stat(k) * w).sum();
    base * (1.0 + rng.gauss(0.0, def.noise))
}

/// Who is eligible?
pub fn eligible(def: &AwardDef, c: &Candidate, season_games: u16) -> bool {
    if (c.games as f64) < def.min_games_pct * season_games as f64 {
        return false;
    }
    if c.mpg < def.min_mpg {
        return false;
    }
    if def.rookies_only && !c.rookie {
        return false;
    }
    if def.bench_only && (c.started as f64) > 0.33 * c.games as f64 {
        return false;
    }
    true
}

/// Results of one award.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwardResult {
    pub season: Season,
    pub award: String,
    pub award_id: String,
    /// Winners (single award: 1). For team awards: players grouped by team index (0 = first team).
    pub winners: Vec<(PlayerId, u8)>,
}

/// Decide an award from candidates. Returns winners as (player, team-index).
pub fn decide(
    def: &AwardDef,
    cands: &[Candidate],
    season_games: u16,
    season: Season,
    rng: &mut Rng,
) -> Option<AwardResult> {
    if season < def.first_year || season > def.last_year {
        return None;
    }
    let mut scored: Vec<(f64, &Candidate)> = cands
        .iter()
        .filter(|c| eligible(def, c, season_games))
        .map(|c| (score(def, c, rng), c))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    if scored.is_empty() {
        return None;
    }
    let winners: Vec<(PlayerId, u8)> = match def.kind {
        AwardKind::Single => vec![(scored[0].1.id, 0)],
        AwardKind::Roster => scored
            .iter()
            .take(def.count as usize)
            .map(|(_, c)| (c.id, 0))
            .collect(),
        AwardKind::AllTeam => {
            // Early eras picked 2 forwards, 1 center, 2 guards; we simply take best 5 per team
            // while balancing so at most 3 players of one position group make a team.
            let mut out = vec![];
            let mut pool: Vec<&Candidate> = scored.iter().map(|(_, c)| *c).collect();
            for t in 0..def.count {
                let mut team: Vec<&Candidate> = vec![];
                let mut i = 0;
                while team.len() < def.team_size as usize && i < pool.len() {
                    let c = pool[i];
                    let same = team
                        .iter()
                        .filter(|x| x.position_group == c.position_group)
                        .count();
                    if same < 3 {
                        team.push(c);
                        pool.remove(i);
                    } else {
                        i += 1;
                    }
                }
                // fill remaining if balance blocked
                while team.len() < def.team_size as usize && !pool.is_empty() {
                    team.push(pool.remove(0));
                }
                out.extend(team.iter().map(|c| (c.id, t)));
            }
            out
        }
    };
    Some(AwardResult {
        season,
        award: def.name.clone(),
        award_id: def.id.clone(),
        winners,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: u32, ppg: f64, win: f64) -> Candidate {
        Candidate {
            id,
            team: 0,
            games: 78,
            started: 78,
            mpg: 34.0,
            ppg,
            rpg: 6.0,
            apg: 4.0,
            spg: 1.0,
            bpg: 0.5,
            ts: 0.57,
            fg_pct: 0.48,
            tov_pg: 2.0,
            team_win_pct: win,
            ovr: 80.0,
            ovr_gain: 0.0,
            def_rating: 100.0,
            rookie: false,
            position_group: (id % 3) as u8,
        }
    }

    #[test]
    fn mvp_goes_to_best_numbers_and_respects_intro_year() {
        let mut rng = Rng::new(1);
        let cands = vec![cand(1, 31.0, 0.7), cand(2, 22.0, 0.5), cand(3, 18.0, 0.4)];
        let defs = builtin_awards();
        let mvp = defs.iter().find(|d| d.id == "mvp").unwrap();
        let r = decide(mvp, &cands, 82, 2000, &mut rng).unwrap();
        assert_eq!(r.winners[0].0, 1);
        assert!(decide(mvp, &cands, 82, 1950, &mut rng).is_none());
    }

    #[test]
    fn all_team_picks_five_per_team() {
        let mut rng = Rng::new(2);
        let cands: Vec<Candidate> = (0..30)
            .map(|i| cand(i, 30.0 - i as f64 * 0.5, 0.5))
            .collect();
        let defs = builtin_awards();
        let at = defs.iter().find(|d| d.id == "all_league").unwrap();
        let r = decide(at, &cands, 82, 2000, &mut rng).unwrap();
        assert_eq!(r.winners.len(), 15);
        assert_eq!(r.winners.iter().filter(|w| w.1 == 0).count(), 5);
    }
}
