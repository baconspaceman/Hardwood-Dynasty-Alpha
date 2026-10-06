//! The calendar: advancing time one step, one week, one season or many years.
//!
//! The simulation is a state machine over `Phase`:
//! Preseason → RegularSeason → (PlayIn →) Playoffs → PostSeason → Draft → FreeAgency → Preseason...
//! Time stops whenever the human needs to decide something (a draft pick, an expiring contract,
//! a life event, an illegal roster) and tells them exactly what to do next.

use crate::league::*;
use crate::offseason::FA_DAYS;
use crate::season::DayReport;
use crate::types::*;

#[derive(Clone, Debug)]
pub enum Goal {
    /// One calendar step (a day in season, a phase otherwise).
    Step,
    Days(u32),
    /// Until a phase is reached.
    UntilPhase(Phase),
    /// All the way to the next preseason (a full season including the offseason).
    EndOfSeason,
    Years(u32),
}

#[derive(Clone, Debug, Default)]
pub struct RunReport {
    pub steps: u32,
    pub lines: Vec<String>,
    /// Why the simulation stopped early, if it did.
    pub stopped: Option<String>,
}

impl League {
    /// Pending decisions that must be answered before time can move.
    pub fn blocking_decisions(&self) -> Vec<&Decision> {
        self.decisions.iter().collect()
    }

    /// Perform one calendar step.
    pub fn step(&mut self) -> Result<Vec<String>, String> {
        // Life events for the human's player block time.
        if let Some(pid) = self.user.player {
            if let Some(l) = &self.p(pid).life {
                if !l.pending.is_empty() && !self.user.auto_decisions {
                    return Err(format!(
                        "You have {} life event(s) to resolve first (see: events).",
                        l.pending.len()
                    ));
                }
            }
        }
        if !self.decisions.is_empty() && !self.user.auto_decisions {
            return Err(format!(
                "You have {} pending decision(s) (see: decisions).",
                self.decisions.len()
            ));
        } else if self.user.auto_decisions {
            self.auto_resolve_decisions();
        }
        let mut notes: Vec<String> = vec![];
        match self.phase {
            Phase::Preseason => {
                self.finish_preseason()?;
                self.phase = Phase::RegularSeason;
                notes.push(format!("Opening night of the {} season!", self.year));
            }
            Phase::RegularSeason => {
                let report = self.sim_regular_season_day();
                self.after_day(&report, &mut notes);
                if report.season_over {
                    self.end_regular_season();
                    notes.push("The regular season is over.".into());
                    if let Some(l) = self.regular_season_summary_line() {
                        notes.push(l);
                    }
                }
            }
            Phase::PlayIn | Phase::Playoffs => {
                let report = self.sim_playoff_day();
                self.after_day(&report, &mut notes);
                if report.season_over || self.playoffs_finished() {
                    self.finish_playoffs();
                    if let Some(h) = self.history.last() {
                        notes.push(format!(
                            "{} champions: {}. Finals MVP: {}.",
                            h.season, h.champion, h.finals_mvp
                        ));
                    }
                }
            }
            Phase::PostSeason => {
                self.finish_college_season();
                self.finish_overseas_season();
                self.run_postseason();
                notes.push("The offseason begins: players age, retirements are announced, the draft order is set.".into());
                self.after_postseason_life();
            }
            Phase::Draft => {
                let done = self.draft.as_ref().map(|d| d.done).unwrap_or(true);
                if !done {
                    let finished = self.run_draft(true);
                    if !finished {
                        let pick = self
                            .draft
                            .as_ref()
                            .and_then(|d| d.order.get(d.next))
                            .map(|s| s.overall)
                            .unwrap_or(0);
                        return Err(format!("You are on the clock with pick #{pick}! Use: draftboard, then: pick <player> (or: autopick)."));
                    }
                    self.finish_draft();
                    notes.push("The draft is complete.".into());
                }
                // resign window warning
                if self
                    .user
                    .team
                    .map(|u| self.user_controls_roster(u))
                    .unwrap_or(false)
                    && !self.user.auto_decisions
                {
                    let u = self.user.team.unwrap();
                    let exp: Vec<String> = self
                        .team(u)
                        .roster
                        .iter()
                        .filter(|&&id| self.p(id).flags.contains("expiring"))
                        .map(|&id| self.p(id).name())
                        .collect();
                    if !exp.is_empty() && !self.season_mods.contains_key("resign_warned") {
                        self.season_mods.insert("resign_warned".into(), 1.0);
                        return Err(format!("Expiring contracts: {}. Re-sign them with 'resign <player>' or advance again to let them test free agency.", exp.join(", ")));
                    }
                }
                self.start_free_agency();
                notes.push("Free agency is open.".into());
            }
            Phase::FreeAgency => {
                self.fa_day();
                if self.fa_day_index % 10 == 0 {
                    self.monthly_life_tick();
                }
                if self.fa_day_index >= FA_DAYS {
                    self.start_next_season();
                    notes.push(format!("{} season preseason begins.", self.year));
                }
            }
        }
        Ok(notes)
    }

