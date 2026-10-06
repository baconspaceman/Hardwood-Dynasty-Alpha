//! Awards, the end of the season, aging, retirements, contracts and free agency.

use crate::awards::{self, AwardKind, Candidate};
use crate::contract::*;
use crate::economy::market_value_pct;
use crate::league::*;
use crate::player::*;
use crate::progression::*;
use crate::rng::Rng;
use crate::team::*;
use crate::types::*;

pub const FA_DAYS: u32 = 30;

impl League {
    // ------------------------------------------------------------------ awards

    /// Build award candidates from this season's statistics.
    pub fn award_candidates(&self, rookies_known: bool) -> Vec<Candidate> {
        let _ = rookies_known;
        let year = self.year;
        let mut out = vec![];
        for t in self.active_team_ids() {
            for &id in &self.team(t).roster {
                let p = self.p(id);
                // combine stints this season
                let mut total = StatLine::default();
                let mut last_ovr = p.ovr as f64;
                let mut prev_ovr = None;
                for r in p.seasons.iter().filter(|r| r.level == Level::Pro) {
                    if r.season == year {
                        total.add(&r.stats);
                        last_ovr = r.ovr as f64;
                    } else if r.season == year - 1 {
                        prev_ovr = Some(r.ovr as f64);
                    }
                }
                if total.g == 0 {
                    continue;
                }
                let tw = self.team(t).record.pct();
                let ppos = match p.position {
                    Position::PG | Position::SG => 0,
                    Position::SF => 1,
                    _ => 2,
                };
                let g = total.g as f64;
                out.push(Candidate {
                    id,
                    team: t,
                    games: total.g,
                    started: total.gs,
                    mpg: total.mpg(),
                    ppg: total.ppg(),
                    rpg: total.rpg(),
                    apg: total.apg(),
                    spg: total.spg(),
                    bpg: total.bpg(),
                    ts: total.ts_pct(),
                    fg_pct: total.fg_pct(),
                    tov_pg: total.tov as f64 / g,
                    team_win_pct: tw,
                    ovr: p.ovr as f64,
                    ovr_gain: prev_ovr
                        .map(|o| p.ovr as f64 - o)
                        .unwrap_or(0.0)
                        .max(last_ovr - last_ovr),
                    def_rating: (p.attrs.get(Attr::InteriorDef) * 0.4
                        + p.attrs.get(Attr::PerimeterDef) * 0.4
                        + p.attrs.get(Attr::Block) * 0.1
                        + p.attrs.get(Attr::Steal) * 0.1)
                        - 40.0,
                    rookie: p.pro_seasons() <= 1,
                    position_group: ppos,
                });
            }
        }
        out
    }

    /// Decide all awards for the season that just ended its regular season.
    pub fn compute_awards(&mut self) {
        let cands = self.award_candidates(true);
        let games = self.games_this_season;
        let defs = self.content.awards.clone();
        let mut rng = Rng::from_label(self.seed, &format!("awards-{}", self.year));
        for def in defs {
            if def.id == "all_star" {
                continue; // decided at the All-Star break
            }
            if let Some(res) = awards::decide(&def, &cands, games, self.year, &mut rng) {
                let year = self.year;
                for (pid, team_idx) in &res.winners {
                    let t = self.p(*pid).team_id();
                    let tn = t.map(|t| self.team(t).name()).unwrap_or_default();
                    let label = match def.kind {
                        AwardKind::AllTeam => {
                            format!("{} ({})", def.name, ordinal(*team_idx as usize + 1))
                        }
                        _ => def.name.clone(),
                    };
                    self.pm(*pid).awards.push(AwardRecord {
                        season: year,
                        award: label,
                        team: tn,
                    });
                }
                if def.kind == AwardKind::Single {
                    let w = res.winners[0].0;
                    let t = self
                        .p(w)
                        .team_id()
                        .map(|t| self.team(t).name())
                        .unwrap_or_default();
                    self.add_news(
                        "major",
                        format!(
                            "{} wins {}: {} ({}).",
                            self.p(w).name(),
                            def.name,
                            self.p(w).name(),
                            t
                        ),
                    );
                }
                self.awards.push(res);
            }
        }
        // Coach of the Year (team success vs. expectation) — simple built-in
        self.coach_of_year();
    }

    fn coach_of_year(&mut self) {
        if self.year < 1962 {
            return;
        }
        let ids = self.active_team_ids();
        let avg = self.avg_team_rating();
        let mut best: Option<(f64, TeamId)> = None;
        for &t in &ids {
            let exp = 0.5 + (self.team_rating_full_health(t) - avg) * 0.045;
            let gain = self.team(t).record.pct() - exp.clamp(0.1, 0.9);
            if best.map(|b| gain > b.0).unwrap_or(true) {
                best = Some((gain, t));
            }
        }
        if let Some((_, t)) = best {
            if let Some(hc) = self.team(t).head_coach {
                let name = self.person(hc).name.clone();
                self.add_news(
                    "major",
                    format!(
                        "{name} of the {} is named Coach of the Year.",
                        self.team(t).name()
                    ),
                );
                self.people[hc as usize].reputation =
                    (self.people[hc as usize].reputation + 6.0).min(100.0);
            }
        }
    }

    // ------------------------------------------------------------------ end of seasons

    pub fn end_regular_season(&mut self) {
        // finalize regular-season awards
        self.compute_awards();
        self.snapshot_regular_season();
        self.start_playoffs();
    }

    fn snapshot_regular_season(&mut self) {
        // Mark playoff results for teams that miss the postseason.
        let ids = self.active_team_ids();
        let made: Vec<TeamId> = self
            .playoffs
            .as_ref()
            .map(|p| p.seeds.values().flatten().copied().collect())
            .unwrap_or_default();
        let _ = made;
        for t in ids {
            self.playoff_results_cache.remove(&t);
        }
    }

