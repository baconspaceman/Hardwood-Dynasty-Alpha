//! Storylines and league-wide events. (Data first; narrative engine added below.)

use crate::events::*;

fn c(var: &str, op: &str, value: f64) -> Cond {
    Cond { var: var.into(), op: op.into(), value }
}

#[allow(clippy::too_many_arguments)]
fn lev(id: &str, title: &str, text: &str, chance: f64, min_year: i32, max_year: i32, conds: Vec<Cond>, effects: Vec<Effect>) -> EventDef {
    EventDef {
        id: id.into(), title: title.into(), text: text.into(), category: "league".into(), chance,
        stages: vec!["league".into()], min_year, max_year, conditions: conds, once: min_year == max_year, cooldown: 0, effects, choices: vec![],
    }
}

/// League-wide events. Scheduled history uses chance 1.0 with a fixed year; random ones have a chance per season.
pub fn builtin_league_events() -> Vec<EventDef> {
    let a = Effect::add;
    vec![
        lev("lockout_1998", "Lockout shortens the season", "Owners and players cannot agree on a labor deal. The season is cut to 50 games.", 1.0, 1998, 1998,
            vec![], vec![Effect::set("league.games", 50.0), a("league.revenue_pct", -12.0)]),
        lev("lockout_2011", "Lockout again", "A second lockout costs the league part of the schedule. 66 games are played.", 1.0, 2011, 2011,
            vec![], vec![Effect::set("league.games", 66.0), a("league.revenue_pct", -8.0)]),
        lev("pandemic_2019", "A pandemic halts the sport", "A global pandemic suspends play. The season resumes in a sealed bubble with no fans.", 1.0, 2019, 2019,
            vec![], vec![Effect::set("league.games", 72.0), a("league.revenue_pct", -25.0), a("league.attendance_pct", -90.0)]),
        lev("pandemic_2020", "A second shortened season", "The next season starts late and runs 72 games with limited crowds.", 1.0, 2020, 2020,
            vec![], vec![Effect::set("league.games", 72.0), a("league.revenue_pct", -12.0), a("league.attendance_pct", -45.0)]),
        lev("tv_deal_1995", "Landmark television deal", "A huge new national TV contract is signed.", 1.0, 1995, 1995, vec![], vec![a("league.revenue_pct", 25.0)]),
        lev("tv_deal_2015", "The TV money boom", "A massive new TV deal sends the salary cap soaring.", 1.0, 2015, 2015, vec![], vec![a("league.revenue_pct", 38.0)]),
        lev("recession", "Economic recession", "A recession bites into ticket sales and sponsorship.", 0.015, 1950, 9999, vec![], vec![a("league.revenue_pct", -7.0)]),
        lev("boom", "Business boom", "A strong economy lifts revenue league-wide.", 0.03, 1950, 9999, vec![], vec![a("league.revenue_pct", 5.0)]),
        lev("new_tv_random", "New media rights deal", "A new media rights deal boosts league revenue.", 0.04, 2000, 9999, vec![c("year", ">=", 2000.0)], vec![a("league.revenue_pct", 10.0)]),
        lev("referee_scandal", "Officiating controversy", "A controversy over officiating dominates headlines.", 0.01, 1950, 9999, vec![], vec![a("league.credibility", -3.0)]),
        lev("star_retirement_wave", "Generational changing of the guard", "Several legends announce their retirement in the same offseason.", 0.01, 1950, 9999, vec![], vec![Effect::news("A generation of stars is stepping away.")]),
    ]
}
