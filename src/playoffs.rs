//! Playoffs: era-correct brackets, series lengths, home-court patterns, and the play-in tournament.

use crate::league::*;
use crate::season::DayReport;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Series {
    pub round: u8,
    pub conf: Option<u8>,
    /// Higher seed (hosts game 1).
    pub high: TeamId,
    pub low: TeamId,
    pub high_seed: u8,
    pub low_seed: u8,
    pub best_of: u8,
    pub wins_high: u8,
    pub wins_low: u8,
    /// (high team points, low team points) per game.
    pub games: Vec<(u16, u16)>,
    pub winner: Option<TeamId>,
}

impl Series {
    pub fn needed(&self) -> u8 {
        self.best_of / 2 + 1
    }
    pub fn done(&self) -> bool {
        self.winner.is_some()
    }
    pub fn score_text(&self, names: impl Fn(TeamId) -> String) -> String {
        match self.winner {
            Some(w) => {
                let (wn, ww, wl) = if w == self.high {
                    (names(w), self.wins_high, self.wins_low)
                } else {
                    (names(w), self.wins_low, self.wins_high)
                };
                format!("{wn} win {ww}-{wl}")
            }
            None => format!(
                "{} {}-{} {}",
                names(self.high),
                self.wins_high,
                self.wins_low,
                names(self.low)
            ),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PlayInConf {
    pub conf: u8,
    /// Seeds 7..=10 as team ids [7th, 8th, 9th, 10th].
    pub teams: Vec<TeamId>,
    pub stage: u8,
    pub seed7: Option<TeamId>,
    pub seed8: Option<TeamId>,
    pub loser_78: Option<TeamId>,
    pub winner_910: Option<TeamId>,
    pub done: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PlayoffState {
    pub rounds: Vec<Vec<Series>>,
    pub round: usize,
    pub series_lengths: Vec<u8>,
    pub conf_rounds: usize,
    pub play_in: Vec<PlayInConf>,
    pub play_in_active: bool,
    /// Direct qualifiers per conference in seed order.
    pub seeds: BTreeMap<u8, Vec<TeamId>>,
    pub champion: Option<TeamId>,
    pub runner_up: Option<TeamId>,
    pub finals_mvp: Option<PlayerId>,
    /// Team -> text of how far they went.
    pub results: BTreeMap<TeamId, String>,
}

fn pattern(best_of: u8, finals_232: bool) -> Vec<bool> {
    // true = higher seed hosts
    match best_of {
        1 => vec![true],
        3 => vec![true, false, true],
        5 => vec![true, true, false, false, true],
        7 if finals_232 => vec![true, true, false, false, false, true, true],
        7 => vec![true, true, false, false, true, false, true],
        n => (0..n).map(|i| i % 2 == 0).collect(),
    }
}

impl League {
    pub fn playoff_round_name(&self, round: usize, total_rounds: usize) -> String {
        let from_end = total_rounds - 1 - round;
        match from_end {
            0 => "Finals".into(),
            1 => "Conference Finals".into(),
            2 => "Conference Semifinals".into(),
            _ => format!("Round {}", round + 1),
        }
    }

    /// Does the league hold a postseason this year?
    pub fn has_playoffs(&self) -> bool {
        self.rules.playoff_teams >= 2 && self.active_team_ids().len() >= 4
    }

    pub fn start_playoffs(&mut self) {
        let mut st = PlayoffState::default();
        st.series_lengths = self.rules.series_lengths.clone();
        if !self.has_playoffs() {
            self.playoffs = Some(st);
            self.phase = Phase::Playoffs;
            return;
        }
        let total = (self.rules.playoff_teams as usize).min(self.active_team_ids().len());
        let per_conf = (total / 2).max(1);
        let direct = if self.rules.play_in {
            per_conf.saturating_sub(2).max(1)
        } else {
            per_conf
        };
        for conf in 0..2u8 {
            let order = self.standings(Some(conf));
            if self.rules.play_in && order.len() >= 10 {
                let q: Vec<TeamId> = order.iter().take(direct).copied().collect();
                st.seeds.insert(conf, q);
                st.play_in.push(PlayInConf {
                    conf,
                    teams: order[direct..(direct + 4).min(order.len())].to_vec(),
                    ..Default::default()
                });
            } else {
                let q: Vec<TeamId> = order
                    .iter()
                    .take(per_conf.min(order.len()))
                    .copied()
                    .collect();
                st.seeds.insert(conf, q);
            }
        }
        st.play_in_active = !st.play_in.is_empty();
        let max_q = st.seeds.values().map(|v| v.len()).max().unwrap_or(1)
            + if st.play_in_active { 2 } else { 0 };
        let mut p2 = 1usize;
        let mut cr = 0usize;
        while p2 < max_q {
            p2 *= 2;
            cr += 1;
        }
        st.conf_rounds = cr.max(1);
        self.playoffs = Some(st);
        if self.playoffs.as_ref().unwrap().play_in_active {
            self.phase = Phase::PlayIn;
            self.add_news("major", "The play-in tournament begins!");
        } else {
            self.phase = Phase::Playoffs;
            self.build_first_round();
            self.add_news("major", format!("The {} playoffs begin!", self.year));
        }
    }

    fn series_len(&self, round: usize, conf_rounds: usize) -> u8 {
        let v = &self.rules.series_lengths;
        if v.is_empty() {
            return 7;
        }
        if round >= conf_rounds {
            return *v.last().unwrap();
        }
        let idx = round.min(v.len().saturating_sub(2));
        v[idx.min(v.len() - 1)]
    }

    /// Build the first round (after the play-in, if any) for both conferences.
    fn build_first_round(&mut self) {
        let mut st = self.playoffs.take().unwrap();
        let cr = st.conf_rounds;
        let mut round: Vec<Series> = vec![];
        for conf in 0..2u8 {
            let seeds = st.seeds.get(&conf).cloned().unwrap_or_default();
            let q = seeds.len();
            if q == 0 {
                continue;
            }
            let mut p2 = 1usize;
            while p2 < q {
                p2 *= 2;
            }
            // standard bracket order: 1 v p2, then the rest folded
            let order = bracket_order(p2);
            for pair in order.chunks(2) {
                let (a, b) = (pair[0], pair[1]);
                // seeds are 1-based; seeds > q are byes
                let a_team = seeds.get(a - 1).copied();
                let b_team = seeds.get(b - 1).copied();
                match (a_team, b_team) {
                    (Some(x), Some(y)) => round.push(Series {
                        round: 0,
                        conf: Some(conf),
                        high: x,
                        low: y,
                        high_seed: a as u8,
                        low_seed: b as u8,
                        best_of: self.series_len(0, cr),
                        wins_high: 0,
                        wins_low: 0,
                        games: vec![],
                        winner: None,
                    }),
                    (Some(x), None) => {
                        // bye: auto-advance as a completed series with no games
                        round.push(Series {
                            round: 0,
                            conf: Some(conf),
                            high: x,
                            low: x,
                            high_seed: a as u8,
                            low_seed: a as u8,
                            best_of: 0,
                            wins_high: 0,
                            wins_low: 0,
                            games: vec![],
                            winner: Some(x),
                        });
                    }
                    _ => {}
                }
            }
        }
        st.rounds = vec![round];
        st.round = 0;
        self.playoffs = Some(st);
        self.settle_byes();
    }

    /// Byes (best_of == 0) are already "won"; if the whole round is byes, advance.
    fn settle_byes(&mut self) {
        loop {
            let (all_done, can_advance) = {
                let st = self.playoffs.as_ref().unwrap();
                let r = &st.rounds[st.round];
                (r.iter().all(|s| s.done()), true)
            };
            if all_done && can_advance {
                if !self.advance_round() {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Build the next round from winners. Returns false if the playoffs are over.
    fn advance_round(&mut self) -> bool {
        let mut st = self.playoffs.take().unwrap();
        let round = st.round;
        let cr = st.conf_rounds;
        let finished: Vec<Series> = st.rounds[round].clone();
        // record eliminations
        let total_rounds = cr + 1;
        for s in &finished {
            if s.best_of == 0 {
                continue;
            }
            if let Some(w) = s.winner {
                let loser = if w == s.high { s.low } else { s.high };
                let name = self.playoff_round_name(round, total_rounds);
                let txt = if round == total_rounds - 1 {
                    "Lost in the Finals".to_string()
                } else {
                    format!("Lost in the {name}")
                };
                st.results.insert(loser, txt);
            }
        }
        if round == total_rounds - 1 || (round >= cr && !finished.is_empty()) {
            // finals complete
            let f = finished.last().unwrap();
            st.champion = f.winner;
            st.runner_up = f.winner.map(|w| if w == f.high { f.low } else { f.high });
            if let Some(c) = st.champion {
                st.results.insert(c, "Champions".to_string());
            }
            self.playoffs = Some(st);
            return false;
        }
        let mut next: Vec<Series> = vec![];
        if round + 1 == cr {
            // conference champions meet in the finals
            let champs: Vec<(u8, TeamId)> = finished
                .iter()
                .filter_map(|s| s.winner.map(|w| (s.conf.unwrap_or(0), w)))
                .collect();
            if champs.len() == 2 {
                let (a, b) = (champs[0].1, champs[1].1);
                let (ra, rb) = (self.team(a).record.pct(), self.team(b).record.pct());
                let (high, low) = if ra >= rb { (a, b) } else { (b, a) };
                next.push(Series {
                    round: (round + 1) as u8,
                    conf: None,
                    high,
                    low,
                    high_seed: 1,
                    low_seed: 1,
                    best_of: self.series_len(round + 1, cr),
                    wins_high: 0,
                    wins_low: 0,
                    games: vec![],
                    winner: None,
                });
            } else if champs.len() == 1 {
                // one-conference league: the lone champion wins outright
                st.champion = Some(champs[0].1);
                st.results.insert(champs[0].1, "Champions".to_string());
                self.playoffs = Some(st);
                return false;
            }
        } else {
            for conf in 0..2u8 {
                let winners: Vec<&Series> =
                    finished.iter().filter(|s| s.conf == Some(conf)).collect();
                // pair adjacent bracket slots (1/2, 3/4...)
                for pair in winners.chunks(2) {
                    if pair.len() == 2 {
                        let (x, y) = (pair[0], pair[1]);
                        let (wx, wy) = (x.winner.unwrap(), y.winner.unwrap());
                        let sx = if wx == x.high {
                            x.high_seed
                        } else {
                            x.low_seed
                        };
                        let sy = if wy == y.high {
                            y.high_seed
                        } else {
                            y.low_seed
                        };
                        let ((h, hs), (l, ls)) = if sx <= sy {
                            ((wx, sx), (wy, sy))
                        } else {
                            ((wy, sy), (wx, sx))
                        };
                        next.push(Series {
                            round: (round + 1) as u8,
                            conf: Some(conf),
                            high: h,
                            low: l,
                            high_seed: hs,
                            low_seed: ls,
                            best_of: self.series_len(round + 1, cr),
                            wins_high: 0,
                            wins_low: 0,
                            games: vec![],
                            winner: None,
                        });
                    } else if pair.len() == 1 {
                        let x = pair[0];
                        let w = x.winner.unwrap();
                        next.push(Series {
                            round: (round + 1) as u8,
                            conf: Some(conf),
                            high: w,
                            low: w,
                            high_seed: x.high_seed,
                            low_seed: x.high_seed,
                            best_of: 0,
                            wins_high: 0,
                            wins_low: 0,
                            games: vec![],
                            winner: Some(w),
                        });
                    }
                }
            }
        }
        st.rounds.push(next);
        st.round = round + 1;
        self.playoffs = Some(st);
        true
    }

    /// Simulate one playoff day (all active series play one game; play-in games when active).
    pub fn sim_playoff_day(&mut self) -> DayReport {
        self.day += 1;
        let mut report = DayReport {
            day: self.day,
            ..Default::default()
        };
        let ctx = self.game_context(true);
        let mut rng = self.rng.clone();
        let mut played = vec![false; self.teams.len()];
        let st_present = self.playoffs.is_some();
        if !st_present || !self.has_playoffs() {
            report.season_over = true;
            self.rng = rng;
            return report;
        }
        let play_in = self.playoffs.as_ref().unwrap().play_in_active;
        if play_in {
            self.sim_play_in_stage(&ctx, &mut rng, &mut report, &mut played);
        } else {
            let round = self.playoffs.as_ref().unwrap().round;
            let n = self.playoffs.as_ref().unwrap().rounds[round].len();
            for i in 0..n {
                let s = self.playoffs.as_ref().unwrap().rounds[round][i].clone();
                if s.done() {
                    continue;
                }
                let gnum = s.games.len();
                let finals_232 = s.conf.is_none() && (1985..=2013).contains(&self.year);
                let pat = pattern(s.best_of, finals_232);
                let high_home = pat.get(gnum).copied().unwrap_or(gnum.is_multiple_of(2));
                let (home, away) = if high_home {
                    (s.high, s.low)
                } else {
                    (s.low, s.high)
                };
                played[home as usize] = true;
                played[away as usize] = true;
                let bs = self.play_game(&ctx, home, away, true, &mut rng);
                let (hp, ap) = (bs.home.pts, bs.away.pts);
                let (high_pts, low_pts) = if high_home { (hp, ap) } else { (ap, hp) };
                {
                    let sr = &mut self.playoffs.as_mut().unwrap().rounds[round][i];
                    sr.games.push((high_pts, low_pts));
                    if high_pts > low_pts {
                        sr.wins_high += 1;
                    } else {
                        sr.wins_low += 1;
                    }
                    if sr.wins_high >= sr.needed() {
                        sr.winner = Some(sr.high);
                    } else if sr.wins_low >= sr.needed() {
                        sr.winner = Some(sr.low);
                    }
                }
                report.scores.push(format!(
                    "{} {} - {} {} (G{} of {})",
                    self.team(away).abbr,
                    bs.away.pts,
                    bs.home.pts,
                    self.team(home).abbr,
                    gnum + 1,
                    s.best_of
                ));
                if Some(home) == self.user.team || Some(away) == self.user.team {
                    report.user_game = Some(bs.clone());
                }
                self.box_log.push(bs);
                if self.box_log.len() > 40 {
                    self.box_log.remove(0);
                }
                let done_now = self.playoffs.as_ref().unwrap().rounds[round][i].done();
                if done_now {
                    let (w, l, txt) = {
                        let sr = &self.playoffs.as_ref().unwrap().rounds[round][i];
                        let w = sr.winner.unwrap();
                        let l = if w == sr.high { sr.low } else { sr.high };
                        (w, l, sr.score_text(|t| self.team(t).name()))
                    };
                    let st = self.playoffs.as_ref().unwrap();
                    let rname = self.playoff_round_name(round, st.conf_rounds + 1);
                    let msg = format!("{rname}: {txt} over {}.", self.team(l).name());
                    report.news.push(msg.clone());
                    self.add_news("playoffs", msg);
                    self.story_series_done(w, l, round);
                }
            }
            // round complete?
            let round_done = self.playoffs.as_ref().unwrap().rounds[round]
                .iter()
                .all(|s| s.done());
            if round_done && !self.advance_round() {
                report.season_over = true;
            } else if round_done {
                self.settle_byes();
                if self.playoffs.as_ref().unwrap().champion.is_some() {
                    report.season_over = true;
                }
            }
        }
        self.after_day_health(&played, true, &mut rng);
        self.rng = rng;
        report
    }

    fn sim_play_in_stage(
        &mut self,
        ctx: &crate::game::GameContext,
        rng: &mut crate::rng::Rng,
        report: &mut DayReport,
        played: &mut [bool],
    ) {
        let n = self.playoffs.as_ref().unwrap().play_in.len();
        let mut all_done = true;
        for i in 0..n {
            let pi = self.playoffs.as_ref().unwrap().play_in[i].clone();
            if pi.done {
                continue;
            }
            all_done = false;
            let mut res: Vec<(TeamId, TeamId, TeamId, TeamId)> = vec![]; // (home, away, winner, loser)
            if pi.stage == 0 {
                for (hi, lo) in [(0usize, 1usize), (2, 3)] {
                    let (h, a) = (pi.teams[hi], pi.teams[lo]);
                    let bs = self.play_game(ctx, h, a, true, rng);
                    played[h as usize] = true;
                    played[a as usize] = true;
                    let (w, l) = if bs.home.pts > bs.away.pts {
                        (h, a)
                    } else {
                        (a, h)
                    };
                    report.scores.push(format!(
                        "PLAY-IN {} {} - {} {}",
                        self.team(a).abbr,
                        bs.away.pts,
                        bs.home.pts,
                        self.team(h).abbr
                    ));
                    res.push((h, a, w, l));
                }
                let pi_mut = &mut self.playoffs.as_mut().unwrap().play_in[i];
                pi_mut.seed7 = Some(res[0].2);
                pi_mut.loser_78 = Some(res[0].3);
                pi_mut.winner_910 = Some(res[1].2);
                pi_mut.stage = 1;
                let loser910 = res[1].3;
                self.playoffs
                    .as_mut()
                    .unwrap()
                    .results
                    .insert(loser910, "Eliminated in the play-in".into());
            } else {
                let (h, a) = (pi.loser_78.unwrap(), pi.winner_910.unwrap());
                let bs = self.play_game(ctx, h, a, true, rng);
                played[h as usize] = true;
                played[a as usize] = true;
                let (w, l) = if bs.home.pts > bs.away.pts {
                    (h, a)
                } else {
                    (a, h)
                };
                report.scores.push(format!(
                    "PLAY-IN {} {} - {} {}",
                    self.team(a).abbr,
                    bs.away.pts,
                    bs.home.pts,
                    self.team(h).abbr
                ));
                let conf = pi.conf;
                let st = self.playoffs.as_mut().unwrap();
                st.play_in[i].seed8 = Some(w);
                st.play_in[i].done = true;
                st.results.insert(l, "Eliminated in the play-in".into());
                if let Some(v) = st.seeds.get_mut(&conf) {
                    v.push(st.play_in[i].seed7.unwrap());
                    v.push(w);
                }
            }
        }
        if all_done
            || self
                .playoffs
                .as_ref()
                .unwrap()
                .play_in
                .iter()
                .all(|p| p.done)
        {
            self.playoffs.as_mut().unwrap().play_in_active = false;
            self.phase = Phase::Playoffs;
            self.build_first_round();
            self.add_news("major", format!("The {} playoff field is set.", self.year));
        }
    }

    /// Is the postseason over?
    pub fn playoffs_finished(&self) -> bool {
        match &self.playoffs {
            None => true,
            Some(st) => !self.has_playoffs() || st.champion.is_some(),
        }
    }
}

/// Seed order for a single-elimination bracket of size `p2` (1 vs p2, ...), adjacent pairs meet later.
pub fn bracket_order(p2: usize) -> Vec<usize> {
    let mut order = vec![1usize];
    let mut size = 1;
    while size < p2 {
        size *= 2;
        let mut next = Vec::with_capacity(size);
        for &s in &order {
            next.push(s);
            next.push(size + 1 - s);
        }
        order = next;
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_orders() {
        assert_eq!(bracket_order(2), vec![1, 2]);
        assert_eq!(bracket_order(4), vec![1, 4, 2, 3]);
        assert_eq!(bracket_order(8), vec![1, 8, 4, 5, 2, 7, 3, 6]);
    }
    #[test]
    fn home_patterns() {
        assert_eq!(pattern(7, false).iter().filter(|x| **x).count(), 4);
        assert_eq!(pattern(5, false).len(), 5);
        assert_eq!(
            pattern(7, true),
            vec![true, true, false, false, false, true, true]
        );
    }
}