    /// Called when the final series is over: record champions, histories and money.
    pub fn finish_playoffs(&mut self) {
        let year = self.year;
        let st = self.playoffs.clone().unwrap_or_default();
        let ids = self.active_team_ids();
        let champ = st.champion.or_else(|| {
            // no playoffs: best record wins
            self.standings(None).first().copied()
        });
        // Finals MVP: best box-score player on the champion in the finals/playoffs.
        let mut finals_mvp_name = String::new();
        if let Some(c) = champ {
            self.team_mut(c).titles.push(year);
            if self.year >= 1969 {
                let mut best: Option<(f64, PlayerId)> = None;
                for &id in &self.team(c).roster.clone() {
                    let p = self.p(id);
                    if let Some(r) = p
                        .seasons
                        .iter()
                        .rev()
                        .find(|r| r.season == year && r.team_id == Some(c))
                    {
                        let sc = r.playoffs.game_score_pg() * (r.playoffs.g.min(7) as f64).max(1.0)
                            + r.playoffs.ppg() * 0.3;
                        if best.map(|b| sc > b.0).unwrap_or(true) {
                            best = Some((sc, id));
                        }
                    }
                }
                if let Some((_, id)) = best {
                    finals_mvp_name = self.p(id).name();
                    let tn = self.team(c).name();
                    self.pm(id).awards.push(AwardRecord {
                        season: year,
                        award: "Finals MVP".into(),
                        team: tn,
                    });
                    if let Some(s) = self.playoffs.as_mut() {
                        s.finals_mvp = Some(id);
                    }
                }
            }
            // titles to players & coach
            for &id in &self.team(c).roster.clone() {
                let tn = self.team(c).name();
                let has_played = self
                    .p(id)
                    .seasons
                    .iter()
                    .any(|r| r.season == year && r.team_id == Some(c) && (r.stats.g > 0));
                if has_played {
                    self.pm(id).awards.push(AwardRecord {
                        season: year,
                        award: "Champion".into(),
                        team: tn,
                    });
                }
            }
            if let Some(hc) = self.team(c).head_coach {
                self.people[hc as usize].titles += 1;
                self.people[hc as usize].reputation =
                    (self.people[hc as usize].reputation + 8.0).min(100.0);
            }
            self.team_mut(c).hype = (self.team(c).hype + 15.0).min(100.0);
            self.team_mut(c).fan_loyalty = (self.team(c).fan_loyalty + 3.0).min(100.0);
        }
        // team season summaries
        let mut mvp = String::new();
        let mut dpoy = String::new();
        let mut roy = String::new();
        for a in self.awards.iter().filter(|a| a.season == year) {
            let w = a
                .winners
                .first()
                .map(|w| self.p(w.0).name())
                .unwrap_or_default();
            match a.award_id.as_str() {
                "mvp" => mvp = w,
                "dpoy" => dpoy = w,
                "roy" => roy = w,
                _ => {}
            }
        }
        // scoring leader
        let cands = self.award_candidates(true);
        let min_g = (self.games_this_season as f64 * 0.5) as u16;
        let leader = cands
            .iter()
            .filter(|c| c.games >= min_g)
            .max_by(|a, b| a.ppg.partial_cmp(&b.ppg).unwrap());
        let scoring_leader = leader
            .map(|c| format!("{} ({:.1})", self.p(c.id).name(), c.ppg))
            .unwrap_or_default();
        let best_rec = self.standings(None).first().copied();
        let mut summary = SeasonSummary {
            season: year,
            league_name: self.rules.league_name.clone(),
            champion: champ.map(|c| self.team(c).name()).unwrap_or_default(),
            champion_id: champ,
            runner_up: st
                .runner_up
                .map(|c| self.team(c).name())
                .unwrap_or_default(),
            finals_result: st
                .rounds
                .last()
                .and_then(|r| r.last())
                .map(|s| s.score_text(|t| self.team(t).abbr.clone()))
                .unwrap_or_default(),
            finals_mvp: finals_mvp_name,
            mvp,
            dpoy,
            roy,
            scoring_leader,
            best_record: best_rec
                .map(|t| {
                    format!(
                        "{} ({}-{})",
                        self.team(t).name(),
                        self.team(t).record.w,
                        self.team(t).record.l
                    )
                })
                .unwrap_or_default(),
            teams: ids.len(),
            games: self.games_this_season,
            cap: self.money.cap,
            avg_ppg: {
                let (mut pf, mut g) = (0u64, 0u64);
                for &t in &ids {
                    pf += self.team(t).record.pf as u64;
                    g += self.team(t).record.games() as u64;
                }
                if g == 0 {
                    0.0
                } else {
                    pf as f64 / g as f64
                }
            },
            notes: vec![],
        };
        for e in self.stories.iter().filter(|s| s.season == year && s.major) {
            summary.notes.push(e.title.clone());
        }
        self.history.push(summary.clone());
        for &t in &ids {
            let result = if Some(t) == champ {
                "Champions".to_string()
            } else if let Some(r) = st.results.get(&t) {
                r.clone()
            } else if st.seeds.values().any(|v| v.contains(&t)) {
                "Made the playoffs".to_string()
            } else {
                "Missed the playoffs".to_string()
            };
            self.playoff_results_cache.insert(t, result);
        }
        self.finish_team_seasons(champ);
        if let Some(c) = champ {
            let msg = format!(
                "{} are the {} champions! ({}) Finals MVP: {}",
                self.team(c).name(),
                year,
                summary.finals_result,
                summary.finals_mvp
            );
            self.add_news("major", msg);
            self.story_champion(c);
        }
        self.phase = Phase::PostSeason;
    }