    fn after_day(&mut self, report: &DayReport, notes: &mut Vec<String>) {
        // college / overseas keep pace with the calendar
        self.sim_college_day(report.day);
        self.sim_overseas_day(report.day);
        if let Some(bs) = &report.user_game {
            let ut = self.user.team;
            if let Some(u) = ut {
                let (hn, an) = (bs.home.name.clone(), bs.away.name.clone());
                let won = (bs.home.team == u && bs.home.pts > bs.away.pts)
                    || (bs.away.team == u && bs.away.pts > bs.home.pts);
                notes.push(format!(
                    "{} {} {} {} {}",
                    if won { "W" } else { "L" },
                    an,
                    bs.away.pts,
                    bs.home.pts,
                    hn
                ));
            }
        }
        for n in &report.news {
            notes.push(n.clone());
        }
        // user's player progress (college/overseas/HS games)
        self.user_player_day_hook(report.day, notes);
    }

    fn regular_season_summary_line(&self) -> Option<String> {
        let t = self.user.team?;
        let r = &self.team(t).record;
        let order = self.standings(None);
        let rank = order.iter().position(|&x| x == t)? + 1;
        Some(format!(
            "Your team finished {}-{} (league rank {} of {}).",
            r.w,
            r.l,
            rank,
            order.len()
        ))
    }

    fn auto_resolve_decisions(&mut self) {
        let decs: Vec<Decision> = self.decisions.clone();
        for d in decs {
            // default to the first option; if it can't be applied, try the others, else drop the decision
            let mut done = false;
            // A retirement choice: veterans eventually hang it up.
            if d.kind == "retire" {
                if let Some(pid) = d.subject {
                    if self.age_of(pid) + 1 >= 37
                        && d.options.iter().any(|o| o.id == "retire")
                        && self.resolve_decision(d.id, "retire", true).is_ok()
                    {
                        continue;
                    }
                }
            }
            for o in &d.options {
                if self.resolve_decision(d.id, &o.id.clone(), true).is_ok() {
                    done = true;
                    break;
                }
            }
            if !done {
                self.decisions.retain(|x| x.id != d.id);
            }
        }
    }

    /// Run until a goal is reached (or the simulation needs you).
    pub fn advance(&mut self, goal: Goal) -> RunReport {
        let mut rep = RunReport::default();
        let start_year = self.year;
        let mut days_done = 0u32;
        let max_steps = 20_000u32;
        loop {
            // goal checks before stepping
            match &goal {
                Goal::UntilPhase(p) if self.phase == *p && rep.steps > 0 => break,
                Goal::EndOfSeason if self.year > start_year && self.phase == Phase::Preseason => {
                    break
                }
                Goal::Years(n)
                    if self.year >= start_year + *n as i32 && self.phase == Phase::Preseason =>
                {
                    break
                }
                Goal::Days(n) if days_done >= *n => break,
                Goal::Step if rep.steps >= 1 => break,
                _ => {}
            }
            if rep.steps >= max_steps {
                rep.stopped = Some("Stopped after a very long run; run again to continue.".into());
                break;
            }
            let was_day_phase = matches!(
                self.phase,
                Phase::RegularSeason | Phase::PlayIn | Phase::Playoffs | Phase::FreeAgency
            );
            match self.step() {
                Ok(notes) => {
                    rep.steps += 1;
                    if was_day_phase {
                        days_done += 1;
                    }
                    rep.lines.extend(notes);
                }
                Err(e) => {
                    rep.stopped = Some(e);
                    break;
                }
            }
            if rep.lines.len() > 400 {
                let keep = rep.lines.len() - 300;
                rep.lines.drain(0..keep);
            }
        }
        rep
    }

    pub fn date_string(&self) -> String {
        match self.phase {
            Phase::RegularSeason => format!(
                "{}-{:02} {} - day {} of ~{}",
                self.year,
                (self.year + 1) % 100,
                self.phase.name(),
                self.day,
                self.schedule.iter().map(|f| f.day).max().unwrap_or(0)
            ),
            Phase::FreeAgency => format!(
                "{} {} - day {} of {}",
                self.year,
                self.phase.name(),
                self.fa_day_index,
                FA_DAYS
            ),
            _ => format!(
                "{}-{:02} {}",
                self.year,
                (self.year + 1) % 100,
                self.phase.name()
            ),
        }
    }
}
