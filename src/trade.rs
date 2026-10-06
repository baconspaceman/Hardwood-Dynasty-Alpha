//! Trades, signings and the cap rules that govern them, plus player mood.
//!
//! Trade AI values every asset with a single "value" scale (stars are worth far more than role
//! players, draft picks depend on how bad the original team is, contracts add or subtract value,
//! and each team's direction - contending or rebuilding - changes what it wants). The same
//! evaluator drives AI-to-AI trades and responses to the human's offers.

use crate::contract::*;
use crate::economy::market_value_pct;
use crate::league::*;
use crate::player::*;
use crate::rng::Rng;
use crate::team::*;
use crate::types::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Asset {
    Player(PlayerId),
    Pick {
        year: Season,
        round: u8,
        original: TeamId,
    },
}

#[derive(Clone, Debug)]
pub struct TradeProposal {
    pub from: TeamId,
    pub to: TeamId,
    /// Assets `from` sends.
    pub give: Vec<Asset>,
    /// Assets `from` receives.
    pub get: Vec<Asset>,
}

#[derive(Clone, Debug, Default)]
pub struct TradeEval {
    /// Passes every rule (cap, roster, deadline...).
    pub legal: bool,
    pub accepted: bool,
    pub errors: Vec<String>,
    pub notes: Vec<String>,
    /// Value the partner gets / gives up, in the partner's own eyes.
    pub partner_gain: f64,
    pub partner_loss: f64,
    pub message: String,
}

impl League {
    // ------------------------------------------------------------------ valuation

    fn exp_value(rating: f64) -> f64 {
        ((rating - 50.0) / 8.5).exp()
    }

    /// Value of a player to `team` (None = neutral league view).
    pub fn trade_value_player(&self, id: PlayerId, team: Option<TeamId>) -> f64 {
        let p = self.p(id);
        let age = (self.year + 1 - p.birth_year) as f64;
        let mut rating = p.ovr as f64;
        let youth = ((25.0 - age) / 5.0).clamp(0.0, 1.0);
        rating += (p.potential as f64 - p.ovr as f64) * 0.55 * youth;
        let mut v = Self::exp_value(rating);
        // aging
        let old = (age - 29.0).max(0.0);
        v *= (0.86f64).powf(old);
        // injuries
        if let Some(i) = &p.injury {
            if i.career_ending {
                return 0.0;
            }
            v *= 1.0 - (i.games_remaining as f64 / 150.0).min(0.7) * 0.8;
        }
        // contract surplus
        if let Some(c) = &p.contract {
            let market = market_value_pct(p.ovr as f64, age, p.potential as f64);
            let sal = c.salary() as f64 / self.money.cap as f64;
            let yrs = (c.years_left() as f64).min(4.0);
            v *= (1.0 + ((market - sal) * 3.2 * yrs).clamp(-0.65, 0.9)).max(0.1);
        } else if matches!(p.affiliation, Affiliation::FreeAgent) {
            v *= 0.5;
        }
        // team direction
        if let Some(t) = team {
            match self.team(t).direction {
                Direction::Rebuild => {
                    v *= if age <= 24.0 {
                        1.25
                    } else if age >= 29.0 {
                        0.65
                    } else {
                        0.95
                    };
                }
                Direction::Contend => {
                    v *= if age >= 29.0 && p.ovr >= 60 {
                        1.1
                    } else if p.ovr < 55 {
                        0.85
                    } else {
                        1.0
                    };
                }
                Direction::Retool => {}
            }
        }
        v
    }

    /// Expected draft position (1 = best pick) for a team in a future draft.
    fn expected_pick_slot(&self, original: TeamId, years_ahead: i32) -> f64 {
        let ids = self.active_team_ids();
        let n = ids.len().max(1) as f64;
        let my = self.team_rating_full_health(original);
        let better = ids
            .iter()
            .filter(|&&t| self.team_rating_full_health(t) > my)
            .count() as f64;
        let slot = n - better; // best team -> 1 (late pick), worst -> n... so invert
        let this_year = slot; // higher = worse team = earlier pick? slot counts teams worse than us + 1
                              // slot = number of teams not better than us = (n - better). Team with many better teams => small slot => early pick.
        let early = this_year.clamp(1.0, n);
        // regress to the middle for future years
        let mid = (n + 1.0) / 2.0;
        let w = (0.6f64).powi(years_ahead.max(0));
        early * w + mid * (1.0 - w)
    }