    fn finish_team_seasons(&mut self, champ: Option<TeamId>) {
        let year = self.year;
        let ids = self.active_team_ids();
        for &t in &ids {
            self.finance_end_season(t);
        }
        self.distribute_tax_pool();
        for t in ids {
            let best_player = self
                .team(t)
                .roster
                .iter()
                .copied()
                .max_by_key(|&id| self.p(id).ovr)
                .map(|id| self.p(id).name())
                .unwrap_or_default();
            let payroll = self.payroll(t);
            let profit = self.team(t).finance.profit();
            let coach = self
                .team(t)
                .head_coach
                .map(|c| self.person(c).name.clone())
                .unwrap_or_default();
            let res = self
                .playoff_results_cache
                .get(&t)
                .cloned()
                .unwrap_or_default();
            let seed = self
                .playoffs
                .as_ref()
                .and_then(|p| {
                    p.seeds
                        .get(&self.team(t).conf)
                        .and_then(|v| v.iter().position(|&x| x == t))
                })
                .map(|i| (i + 1) as u8);
            let rec = self.team(t).record.clone();
            let name = self.team(t).name();
            self.team_mut(t).history.push(TeamSeason {
                season: year,
                name,
                w: rec.w,
                l: rec.l,
                seed,
                playoff_result: res,
                champion: Some(t) == champ,
                payroll,
                profit,
                coach,
                best_player,
            });
            // coach record
            if let Some(hc) = self.team(t).head_coach {
                let p = &mut self.people[hc as usize];
                p.wins += rec.w as u32;
                p.losses += rec.l as u32;
                p.years_in_role += 1;
            }
        }
    }

    // ------------------------------------------------------------------ post-season

    /// Aging, progression, retirements, staff moves. Runs once after the playoffs.
    pub fn run_postseason(&mut self) {
        self.with_rng(|l, rng| {
            l.progress_all_players(rng);
            l.process_retirements(rng);
            l.hall_of_fame_check();
            l.staff_changes(rng);
            l.update_team_directions();
            l.project_next_season_money(rng);
            l.league_postseason_events(rng);
        });
        self.story_offseason();
        self.offer_promotions();
        self.mark_expiring();
        self.phase = Phase::Draft;
        self.with_rng(|l, rng| l.setup_draft(rng));
    }

    fn mark_expiring(&mut self) {
        // Players on the last year of their contract are flagged so the user can re-sign them.
        for t in self.active_team_ids() {
            for id in self.team(t).roster.clone() {
                let exp = self
                    .p(id)
                    .contract
                    .as_ref()
                    .map(|c| c.expiring())
                    .unwrap_or(false);
                let p = self.pm(id);
                if exp {
                    p.flags.insert("expiring".into());
                } else {
                    p.flags.remove("expiring");
                }
            }
        }
    }

    fn progress_all_players(&mut self, rng: &mut Rng) {
        let year = self.year;
        let science = if self.settings.bool("progression.sports_science") {
            sports_science_factor(year)
        } else {
            1.0
        };
        let prm = ProgParams {
            speed: self.settings.num("progression.speed"),
            variance: self.settings.num("progression.variance"),
            late_bloomers: self.settings.num("progression.late_bloomers"),
            longevity: self.settings.num("progression.longevity"),
            science,
            hidden_potential: self.settings.bool("progression.hidden_potential"),
        };
        let n = self.players.len();
        for i in 0..n {
            if self.players[i].is_retired() {
                continue;
            }
            let (coaching, facilities) = match self.players[i].team_id() {
                Some(t) => (
                    self.coaching_quality(t),
                    (self.team(t).budget.facilities * 50.0).clamp(10.0, 100.0),
                ),
                None => (45.0, 45.0),
            };
            let mpg = self.players[i]
                .seasons
                .iter()
                .rev()
                .find(|r| r.season == year && r.level == Level::Pro)
                .map(|r| r.stats.mpg())
                .unwrap_or(0.0);
            let share = (mpg / 36.0).clamp(0.0, 1.0);
            let (ls, lp) = match &self.players[i].life {
                Some(_) => {
                    let acc = self.players[i]
                        .custom
                        .remove("dev_acc_skill")
                        .unwrap_or(300.0);
                    let accp = self.players[i]
                        .custom
                        .remove("dev_acc_phys")
                        .unwrap_or(300.0);
                    (
                        (0.85 + acc / 700.0).clamp(0.8, 1.7),
                        (0.85 + accp / 700.0).clamp(0.8, 1.7),
                    )
                }
                None => (1.0, 1.0),
            };
            let ctx = DevContext {
                coaching,
                facilities,
                minutes_share: if self.players[i].is_active_pro() {
                    share
                } else {
                    0.6
                },
                life_skill: ls,
                life_phys: lp,
            };
            let next_age = year + 1 - self.players[i].birth_year;
            develop(&mut self.players[i], next_age, &ctx, &prm, rng);
            // AI picks a practice focus for the coming year
            if self.players[i].user_controlled {
                continue;
            }
            self.players[i].dev_focus = ai_focus(&self.players[i], rng);
            // injured players heal some in the offseason
            if let Some(inj) = &mut self.players[i].injury {
                if !inj.career_ending {
                    let heal = 38u16;
                    inj.games_remaining = inj.games_remaining.saturating_sub(heal);
                    if inj.games_remaining == 0 {
                        let rec = InjuryRecordLite::from(&*inj);
                        let _ = rec;
                    }
                }
            }
        }
        // finish healed injuries (apply permanent loss)
        let med: Vec<f64> = (0..self.teams.len())
            .map(|t| self.medical_quality(t as TeamId))
            .collect();
        for i in 0..n {
            let done = self.players[i]
                .injury
                .as_ref()
                .map(|inj| inj.games_remaining == 0 && !inj.career_ending)
                .unwrap_or(false);
            if done {
                let m = self.players[i]
                    .team_id()
                    .map(|t| med[t as usize])
                    .unwrap_or(45.0);
                // heal_one_game with 0 remaining applies the aftermath
                if let Some(inj) = &mut self.players[i].injury {
                    inj.games_remaining = 1;
                }
                crate::injury::heal_one_game(&mut self.players[i], m, rng);
            }
        }
    }

