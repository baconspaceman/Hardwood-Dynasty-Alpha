//! The draft: eligibility, the lottery (for every era), scouting fog, and the picks themselves.

use crate::era::Lottery;
use crate::league::*;
use crate::player::*;
use crate::rng::{hash_label, Rng};
use crate::team::DraftPick;
use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PickSlot {
    pub overall: u16,
    pub round: u8,
    pub team: TeamId,
    pub original: TeamId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DraftResult {
    pub overall: u16,
    pub round: u8,
    pub team: TeamId,
    pub original: TeamId,
    pub player: PlayerId,
    pub territorial: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LotteryLine {
    pub team: TeamId,
    pub record: String,
    pub best_odds: f64,
    pub pick: u16,
    pub moved: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DraftState {
    pub year: Season,
    pub order: Vec<PickSlot>,
    pub pool: Vec<PlayerId>,
    pub next: usize,
    pub results: Vec<DraftResult>,
    pub lottery: Vec<LotteryLine>,
    pub lottery_text: String,
    pub done: bool,
}

/// Ping-pong-ball style weights for the lottery in each era (resampled to the number of teams).
fn lottery_weights(kind: Lottery, n: usize) -> (Vec<f64>, usize) {
    let (base, draws): (Vec<f64>, usize) = match kind {
        Lottery::Envelope => (vec![1.0; 7], 7),
        Lottery::Weighted => (
            vec![11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0],
            3,
        ),
        Lottery::Weighted94 => (
            vec![
                250.0, 199.0, 156.0, 119.0, 88.0, 63.0, 43.0, 28.0, 17.0, 11.0, 8.0, 7.0, 6.0, 5.0,
            ],
            4,
        ),
        Lottery::Flattened => (
            vec![
                140.0, 140.0, 140.0, 125.0, 105.0, 90.0, 75.0, 60.0, 45.0, 30.0, 20.0, 15.0, 10.0,
                5.0,
            ],
            4,
        ),
        _ => (vec![1.0; n.max(1)], 0),
    };
    // resample to n entries
    let m = base.len();
    let mut w = Vec::with_capacity(n);
    for i in 0..n {
        let pos = if n <= 1 {
            0.0
        } else {
            i as f64 * (m as f64 - 1.0) / (n as f64 - 1.0)
        };
        let lo = pos.floor() as usize;
        let hi = (lo + 1).min(m - 1);
        let t = pos - lo as f64;
        w.push(base[lo] * (1.0 - t) + base[hi] * t);
    }
    (w, draws.min(n))
}

impl League {
    /// What a team believes about a prospect: (overall, potential, uncertainty). Deterministic per (viewer, prospect, year).
    pub fn scouted_view(&self, viewer: Option<TeamId>, id: PlayerId) -> (f64, f64, f64) {
        let p = self.p(id);
        let fog = self.settings.num("draft.scouting_fog");
        let hidden_pot = self.settings.bool("progression.hidden_potential");
        if fog <= 0.0 && !hidden_pot {
            return (p.ovr as f64, p.potential as f64, 0.0);
        }
        let quality = viewer.map(|t| self.scouting_quality(t)).unwrap_or(50.0);
        let familiarity = match viewer {
            Some(t) if self.user.team == Some(t) => {
                self.user.scouted.get(&id).copied().unwrap_or(0) as f64
            }
            _ => 25.0,
        };
        let sigma_o = 5.5 * fog * (1.45 - quality / 100.0) * (1.0 - familiarity / 140.0);
        let sigma_p = if hidden_pot {
            (sigma_o * 1.7).max(2.0 + 5.0 * (1.0 - familiarity / 130.0))
        } else {
            0.0
        };
        let key = format!(
            "{}:{}:{}:{}",
            self.seed,
            self.year,
            viewer.map(|v| v as i32).unwrap_or(-1),
            id
        );
        let mut r = Rng::new(hash_label(&key));
        let o = (p.ovr as f64 + r.gauss(0.0, sigma_o)).clamp(25.0, 99.0);
        let pt = (p.potential as f64 + r.gauss(0.0, sigma_p)).clamp(o, 99.0);
        (o, pt, sigma_o)
    }

    /// Pick value the AI uses for its draft board.
    fn board_value(&self, t: TeamId, id: PlayerId) -> f64 {
        let (o, pt, _) = self.scouted_view(Some(t), id);
        let p = self.p(id);
        let age = (self.year + 1 - p.birth_year) as f64;
        let dir = self.team(t).direction;
        let youth = (22.0 - age).clamp(-2.0, 3.0);
        let mut v = o * 0.5 + pt * 0.5;
        match dir {
            crate::team::Direction::Rebuild => v += youth * 0.8 + (pt - o) * 0.1,
            crate::team::Direction::Contend => v += (o - 50.0).max(0.0) * 0.12 - youth * 0.3,
            _ => {}
        }
        // positional need
        let pos = p.position;
        let have = self
            .team(t)
            .roster
            .iter()
            .filter(|&&r| self.p(r).position == pos)
            .count();
        v -= (have as f64 - 2.5).max(0.0) * 0.8;
        v
    }

    pub fn setup_draft(&mut self, rng: &mut Rng) {
        let year = self.year + 1;
        let dr = self.effective_rules(year);
        let mut pool = self.declared_prospects(rng);
        // the human's created player may have declared
        for p in self
            .players
            .iter()
            .filter(|p| p.user_controlled && p.flags.contains("declared") && !p.is_retired())
        {
            if !pool.contains(&p.id) {
                pool.push(p.id);
            }
        }
        // order
        let ids = self.active_team_ids();
        let mut non_playoff: Vec<TeamId> = vec![];
        let mut playoff: Vec<TeamId> = vec![];
        let in_playoffs: Vec<TeamId> = self
            .playoffs
            .as_ref()
            .map(|p| p.seeds.values().flatten().copied().collect())
            .unwrap_or_default();
        for &t in &ids {
            if in_playoffs.contains(&t) {
                playoff.push(t);
            } else {
                non_playoff.push(t);
            }
        }
        let by_record = |l: &League, v: &mut Vec<TeamId>| {
            v.sort_by(|&a, &b| {
                let (ra, rb) = (&l.team(a).record, &l.team(b).record);
                ra.pct()
                    .partial_cmp(&rb.pct())
                    .unwrap()
                    .then(ra.diff_pg().partial_cmp(&rb.diff_pg()).unwrap())
            })
        };
        by_record(self, &mut non_playoff);
        by_record(self, &mut playoff);
        // playoff teams: better playoff finish picks later (champion last)
        let champ = self.playoffs.as_ref().and_then(|p| p.champion);
        let runner = self.playoffs.as_ref().and_then(|p| p.runner_up);
        playoff.sort_by_key(|&t| {
            if Some(t) == champ {
                3
            } else if Some(t) == runner {
                2
            } else {
                0
            }
        });
        let mut kind = dr.lottery;
        match self.settings.text("draft.lottery_mode").as_str() {
            "worst_first" => kind = Lottery::WorstFirst,
            "flat_lottery" => kind = Lottery::Envelope,
            "random" => kind = Lottery::Envelope,
            _ => {}
        }
        let mode = self.settings.text("draft.lottery_mode");
        let mut lottery_lines: Vec<LotteryLine> = vec![];
        let mut first_round: Vec<TeamId> = vec![];
        let mut text = String::new();
        // all teams in reverse-standing order before the lottery
        let mut all_sorted: Vec<TeamId> = ids.clone();
        by_record(self, &mut all_sorted);
        let basis: Vec<TeamId> = if mode == "random" {
            all_sorted.clone()
        } else {
            non_playoff.clone()
        };
        match kind {
            Lottery::WorstFirst => {
                first_round = all_sorted.clone();
            }
            Lottery::CoinFlip => {
                // worst team in each conference flips a coin for pick 1 (winner gets 1, loser 2)
                let mut order = all_sorted.clone();
                let mut worst_e = None;
                let mut worst_w = None;
                for &t in &order {
                    if self.team(t).conf == 0 && worst_e.is_none() {
                        worst_e = Some(t);
                    }
                    if self.team(t).conf == 1 && worst_w.is_none() {
                        worst_w = Some(t);
                    }
                }
                if let (Some(e), Some(w)) = (worst_e, worst_w) {
                    let (first, second) = if rng.chance(0.5) { (e, w) } else { (w, e) };
                    order.retain(|&t| t != e && t != w);
                    first_round = vec![first, second];
                    first_round.extend(order);
                    text = format!(
                        "Coin flip: {} win the toss for the first pick over {}.",
                        self.team(first).name(),
                        self.team(second).name()
                    );
                } else {
                    first_round = order;
                }
            }
            Lottery::Envelope | Lottery::Weighted | Lottery::Weighted94 | Lottery::Flattened => {
                let n = basis.len().min(14.max(basis.len().min(16)));
                let lot_teams: Vec<TeamId> = basis.iter().take(n).copied().collect();
                let rest_non: Vec<TeamId> = basis.iter().skip(n).copied().collect();
                let (weights, draws) = if mode == "flat_lottery" || mode == "random" {
                    (vec![1.0; lot_teams.len()], lot_teams.len().min(5))
                } else {
                    lottery_weights(kind, lot_teams.len())
                };
                let mut remaining: Vec<(TeamId, f64)> = lot_teams
                    .iter()
                    .copied()
                    .zip(weights.iter().copied())
                    .collect();
                let total_w: f64 = weights.iter().sum();
                let best_odds: Vec<f64> = weights.iter().map(|w| w / total_w * 100.0).collect();
                let mut winners: Vec<TeamId> = vec![];
                for _ in 0..draws {
                    let w: Vec<f64> = remaining.iter().map(|x| x.1).collect();
                    let k = rng.weighted(&w);
                    winners.push(remaining[k].0);
                    remaining.remove(k);
                }
                let mut ordered: Vec<TeamId> = winners.clone();
                ordered.extend(remaining.iter().map(|x| x.0));
                ordered.extend(rest_non.iter());
                // if the lottery covers all teams (random mode), done; else add playoff teams worst-first
                if mode != "random" {
                    ordered.extend(playoff.iter());
                }
                first_round = ordered;
                for (slot, &t) in lot_teams.iter().enumerate() {
                    let pick = first_round.iter().position(|&x| x == t).unwrap_or(slot) as u16 + 1;
                    let r = &self.team(t).record;
                    lottery_lines.push(LotteryLine {
                        team: t,
                        record: format!("{}-{}", r.w, r.l),
                        best_odds: best_odds[slot],
                        pick,
                        moved: slot as i32 + 1 - pick as i32,
                    });
                }
                if let Some(&w) = winners.first() {
                    text = format!(
                        "The {} draft lottery: {} win the top pick{}.",
                        year,
                        self.team(w).name(),
                        if lot_teams.first() == Some(&w) {
                            " (as the favorite)"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
        if kind != Lottery::Envelope
            && kind != Lottery::Weighted
            && kind != Lottery::Weighted94
            && kind != Lottery::Flattened
            && mode != "random"
            && first_round.is_empty()
        {
            first_round = all_sorted.clone();
        }
        // later rounds: reverse standings (worst first)
        let later: Vec<TeamId> = all_sorted.clone();
        let rounds = dr.draft_rounds as usize;
        let mut order: Vec<PickSlot> = vec![];
        let mut overall: u16 = 1;
        for r in 1..=rounds {
            let seq = if r == 1 {
                first_round.clone()
            } else {
                later.clone()
            };
            for (pos, &orig) in seq.iter().enumerate() {
                // who owns this pick?
                let owner = self
                    .team(orig)
                    .picks
                    .iter()
                    .find(|p| p.year == year && p.round == r as u8 && p.original == orig)
                    .map(|p| p.owner);
                let owner_actual = owner.or_else(|| {
                    // pick traded away: find in other teams
                    self.teams
                        .iter()
                        .find(|t| {
                            t.picks
                                .iter()
                                .any(|p| p.year == year && p.round == r as u8 && p.original == orig)
                        })
                        .map(|t| t.id)
                });
                let owner_id = owner_actual.unwrap_or(orig);
                let _ = pos;
                order.push(PickSlot {
                    overall,
                    round: r as u8,
                    team: owner_id,
                    original: orig,
                });
                overall += 1;
            }
        }
        // remove used picks from team pick lists (they're consumed by the draft)
        for t in self.teams.iter_mut() {
            t.picks.retain(|p| p.year != year);
        }
        // protections: top-N protected picks revert (the obligation lapses)
        let mut st = DraftState {
            year,
            order,
            pool,
            next: 0,
            results: vec![],
            lottery: lottery_lines,
            lottery_text: text.clone(),
            done: false,
        };
        if dr.territorial_picks {
            self.territorial_phase(&mut st, rng);
        }
        self.draft = Some(st);
        if !text.is_empty() {
            self.add_news("major", text);
        }
        self.add_news(
            "draft",
            format!(
                "The {} draft class has {} prospects.",
                year,
                self.draft.as_ref().map(|d| d.pool.len()).unwrap_or(0)
            ),
        );
    }

    fn territorial_phase(&mut self, st: &mut DraftState, rng: &mut Rng) {
        // Each team may claim one local star instead of using its first-round pick.
        let mut pool_sorted = st.pool.clone();
        pool_sorted.sort_by(|&a, &b| self.p(b).potential.cmp(&self.p(a).potential));
        let teams = self.active_team_ids();
        let mut taken: Vec<PlayerId> = vec![];
        for &t in &teams {
            if !rng.chance(0.3) {
                continue;
            }
            let cand: Vec<PlayerId> = pool_sorted
                .iter()
                .take(12)
                .copied()
                .filter(|id| !taken.contains(id))
                .collect();
            if cand.is_empty() {
                break;
            }
            let id = cand[rng.range_usize(cand.len().min(8))];
            // team forfeits its first-round pick
            if let Some(pos) = st.order.iter().position(|s| s.round == 1 && s.team == t) {
                st.order.remove(pos);
                taken.push(id);
                st.results.push(DraftResult {
                    overall: 0,
                    round: 1,
                    team: t,
                    original: t,
                    player: id,
                    territorial: true,
                });
            }
        }
        for (i, s) in st.order.iter_mut().enumerate() {
            s.overall = i as u16 + 1;
        }
        // territorial picks are made first
        let terr: Vec<DraftResult> = st.results.clone();
        for r in terr {
            self.apply_pick(r.player, r.team, 0, 1, true);
        }
        st.pool.retain(|id| !taken.contains(id));
    }

    /// True if the user must pick next.
    pub fn user_on_clock(&self) -> bool {
        match (&self.draft, self.user.team) {
            (Some(d), Some(u)) if !d.done => d
                .order
                .get(d.next)
                .map(|s| s.team == u && self.user_controls_roster(u))
                .unwrap_or(false),
            _ => false,
        }
    }

    /// Make the next pick for the team on the clock (AI logic). Returns the result.
    pub fn draft_next_ai(&mut self) -> Option<DraftResult> {
        let (slot, pool) = {
            let d = self.draft.as_ref()?;
            if d.done {
                return None;
            }
            (d.order.get(d.next)?.clone(), d.pool.clone())
        };
        if pool.is_empty() {
            return self.finish_draft_state();
        }
        let mut best: Option<(f64, PlayerId)> = None;
        for &id in &pool {
            let v = self.board_value(slot.team, id);
            if best.map(|b| v > b.0).unwrap_or(true) {
                best = Some((v, id));
            }
        }
        let (_, id) = best?;
        self.make_pick(id)
    }

    /// Execute a pick for whoever is on the clock.
    pub fn make_pick(&mut self, id: PlayerId) -> Option<DraftResult> {
        let slot = self
            .draft
            .as_ref()?
            .order
            .get(self.draft.as_ref()?.next)?
            .clone();
        self.apply_pick(id, slot.team, slot.overall, slot.round, false);
        let res = DraftResult {
            overall: slot.overall,
            round: slot.round,
            team: slot.team,
            original: slot.original,
            player: id,
            territorial: false,
        };
        let d = self.draft.as_mut()?;
        d.results.push(res.clone());
        d.pool.retain(|&x| x != id);
        d.next += 1;
        if d.next >= d.order.len() {
            d.done = true;
        }
        // headlines for top picks
        if slot.overall <= 5 || slot.overall == 1 {
            let p = self.p(id);
            let msg = format!(
                "DRAFT: With pick #{}, {} select {} ({}, {}, {}).",
                slot.overall,
                self.team(slot.team).name(),
                p.name(),
                p.position.name(),
                p.height_str(),
                p.origin.school
            );
            self.add_news("draft", msg);
        }
        Some(res)
    }

    fn finish_draft_state(&mut self) -> Option<DraftResult> {
        if let Some(d) = self.draft.as_mut() {
            d.done = true;
        }
        None
    }

    fn apply_pick(&mut self, id: PlayerId, t: TeamId, overall: u16, round: u8, territorial: bool) {
        let year = self.year + 1;
        let was_overseas = matches!(self.p(id).affiliation, Affiliation::Overseas(_));
        // remove from the place he came from
        match self.p(id).affiliation {
            Affiliation::College(c) => self.colleges[c as usize].roster.retain(|&x| x != id),
            _ => {}
        }
        let tname = self.team(t).name();
        let age_at_draft = year - self.p(id).birth_year;
        let ovr = self.p(id).ovr;
        let stash = was_overseas
            && self.settings.bool("draft.draft_and_stash")
            && (age_at_draft < 22 || ovr < 52)
            && round > 0
            && !self.p(id).user_controlled;
        let _ = territorial;
        {
            let p = self.pm(id);
            p.draft = Some(DraftInfo {
                year,
                round,
                pick: overall,
                team: tname.clone(),
                team_id: Some(t),
            });
            p.flags.remove("declared");
        }
        if stash {
            self.pm(id).draft_rights = Some(t);
            return;
        }
        // Rookie contract
        if round == 0 {
            return;
        }
        let c = self.rookie_contract_for(overall.max(1), round.max(1));
        let user_player = self.p(id).user_controlled;
        if let Affiliation::Overseas(cid) = self.p(id).affiliation {
            self.clubs[cid as usize].roster.retain(|&x| x != id);
        }
        let p = self.pm(id);
        p.contract = Some(c);
        if round >= 2 || overall > 45 {
            if let Some(c) = &mut p.contract {
                c.guaranteed = false;
            }
        }
        p.affiliation = Affiliation::Nba(t);
        p.years_pro = 0;
        p.flags.remove("expiring");
        p.mood.overall = 75.0;
        p.flags.insert("fresh_rookie".into());
        if user_player {
            p.flags.insert("drafted".into());
        }
        self.team_mut(t).roster.push(id);
    }

    /// Run the entire draft for AI teams, stopping if the user is on the clock.
    /// Returns true when the draft is complete.
    pub fn run_draft(&mut self, stop_for_user: bool) -> bool {
        loop {
            if self.draft.as_ref().map(|d| d.done).unwrap_or(true) {
                return true;
            }
            if stop_for_user && self.user_on_clock() {
                return false;
            }
            if self.draft_next_ai().is_none() {
                return self.draft.as_ref().map(|d| d.done).unwrap_or(true);
            }
        }
    }

    /// Wrap-up after the last pick: undrafted prospects choose paths; amateurs roll over.
    pub fn finish_draft(&mut self) {
        let mut rng = self.rng.clone();
        let year = self.year + 1;
        // everyone left in the pool is undrafted
        let leftovers: Vec<PlayerId> = self
            .draft
            .as_ref()
            .map(|d| d.pool.clone())
            .unwrap_or_default();
        for id in leftovers {
            let p = self.p(id);
            let user = p.user_controlled;
            if user {
                self.pm(id).flags.insert("undrafted".into());
                continue;
            }
            let aff = p.affiliation.clone();
            let ovr = p.ovr;
            let early = matches!(aff, Affiliation::College(_)) && crate::college::class_of(p) < 4;
            let hs = matches!(aff, Affiliation::HighSchool);
            self.pm(id).flags.remove("declared");
            if early && self.year >= 2016 {
                // return to school (modern withdrawal rules); otherwise they stay in the pros path
            } else if hs {
                // undrafted HS player: goes to college if possible, else overseas
                // (handled by rollover recruiting for those still in HS pool)
            } else if matches!(aff, Affiliation::College(_)) {
                // undrafted early entrant: ends college eligibility pre-modern; try a camp invite
                if let Affiliation::College(c) = aff {
                    self.colleges[c as usize].roster.retain(|&x| x != id);
                }
                let p = self.pm(id);
                p.affiliation = Affiliation::FreeAgent;
                p.origin.kind = OriginKind::Undrafted;
                self.free_agents.push(id);
            } else if ovr >= 40 && !matches!(aff, Affiliation::Overseas(_)) {
                let p = self.pm(id);
                p.affiliation = Affiliation::FreeAgent;
                self.free_agents.push(id);
            }
        }
        // rollover amateur worlds
        self.college_rollover(&mut rng);
        self.overseas_rollover(&mut rng);
        self.rng = rng;
        self.add_news("draft", format!("The {} draft is complete.", year));
        self.story_draft_done();
        self.handle_user_player_after_draft();
        if let Some(d) = self.draft.as_mut() {
            d.done = true;
        }
    }

    /// Draft board for display: prospects sorted by what a team believes.
    pub fn draft_board(&self, viewer: Option<TeamId>, limit: usize) -> Vec<(PlayerId, f64, f64)> {
        let pool: Vec<PlayerId> = match &self.draft {
            Some(d) => d.pool.clone(),
            None => vec![],
        };
        let mut v: Vec<(PlayerId, f64, f64)> = pool
            .iter()
            .map(|&id| {
                let (o, pt, _) = self.scouted_view(viewer, id);
                (id, o, pt)
            })
            .collect();
        v.sort_by(|a, b| {
            (b.1 * 0.5 + b.2 * 0.5)
                .partial_cmp(&(a.1 * 0.5 + a.2 * 0.5))
                .unwrap()
        });
        v.truncate(limit);
        v
    }

    /// Trade or give a draft pick to another team.
    pub fn move_pick(
        &mut self,
        year: Season,
        round: u8,
        original: TeamId,
        from: TeamId,
        to: TeamId,
    ) -> bool {
        let pos = self
            .team(from)
            .picks
            .iter()
            .position(|p| p.year == year && p.round == round && p.original == original);
        match pos {
            Some(i) => {
                let mut pk: DraftPick = self.team_mut(from).picks.remove(i);
                pk.owner = to;
                self.team_mut(to).picks.push(pk);
                true
            }
            None => false,
        }
    }
}
