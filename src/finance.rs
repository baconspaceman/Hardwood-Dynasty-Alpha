//! Money: team revenue and expenses, owners, budgets, mandates, staff changes, and the cap's path.
//!
//! The model is built from "cap units" (a team's revenue is a multiple of the salary cap), so it
//! works in every era, from a 1947 barnstorming budget to a modern media-rights economy.

use crate::economy::*;
use crate::league::*;
use crate::rng::Rng;
use crate::team::*;
use crate::types::*;

/// Projection of what an extra payroll commitment would cost (for Owner "Chase Mode").
#[derive(Clone, Debug)]
pub struct ChaseProjection {
    pub extra_salary: Money,
    pub new_payroll: Money,
    pub tax_line: Money,
    pub luxury_tax_before: Money,
    pub luxury_tax_after: Money,
    pub extra_tax: Money,
    pub total_extra_cost: Money,
    pub profit_before: Money,
    pub profit_after: Money,
    pub title_odds_before: f64,
    pub title_odds_after: f64,
    pub apron_warning: String,
    pub repeater: bool,
}

impl League {
    fn national_tv_share(&self) -> f64 {
        crate::era::interp(
            &[
                (1946, 0.0),
                (1955, 0.02),
                (1965, 0.12),
                (1980, 0.25),
                (1985, 0.32),
                (1998, 0.38),
                (2016, 0.45),
                (2040, 0.5),
            ],
            self.year as f64,
        )
    }

    /// Average attendance for a home game.
    pub fn attendance_for(&self, t: TeamId, playoffs: bool) -> u32 {
        let tm = self.team(t);
        let sens = self.settings.num("economy.ticket_sensitivity");
        let star = tm
            .roster
            .iter()
            .map(|&id| self.p(id).ovr as f64)
            .fold(0.0, f64::max);
        let demand = 0.50
            + 0.30 * tm.hype / 100.0
            + 0.30 * (tm.record.pct() - 0.5)
            + 0.18 * tm.market
            + (star - 70.0).max(0.0) * 0.004
            + 0.1 * (tm.fan_loyalty - 50.0) / 100.0
            - (tm.budget.ticket_price - 1.0) * sens * 0.40
            + (tm.budget.marketing - 1.0) * 0.05
            + if playoffs { 0.35 } else { 0.0 };
        let att_mult = 1.0
            + self
                .season_mods
                .get("attendance_pct")
                .copied()
                .unwrap_or(0.0)
                / 100.0;
        let a = (demand * tm.arena.capacity as f64 * att_mult).clamp(0.0, tm.arena.capacity as f64);
        a as u32
    }

    pub fn ticket_price(&self, t: TeamId) -> f64 {
        let tm = self.team(t);
        // average ticket ~ $100 in 2024
        100.0
            * crate::life::cpi(self.year)
            * tm.budget.ticket_price
            * (0.7 + 0.6 * tm.market)
            * (0.85 + 0.3 * tm.arena.quality / 100.0)
    }

    pub fn finance_home_game(&mut self, h: TeamId, playoffs: bool, home_won: bool) {
        if !self.team(h).active {
            return;
        }
        let att = self.attendance_for(h, playoffs) as f64;
        let price = self.ticket_price(h) * if playoffs { 2.4 } else { 1.0 };
        let conc = 22.0 * crate::life::cpi(self.year) * self.team(h).budget.concession_price * att;
        let tm = self.team_mut(h);
        let gate = (att * price) as i64;
        if playoffs {
            tm.finance.playoffs += gate + conc as i64;
        } else {
            tm.finance.gate += gate;
            tm.finance.concessions += conc as i64;
        }
        tm.finance.home_games += 1;
        tm.finance.attendance_total += att as u64;
        // fan mood
        tm.hype = (tm.hype + if home_won { 0.12 } else { -0.10 }).clamp(0.0, 100.0);
    }