    fn process_retirements(&mut self, rng: &mut Rng) {
        let year = self.year;
        let replacement = 45.0;
        let longevity = self.settings.num("progression.longevity")
            * if self.settings.bool("progression.sports_science") {
                sports_science_factor(year)
            } else {
                1.0
            };
        let mut retirees = vec![];
        for p in self.players.iter() {
            if p.is_retired()
                || !matches!(
                    p.affiliation,
                    Affiliation::Nba(_)
                        | Affiliation::FreeAgent
                        | Affiliation::GLeague(_)
                        | Affiliation::Overseas(_)
                )
            {
                continue;
            }
            if p.user_controlled {
                continue; // the human decides when to retire
            }
            let next_age = year + 1 - p.birth_year;
            let wealth = 0.0;
            let mut c = retirement_chance(p, next_age, replacement, wealth, longevity);
            if matches!(p.affiliation, Affiliation::FreeAgent) && next_age >= 33 {
                c = (c + 0.25).min(0.98);
            }
            // good players on contracts rarely retire before 35
            if p.ovr >= 70 && next_age < 36 {
                c *= 0.3;
            }
            if rng.chance(c) {
                retirees.push(p.id);
            }
        }
        for id in retirees {
            let notable = self.p(id).ovr >= 66 || !self.p(id).awards.is_empty();
            let name = self.p(id).name();
            let team = self.p(id).team_id().map(|t| self.team(t).name());
            self.retire_player(id, "age");
            if notable {
                let career = self.p(id).career_pro();
                let msg = format!(
                    "{name} retires{} after {} seasons ({:.1} ppg).",
                    team.map(|t| format!(" from {t}")).unwrap_or_default(),
                    self.p(id).pro_seasons(),
                    career.ppg()
                );
                self.add_news("retirement", msg);
            }
            self.retired_number_check(id);
        }
    }

    /// Retire a player now (age, injury, choice). Removes him from rosters.
    pub fn retire_player(&mut self, id: PlayerId, reason: &str) {
        let year = self.year;
        let t = self.p(id).team_id();
        if let Some(t) = t {
            self.team_mut(t).roster.retain(|&x| x != id);
            self.team_mut(t).gleague.retain(|&x| x != id);
        }
        self.free_agents.retain(|&x| x != id);
        for c in self.clubs.iter_mut() {
            c.roster.retain(|&x| x != id);
        }
        let p = self.pm(id);
        p.retired = Some(year);
        p.affiliation = Affiliation::Retired;
        p.contract = None;
        p.draft_rights = None;
        p.flags.insert(format!("retired:{reason}"));
        if let Some(l) = &mut p.life {
            l.stage = crate::life::LifeStage::Retired;
        }
    }

    pub fn hof_score(&self, p: &Player) -> f64 {
        let mut s = 0.0;
        for a in &p.awards {
            let n = a.award.as_str();
            s += if n.starts_with("Most Valuable") {
                9.0
            } else if n == "Finals MVP" {
                4.5
            } else if n == "Champion" {
                2.2
            } else if n.starts_with("Defensive Player") {
                4.0
            } else if n.starts_with("All-League Team (1st)") {
                4.0
            } else if n.starts_with("All-League Team (2nd)") {
                2.2
            } else if n.starts_with("All-League Team (3rd)") {
                1.2
            } else if n.starts_with("All-Defensive") {
                0.9
            } else if n == "All-Star" {
                1.1
            } else if n.starts_with("Rookie of the Year") {
                1.0
            } else if n.starts_with("Most Improved") || n.starts_with("Sixth") {
                0.8
            } else {
                0.0
            };
        }
        let c = p.career_pro();
        s += (c.pts as f64 / 1000.0) * 0.35
            + (c.reb() as f64 / 1000.0) * 0.2
            + (c.ast as f64 / 1000.0) * 0.3;
        s
    }

    fn hall_of_fame_check(&mut self) {
        let year = self.year;
        let wait = 3;
        let mut inducted = vec![];
        for p in self.players.iter() {
            if p.hall_of_fame.is_some() || p.retired.is_none() {
                continue;
            }
            let r = p.retired.unwrap();
            if year - r != wait {
                continue;
            }
            if self.hof_score(p) >= 16.0 {
                inducted.push(p.id);
            }
        }
        for id in inducted {
            self.pm(id).hall_of_fame = Some(year);
            self.hall_of_fame.push(id);
            let p = self.p(id);
            let msg = format!(
                "{} is inducted into the Hall of Fame (career: {:.1} ppg, {} titles).",
                p.name(),
                p.career_pro().ppg(),
                p.awards.iter().filter(|a| a.award == "Champion").count()
            );
            self.add_news("major", msg);
        }
    }

    fn retired_number_check(&mut self, id: PlayerId) {
        let p = self.p(id);
        // longest-serving team
        let mut tally: std::collections::BTreeMap<TeamId, u32> = Default::default();
        for r in p.seasons.iter().filter(|r| r.level == Level::Pro) {
            if let Some(t) = r.team_id {
                *tally.entry(t).or_insert(0) += 1;
            }
        }
        if let Some((&t, &yrs)) = tally.iter().max_by_key(|(_, &y)| y) {
            if yrs >= 8 && self.hof_score(p) >= 12.0 {
                let name = p.name();
                let num = (p.id % 90 + 1) as u8;
                self.team_mut(t).retired_numbers.push((name.clone(), num));
                let tn = self.team(t).name();
                self.add_news(
                    "retirement",
                    format!("{tn} will retire #{num} in honor of {name}."),
                );
            }
        }
    }

    fn update_team_directions(&mut self) {
        let avg = self.avg_team_rating();
        for t in self.active_team_ids() {
            let r = self.team_rating_full_health(t);
            let ages: f64 = {
                let ids = &self.team(t).roster;
                if ids.is_empty() {
                    27.0
                } else {
                    ids.iter()
                        .take(8)
                        .map(|&id| (self.year + 1 - self.p(id).birth_year) as f64)
                        .sum::<f64>()
                        / ids.len().min(8) as f64
                }
            };
            let dir = if r > avg + 2.2 {
                Direction::Contend
            } else if r < avg - 2.0 || (r < avg && ages > 29.5) {
                Direction::Rebuild
            } else {
                Direction::Retool
            };
            if self.settings.bool("ai.smart_rebuilds") {
                self.team_mut(t).direction = dir;
            } else {
                self.team_mut(t).direction = Direction::Contend;
            }
        }
    }

    // ------------------------------------------------------------------ contracts & free agency