    pub fn trade_value_pick(
        &self,
        year: Season,
        round: u8,
        original: TeamId,
        team: Option<TeamId>,
    ) -> f64 {
        let ahead = (year
            - self.year
            - if self.phase >= Phase::Draft || self.phase == Phase::PostSeason {
                0
            } else {
                1
            })
        .max(0);
        let slot = self.expected_pick_slot(original, ahead);
        let n = self.active_team_ids().len().max(1) as f64;
        let rel = slot / n; // small = early pick
        let base = if round == 1 {
            9.0 * (-2.3 * rel).exp()
        } else {
            0.5 * (-1.5 * rel).exp() + 0.1
        };
        let mut v = base * (0.92f64).powi(ahead);
        if let Some(t) = team {
            match self.team(t).direction {
                Direction::Rebuild => v *= 1.3,
                Direction::Contend => v *= 0.8,
                _ => {}
            }
        }
        v
    }

    pub fn asset_value(&self, a: &Asset, team: Option<TeamId>) -> f64 {
        match a {
            Asset::Player(id) => self.trade_value_player(*id, team),
            Asset::Pick {
                year,
                round,
                original,
            } => self.trade_value_pick(*year, *round, *original, team),
        }
    }

    pub fn asset_label(&self, a: &Asset) -> String {
        match a {
            Asset::Player(id) => format!(
                "{} ({} ovr, {})",
                self.p(*id).name(),
                self.p(*id).ovr,
                fmt_money(self.p(*id).current_salary())
            ),
            Asset::Pick {
                year,
                round,
                original,
            } => format!(
                "{} round {} pick ({})",
                year,
                round,
                self.team(*original).abbr
            ),
        }
    }

    // ------------------------------------------------------------------ rules

    /// Salary-matching rules by era. Returns (max incoming ratio, flat allowance).
    pub fn trade_matching(&self) -> (f64, Money) {
        let y = self.year;
        let flat = (self.money.cap as f64 * 0.0006).max(2_000.0) as i64;
        if !self.money.cap_enforced {
            (f64::MAX, 0)
        } else if y >= 2023 && self.rules.aprons {
            (1.25, flat)
        } else if y >= 2005 {
            (1.25, flat)
        } else if y >= 1988 {
            (1.0, flat)
        } else {
            (1.0, flat)
        }
    }

    fn can_trade_now(&self) -> Result<(), String> {
        match self.phase {
            Phase::RegularSeason => {
                if self.trade_deadline_day > 0 && self.day > self.trade_deadline_day as u32 {
                    return Err("The trade deadline has passed.".into());
                }
                Ok(())
            }
            Phase::PlayIn | Phase::Playoffs => {
                Err("Trades are frozen during the postseason.".into())
            }
            _ => Ok(()),
        }
    }

    pub fn first_round_picks_in_year(&self, t: TeamId, y: Season) -> usize {
        self.team(t)
            .picks
            .iter()
            .filter(|p| p.year == y && p.round == 1)
            .count()
    }

