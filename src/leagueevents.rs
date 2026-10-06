//! League-wide events and franchise history: lockouts, TV deals, expansion, relocation, folding.
//!
//! League events are the same declarative `EventDef`s used by the life sim (see `events.rs`),
//! applied to the league itself. Franchise changes follow the franchise table (`franchise.rs`),
//! so a mod can invent a whole new league history.

use crate::events::*;
use crate::league::*;
use crate::rng::Rng;
use crate::team::*;
use crate::types::*;

/// Lets the event engine read and change league state.
pub struct LeagueCtx<'a> {
    pub league: &'a mut League,
    pub fired_ids: Vec<String>,
    pub log: Vec<String>,
}

impl<'a> EventContext for LeagueCtx<'a> {
    fn var(&self, name: &str) -> f64 {
        match name {
            "year" => self.league.year as f64,
            "revenue_index" => self.league.revenue_index,
            "teams" => self.league.active_team_ids().len() as f64,
            "cap_millions" => self.league.money.cap as f64 / 1e6,
            _ => 0.0,
        }
    }
    fn has_flag(&self, name: &str) -> bool {
        self.league.events_fired.iter().any(|e| e == name)
    }
    fn stage(&self) -> String {
        "league".into()
    }
    fn year(&self) -> i32 {
        self.league.year
    }
    fn subject_name(&self) -> String {
        self.league.name.clone()
    }
    fn since_fired(&self, id: &str) -> u32 {
        if self.league.events_fired.iter().any(|e| e == id) {
            0
        } else {
            u32::MAX
        }
    }
    fn mark_fired(&mut self, id: &str) {
        self.fired_ids.push(id.to_string());
    }
    fn apply(&mut self, e: &Effect, log: &mut Vec<String>) {
        let _ = log;
        let v = e.value;
        match e.target.as_str() {
            "league.games" => {
                if e.op == "set" {
                    self.league.games_this_season = v as u16;
                } else {
                    self.league.games_this_season =
                        (self.league.games_this_season as f64 + v).max(10.0) as u16;
                }
            }
            "league.revenue_pct" => {
                *self
                    .league
                    .season_mods
                    .entry("revenue_pct".into())
                    .or_insert(0.0) += v;
                self.league.revenue_index *= 1.0 + v / 100.0 * 0.5;
            }
            "league.attendance_pct" => {
                *self
                    .league
                    .season_mods
                    .entry("attendance_pct".into())
                    .or_insert(0.0) += v;
            }
            "league.credibility" => {}
            "news" => self.log.push(e.text.clone()),
            "flag" if !e.text.is_empty() => {
                self.league.events_fired.push(e.text.clone());
            }
            _ => {}
        }
    }
}

impl League {
    /// Scheduled history and random shocks. Runs at the start of each season.
    pub fn apply_league_events(&mut self) {
        let hist = self.settings.bool("realism.historical_events");
        let random = self.settings.bool("story.random_events");
        let defs: Vec<EventDef> = self
            .content
            .league_events
            .iter()
            .filter(|e| {
                let scheduled = e.min_year == e.max_year && e.chance >= 1.0;
                (scheduled && hist) || (!scheduled && random)
            })
            .cloned()
            .collect();
        if defs.is_empty() {
            return;
        }
        let mut rng = Rng::from_label(self.seed, &format!("events-{}", self.year));
        let mut log = vec![];
        let mut fired = vec![];
        {
            let mut ctx = LeagueCtx {
                league: self,
                fired_ids: vec![],
                log: vec![],
            };
            let out = roll(&defs, &mut ctx, &mut rng, 1.0, 3, true);
            log.extend(out.log);
            log.extend(std::mem::take(&mut ctx.log));
            fired.extend(std::mem::take(&mut ctx.fired_ids));
        }
        for f in fired {
            self.events_fired.push(f);
        }
        for l in log {
            self.add_news("major", l);
        }
    }