    /// How much a player wants (first-year salary) and for how long.
    pub fn player_ask(&self, id: PlayerId) -> (Money, u8) {
        let p = self.p(id);
        let age = (self.year + 1 - p.birth_year) as f64;
        let pct = market_value_pct(p.ovr as f64, age, p.potential as f64);
        let greed = 0.9
            + 0.2 * p.hidden.greed as f64 / 100.0
                * self.settings.num("economy.player_greed").max(0.2);
        let mut salary = (self.money.cap as f64 * pct * greed) as i64;
        if self.rules.max_contract {
            salary = salary.min(self.content.economy.max_salary(&self.money, p.years_pro));
        }
        salary = salary.max(self.money.min_salary);
        let years = if age <= 25.0 {
            if p.ovr >= 62 {
                4
            } else {
                3
            }
        } else if age <= 29.0 {
            if p.ovr >= 62 {
                4
            } else {
                3
            }
        } else if age <= 32.0 {
            3
        } else {
            1
        };
        let years = if salary <= self.money.min_salary + 1 {
            years.min(2)
        } else {
            years
        };
        (salary, years as u8)
    }

    /// Value of a player to a team (for AI roster decisions): ovr with age/potential adjustments.
    pub fn player_value(&self, id: PlayerId) -> f64 {
        let p = self.p(id);
        let age = (self.year + 1 - p.birth_year) as f64;
        let youth = ((25.0 - age) / 6.0).clamp(0.0, 1.0);
        let old = ((age - 30.0) / 6.0).clamp(0.0, 1.0);
        p.ovr as f64 + (p.potential as f64 - p.ovr as f64) * 0.45 * youth - old * 7.0
    }

    /// Players on teams whose contracts end this offseason are re-signed by AI or released.
    pub fn start_free_agency(&mut self) {
        let mut rng = self.rng.clone();
        // 1. options
        for t in self.active_team_ids() {
            for id in self.team(t).roster.clone() {
                let (has_option, opt, ncontract) = match &self.p(id).contract {
                    Some(c) => (
                        c.option != OptionKind::None && c.salaries.len() == 2,
                        c.option,
                        c.salaries.len(),
                    ),
                    None => (false, OptionKind::None, 0),
                };
                let _ = ncontract;
                if !has_option {
                    continue;
                }
                let (ask, _) = self.player_ask(id);
                let last = self
                    .p(id)
                    .contract
                    .as_ref()
                    .map(|c| *c.salaries.last().unwrap())
                    .unwrap_or(0);
                let exercise = match opt {
                    OptionKind::Team => {
                        if self.user_controls_roster(t) && !self.user.auto_decisions {
                            self.push_decision(
                                "team_option",
                                &format!("Team option: {}", self.p(id).name()),
                                &format!(
                                    "Exercise {}'s option at {}? Market value is about {}.",
                                    self.p(id).name(),
                                    fmt_money(last),
                                    fmt_money(ask)
                                ),
                                vec![
                                    DecisionOption {
                                        id: "exercise".into(),
                                        label: "Exercise".into(),
                                        explain: "Keep him at the option salary.".into(),
                                    },
                                    DecisionOption {
                                        id: "decline".into(),
                                        label: "Decline".into(),
                                        explain: "He becomes a free agent.".into(),
                                    },
                                ],
                                Some(id),
                            );
                            true // default; resolved later by decision handler
                        } else {
                            self.player_value(id) >= 50.0 && last as f64 <= ask as f64 * 1.15
                        }
                    }
                    OptionKind::Player => {
                        let opt_out = ask as f64 > last as f64 * 1.12;
                        !opt_out
                    }
                    OptionKind::None => true,
                };
                if !exercise {
                    if let Some(c) = &mut self.pm(id).contract {
                        c.salaries.pop();
                    }
                }
                if let Some(c) = &mut self.pm(id).contract {
                    c.option = OptionKind::None;
                }
            }
        }
        // 2. tick all contracts; expired players
        let mut expiring: Vec<(PlayerId, TeamId)> = vec![];
        for t in self.active_team_ids() {
            for id in self.team(t).roster.clone() {
                let alive = self
                    .pm(id)
                    .contract
                    .as_mut()
                    .map(|c| c.tick())
                    .unwrap_or(false);
                if !alive {
                    self.pm(id).contract = None;
                    expiring.push((id, t));
                }
            }
        }
        // dead money ticks away
        let y = self.year;
        for t in self.teams.iter_mut() {
            t.dead_money.retain(|(yr, _)| *yr > y);
        }
        // 3. re-sign decisions
        for (id, t) in expiring {
            let keep = self.ai_wants_to_resign(id, t, &mut rng)
                && !(self.user_controls_roster(t)
                    && self.p(id).flags.contains("expiring")
                    && !self.user.auto_decisions
                    && !self.p(id).flags.contains("force_ai"));
            // The user's expiring players that weren't re-signed leave.
            if keep && !self.user_controls_roster(t) {
                let (ask, years) = self.player_ask(id);
                let mood_ok = rng.f64()
                    < (0.55
                        + (self.p(id).mood.overall as f64 - 50.0) / 120.0
                        + self.p(id).hidden.loyalty as f64 / 400.0)
                        .clamp(0.1, 0.97);
                if mood_ok {
                    let mut c =
                        Contract::rising(ask, years, 0.04, ContractKind::BirdRights, self.year + 1);
                    c.guaranteed = true;
                    self.pm(id).contract = Some(c);
                    self.add_transaction(
                        "resign",
                        format!(
                            "{} re-signs with {}.",
                            self.p(id).name(),
                            self.team(t).name()
                        ),
                        vec![t],
                    );
                    continue;
                }
            }
            // becomes a free agent
            self.release_to_fa(id, t, "contract expired");
        }
        self.rng = rng;
        // 4. unsigned, un-drafted prospects, and rookies' contracts are handled in the draft.
        self.fa_day_index = 0;
        self.phase = Phase::FreeAgency;
        self.add_news(
            "major",
            format!(
                "Free agency opens with {} players available.",
                self.free_agents.len()
            ),
        );
        self.story_free_agency_open();
        if let Some(pid) = self.user.player {
            if matches!(self.p(pid).affiliation, Affiliation::FreeAgent)
                && !self.p(pid).is_retired()
                && self.p(pid).user_controlled
                && !self.decisions.iter().any(|d| d.kind == "fa_offers")
            {
                self.make_fa_offers(pid);
            }
        }
    }