    /// Validate a proposal against all the rules without moving anything.
    pub fn validate_trade(&self, tr: &TradeProposal) -> Vec<String> {
        let mut errs = vec![];
        if let Err(e) = self.can_trade_now() {
            errs.push(e);
            return errs;
        }
        if tr.from == tr.to {
            errs.push("A team can't trade with itself.".into());
            return errs;
        }
        if tr.give.is_empty() && tr.get.is_empty() {
            errs.push("The trade is empty.".into());
            return errs;
        }
        // asset ownership
        for (team, assets) in [(tr.from, &tr.give), (tr.to, &tr.get)] {
            for a in assets {
                match a {
                    Asset::Player(id) => {
                        let p = self.p(*id);
                        if p.team_id() != Some(team)
                            || !matches!(p.affiliation, Affiliation::Nba(_))
                        {
                            errs.push(format!(
                                "{} isn't on the {}.",
                                p.name(),
                                self.team(team).name()
                            ));
                        } else if p.contract.as_ref().map(|c| c.no_trade).unwrap_or(false) {
                            errs.push(format!("{} has a no-trade clause.", p.name()));
                        }
                    }
                    Asset::Pick {
                        year,
                        round,
                        original,
                    } => {
                        if !self.team(team).picks.iter().any(|p| {
                            p.year == *year && p.round == *round && p.original == *original
                        }) {
                            errs.push(format!(
                                "The {} doesn't own that {} round {} pick.",
                                self.team(team).name(),
                                year,
                                round
                            ));
                        }
                    }
                }
            }
        }
        if !errs.is_empty() {
            return errs;
        }
        // roster sizes
        let max = self.rules.roster_max as usize;
        let n = |t: TeamId, out: &Vec<Asset>, inn: &Vec<Asset>| {
            let outc = out.iter().filter(|a| matches!(a, Asset::Player(_))).count();
            let inc = inn.iter().filter(|a| matches!(a, Asset::Player(_))).count();
            self.team(t).roster.len() + inc - outc
        };
        let (nf, nt) = (n(tr.from, &tr.give, &tr.get), n(tr.to, &tr.get, &tr.give));
        let offseason = !matches!(self.phase, Phase::RegularSeason);
        if !offseason {
            if nf > max {
                errs.push(format!(
                    "{} would have {} players (limit {}). Waive someone first.",
                    self.team(tr.from).name(),
                    nf,
                    max
                ));
            }
            if nt > max {
                errs.push(format!(
                    "{} would have {} players (limit {}).",
                    self.team(tr.to).name(),
                    nt,
                    max
                ));
            }
        }
        // salary matching (cap eras)
        if self.money.cap_enforced {
            let (ratio, flat) = self.trade_matching();
            let sal = |ids: &Vec<Asset>| -> Money {
                ids.iter()
                    .map(|a| {
                        if let Asset::Player(id) = a {
                            self.p(*id).current_salary()
                        } else {
                            0
                        }
                    })
                    .sum()
            };
            let (out_f, in_f) = (sal(&tr.give), sal(&tr.get));
            for (team, out_s, in_s) in [(tr.from, out_f, in_f), (tr.to, in_f, out_f)] {
                let payroll_after = self.payroll(team) - out_s + in_s;
                let cap = self.money.cap;
                if payroll_after > cap && in_s > 0 {
                    let allowed = (out_s as f64 * ratio) as i64 + flat;
                    let mut ok = in_s <= allowed;
                    // absorbing into cap room is fine up to the cap
                    if !ok && self.payroll(team) - out_s + in_s <= cap + flat {
                        ok = true;
                    }
                    if !ok {
                        errs.push(format!(
                            "{} would be over the cap and can take back at most {} (sends {}, receives {}). Add salary or remove some.",
                            self.team(team).name(),
                            fmt_money(allowed),
                            fmt_money(out_s),
                            fmt_money(in_s)
                        ));
                    }
                }
                // aprons
                if self.rules.aprons {
                    if payroll_after > self.money.second_apron && in_s > out_s {
                        errs.push(format!("{} would be above the second apron: it must send out at least as much salary as it takes back.", self.team(team).name()));
                    } else if payroll_after > self.money.first_apron
                        && in_s as f64 > out_s as f64 * 1.1 + flat as f64
                    {
                        errs.push(format!("{} would be above the first apron: incoming salary can't exceed 110% of outgoing.", self.team(team).name()));
                    }
                }
            }
        }
        // Stepien rule: no giving up first-round picks in consecutive years
        if self.year >= 1982 {
            for (team, out) in [(tr.from, &tr.give), (tr.to, &tr.get)] {
                let mut years: Vec<Season> = ((self.year + 1)..=(self.year + 4)).collect();
                years.sort();
                let own_after =
                    |y: Season| -> i32 {
                        let mut c = self.first_round_picks_in_year(team, y) as i32;
                        for a in out.iter() {
                            if let Asset::Pick {
                                year,
                                round,
                                original,
                            } = a
                            {
                                if *year == y
                                    && *round == 1
                                    && self.team(team).picks.iter().any(|p| {
                                        p.year == y && p.round == 1 && p.original == *original
                                    })
                                {
                                    c -= 1;
                                }
                            }
                        }
                        // incoming
                        let incoming = if team == tr.from { &tr.get } else { &tr.give };
                        for a in incoming.iter() {
                            if let Asset::Pick { year, round, .. } = a {
                                if *year == y && *round == 1 {
                                    c += 1;
                                }
                            }
                        }
                        c
                    };
                for w in years.windows(2) {
                    if own_after(w[0]) <= 0 && own_after(w[1]) <= 0 {
                        errs.push(format!("{} can't trade away first-round picks in back-to-back years ({} and {}): the Stepien rule.", self.team(team).name(), w[0], w[1]));
                    }
                }
            }
        }
        errs
    }