    /// Revenue and cost components for the season that just ended.
    pub fn finance_end_season(&mut self, t: TeamId) {
        let cap = self.money.cap as f64;
        let natl = self.national_tv_share();
        let multiple = self.content.economy.revenue_cap_multiple;
        let idx = self.revenue_index;
        let tm = self.team(t).clone();
        // Base season components in cap units for an average team (market .5, hype 55).
        let mkt = tm.market;
        let hype_f = (tm.hype / 55.0).clamp(0.4, 1.8);
        let win_f = 0.8 + 0.4 * tm.record.pct();
        let local_tv =
            cap * multiple * 0.18 * (0.35 + 1.3 * mkt) * hype_f * win_f * (1.0 - natl * 0.5);
        let national = cap * multiple * natl.max(0.0) * 0.9;
        let merch = cap
            * multiple
            * 0.06
            * (0.4 + mkt * 1.2)
            * hype_f
            * (0.8
                + (tm
                    .roster
                    .iter()
                    .map(|&id| self.p(id).ovr as f64)
                    .fold(0.0, f64::max)
                    - 60.0)
                    .max(0.0)
                    * 0.012);
        let sponsors =
            cap * multiple * 0.11 * (0.45 + mkt) * hype_f * (0.8 + tm.budget.marketing * 0.2);
        let _ = idx;
        let payroll = self.payroll(t);
        let tax = if self.rules.luxury_tax {
            self.content
                .economy
                .luxury_tax(&self.money, payroll, tm.tax_years >= 3)
        } else {
            0
        };
        let staff: Money = [tm.gm, tm.head_coach, tm.trainer]
            .iter()
            .flatten()
            .map(|&p| self.person(p).salary)
            .sum::<i64>()
            + tm.assistants
                .iter()
                .map(|&p| self.person(p).salary)
                .sum::<i64>()
            + tm.scouts
                .iter()
                .map(|&p| self.person(p).salary)
                .sum::<i64>();
        let facilities = (cap * multiple * 0.035 * tm.budget.facilities) as i64;
        let marketing = (cap * multiple * 0.014 * tm.budget.marketing) as i64;
        let operations = (cap * multiple * 0.12) as i64
            + (tm.arena.capacity as f64 * 380.0 * crate::life::cpi(self.year)) as i64 / 40;
        let minor = if self.rules.g_league {
            (cap * 0.01) as i64
        } else {
            0
        };
        let year = self.year;
        let tm = self.team_mut(t);
        tm.finance.season = year;
        tm.finance.local_tv = local_tv as i64;
        tm.finance.national_tv = national as i64;
        tm.finance.merchandise = merch as i64;
        tm.finance.sponsors = sponsors as i64;
        tm.finance.payroll = payroll;
        tm.finance.luxury_tax = tax;
        tm.finance.staff = staff;
        tm.finance.facilities = facilities;
        tm.finance.marketing = marketing;
        tm.finance.operations = operations;
        tm.finance.minor_league = minor;
        if tax > 0 {
            tm.tax_years = tm.tax_years.saturating_add(1);
        } else {
            tm.tax_years = 0;
        }
    }

    /// Tax money is split among teams under the tax line (modern revenue sharing).
    pub fn distribute_tax_pool(&mut self) {
        let ids = self.active_team_ids();
        let pool: Money = ids.iter().map(|&t| self.team(t).finance.luxury_tax).sum();
        if pool <= 0 {
            return;
        }
        let eligible: Vec<TeamId> = ids
            .iter()
            .copied()
            .filter(|&t| self.team(t).finance.luxury_tax == 0)
            .collect();
        if eligible.is_empty() {
            return;
        }
        let share = pool / eligible.len() as i64;
        for t in eligible {
            self.team_mut(t).finance.revenue_sharing = share;
        }
    }

    /// After finances are final: update owner approval and archive the year's statement.
    pub fn owner_review(&mut self, t: TeamId) {
        let ids = self.active_team_ids();
        let rec_rank = {
            let order = self.standings(None);
            order.iter().position(|&x| x == t).unwrap_or(ids.len() / 2)
        };
        let n = ids.len().max(1) as f64;
        let tm = self.team(t);
        let result = self
            .playoff_results_cache
            .get(&t)
            .cloned()
            .unwrap_or_default();
        let wins_goal = tm.owner.mandate_wins as f64;
        let wins = tm.record.w as f64;
        let mut delta = (wins - wins_goal) * 0.5;
        delta += (0.5 - rec_rank as f64 / n) * 10.0;
        if result == "Champions" {
            delta += 20.0;
        } else if result.starts_with("Lost in the Finals") {
            delta += 9.0;
        } else if result == "Missed the playoffs" {
            delta -= 3.0 * tm.owner.win_now;
        }
        let margin = tm.finance.profit() as f64 / tm.finance.revenue().max(1) as f64;
        delta += (margin - 0.04)
            * 60.0
            * (1.0 - tm.owner.win_now)
            * self.settings.num("difficulty.budget_strictness").max(0.1);
        let patience = tm.owner.patience / 50.0 * self.settings.num("difficulty.owner_patience");
        let tm = self.team_mut(t);
        tm.owner.approval =
            (tm.owner.approval * 0.65 + (60.0 + delta * 1.4) * 0.35 + 0.0 * patience)
                .clamp(0.0, 100.0);
        tm.owner.years_owned += 1;
        let fin = tm.finance.clone();
        tm.finance_history.push(fin);
        if tm.finance_history.len() > 60 {
            tm.finance_history.remove(0);
        }
    }

