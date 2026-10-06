//! The regular season: schedules, building game rosters, simulating days, standings, health.

use crate::game::*;
use crate::injury::{self, InjuryParams};
use crate::league::*;
use crate::player::*;
use crate::rng::Rng;
use crate::team::*;
use crate::types::*;

/// What happened on one simulated day.
#[derive(Clone, Debug, Default)]
pub struct DayReport {
    pub day: u32,
    pub scores: Vec<String>,
    pub news: Vec<String>,
    /// Box score of the user's team's game, if they played.
    pub user_game: Option<BoxScore>,
    pub season_over: bool,
}

impl League {
    /// Run `f` with a temporary copy of the league RNG, then store the advanced state back.
    pub fn with_rng<R>(&mut self, f: impl FnOnce(&mut League, &mut Rng) -> R) -> R {
        let mut rng = self.rng.clone();
        let r = f(self, &mut rng);
        self.rng = rng;
        r
    }

    // ------------------------------------------------------------------ season set-up

    /// Resolve this season's rules, style and money (cap table or simulated cap).
    pub fn refresh_season_context(&mut self) {
        let ry = self.rules_year_for(self.year);
        let rules = self.effective_rules(self.year);
        let cap = match self.cap_history.iter().find(|(y, _)| *y == self.year) {
            Some((_, c)) => *c,
            None => {
                let c = self.content.economy.table_cap(self.year);
                self.cap_history.push((self.year, c));
                c
            }
        };
        let from_table =
            self.settings.text("realism.cap_mode") == "historical" && self.year <= 2025;
        self.money = self.content.economy.season_money(&rules, cap, from_table);
        self.style = self.content.style(ry);
        self.rules = rules;
    }

    /// Called at the start of every season's preseason (and when a league is created).
    pub fn begin_season_prep(&mut self) {
        self.refresh_season_context();
        self.season_mods.clear();
        self.games_this_season = self.rules.games;
        self.apply_league_events();
        let ids = self.active_team_ids();
        for &t in &ids {
            let year = self.year;
            let tm = self.team_mut(t);
            tm.record = Record::default();
            tm.finance = Financials {
                season: year,
                ..Default::default()
            };
            tm.exceptions = Exceptions::default();
        }
        // Season record rows for every pro player.
        for &t in &ids {
            self.open_season_records(t);
        }
        self.with_rng(|l, rng| l.generate_schedule(rng));
        self.with_rng(|l, rng| {
            if !l.colleges.is_empty() {
                l.generate_college_schedule(rng);
            }
            if !l.clubs.is_empty() {
                l.generate_overseas_schedule(rng);
            }
        });
        self.calibrate_engine();
        self.playoffs = None;
        self.day = 0;
        let last_day = self.schedule.iter().map(|f| f.day).max().unwrap_or(150);
        self.trade_deadline_day = (last_day as f64 * 0.68) as u16;
        self.all_star_day = if self.rules.all_star_game {
            (last_day as f64 * 0.55) as u16
        } else {
            0
        };
    }

    pub fn open_season_records(&mut self, t: TeamId) {
        let year = self.year;
        let name = self.team(t).name();
        let ids: Vec<PlayerId> = self.team(t).roster.clone();
        for id in ids {
            self.ensure_record(id, t, &name, year);
        }
    }

    pub fn ensure_record(&mut self, id: PlayerId, t: TeamId, team_name: &str, year: Season) {
        let age = year - self.p(id).birth_year;
        let salary = self.p(id).current_salary();
        let p = self.pm(id);
        let need = !matches!(p.seasons.last(), Some(r) if r.season == year && r.level == Level::Pro && r.team_id == Some(t));
        if need {
            p.seasons.push(SeasonRecord {
                season: year,
                level: Level::Pro,
                team: team_name.to_string(),
                team_id: Some(t),
                age: age.max(0) as u8,
                ovr: p.ovr,
                stats: StatLine::default(),
                playoffs: StatLine::default(),
                salary,
            });
        }
    }

    // ------------------------------------------------------------------ schedule