    /// Evaluate a proposal: legality plus whether the partner's front office would accept.
    pub fn evaluate_trade(&self, tr: &TradeProposal) -> TradeEval {
        let mut ev = TradeEval::default();
        ev.errors = self.validate_trade(tr);
        ev.legal = ev.errors.is_empty();
        // partner perspective
        let gain: f64 = tr
            .give
            .iter()
            .map(|a| self.asset_value(a, Some(tr.to)))
            .sum();
        let loss: f64 = tr
            .get
            .iter()
            .map(|a| self.asset_value(a, Some(tr.to)))
            .sum();
        // Quantity penalty: AI dislikes taking many mediocre pieces for one good one (roster spots).
        let n_in = tr
            .give
            .iter()
            .filter(|a| matches!(a, Asset::Player(_)))
            .count() as f64;
        let n_out = tr
            .get
            .iter()
            .filter(|a| matches!(a, Asset::Player(_)))
            .count() as f64;
        let best_in = tr
            .give
            .iter()
            .map(|a| self.asset_value(a, Some(tr.to)))
            .fold(0.0, f64::max);
        let best_out = tr
            .get
            .iter()
            .map(|a| self.asset_value(a, Some(tr.to)))
            .fold(0.0, f64::max);
        let consolidation = if best_out > best_in * 1.3 { 1.08 } else { 1.0 };
        let _ = (n_in, n_out);
        ev.partner_gain = gain;
        ev.partner_loss = loss;
        let pick = self.settings.num("ai.trade_pickiness");
        let need = 1.0 + (pick - 1.0) * 0.15 + 0.06;
        // trading with your own team: the partner is AI; unless it's also user-run
        let accept =
            gain >= loss * need * consolidation && gain > 0.0 || (loss == 0.0 && gain > 0.0);
        ev.accepted = ev.legal && accept;
        ev.message = if !ev.legal {
            "This trade breaks the league's rules.".to_string()
        } else if ev.accepted {
            format!("{} accept the offer.", self.team(tr.to).name())
        } else if gain >= loss * 0.8 {
            format!(
                "{} are close, but want a little more.",
                self.team(tr.to).name()
            )
        } else {
            format!(
                "{} decline: they value what they give up much more than what they get.",
                self.team(tr.to).name()
            )
        };
        ev
    }

    /// Carry out an agreed trade.
    pub fn execute_trade(&mut self, tr: &TradeProposal) {
        let (a, b) = (tr.from, tr.to);
        let year = self.year;
        let (na, nb) = (self.team(a).name(), self.team(b).name());
        let mut desc_give = vec![];
        let mut desc_get = vec![];
        for asset in &tr.give {
            desc_give.push(self.asset_label(asset));
            self.transfer_asset(asset, a, b);
        }
        for asset in &tr.get {
            desc_get.push(self.asset_label(asset));
            self.transfer_asset(asset, b, a);
        }
        let text = format!(
            "TRADE: {na} send {} to {nb} for {}.",
            desc_give.join(", "),
            desc_get.join(", ")
        );
        self.add_transaction("trade", text.clone(), vec![a, b]);
        let involves_star = tr
            .give
            .iter()
            .chain(tr.get.iter())
            .any(|x| matches!(x, Asset::Player(id) if self.p(*id).ovr >= 70));
        if involves_star || self.is_user_team(a) || self.is_user_team(b) {
            self.add_news("trade", text);
        }
        let _ = year;
        // chemistry hit
        for t in [a, b] {
            let tm = self.team_mut(t);
            tm.chemistry = (tm.chemistry - 2.0).max(25.0);
        }
    }

    fn transfer_asset(&mut self, asset: &Asset, from: TeamId, to: TeamId) {
        match asset {
            Asset::Player(id) => {
                let id = *id;
                self.team_mut(from).roster.retain(|&x| x != id);
                self.team_mut(from).gleague.retain(|&x| x != id);
                let tname = self.team(to).name();
                let year = self.year;
                let p = self.pm(id);
                p.affiliation = Affiliation::Nba(to);
                p.mood.wants_trade = false;
                p.mood.overall = (p.mood.overall + 4.0).min(100.0);
                p.flags.remove("expiring");
                self.team_mut(to).roster.push(id);
                if matches!(self.phase, Phase::RegularSeason | Phase::Preseason) {
                    self.ensure_record(id, to, &tname, year);
                }
                if let Some(c) = self.pm(id).contract.as_mut() {
                    // trade kicker
                    if c.trade_kicker > 0.0 {
                        let bump = (c.salary() as f64 * c.trade_kicker as f64 / 100.0) as i64;
                        if let Some(s) = c.salaries.first_mut() {
                            *s += bump;
                        }
                    }
                }
            }
            Asset::Pick {
                year,
                round,
                original,
            } => {
                self.move_pick(*year, *round, *original, from, to);
            }
        }
    }