    /// Set (AI) owner mandates and budgets for the coming season; also adjusts payroll targets.
    pub fn owner_set_mandates(&mut self, rng: &mut Rng) {
        let avg = self.avg_team_rating();
        for t in self.active_team_ids() {
            let r = self.team_rating_full_health(t);
            let wins = ((0.5 + (r - avg) * 0.045).clamp(0.12, 0.85) * self.games_this_season as f64)
                .round() as u16;
            let mandate = if r > avg + 5.0 {
                "Contend for the championship".to_string()
            } else if r > avg + 1.0 {
                format!("Make the playoffs (about {wins} wins)")
            } else if r > avg - 3.0 {
                format!("Compete for a playoff spot (about {wins} wins)")
            } else {
                "Rebuild: develop young players and stay financially responsible".to_string()
            };
            let cap = self.money.cap;
            let last_margin = self
                .team(t)
                .finance_history
                .last()
                .map(|f| f.profit() as f64 / f.revenue().max(1) as f64)
                .unwrap_or(0.05);
            let tm = self.team_mut(t);
            tm.owner.mandate = mandate;
            tm.owner.mandate_wins = wins;
            if !(self.user.team == Some(t) && self.user.has(Role::Owner)) {
                let win_now = self.team(t).owner.win_now;
                let tm = self.team_mut(t);
                let target = cap as f64
                    * (0.90
                        + 0.18 * win_now
                        + 0.12 * (last_margin > 0.08) as i32 as f64
                        + 0.10 * tm.market
                        + rng.gauss(0.0, 0.02));
                tm.budget.payroll_target = target as i64;
                tm.budget.tax_tolerance =
                    (cap as f64 * 0.08 * (tm.owner.wealth / 50.0) * (0.3 + win_now)).max(0.0)
                        as i64;
            }
        }
    }

    // ------------------------------------------------------------------ cap path

    pub fn project_next_season_money(&mut self, rng: &mut Rng) {
        let next = self.year + 1;
        // revenue index follows trend + noise + event shocks
        let trend = 0.045 * self.settings.num("economy.cap_growth");
        let vol = self.settings.num("economy.revenue_volatility");
        let shock = self.season_mods.get("revenue_pct").copied().unwrap_or(0.0) / 100.0;
        let growth = (trend + rng.gauss(0.0, 0.02 * vol) + shock * 0.8).clamp(-0.25, 0.6);
        self.revenue_index *= 1.0 + growth;
        let mode = self.settings.text("realism.cap_mode");
        let table_has = self
            .content
            .economy
            .cap_table
            .iter()
            .any(|r| r.year == next)
            && next <= 2025;
        let this_cap = self.money.cap;
        let new_cap = match mode.as_str() {
            "frozen" => this_cap,
            "historical" if table_has => self.content.economy.table_cap(next),
            _ => {
                let mut g = growth;
                // modern smoothing: the cap can't rise more than 10% a year (post-2016 CBA); earlier eras could jump
                let max_up = if self.rules.aprons {
                    0.10
                } else if self.rules.luxury_tax {
                    0.15
                } else {
                    0.30
                };
                g = g.min(max_up);
                (this_cap as f64 * (1.0 + g)) as i64
            }
        };
        if !self.cap_history.iter().any(|(y, _)| *y == next) {
            self.cap_history.push((next, new_cap));
        }
    }

    // ------------------------------------------------------------------ staff