    pub fn release_to_fa(&mut self, id: PlayerId, from: TeamId, why: &str) {
        let tname = self.team(from).name();
        self.team_mut(from).roster.retain(|&x| x != id);
        self.team_mut(from).gleague.retain(|&x| x != id);
        let p = self.pm(id);
        p.affiliation = Affiliation::FreeAgent;
        p.flags.remove("expiring");
        p.mood.wants_trade = false;
        if !self.free_agents.contains(&id) {
            self.free_agents.push(id);
        }
        if self.p(id).ovr >= 66 && why == "contract expired" {
            let n = self.p(id).name();
            self.add_news(
                "freeagency",
                format!("{n} hits free agency after leaving {tname}."),
            );
        }
    }

    fn ai_wants_to_resign(&self, id: PlayerId, t: TeamId, rng: &mut Rng) -> bool {
        let v = self.player_value(id);
        let (ask, _) = self.player_ask(id);
        let payroll_after = self.payroll(t) + ask;
        let tm = self.team(t);
        let limit = if self.money.cap_enforced {
            tm.budget.payroll_target
                + tm.budget.tax_tolerance.min(
                    self.money
                        .tax_line
                        .saturating_sub(tm.budget.payroll_target)
                        .max(0)
                        + tm.budget.tax_tolerance,
                )
        } else {
            (self.money.cap as f64 * 1.35) as i64
        };
        let affordable = payroll_after <= limit.max(self.money.cap * 11 / 10);
        let base = v
            >= match tm.direction {
                Direction::Rebuild => {
                    58.0 + (self.year + 1 - self.p(id).birth_year - 26).max(0) as f64 * 1.5
                }
                _ => 52.0,
            };
        affordable && base && rng.chance(0.93)
    }

    /// Run one day of free agency: AI teams make signings.
    pub fn fa_day(&mut self) {
        let mut rng = self.rng.clone();
        self.fa_day_index += 1;
        if let Some(pid) = self.user.player {
            if self.p(pid).flags.contains("waiting")
                && self.fa_day_index % 7 == 0
                && matches!(self.p(pid).affiliation, Affiliation::FreeAgent)
            {
                self.pm(pid).flags.remove("waiting");
                self.make_fa_offers(pid);
            }
        }
        let mut order = self.active_team_ids();
        rng.shuffle(&mut order);
        let pressure = self.settings.num("ai.free_agency_aggression");
        // cooling market: asks fall over the month
        let discount = 1.0 - 0.012 * self.fa_day_index as f64;
        for t in order {
            if self.user_controls_roster(t) {
                continue;
            }
            let roster_max = self.rules.roster_max as usize;
            // Teams sign at most one player per day (two on day 1-3 for needy teams).
            let mut signings = 0;
            let cap_slots = if self.fa_day_index <= 3 { 2 } else { 1 };
            while signings < cap_slots && self.team(t).roster.len() < roster_max {
                if !rng.chance(
                    (0.55 * pressure).min(0.95)
                        + if self.team(t).roster.len() < self.rules.roster_min as usize + 1 {
                            0.4
                        } else {
                            0.0
                        },
                ) {
                    break;
                }
                if !self.ai_sign_best(t, discount, &mut rng) {
                    break;
                }
                signings += 1;
            }
        }
        self.rng = rng;
        if self.fa_day_index == FA_DAYS {
            self.finish_free_agency();
        }
    }

    /// AI team signs the best FA it can afford and who is willing. Returns whether it signed.
    fn ai_sign_best(&mut self, t: TeamId, discount: f64, rng: &mut Rng) -> bool {
        let payroll = self.payroll(t);
        let cap = self.money.cap;
        let enforced = self.money.cap_enforced;
        let dir = self.team(t).direction;
        let tm_budget = self.team(t).budget.clone();
        let room = if enforced {
            cap - payroll
        } else {
            (tm_budget.payroll_target - payroll).max(0)
        };
        let mle = if self.rules.mid_level_exception && enforced {
            self.mle_amount() - self.team(t).exceptions.mle_used
        } else {
            0
        };
        let minsal = self.money.min_salary;
        let rating = self.team_rating(t);
        let mut best: Option<(f64, PlayerId, Money, u8)> = None;
        let mut checked = 0;
        let mut fas: Vec<PlayerId> = self.free_agents.clone();
        fas.sort_by(|&a, &b| {
            self.player_value(b)
                .partial_cmp(&self.player_value(a))
                .unwrap()
        });
        for id in fas {
            checked += 1;
            if checked > 40 {
                break;
            }
            let p = self.p(id);
            if p.user_controlled || p.is_retired() {
                continue;
            }
            let (mut ask, years) = self.player_ask(id);
            ask = ((ask as f64 * discount) as i64).max(minsal);
            // can we afford it?
            let pay_ok = if enforced {
                ask <= minsal || ask <= room || ask <= mle.max(0)
            } else {
                payroll + ask <= tm_budget.payroll_target + tm_budget.tax_tolerance
            };
            if !pay_ok {
                continue;
            }
            // the tax: don't blow past the owner's tolerance
            if payroll + ask > self.money.tax_line + tm_budget.tax_tolerance && ask > minsal {
                continue;
            }
            // team needs: contenders want quality, rebuilders want youth
            let v = self.player_value(id);
            let age = (self.year + 1 - p.birth_year) as f64;
            let mut score = v - ask as f64 / cap as f64 * 80.0;
            match dir {
                Direction::Rebuild => score += (27.0 - age).clamp(-4.0, 6.0),
                Direction::Contend => score += (v - rating).max(0.0) * 0.5,
                Direction::Retool => {}
            }
            // does he improve us?
            let improves = v > self.nth_best_value(t, 8) - 2.0;
            if !improves && self.team(t).roster.len() >= self.rules.roster_min as usize + 1 {
                continue;
            }
            // willingness
            let appeal = self.team_appeal(t, id);
            let willing = rng.f64()
                < (0.25 + (appeal - 50.0) / 60.0 + if ask >= minsal * 2 { 0.0 } else { 0.25 })
                    .clamp(0.05, 0.97);
            if !willing {
                continue;
            }
            if best.map(|b| score > b.0).unwrap_or(true) {
                best = Some((score, id, ask, years));
            }
        }
        if let Some((_, id, ask, years)) = best {
            let used_mle = enforced && ask > room && ask > minsal;
            self.sign_player(
                id,
                t,
                ask,
                years,
                if ask <= minsal {
                    ContractKind::Minimum
                } else {
                    ContractKind::Standard
                },
                "free agency",
            );
            if used_mle {
                self.team_mut(t).exceptions.mle_used += ask;
            }
            true
        } else {
            false
        }
    }