    /// Ask the partner what it would take: finds assets from `from`'s roster/picks that close the gap.
    pub fn suggest_additions(&self, tr: &TradeProposal) -> Vec<Asset> {
        let gain: f64 = tr
            .give
            .iter()
            .map(|a| self.asset_value(a, Some(tr.to)))
            .sum();
        let loss: f64 = tr
            .get
            .iter()
            .map(|a| self.asset_value(a, Some(tr.to)))
            .sum();
        let pick = self.settings.num("ai.trade_pickiness");
        let need = loss * (1.0 + (pick - 1.0) * 0.15 + 0.06) - gain;
        if need <= 0.0 {
            return vec![];
        }
        let mut cands: Vec<(f64, Asset)> = vec![];
        for &id in &self.team(tr.from).roster {
            let a = Asset::Player(id);
            if tr.give.contains(&a) {
                continue;
            }
            let v = self.asset_value(&a, Some(tr.to));
            cands.push((v, a));
        }
        for pk in &self.team(tr.from).picks {
            let a = Asset::Pick {
                year: pk.year,
                round: pk.round,
                original: pk.original,
            };
            if tr.give.contains(&a) {
                continue;
            }
            cands.push((self.asset_value(&a, Some(tr.to)), a));
        }
        // the cheapest single asset that closes the gap, else the two cheapest that do
        cands.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if let Some((_, a)) = cands.iter().find(|(v, _)| *v >= need) {
            return vec![a.clone()];
        }
        let mut acc = 0.0;
        let mut out = vec![];
        for (v, a) in cands.iter().rev() {
            out.push(a.clone());
            acc += v;
            if acc >= need || out.len() >= 3 {
                break;
            }
        }
        out
    }

    // ------------------------------------------------------------------ signings under the cap

    /// Can `t` sign a player at `salary`? Returns the exception used, or an explanation.
    pub fn check_signing(
        &self,
        t: TeamId,
        salary: Money,
        kind: ContractKind,
    ) -> Result<String, String> {
        let roster_max = self.rules.roster_max as usize;
        if self.team(t).roster.len() >= roster_max
            && !matches!(self.phase, Phase::Draft | Phase::FreeAgency)
        {
            return Err(format!(
                "Your roster is full ({roster_max}). Waive someone first."
            ));
        }
        let payroll = self.payroll(t);
        let minsal = self.money.min_salary;
        if !self.money.cap_enforced {
            let limit = self.team(t).budget.payroll_target + self.team(t).budget.tax_tolerance;
            if payroll + salary > limit.max((self.money.cap as f64 * 1.6) as i64)
                && !self.user.has(Role::Owner)
            {
                return Err(format!(
                    "That would push payroll to {} which is beyond the owner's budget.",
                    fmt_money(payroll + salary)
                ));
            }
            return Ok("no salary cap this era".into());
        }
        if kind == ContractKind::BirdRights {
            return Ok("Bird rights (re-signing your own player)".into());
        }
        if salary <= minsal {
            return Ok("minimum contract exception".into());
        }
        if payroll + salary <= self.money.cap {
            return Ok("cap space".into());
        }
        if self.rules.mid_level_exception {
            let avail = self.mle_amount() - self.team(t).exceptions.mle_used;
            let mle_blocked = self.rules.aprons && payroll > self.money.second_apron;
            if mle_blocked {
                return Err(
                    "Teams above the second apron can't use the mid-level exception.".into(),
                );
            }
            if salary <= avail
                && !(self.rules.aprons && payroll > self.money.first_apron && salary > avail / 2)
            {
                return Ok("mid-level exception".into());
            }
        }
        Err(format!(
            "No room: payroll {} + {} would exceed the cap ({}). You can sign minimum contracts, use the mid-level exception ({} left), or clear space.",
            fmt_money(payroll),
            fmt_money(salary),
            fmt_money(self.money.cap),
            fmt_money((self.mle_amount() - self.team(t).exceptions.mle_used).max(0))
        ))
    }