    pub fn staff_changes(&mut self, rng: &mut Rng) {
        let year = self.year;
        // retired players turn into coaches, scouts, executives
        let new_retirees: Vec<PlayerId> = self
            .players
            .iter()
            .filter(|p| {
                p.retired == Some(year)
                    && p.peak_ovr >= 60
                    && p.years_pro >= 4
                    && !p.user_controlled
            })
            .map(|p| p.id)
            .collect();
        for id in new_retirees {
            if !rng.chance(0.18) {
                continue;
            }
            let (name, born, peak, iq) = {
                let p = self.p(id);
                (
                    p.name(),
                    p.birth_year,
                    p.peak_ovr as f64,
                    p.attrs.get(crate::player::Attr::ShotIq),
                )
            };
            let role = if rng.chance(0.6) {
                StaffRole::AssistantCoach
            } else if rng.chance(0.5) {
                StaffRole::Scout
            } else {
                StaffRole::GeneralManager
            };
            let pid = self.new_person(
                rng,
                role,
                40.0 + (peak - 55.0) * 0.4 + (iq - 55.0) * 0.2,
                None,
            );
            let per = &mut self.people[pid as usize];
            per.name = name;
            per.born = born;
            per.former_player = Some(id);
            per.reputation = (peak - 20.0).clamp(20.0, 90.0);
        }
        // firings & hirings
        let ids = self.active_team_ids();
        for &t in &ids {
            let user_is_gm = self.user.team == Some(t) && self.user.has(Role::Gm);
            let user_is_coach = self.user.team == Some(t) && self.user.has(Role::HeadCoach);
            let approval = self.team(t).owner.approval;
            let pct = self.team(t).record.pct();
            let patience =
                self.team(t).owner.patience * self.settings.num("difficulty.owner_patience");
            // owner reviews finances/approval once per season
            self.owner_review(t);
            // fire the coach after bad results
            if !user_is_coach
                && !(self.user.team == Some(t)
                    && self.user.has(Role::Owner)
                    && !self.user.delegate_coach)
            {
                if let Some(hc) = self.team(t).head_coach {
                    let yrs = self.person(hc).years_in_role;
                    let fire_p = if pct < 0.35 {
                        0.5
                    } else if pct < 0.42 {
                        0.25
                    } else {
                        0.03
                    } * (60.0 / patience.max(10.0))
                        * if approval < 35.0 { 1.4 } else { 1.0 }
                        * if yrs < 2 { 0.5 } else { 1.0 };
                    let expiring = self.person(hc).years_left <= 1;
                    if rng.chance(fire_p.clamp(0.0, 0.85)) || (expiring && rng.chance(0.2)) {
                        self.replace_coach(t, rng);
                    }
                }
            }
            if !user_is_gm
                && !(self.user.team == Some(t)
                    && self.user.has(Role::Owner)
                    && !self.user.delegate_gm)
            {
                if let Some(gm) = self.team(t).gm {
                    let fire_p = if approval < 30.0 {
                        0.25
                    } else if approval < 42.0 {
                        0.08
                    } else {
                        0.01
                    };
                    if rng.chance(fire_p) {
                        self.replace_gm(t, gm, rng);
                    }
                }
            }
            // staff contract/age upkeep
            let all: Vec<PersonId> = [
                self.team(t).gm,
                self.team(t).head_coach,
                self.team(t).trainer,
            ]
            .iter()
            .flatten()
            .copied()
            .chain(self.team(t).assistants.iter().copied())
            .chain(self.team(t).scouts.iter().copied())
            .collect();
            for pid in all {
                let p = &mut self.people[pid as usize];
                p.years_left = p.years_left.saturating_sub(1).max(1);
                // skills drift
                p.offense = (p.offense + rng.gauss(0.2, 1.5)).clamp(10.0, 99.0);
                p.defense = (p.defense + rng.gauss(0.2, 1.5)).clamp(10.0, 99.0);
                p.development = (p.development + rng.gauss(0.2, 1.5)).clamp(10.0, 99.0);
                p.tactics = (p.tactics + rng.gauss(0.2, 1.5)).clamp(10.0, 99.0);
                p.evaluation = (p.evaluation + rng.gauss(0.1, 1.5)).clamp(10.0, 99.0);
            }
            // refresh the head coach's strategy from his philosophy
            if let Some(hc) = self.team(t).head_coach {
                if !(self.user_controls_lineup(t)) {
                    let mut s = self.person(hc).philosophy.clone();
                    s.tactics = self.person(hc).tactics;
                    self.team_mut(t).strategy = s;
                }
            }
        }
        // retire old staff in the unattached pool
        for p in self.people.iter_mut() {
            if p.team.is_none() && (year - p.born) > 68 {
                p.retired = true;
            }
        }
        // user: job security check
        self.check_user_job_security();
        self.distribute_tax_pool_if_needed();
    }