    fn nth_best_value(&self, t: TeamId, n: usize) -> f64 {
        let mut v: Vec<f64> = self
            .team(t)
            .roster
            .iter()
            .map(|&id| self.player_value(id))
            .collect();
        v.sort_by(|a, b| b.partial_cmp(a).unwrap());
        v.get(n).copied().unwrap_or(40.0)
    }

    /// How attractive a team is to a player (0-100).
    pub fn team_appeal(&self, t: TeamId, id: PlayerId) -> f64 {
        let p = self.p(id);
        let tm = self.team(t);
        let ids = self.active_team_ids();
        let my = self.team_rating_full_health(t);
        let better = ids
            .iter()
            .filter(|&&o| self.team_rating_full_health(o) > my)
            .count();
        let winning = 100.0 - better as f64 / ids.len().max(1) as f64 * 100.0;
        let market = tm.market * 100.0;
        // playing time: would he crack the top 8?
        let v = self.player_value(id);
        let spot = (v - self.nth_best_value(t, 7)).clamp(-15.0, 15.0) * 2.0 + 50.0;
        let w = p.hidden.winning as f64 / 100.0;
        let g = p.hidden.greed as f64 / 100.0;
        let base = winning * (0.25 + 0.4 * w)
            + market * (0.15 + 0.1 * g)
            + spot * 0.25
            + tm.hype * 0.1
            + tm.owner.approval * 0.05
            + (self.coach_of(t).map(|c| c.motivation).unwrap_or(50.0)) * 0.1;
        let mut total = base / (0.25 + 0.4 * w + 0.15 + 0.1 * g + 0.25 + 0.1 + 0.05 + 0.1);
        if p.last_team_bonus(t) {
            total += 6.0 + p.hidden.loyalty as f64 * 0.08;
        }
        total.clamp(0.0, 100.0)
    }

    pub fn mle_amount(&self) -> Money {
        // Non-taxpayer mid-level exception ≈ 8.5% of cap (about 5.2M at a 58M cap, 12M at 140M).
        (self.money.cap as f64 * 0.085) as i64
    }

    /// Sign a free agent (or draft pick / trade target) to a new contract.
    pub fn sign_player(
        &mut self,
        id: PlayerId,
        t: TeamId,
        first_year: Money,
        years: u8,
        kind: ContractKind,
        why: &str,
    ) {
        self.free_agents.retain(|&x| x != id);
        let raise = if self.p(id).ovr >= 60 { 0.05 } else { 0.0 };
        let mut c = Contract::rising(first_year, years, raise, kind, self.year + 1);
        // Veterans on 3+ years sometimes get a player option.
        if years >= 3
            && self.p(id).ovr >= 68
            && first_year > self.money.min_salary * 3
            && self.fa_day_index >= 1
        {
            c.option = OptionKind::Player;
        }
        let year = self.year;
        let tname = self.team(t).name();
        let team_year_pro = self.phase == Phase::RegularSeason || self.phase == Phase::Preseason;
        let p = self.pm(id);
        p.contract = Some(c);
        p.affiliation = Affiliation::Nba(t);
        p.flags.remove("expiring");
        p.mood.wants_trade = false;
        p.mood.overall = (p.mood.overall + 8.0).min(100.0);
        self.team_mut(t).roster.push(id);
        let name = self.p(id).name();
        if why == "free agency" || why == "waiver" || why == "signing" {
            let pv = self.p(id).ovr;
            if pv >= 62 {
                self.add_news(
                    "freeagency",
                    format!(
                        "{name} signs with {tname}: {} years, {} per year.",
                        years,
                        fmt_money(first_year)
                    ),
                );
            }
            self.add_transaction(
                "signing",
                format!(
                    "{tname} sign {name} ({years} yr, {}).",
                    fmt_money(first_year)
                ),
                vec![t],
            );
        }
        if team_year_pro {
            self.ensure_record(id, t, &tname, year);
        }
        if self.p(id).ovr >= 70 {
            self.story_signing(id, t);
        }
    }

    /// End of free agency: remaining players sign cheap deals, go overseas, or retire; rosters fill.
    fn finish_free_agency(&mut self) {
        let mut rng = self.rng.clone();
        let minsal = self.money.min_salary;
        // fill rosters to the minimum with the best remaining free agents (AI teams)
        for t in self.active_team_ids() {
            if self.user_controls_roster(t) {
                continue;
            }
            let need =
                (self.rules.roster_min as usize + 2).saturating_sub(self.team(t).roster.len());
            for _ in 0..need {
                let mut fas = self.free_agents.clone();
                fas.sort_by(|&a, &b| {
                    self.player_value(b)
                        .partial_cmp(&self.player_value(a))
                        .unwrap()
                });
                if let Some(&id) = fas.iter().find(|&&id| !self.p(id).user_controlled) {
                    self.sign_player(id, t, minsal, 1, ContractKind::Minimum, "signing");
                }
            }
        }
        // Leftovers: overseas or retire.
        let leftovers = self.free_agents.clone();
        for id in leftovers {
            if self.p(id).user_controlled {
                continue;
            }
            let age = self.year + 1 - self.p(id).birth_year;
            let ovr = self.p(id).ovr;
            if age >= 35 && ovr < 55 {
                self.retire_player(id, "age");
            } else if ovr >= 42 && !self.clubs.is_empty() && rng.chance(0.55) {
                self.send_overseas(id, &mut rng);
            }
        }
        self.rng = rng;
        self.add_news("major", "Free agency closes. Training camps open soon.");
    }