    /// The human signs a free agent. Explains refusals in plain English.
    pub fn user_sign_free_agent(
        &mut self,
        id: PlayerId,
        years: u8,
        salary: Money,
    ) -> Result<String, String> {
        let t = self.user.team.ok_or("You don't run a team.")?;
        if !self.user_controls_roster(t) {
            return Err("Only a General Manager or Owner can sign players.".into());
        }
        if !matches!(self.p(id).affiliation, Affiliation::FreeAgent) {
            return Err(format!("{} is not a free agent.", self.p(id).name()));
        }
        let (ask, ask_years) = self.player_ask(id);
        let kind = if salary <= self.money.min_salary {
            ContractKind::Minimum
        } else {
            ContractKind::Standard
        };
        let how = self.check_signing(t, salary, kind)?;
        // willingness: money vs. appeal
        let appeal = self.team_appeal(t, id);
        let money_ratio = salary as f64 / ask.max(1) as f64;
        let greed =
            self.p(id).hidden.greed as f64 / 100.0 * self.settings.num("economy.player_greed");
        let score = (money_ratio - 1.0) * (30.0 + 40.0 * greed) + (appeal - 50.0) * 0.55;
        if score < -6.0 {
            let mut msg = format!("{} turns you down. ", self.p(id).name());
            if money_ratio < 0.95 {
                msg += &format!(
                    "He wants about {} per year ({} years). ",
                    fmt_money(ask),
                    ask_years
                );
            }
            if appeal < 45.0 {
                msg += "He isn't excited about the team's direction, role or market.";
            }
            return Err(msg);
        }
        if self.rules.max_contract {
            let max = self
                .content
                .economy
                .max_salary(&self.money, self.p(id).years_pro);
            if salary > max {
                return Err(format!("The maximum salary for him is {}.", fmt_money(max)));
            }
        }
        let used_mle = how.starts_with("mid-level");
        self.sign_player(id, t, salary, years, kind, "free agency");
        if used_mle {
            self.team_mut(t).exceptions.mle_used += salary;
        }
        Ok(format!(
            "{} signs for {} years at {} per year ({}).",
            self.p(id).name(),
            years,
            fmt_money(salary),
            how
        ))
    }

    /// Re-sign one of your expiring players before free agency (uses Bird rights).
    pub fn user_resign(
        &mut self,
        id: PlayerId,
        years: u8,
        salary: Money,
    ) -> Result<String, String> {
        let t = self.user.team.ok_or("You don't run a team.")?;
        if !self.user_controls_roster(t) {
            return Err("Only a General Manager or Owner can negotiate contracts.".into());
        }
        if self.p(id).team_id() != Some(t) {
            return Err("He isn't on your roster.".into());
        }
        if !self.p(id).flags.contains("expiring") {
            return Err("His contract isn't expiring.".into());
        }
        let (ask, ask_years) = self.player_ask(id);
        let mood = self.p(id).mood.overall as f64;
        let loyalty = self.p(id).hidden.loyalty as f64;
        let money_ratio = salary as f64 / ask.max(1) as f64;
        let score = (money_ratio - 1.0) * 55.0 + (mood - 50.0) * 0.5 + (loyalty - 50.0) * 0.1;
        if score < -6.0 {
            return Err(format!("{} wants more: about {} per year for {} years. His mood toward the team is {:.0}/100.", self.p(id).name(), fmt_money(ask), ask_years, mood));
        }
        if self.rules.max_contract
            && salary
                > self
                    .content
                    .economy
                    .max_salary(&self.money, self.p(id).years_pro)
        {
            return Err("That exceeds the maximum salary.".into());
        }
        // The new years are appended after the current (expiring) season and take effect when it ends.
        let new_c = Contract::rising(salary, years, 0.05, ContractKind::BirdRights, self.year + 1);
        let pm = self.pm(id);
        if let Some(cc) = &mut pm.contract {
            cc.salaries.extend(new_c.salaries);
            cc.kind = ContractKind::BirdRights;
        }
        self.pm(id).flags.remove("expiring");
        Ok(format!(
            "{} re-signs: {} years at {} per year.",
            self.p(id).name(),
            years,
            fmt_money(salary)
        ))
    }

    // ------------------------------------------------------------------ AI activity