    fn distribute_tax_pool_if_needed(&mut self) {}

    pub fn replace_coach(&mut self, t: TeamId, rng: &mut Rng) {
        let old = self.team(t).head_coach;
        if let Some(o) = old {
            self.people[o as usize].team = None;
            self.people[o as usize].reputation =
                (self.people[o as usize].reputation - 6.0).max(5.0);
            let n = self.people[o as usize].name.clone();
            let tn = self.team(t).name();
            self.add_news("staff", format!("{tn} fire head coach {n}."));
        }
        let cand = self.best_available(StaffRole::HeadCoach, rng);
        let newc = match cand {
            Some(c) => c,
            None => self.new_person(rng, StaffRole::HeadCoach, 48.0, None),
        };
        self.people[newc as usize].team = Some(t);
        self.people[newc as usize].years_left = rng.range(2, 4) as u8;
        self.people[newc as usize].years_in_role = 0;
        self.team_mut(t).head_coach = Some(newc);
        let tn = self.team(t).name();
        let nn = self.people[newc as usize].name.clone();
        self.add_news("staff", format!("{tn} hire {nn} as head coach."));
    }

    pub fn replace_gm(&mut self, t: TeamId, old: PersonId, rng: &mut Rng) {
        self.people[old as usize].team = None;
        let n = self.people[old as usize].name.clone();
        let tn = self.team(t).name();
        self.add_news("staff", format!("{tn} part ways with general manager {n}."));
        let newg = self
            .best_available(StaffRole::GeneralManager, rng)
            .unwrap_or_else(|| self.new_person(rng, StaffRole::GeneralManager, 48.0, None));
        self.people[newg as usize].team = Some(t);
        self.team_mut(t).gm = Some(newg);
    }

    fn best_available(&mut self, role: StaffRole, rng: &mut Rng) -> Option<PersonId> {
        let mut c: Vec<PersonId> = self
            .people
            .iter()
            .filter(|p| p.role == role && p.team.is_none() && !p.retired)
            .map(|p| p.id)
            .collect();
        c.sort_by(|&a, &b| {
            self.people[b as usize]
                .overall()
                .partial_cmp(&self.people[a as usize].overall())
                .unwrap()
        });
        // choose among the top 3
        let k = c.len().min(3);
        if k == 0 {
            return None;
        }
        let pick = c[rng.range_usize(k)];
        Some(pick)
    }

    // ------------------------------------------------------------------ owner tools

    /// Cost/benefit preview of taking on extra salary this season.
    pub fn chase_projection(&self, t: TeamId, extra_salary: Money) -> ChaseProjection {
        let payroll = self.payroll(t);
        let tm = self.team(t);
        let repeater = tm.tax_years >= 3;
        let tax_before = if self.rules.luxury_tax {
            self.content
                .economy
                .luxury_tax(&self.money, payroll, repeater)
        } else {
            0
        };
        let tax_after = if self.rules.luxury_tax {
            self.content
                .economy
                .luxury_tax(&self.money, payroll + extra_salary, repeater)
        } else {
            0
        };
        let profit_before = self.projected_profit(t, payroll, tax_before);
        let profit_after = self.projected_profit(t, payroll + extra_salary, tax_after);
        let before = self.title_odds_for(t, 0.0);
        let after =
            self.title_odds_for(t, extra_salary as f64 / self.money.cap as f64 * 28.0 * 0.45);
        let mut warn = String::new();
        if self.rules.aprons {
            if payroll + extra_salary > self.money.second_apron {
                warn = "Above the SECOND apron: no aggregating salaries in trades, no mid-level exception, frozen future picks, no sign-and-trade.".into();
            } else if payroll + extra_salary > self.money.first_apron {
                warn = "Above the FIRST apron: no taxpayer mid-level exception, trades must send out at least as much salary as you take back.".into();
            }
        }
        ChaseProjection {
            extra_salary,
            new_payroll: payroll + extra_salary,
            tax_line: self.money.tax_line,
            luxury_tax_before: tax_before,
            luxury_tax_after: tax_after,
            extra_tax: tax_after - tax_before,
            total_extra_cost: extra_salary + (tax_after - tax_before),
            profit_before,
            profit_after,
            title_odds_before: before,
            title_odds_after: after,
            apron_warning: warn,
            repeater,
        }
    }