    /// Franchise moves, expansion, mergers and folds taking effect for the coming season.
    pub fn league_postseason_events(&mut self, rng: &mut Rng) {
        if !self.settings.bool("realism.historical_events") {
            return;
        }
        let next = self.year + 1;
        let include_spec = next >= 2026;
        let franchises = self.content.franchises.clone();
        for (fi, fr) in franchises.iter().enumerate() {
            let existing = self.teams.iter().position(|t| t.franchise == fr.key);
            let should_be = fr.active_in_with(next, self.settings.bool("realism.aba_merger"))
                && (!fr.speculative || include_spec);
            match existing {
                None if should_be => self.expand_league(fi, next, rng),
                Some(ti)
                    if self.teams[ti].active
                        && !should_be
                        && fr.last_year.map(|l| l < next).unwrap_or(false) =>
                {
                    self.fold_franchise(ti as TeamId, rng)
                }
                Some(ti) if self.teams[ti].active => {
                    let ident = fr.identity_in(next).clone();
                    let t = &self.teams[ti];
                    if t.city != ident.city || t.nickname != ident.nickname {
                        let old = t.name();
                        let new_name = format!("{} {}", ident.city, ident.nickname);
                        let moved = t.city != ident.city;
                        let tm = &mut self.teams[ti];
                        tm.city = ident.city.clone();
                        tm.nickname = ident.nickname.clone();
                        tm.abbr = ident.abbr.clone();
                        tm.lon = ident.lon;
                        tm.lat = ident.lat;
                        tm.market = ident.market;
                        tm.hype = (tm.hype * 0.8).max(30.0);
                        let verb = if moved { "relocate" } else { "rebrand" };
                        self.add_news(
                            "major",
                            format!("FRANCHISE MOVE: the {old} {verb} as the {new_name}."),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Add a new team (expansion or merger).
    fn expand_league(&mut self, fi: usize, next: Season, rng: &mut Rng) {
        let ident = self.content.franchises[fi].identity_in(next).clone();
        // conference by longitude vs. the median
        let mut lons: Vec<f64> = self
            .active_team_ids()
            .iter()
            .map(|&t| self.team(t).lon)
            .collect();
        lons.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let med = lons.get(lons.len() / 2).copied().unwrap_or(-90.0);
        let east_n = self
            .teams
            .iter()
            .filter(|t| t.active && t.conf == 0)
            .count();
        let west_n = self
            .teams
            .iter()
            .filter(|t| t.active && t.conf == 1)
            .count();
        let conf = if east_n < west_n {
            0
        } else if west_n < east_n {
            1
        } else if ident.lon > med {
            0
        } else {
            1
        };
        let div = conf * 3 + rng.range(0, 2) as u8;
        // build with next-season money
        let t = self.build_team_shell(rng, fi, ident, conf, div);
        self.create_staff_for_team(rng, t);
        let merger = fr_is_merger(&self.content.franchises[fi], next);
        let quality = if merger { -0.5 } else { -3.5 };
        let roster_n = (self.rules.roster_max as usize)
            .saturating_sub(1)
            .clamp(8, 14);
        self.create_roster_for_team(rng, t, quality, roster_n);
        for y in (next + 1)..=(next + 4) {
            self.add_picks_for(t, y);
        }
        // adjust contracts to the current scale
        let scale = self.money.cap as f64 * 0.95 / (self.payroll(t).max(1) as f64);
        for id in self.team(t).roster.clone() {
            let (min, max, use_max) = (
                self.money.min_salary,
                self.content.economy.max_salary(&self.money, 10),
                self.rules.max_contract,
            );
            if let Some(c) = &mut self.pm(id).contract {
                for s in c.salaries.iter_mut() {
                    let mut v = ((*s as f64) * scale).max(min as f64) as i64;
                    if use_max {
                        v = v.min(max);
                    }
                    *s = v;
                }
            }
        }
        self.team_mut(t).direction = Direction::Rebuild;
        let name = self.team(t).name();
        let kind = if merger {
            "join the league in a merger"
        } else {
            "are awarded an expansion franchise"
        };
        self.add_news(
            "major",
            format!("EXPANSION: the {name} {kind} for the {next} season."),
        );
        // expansion picks: an extra high pick in the coming draft's first round is represented by their normal pick
    }

    /// A franchise folds: players are dispersed to the free-agent pool.
    fn fold_franchise(&mut self, t: TeamId, _rng: &mut Rng) {
        let name = self.team(t).name();
        for id in self.team(t).roster.clone() {
            self.release_to_fa(id, t, "franchise folded");
            self.pm(id).contract = None;
        }
        // Picks revert to the league's surviving teams randomly (simplified: dropped)
        let picks = std::mem::take(&mut self.team_mut(t).picks);
        let survivors: Vec<TeamId> = self
            .active_team_ids()
            .into_iter()
            .filter(|&x| x != t)
            .collect();
        for (i, mut pk) in picks.into_iter().enumerate() {
            if let Some(&o) = survivors.get(i % survivors.len().max(1)) {
                pk.owner = o;
                self.team_mut(o).picks.push(pk);
            }
        }
        self.team_mut(t).active = false;
        self.team_mut(t).roster.clear();
        self.add_news(
            "major",
            format!("The {name} franchise folds. Its players enter a dispersal pool."),
        );
        if self.user.team == Some(t) {
            self.user.team = None;
            self.user
                .log
                .push(format!("{}: your franchise folded.", self.year));
        }
    }
}

/// Did this franchise arrive by merger (rather than as a weak expansion team)?
fn fr_is_merger(f: &crate::franchise::Franchise, next: i32) -> bool {
    f.tags.iter().any(|t| t == "aba") && next == 1976 || next == 1949
}