    /// Advance from free agency into the next season's preseason.
    pub fn start_next_season(&mut self) {
        self.year += 1;
        self.phase = Phase::Preseason;
        self.draft = None;
        // keep the pick horizon 4 years out
        for t in self.active_team_ids() {
            let horizon = self.year + 4;
            if !self.team(t).picks.iter().any(|p| p.year == horizon) {
                self.add_picks_for(t, horizon);
            }
        }
        self.begin_season_prep();
        self.preseason_hooks();
        self.add_news(
            "major",
            format!(
                "The {}-{:02} season is about to begin.",
                self.year,
                (self.year + 1) % 100
            ),
        );
    }

    /// Make rosters legal (AI) before opening night.
    pub fn finish_preseason(&mut self) -> Result<(), String> {
        let max = self.rules.roster_max as usize;
        let min = self.rules.roster_min as usize;
        let uid = self.user.team;
        if let Some(u) = uid {
            if self.user_controls_roster(u) {
                let n = self.team(u).roster.len();
                if n > max {
                    return Err(format!("Your roster has {n} players but the limit is {max}. Waive {} (use: waive <player>).", n - max));
                }
                if n < min {
                    return Err(format!("Your roster has only {n} players; you need at least {min}. Sign free agents (use: fa, sign <player>)."));
                }
            }
        }
        let mut rng = self.rng.clone();
        for t in self.active_team_ids() {
            if self.user_controls_roster(t) {
                continue;
            }
            // cut down
            while self.team(t).roster.len() > max {
                let worst = self
                    .team(t)
                    .roster
                    .iter()
                    .copied()
                    .min_by(|&a, &b| self.cut_value(a).partial_cmp(&self.cut_value(b)).unwrap())
                    .unwrap();
                self.waive_player(worst, t);
            }
            // sign up
            while self.team(t).roster.len() < min {
                let mut fas = self.free_agents.clone();
                fas.sort_by(|&a, &b| {
                    self.player_value(b)
                        .partial_cmp(&self.player_value(a))
                        .unwrap()
                });
                match fas.into_iter().find(|&id| !self.p(id).user_controlled) {
                    Some(id) => {
                        let ms = self.money.min_salary;
                        self.sign_player(id, t, ms, 1, ContractKind::Minimum, "signing");
                    }
                    None => break,
                }
            }
            let _ = &mut rng;
        }
        self.rng = rng;
        for t in self.active_team_ids() {
            self.open_season_records(t);
        }
        self.calibrate_engine();
        Ok(())
    }

    fn cut_value(&self, id: PlayerId) -> f64 {
        let p = self.p(id);
        let dead = p.contract.as_ref().map(|c| c.dead_money()).unwrap_or(0) as f64
            / self.money.cap as f64
            * 60.0;
        self.player_value(id) + dead * 0.4
    }

    /// Waive a player: his guaranteed money stays on the books as dead money.
    pub fn waive_player(&mut self, id: PlayerId, t: TeamId) {
        let year = self.year;
        let dead: Vec<(Season, Money)> = match &self.p(id).contract {
            Some(c) if c.guaranteed && c.kind != ContractKind::Minimum => c
                .salaries
                .iter()
                .enumerate()
                .map(|(i, s)| (year + i as i32, *s))
                .collect(),
            Some(c) if c.guaranteed => c
                .salaries
                .iter()
                .take(1)
                .enumerate()
                .map(|(i, s)| (year + i as i32, *s))
                .collect(),
            _ => vec![],
        };
        let name = self.p(id).name();
        let tname = self.team(t).name();
        for d in dead {
            self.team_mut(t).dead_money.push(d);
        }
        self.pm(id).contract = None;
        self.release_to_fa(id, t, "waived");
        self.add_transaction("waive", format!("{tname} waive {name}."), vec![t]);
    }

    fn preseason_hooks(&mut self) {
        // owners set mandates, moods reset, new coach hires etc.
        self.with_rng(|l, rng| l.owner_set_mandates(rng));
        for t in self.active_team_ids() {
            self.team_mut(t).chemistry =
                (self.team(t).chemistry * 0.7 + 55.0 * 0.3).clamp(30.0, 90.0);
        }
    }
}

/// Short helper type so the injury recovery code above reads clearly.
pub struct InjuryRecordLite;
impl From<&crate::injury::Injury> for InjuryRecordLite {
    fn from(_: &crate::injury::Injury) -> Self {
        InjuryRecordLite
    }
}

fn ordinal(n: usize) -> String {
    match n {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        n => format!("{n}th"),
    }
}

fn ai_focus(p: &Player, rng: &mut Rng) -> DevFocus {
    // Pick the weakest family relative to position and age.
    if p.hidden.work_ethic < 35 && rng.chance(0.5) {
        return DevFocus::Balanced;
    }
    let f = |fam| p.attrs.family_avg(fam);
    let mut options: Vec<(DevFocus, f64)> = vec![
        (
            DevFocus::Shooting,
            62.0 - f(Family::Three).min(f(Family::Mid)),
        ),
        (DevFocus::Finishing, 58.0 - f(Family::Inside)),
        (
            DevFocus::Playmaking,
            if p.position == Position::PG || p.position == Position::SG {
                60.0 - f(Family::Playmaking)
            } else {
                0.0
            },
        ),
        (
            DevFocus::Defense,
            58.0 - f(Family::PerimeterD).max(f(Family::InteriorD)),
        ),
        (
            DevFocus::Rebounding,
            if p.height_in >= 80 {
                58.0 - f(Family::Rebounding)
            } else {
                0.0
            },
        ),
        (
            DevFocus::Athleticism,
            if p.birth_year > 0 {
                55.0 - f(Family::Athletic)
            } else {
                0.0
            },
        ),
    ];
    options.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    if options[0].1 < 0.0 {
        DevFocus::Balanced
    } else {
        options[0].0
    }
}

impl Player {
    /// Did this player's last pro season end with the given team? (Re-signing loyalty bonus.)
    pub fn last_team_bonus(&self, t: TeamId) -> bool {
        self.seasons
            .iter()
            .rev()
            .find(|r| r.level == Level::Pro)
            .map(|r| r.team_id == Some(t))
            .unwrap_or(false)
    }
}