    /// A quick estimate of this season's profit given a payroll (uses last season's revenue as the guide).
    pub fn projected_profit(&self, t: TeamId, payroll: Money, tax: Money) -> Money {
        let tm = self.team(t);
        let base = tm
            .finance_history
            .last()
            .cloned()
            .unwrap_or_else(|| tm.finance.clone());
        let rev = if base.revenue() > 0 {
            base.revenue()
        } else {
            (self.money.cap as f64 * self.content.economy.revenue_cap_multiple) as i64
        };
        let other = if base.expenses() > 0 {
            base.expenses() - base.payroll - base.luxury_tax
        } else {
            (self.money.cap as f64 * 0.58) as i64
        };
        rev - payroll - tax - other
    }

    /// Title odds for a team given a rating boost (Monte Carlo over the bracket using ratings).
    pub fn title_odds_for(&self, t: TeamId, boost: f64) -> f64 {
        let ids = self.active_team_ids();
        let mut ratings: Vec<(TeamId, f64)> = ids
            .iter()
            .map(|&x| {
                (
                    x,
                    self.team_rating_full_health(x) + if x == t { boost } else { 0.0 },
                )
            })
            .collect();
        ratings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let n = (self.rules.playoff_teams as usize).clamp(2, ids.len());
        let field: Vec<(TeamId, f64)> = ratings.into_iter().take(n).collect();
        let mut rng = Rng::from_label(
            self.seed,
            &format!("odds-{}-{}-{}", self.year, t, (boost * 100.0) as i64),
        );
        let trials = 600;
        let mut wins = 0;
        let best_of = *self.rules.series_lengths.last().unwrap_or(&7) as u32;
        for _ in 0..trials {
            let mut alive: Vec<(TeamId, f64)> = field.clone();
            while alive.len() > 1 {
                let mut next = vec![];
                let m = alive.len();
                for i in 0..m / 2 {
                    let (a, b) = (alive[i], alive[m - 1 - i]);
                    let p = self.win_prob(a.1, b.1);
                    let need = best_of / 2 + 1;
                    let (mut wa, mut wb) = (0, 0);
                    while wa < need && wb < need {
                        if rng.f64() < p {
                            wa += 1;
                        } else {
                            wb += 1;
                        }
                    }
                    next.push(if wa >= need { a } else { b });
                }
                if m % 2 == 1 {
                    next.push(alive[m / 2]);
                }
                alive = next;
            }
            if alive[0].0 == t {
                wins += 1;
            }
        }
        wins as f64 / trials as f64
    }

    /// Estimated title odds for every playoff-calibre team.
    pub fn title_odds_table(&self) -> Vec<(TeamId, f64)> {
        let mut v: Vec<(TeamId, f64)> = self
            .active_team_ids()
            .into_iter()
            .map(|t| (t, self.title_odds_for(t, 0.0)))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        v
    }

    /// The Owner can set budget levers (clamped to sane ranges).
    pub fn set_budget(&mut self, t: TeamId, key: &str, value: f64) -> Result<String, String> {
        let cap = self.money.cap as f64;
        let b = &mut self.teams[t as usize].budget;
        match key {
            "payroll" | "payroll_target" => {
                if value < 0.2 * cap || value > 2.5 * cap {
                    return Err("Payroll target must be between 20% and 250% of the salary cap.".into());
                }
                b.payroll_target = value as i64;
            }
            "tax" | "tax_tolerance" => {
                if value < 0.0 {
                    return Err("Tax tolerance can't be negative.".into());
                }
                b.tax_tolerance = value as i64;
            }
            "coaching" => b.coaching = value.clamp(0.5, 2.0),
            "medical" => b.medical = value.clamp(0.5, 2.0),
            "scouting" => b.scouting = value.clamp(0.5, 2.0),
            "facilities" => b.facilities = value.clamp(0.5, 2.0),
            "marketing" => b.marketing = value.clamp(0.5, 2.0),
            "ticket" | "ticket_price" => b.ticket_price = value.clamp(0.5, 2.5),
            "concessions" | "concession_price" => b.concession_price = value.clamp(0.5, 2.5),
            _ => return Err(format!("Unknown budget item '{key}'. Try: payroll, tax, coaching, medical, scouting, facilities, marketing, ticket, concessions.")),
        }
        Ok(format!("Budget updated: {key} = {value}."))
    }
}