    pub fn ai_in_season_moves(&mut self, rng: &mut Rng) {
        let freq = self.settings.num("ai.trade_frequency");
        if freq <= 0.0 {
            return;
        }
        if self.trade_deadline_day > 0 && self.day > self.trade_deadline_day as u32 {
            return;
        }
        let ids = self.active_team_ids();
        let attempts = (3.0 * freq) as usize + 1;
        for _ in 0..attempts {
            if !rng.chance(0.35 * freq.min(2.0)) {
                continue;
            }
            let a = ids[rng.range_usize(ids.len())];
            let b = ids[rng.range_usize(ids.len())];
            if a == b || self.user_controls_roster(a) || self.user_controls_roster(b) {
                continue;
            }
            self.try_ai_trade(a, b, rng);
        }
        // AI teams grant trade requests
        let wanters: Vec<PlayerId> = self
            .players
            .iter()
            .filter(|p| p.mood.wants_trade && p.is_active_pro())
            .map(|p| p.id)
            .collect();
        for id in wanters {
            let t = match self.p(id).team_id() {
                Some(t) => t,
                None => continue,
            };
            if self.user_controls_roster(t) || !rng.chance(0.25) {
                continue;
            }
            // find a partner that gives back fair value
            let mut best: Option<(f64, TradeProposal)> = None;
            for &o in &ids {
                if o == t || self.user_controls_roster(o) {
                    continue;
                }
                // ask for their best asset of similar value
                let v = self.trade_value_player(id, Some(o));
                let cand = self
                    .team(o)
                    .roster
                    .iter()
                    .copied()
                    .filter(|&c| {
                        (self.trade_value_player(c, Some(o)) - v * 0.8).abs() < v * 0.5 + 0.5
                    })
                    .max_by(|&x, &y| {
                        self.trade_value_player(x, Some(t))
                            .partial_cmp(&self.trade_value_player(y, Some(t)))
                            .unwrap()
                    });
                if let Some(c) = cand {
                    let tr = TradeProposal {
                        from: t,
                        to: o,
                        give: vec![Asset::Player(id)],
                        get: vec![Asset::Player(c)],
                    };
                    let ev = self.evaluate_trade(&tr);
                    if ev.accepted {
                        let net = self.trade_value_player(c, Some(t))
                            - self.trade_value_player(id, Some(t)) * 0.8;
                        if best.as_ref().map(|b| net > b.0).unwrap_or(true) {
                            best = Some((net, tr));
                        }
                    }
                }
            }
            if let Some((_, tr)) = best {
                self.execute_trade(&tr);
            }
        }
    }

    fn try_ai_trade(&mut self, a: TeamId, b: TeamId, rng: &mut Rng) {
        // a offers a package for one of b's players
        let target = {
            let mut c: Vec<PlayerId> = self.team(b).roster.clone();
            c.sort_by(|&x, &y| self.p(y).ovr.cmp(&self.p(x).ovr));
            if c.is_empty() {
                return;
            }
            let k = rng.range_usize(4.min(c.len()));
            c[k]
        };
        // a pays with a player of similar salary + maybe a pick
        let tv = self.trade_value_player(target, Some(b));
        let mut give: Vec<Asset> = vec![];
        let mut total = 0.0;
        let mut pool: Vec<(f64, Asset)> = self
            .team(a)
            .roster
            .iter()
            .map(|&id| (self.trade_value_player(id, Some(b)), Asset::Player(id)))
            .collect();
        pool.extend(self.team(a).picks.iter().map(|p| {
            (
                self.trade_value_pick(p.year, p.round, p.original, Some(b)),
                Asset::Pick {
                    year: p.year,
                    round: p.round,
                    original: p.original,
                },
            )
        }));
        pool.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        // never trade away the best two players casually
        let protected: Vec<PlayerId> = {
            let mut r = self.team(a).roster.clone();
            r.sort_by(|&x, &y| self.p(y).ovr.cmp(&self.p(x).ovr));
            r.into_iter()
                .take(if self.team(a).direction == Direction::Contend {
                    3
                } else {
                    1
                })
                .collect()
        };
        for (v, asset) in pool {
            if let Asset::Player(id) = &asset {
                if protected.contains(id) || *id == target {
                    continue;
                }
            }
            if total + v <= tv * 1.5 {
                give.push(asset);
                total += v;
            }
            if total >= tv * 1.05 || give.len() >= 3 {
                break;
            }
        }
        if give.is_empty() {
            return;
        }
        let tr = TradeProposal {
            from: a,
            to: b,
            give,
            get: vec![Asset::Player(target)],
        };
        let ev = self.evaluate_trade(&tr);
        if !ev.accepted {
            return;
        }
        // a must also like it: it must gain by its own valuation
        let a_gain: f64 = tr.get.iter().map(|x| self.asset_value(x, Some(a))).sum();
        let a_loss: f64 = tr.give.iter().map(|x| self.asset_value(x, Some(a))).sum();
        if a_gain > a_loss * 1.05 {
            self.execute_trade(&tr);
        }
    }