    pub fn generate_schedule(&mut self, rng: &mut Rng) {
        let ids = self.active_team_ids();
        let n = ids.len();
        let g = self.games_this_season as f64;
        let mut want = vec![vec![0.0f64; n]; n];
        let weight = |a: &Team, b: &Team| -> f64 {
            if a.conf == b.conf && a.div == b.div {
                2.0
            } else if a.conf == b.conf {
                1.3
            } else {
                1.0
            }
        };
        for i in 0..n {
            let mut tot = 0.0;
            for j in 0..n {
                if i != j {
                    tot += weight(self.team(ids[i]), self.team(ids[j]));
                }
            }
            for j in 0..n {
                if i != j {
                    want[i][j] = g * weight(self.team(ids[i]), self.team(ids[j])) / tot;
                }
            }
        }
        // symmetric float matrix then stochastic rounding
        let mut games = vec![vec![0u16; n]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let w = (want[i][j] + want[j][i]) / 2.0;
                let base = w.floor();
                let c = base as u16 + if rng.f64() < w - base { 1 } else { 0 };
                games[i][j] = c;
                games[j][i] = c;
            }
        }
        // repair per-team totals toward g
        for _ in 0..(n * 8) {
            let deg: Vec<i32> = (0..n)
                .map(|i| games[i].iter().map(|&x| x as i32).sum())
                .collect();
            let gi = g as i32;
            let over: Vec<usize> = (0..n).filter(|&i| deg[i] > gi).collect();
            let under: Vec<usize> = (0..n).filter(|&i| deg[i] < gi).collect();
            if over.is_empty() && under.is_empty() {
                break;
            }
            if !over.is_empty() {
                // remove a game between two over teams, else between over team and its most-played opponent
                let i = over[rng.range_usize(over.len())];
                let cand: Vec<usize> = (0..n).filter(|&j| j != i && games[i][j] > 0).collect();
                if let Some(&j) = cand
                    .iter()
                    .max_by_key(|&&j| (over.contains(&j) as i32, games[i][j]))
                {
                    games[i][j] -= 1;
                    games[j][i] -= 1;
                }
            } else {
                let i = under[rng.range_usize(under.len())];
                let cand: Vec<usize> = (0..n).filter(|&j| j != i && deg[j] < gi).collect();
                let j = if let Some(&j) = cand.get(rng.range_usize(cand.len().max(1))) {
                    j
                } else {
                    (i + 1 + rng.range_usize(n - 1)) % n
                };
                if j != i {
                    games[i][j] += 1;
                    games[j][i] += 1;
                }
            }
        }
        // fixtures with home/away balance
        let mut fixtures: Vec<(TeamId, TeamId)> = vec![];
        for i in 0..n {
            for j in (i + 1)..n {
                let c = games[i][j];
                let mut home_i = c / 2;
                if c % 2 == 1 && rng.chance(0.5) {
                    home_i += 1;
                }
                for k in 0..c {
                    if k < home_i {
                        fixtures.push((ids[i], ids[j]));
                    } else {
                        fixtures.push((ids[j], ids[i]));
                    }
                }
            }
        }
        rng.shuffle(&mut fixtures);
        // assign days greedily
        let mut sched: Vec<Fixture> = Vec::with_capacity(fixtures.len());
        let mut remaining = fixtures;
        let mut day: u16 = 1;
        let max_teams = self.teams.len();
        while !remaining.is_empty() && day < 400 {
            let mut busy = vec![false; max_teams];
            let mut next: Vec<(TeamId, TeamId)> = Vec::with_capacity(remaining.len());
            // Real calendars have rest days: only about 55% of teams play on a given date.
            let max_per_day = ((n as f64) * 0.29).ceil() as usize;
            let mut today = 0usize;
            for (h, a) in remaining {
                if today < max_per_day && !busy[h as usize] && !busy[a as usize] {
                    today += 1;
                    busy[h as usize] = true;
                    busy[a as usize] = true;
                    sched.push(Fixture {
                        day,
                        home: h,
                        away: a,
                        done: false,
                        home_pts: 0,
                        away_pts: 0,
                        overtimes: 0,
                        tag: String::new(),
                    });
                } else {
                    next.push((h, a));
                }
            }
            remaining = next;
            day += 1;
        }
        self.schedule = sched;
    }

    // ------------------------------------------------------------------ building game rosters

    pub fn injury_params(&self, t: TeamId) -> InjuryParams {
        InjuryParams {
            frequency: self.settings.num("injuries.frequency"),
            severity: self.settings.num("injuries.severity"),
            career_ending: self.settings.num("injuries.career_ending"),
            permanent: self.settings.num("injuries.permanent_damage"),
            in_game: self.settings.bool("injuries.in_game"),
            medical: self.medical_quality(t),
            year: self.year,
        }
    }

    pub fn game_context(&self, playoffs: bool) -> GameContext {
        let s = &self.settings;
        GameContext {
            refs: self.refs,
            cal: self.cal,
            season: self.year,
            rules: self.rules.clone(),
            style: self.style,
            tune: Tune::from_map(&self.content.tuning),
            home_court: s.num("sim.home_court"),
            randomness: s.num("sim.randomness"),
            star_power: s.num("sim.star_power"),
            fatigue: s.num("sim.fatigue"),
            foul_rate: s.num("sim.foul_rate"),
            hot_hand: s.bool("sim.hot_hand"),
            clutch: s.bool("sim.clutch"),
            coach_adjust: s.bool("sim.coach_in_game"),
            possession_detail: s.num("sim.possession_detail"),
            in_game_injuries: s.bool("injuries.in_game"),
            playoffs,
            neutral_site: false,
            play_by_play: s.bool("sim.play_by_play"),
        }
    }

    /// Who plays for a team tonight, with minutes and starters.
    pub fn build_game_team(
        &self,
        t: TeamId,
        playoffs: bool,
        rng: Option<&mut Rng>,
        in_game_injuries: bool,
    ) -> GameTeam {
        let team = self.team(t);
        let params = self.injury_params(t);
        let mut avail: Vec<PlayerId> = vec![];
        let mut hurt: Vec<PlayerId> = vec![];
        for &id in &team.roster {
            let p = self.p(id);
            match &p.injury {
                None => avail.push(id),
                Some(i) if i.playing_through => avail.push(id),
                Some(_) => hurt.push(id),
            }
        }
        // Load management: rest tired veteran stars sometimes (never in the playoffs).
        if let Some(rng) = rng {
            if !playoffs && self.settings.bool("injuries.load_management") && avail.len() > 9 {
                let mut rest = vec![];
                for &id in &avail {
                    let p = self.p(id);
                    let age = self.year - p.birth_year;
                    if p.ovr >= 72
                        && (age >= 31 || p.wear > 35.0 || p.fitness < 55.0)
                        && rng.chance(0.045 + (60.0 - p.fitness as f64).max(0.0) / 400.0)
                    {
                        rest.push(id);
                    }
                }
                avail.retain(|id| !rest.contains(id));
            }
        }
        // Emergency: not enough bodies, play hurt players.
        if avail.len() < 7 {
            hurt.sort_by_key(|&id| {
                self.p(id)
                    .injury
                    .as_ref()
                    .map(|i| i.games_remaining)
                    .unwrap_or(0)
            });
            for id in hurt {
                if avail.len() >= 8 {
                    break;
                }
                avail.push(id);
            }
        }
        avail.sort_by(|&a, &b| self.p(b).ovr.cmp(&self.p(a).ovr));
        avail.truncate(self.rules.active_max as usize);
        let mut players: Vec<GamePlayer> = avail
            .iter()
            .map(|&id| {
                let p = self.p(id);
                let mut perf = 1.0
                    + (p.mood.overall as f64 - 60.0) / 2500.0
                    + (team.chemistry - 55.0) / 2500.0;
                if let Some(l) = &p.life {
                    if self.settings.bool("life.mental_health") {
                        perf *= l.performance_mod();
                    }
                }
                let risk = if in_game_injuries {
                    injury::BASE_PER_100_MIN
                        * injury::risk_multiplier(p, self.year, &params)
                        * params.frequency
                } else {
                    0.0
                };
                let mut gp = GamePlayer::from_player(p, &self.content.badges, perf, risk);
                if let Some(m) = team.minutes_override.get(&id) {
                    gp.target_min = *m;
                }
                gp
            })
            .collect();
        assign_rotation(&mut players, playoffs, &self.rules);
        if !team.starters_override.is_empty() {
            let wanted: Vec<PlayerId> = team
                .starters_override
                .iter()
                .copied()
                .filter(|id| players.iter().any(|p| p.id == *id))
                .collect();
            if wanted.len() == 5 {
                for p in players.iter_mut() {
                    p.starter = wanted.contains(&p.id);
                    if p.starter && p.target_min < 18.0 {
                        p.target_min = 24.0;
                    }
                }
            }
        }
        let mut strat = team.strategy.clone();
        if let Some(hc) = team.head_coach {
            strat.tactics = self.person(hc).tactics + (team.chemistry - 55.0) * 0.3;
        }
        GameTeam {
            id: t,
            name: team.name(),
            players,
            strategy: strat,
        }
    }

    /// Run the engine's self-calibration with the current rosters.
    pub fn calibrate_engine(&mut self) {
        let ids = self.active_team_ids();
        if ids.len() < 2 {
            return;
        }
        let mut rng = Rng::from_label(self.seed, &format!("cal-{}", self.year));
        let teams: Vec<GameTeam> = ids
            .iter()
            .map(|&t| self.build_game_team(t, false, None, false))
            .collect();
        let refs = Refs::from_players(teams.iter().flat_map(|t| t.players.iter()));
        let mut ctx = self.game_context(false);
        ctx.refs = refs;
        ctx.in_game_injuries = false;
        ctx.play_by_play = false;
        ctx.cal = self.cal;
        let (games, rounds) = if self.cal.pace_mul == 1.0 && self.cal.two_off == 0.0 {
            (140, 3)
        } else {
            (90, 2)
        };
        calibrate(&mut ctx, &teams, &mut rng, games, rounds);
        self.refs = refs;
        self.cal = ctx.cal;
    }

    // ------------------------------------------------------------------ simulating days

    /// Simulate the next day of the regular season.
    pub fn sim_regular_season_day(&mut self) -> DayReport {
        let day = (self.day + 1) as u16;
        let mut report = DayReport {
            day: day as u32,
            ..Default::default()
        };
        let idxs: Vec<usize> = self
            .schedule
            .iter()
            .enumerate()
            .filter(|(_, f)| f.day == day && !f.done)
            .map(|(i, _)| i)
            .collect();
        let mut played: Vec<bool> = vec![false; self.teams.len()];
        let ctx = self.game_context(false);
        let mut rng = self.rng.clone();
        for i in idxs {
            let (h, a) = (self.schedule[i].home, self.schedule[i].away);
            if !self.team(h).active || !self.team(a).active {
                self.schedule[i].done = true;
                continue;
            }
            played[h as usize] = true;
            played[a as usize] = true;
            let bs = self.play_game(&ctx, h, a, false, &mut rng);
            self.schedule[i].done = true;
            self.schedule[i].home_pts = bs.home.pts;
            self.schedule[i].away_pts = bs.away.pts;
            self.schedule[i].overtimes = bs.overtimes;
            let line = format!(
                "{} {} - {} {}{}",
                self.team(a).abbr,
                bs.away.pts,
                bs.home.pts,
                self.team(h).abbr,
                if bs.overtimes > 0 {
                    format!(" ({}OT)", bs.overtimes)
                } else {
                    String::new()
                }
            );
            report.scores.push(line);
            if Some(h) == self.user.team || Some(a) == self.user.team {
                report.user_game = Some(bs.clone());
            }
            // keep a window of recent box scores
            self.box_log.push(bs);
            if self.box_log.len() > 40 {
                self.box_log.remove(0);
            }
        }
        self.after_day_health(&played, false, &mut rng);
        self.rng = rng;
        self.day = day as u32;
        self.day_hooks(&mut report);
        let remaining = self.schedule.iter().any(|f| !f.done);
        if !remaining {
            report.season_over = true;
        }
        report
    }

    /// Weekly/daily hooks that live inside the season: all-star game, AI trades, finances, stories.
    fn day_hooks(&mut self, report: &mut DayReport) {
        let day = self.day;
        if self.rules.all_star_game && self.all_star_day > 0 && day == self.all_star_day as u32 {
            if let Some(line) = self.play_all_star_game() {
                report.news.push(line);
            }
        }
        // weekly: AI trades, mood & story updates
        if day % 7 == 0 {
            self.with_rng(|l, rng| l.ai_in_season_moves(rng));
            self.update_moods_and_chemistry();
            self.story_weekly();
        }
        if day % 30 == 0 {
            self.monthly_life_tick();
        }
        // Re-tune the engine once the season's real health and fatigue patterns are in place.
        if day == 20 || day == 60 {
            self.calibrate_engine();
        }
    }

    /// Simulate one game between two teams and apply everything that follows from it.
    pub fn play_game(
        &mut self,
        ctx: &GameContext,
        h: TeamId,
        a: TeamId,
        playoffs: bool,
        rng: &mut Rng,
    ) -> BoxScore {
        let mut c = ctx.clone();
        c.playoffs = playoffs;
        let mut ht = self.build_game_team(h, playoffs, Some(rng), c.in_game_injuries);
        let mut at = self.build_game_team(a, playoffs, Some(rng), c.in_game_injuries);
        if playoffs {
            // AI coaches play through minor injuries in the playoffs.
            self.playoff_play_through(h, rng);
            self.playoff_play_through(a, rng);
            ht = self.build_game_team(h, playoffs, Some(rng), c.in_game_injuries);
            at = self.build_game_team(a, playoffs, Some(rng), c.in_game_injuries);
        }
        let bs = simulate_game(&c, &ht, &at, rng);
        self.apply_box(&bs, h, a, playoffs, rng);
        bs
    }

    fn playoff_play_through(&mut self, t: TeamId, rng: &mut Rng) {
        if !self.settings.bool("injuries.rush_back") {
            return;
        }
        let ids = self.team(t).roster.clone();
        for id in ids {
            let (ok, star) = {
                let p = self.p(id);
                match &p.injury {
                    Some(i) if !i.playing_through && !i.career_ending => (
                        i.games_remaining <= 3 && i.severity <= injury::Severity::Moderate,
                        p.ovr >= 72,
                    ),
                    _ => (false, false),
                }
            };
            if ok && (star || rng.chance(0.5)) {
                if let Some(i) = &mut self.pm(id).injury {
                    i.playing_through = true;
                }
            }
        }
    }

    /// Apply a finished game to records, stats, injuries, fitness and finances.
    pub fn apply_box(
        &mut self,
        bs: &BoxScore,
        h: TeamId,
        a: TeamId,
        playoffs: bool,
        rng: &mut Rng,
    ) {
        let year = self.year;
        let home_won = bs.home.pts > bs.away.pts;
        if !playoffs {
            self.update_record(h, bs.home.pts, bs.away.pts, true, a);
            self.update_record(a, bs.away.pts, bs.home.pts, false, h);
        }
        for (tid, tb) in [(h, &bs.home), (a, &bs.away)] {
            let tname = self.team(tid).name();
            for pb in &tb.players {
                if pb.stats.g == 0 {
                    continue;
                }
                self.ensure_record(pb.id, tid, &tname, year);
                let p = self.pm(pb.id);
                if let Some(r) = p.seasons.last_mut() {
                    if playoffs {
                        r.playoffs.add(&pb.stats);
                    } else {
                        r.stats.add(&pb.stats);
                    }
                }
                // fitness cost
                p.fitness = (p.fitness - (pb.stats.min as f32) * 0.30).max(10.0);
                // played-through injury may get worse
                if let Some(i) = &mut p.injury {
                    if i.playing_through && rng.chance(0.05) {
                        i.games_remaining =
                            i.games_remaining.saturating_add(rng.range(3, 25) as u16);
                        i.games_total = i.games_total.saturating_add(10);
                    }
                }
                if let Some(c) = p.custom.get_mut("ramp_games") {
                    *c = (*c - 1.0).max(0.0);
                }
            }
            // injuries that happened in the game
            for pb in &tb.players {
                if pb.injured_in_game {
                    self.new_injury(pb.id, true, rng);
                }
            }
        }
        // revenue for the home team
        self.finance_home_game(h, playoffs, home_won);
        // records & headlines
        let heads = self.records.check_game(bs, year, playoffs);
        for hline in heads {
            self.add_news("record", hline);
        }
        for tb in [&bs.home, &bs.away] {
            for pb in &tb.players {
                let s = &pb.stats;
                if s.pts >= 50
                    || (s.pts >= 20 && s.reb() >= 20)
                    || (s.pts >= 10 && s.reb() >= 10 && s.ast >= 10 && (s.stl >= 5 || s.blk >= 5))
                {
                    let msg = format!(
                        "{} ({}) puts up {} pts, {} reb, {} ast.",
                        pb.name,
                        tb.name,
                        s.pts,
                        s.reb(),
                        s.ast
                    );
                    self.add_news("game", msg);
                }
            }
        }
    }

    fn update_record(&mut self, t: TeamId, pf: u16, pa: u16, home: bool, opp: TeamId) {
        let same_conf = self.team(t).conf == self.team(opp).conf;
        let r = &mut self.team_mut(t).record;
        let win = pf > pa;
        if win {
            r.w += 1;
            if home {
                r.home_w += 1;
            }
            if same_conf {
                r.conf_w += 1;
            }
            r.streak = if r.streak > 0 { r.streak + 1 } else { 1 };
        } else {
            r.l += 1;
            if home {
                r.home_l += 1;
            }
            if same_conf {
                r.conf_l += 1;
            }
            r.streak = if r.streak < 0 { r.streak - 1 } else { -1 };
        }
        r.pf += pf as u32;
        r.pa += pa as u32;
        r.last10.push(win);
        if r.last10.len() > 10 {
            r.last10.remove(0);
        }
    }

    /// Create an injury for a player (known to have happened), and handle career-ending ones.
    pub fn new_injury(&mut self, id: PlayerId, in_game: bool, rng: &mut Rng) {
        let (tid, params) = {
            let p = self.p(id);
            let t = p.team_id().unwrap_or(0);
            (p.team_id(), self.injury_params(t))
        };
        let inj = injury::random_injury(
            rng,
            &self.content.injuries,
            self.p(id),
            self.year,
            in_game,
            &params,
        );
        let ce = inj.career_ending;
        let name = self.p(id).name();
        let desc = injury::describe(&inj, false);
        let ovr = self.p(id).ovr;
        let star = ovr >= 70;
        injury::start_injury(self.pm(id), inj);
        let tname = tid.map(|t| self.team(t).name()).unwrap_or_default();
        if ce {
            self.retire_player(id, "a career-ending injury");
            self.add_news(
                "major",
                format!(
                    "CAREER OVER: {name} ({tname}) is forced to retire after a devastating injury."
                ),
            );
            self.on_career_ending_injury(id);
        } else if star || ovr >= 62 {
            self.add_news("injury", format!("{name} ({tname}) injured: {desc}."));
        }
        if let Some(t) = tid {
            if self.is_user_team(t) {
                let inj_desc = self
                    .p(id)
                    .injury
                    .as_ref()
                    .map(|i| injury::describe(i, self.settings.bool("injuries.hide_details")))
                    .unwrap_or_default();
                self.add_news("user", format!("INJURY: {name} - {inj_desc}"));
            }
        }
    }

    /// Daily health: healing, off-day injuries, fitness recovery.
    pub(crate) fn after_day_health(&mut self, played: &[bool], playoffs: bool, rng: &mut Rng) {
        let ids = self.active_team_ids();
        for t in ids {
            let plays = played.get(t as usize).copied().unwrap_or(false);
            let medical = self.medical_quality(t);
            let params = self.injury_params(t);
            let roster = self.team(t).roster.clone();
            for id in roster {
                // heal
                let heal_today =
                    plays || (!playoffs && rng.chance(0.42)) || (playoffs && rng.chance(0.6));
                if self.p(id).injury.is_some() && heal_today {
                    let rec = injury::heal_one_game(self.pm(id), medical, rng);
                    if let Some(r) = rec {
                        let p = self.p(id);
                        if p.ovr >= 68 || r.games_missed > 30 {
                            let msg = format!(
                                "{} returns from {} after {} games.",
                                p.name(),
                                r.name.to_lowercase(),
                                r.games_missed
                            );
                            let prominent = r.games_missed > 40;
                            let nm = p.name();
                            self.add_news("injury", msg);
                            if prominent {
                                self.pm(id).flags.insert("recent_return".into());
                                self.story_comeback(id, r.games_missed);
                            }
                            let _ = nm;
                        }
                    }
                } else if !plays {
                    let f = &mut self.pm(id).fitness;
                    *f = (*f + 6.0).min(100.0);
                }
                // new off-day injury
                if self.p(id).injury.is_none() {
                    if let Some(inj) = injury::roll_injury(
                        rng,
                        &self.content.injuries,
                        self.p(id),
                        self.year,
                        1.0,
                        false,
                        &params,
                    ) {
                        let ce = inj.career_ending;
                        let name = self.p(id).name();
                        let desc = injury::describe(&inj, false);
                        let ovr = self.p(id).ovr;
                        injury::start_injury(self.pm(id), inj);
                        if ce {
                            self.retire_player(id, "a career-ending injury");
                            self.add_news("major", format!("CAREER OVER: {name} is forced to retire after a devastating injury."));
                            self.on_career_ending_injury(id);
                        } else if ovr >= 66 {
                            self.add_news(
                                "injury",
                                format!("{name} injured away from games: {desc}."),
                            );
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ all-star

    fn play_all_star_game(&mut self) -> Option<String> {
        let def = self
            .content
            .awards
            .iter()
            .find(|a| a.id == "all_star")?
            .clone();
        let cands = self.award_candidates(false);
        let mut rng = Rng::from_label(self.seed, &format!("allstar-{}", self.year));
        let res = crate::awards::decide(
            &def,
            &cands,
            (self.day.max(1)) as u16 * 1,
            self.year,
            &mut rng,
        )?;
        let mut east: Vec<PlayerId> = vec![];
        let mut west: Vec<PlayerId> = vec![];
        for (id, _) in &res.winners {
            let t = self.p(*id).team_id();
            if let Some(t) = t {
                if self.team(t).conf == 0 {
                    east.push(*id);
                } else {
                    west.push(*id);
                }
                let year = self.year;
                let tn = self.team(t).name();
                self.pm(*id).awards.push(AwardRecord {
                    season: year,
                    award: "All-Star".into(),
                    team: tn,
                });
            }
        }
        east.truncate(12);
        west.truncate(12);
        if east.len() < 5 || west.len() < 5 {
            return None;
        }
        let mk = |l: &League, ids: &[PlayerId], name: &str, tid: TeamId| {
            let mut players: Vec<GamePlayer> = ids
                .iter()
                .map(|&id| GamePlayer::from_player(l.p(id), &l.content.badges, 1.0, 0.0))
                .collect();
            assign_rotation(&mut players, false, &l.rules);
            GameTeam {
                id: tid,
                name: name.to_string(),
                players,
                strategy: Strategy {
                    tempo: 0.5,
                    three_emphasis: 0.4,
                    tactics: 50.0,
                    ..Default::default()
                },
            }
        };
        let e = mk(self, &east, "East All-Stars", 0);
        let w = mk(self, &west, "West All-Stars", 1);
        let mut ctx = self.game_context(false);
        ctx.neutral_site = true;
        ctx.in_game_injuries = false;
        ctx.play_by_play = false;
        let bs = simulate_game(&ctx, &e, &w, &mut rng);
        let mvp = bs
            .home
            .players
            .iter()
            .chain(bs.away.players.iter())
            .max_by_key(|p| p.stats.pts + p.stats.reb() + p.stats.ast)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let line = format!(
            "ALL-STAR GAME: East {} - West {}. MVP: {}",
            bs.home.pts, bs.away.pts, mvp
        );
        self.add_news("major", line.clone());
        Some(line)
    }
}