    // ------------------------------------------------------------------ mood & chemistry

    pub fn update_moods_and_chemistry(&mut self) {
        let year = self.year;
        let avg_rating = self.avg_team_rating();
        let games = self.games_this_season.max(1) as f64;
        for t in self.active_team_ids() {
            let roster = self.team(t).roster.clone();
            let win = self.team(t).record.pct();
            let mut sum_mood = 0.0;
            let mut count = 0.0;
            for &id in &roster {
                let (mpg_share, ovr_rank) = {
                    let mut v = roster.clone();
                    v.sort_by(|&a, &b| self.p(b).ovr.cmp(&self.p(a).ovr));
                    let rank = v.iter().position(|&x| x == id).unwrap_or(5);
                    let st = self
                        .p(id)
                        .seasons
                        .iter()
                        .rev()
                        .find(|r| r.season == year && r.level == Level::Pro);
                    let g = st.map(|r| r.stats.g).unwrap_or(0) as f64;
                    let mpg = st.map(|r| r.stats.mpg()).unwrap_or(0.0);
                    (if g > games * 0.15 { mpg / 34.0 } else { 0.5 }, rank)
                };
                let cap_now = self.money.cap as f64;
                let chem_now = self.teams[t as usize].chemistry;
                let p = &mut self.players[id as usize];
                let expected_share = match ovr_rank {
                    0..=2 => 0.95,
                    3..=4 => 0.8,
                    5..=7 => 0.5,
                    _ => 0.15,
                };
                let pt = (60.0 + (mpg_share - expected_share) * 90.0).clamp(0.0, 100.0);
                let winning =
                    (25.0 + win * 100.0 * 0.9 + p.hidden.winning as f64 * 0.1).clamp(0.0, 100.0);
                let contract = match &p.contract {
                    Some(c) => {
                        let age = (year + 1 - p.birth_year) as f64;
                        let mv = market_value_pct(p.ovr as f64, age, p.potential as f64) * cap_now;
                        (60.0 + (c.salary() as f64 / mv.max(1.0) - 1.0) * 55.0).clamp(0.0, 100.0)
                    }
                    None => 50.0,
                };
                let role = if ovr_rank <= 1 { 75.0 } else { pt };
                let tc = chem_now.clamp(0.0, 100.0);
                let old = p.mood.overall as f64;
                let mut now = 0.30 * pt + 0.25 * winning + 0.2 * contract + 0.1 * role + 0.15 * tc;
                // ego: stars with big egos need more
                if p.hidden.ego > 70 && ovr_rank >= 2 {
                    now -= 8.0;
                }
                // life sim for the human's own player keeps its own mood channel
                let blended = if p.user_controlled {
                    old * 0.8 + now * 0.2
                } else {
                    old * 0.7 + now * 0.3
                };
                p.mood = Mood {
                    overall: blended.clamp(0.0, 100.0) as f32,
                    playing_time: pt as f32,
                    winning: winning as f32,
                    contract: contract as f32,
                    role: role as f32,
                    team_chemistry: tc as f32,
                    wants_trade: p.mood.wants_trade,
                };
                if p.mood.overall < 32.0 && p.ovr >= 55 && !p.user_controlled {
                    p.mood.wants_trade = true;
                } else if p.mood.overall > 55.0 {
                    p.mood.wants_trade = false;
                }
                sum_mood += p.mood.overall as f64;
                count += 1.0;
            }
            let avg_mood = if count > 0.0 { sum_mood / count } else { 55.0 };
            let ego_load = roster
                .iter()
                .filter(|&&id| self.p(id).hidden.ego > 70 && self.p(id).ovr >= 66)
                .count() as f64;
            let target = 40.0 + avg_mood * 0.35 + (win - 0.5) * 20.0
                - (ego_load - 1.0).max(0.0) * 4.0
                + self.coach_of(t).map(|c| c.motivation - 50.0).unwrap_or(0.0) * 0.15
                + (self.team_rating_full_health(t) - avg_rating).clamp(-6.0, 6.0) * 0.5;
            let tm = self.team_mut(t);
            tm.chemistry = (tm.chemistry * 0.85 + target * 0.15).clamp(15.0, 98.0);
        }
    }
}
