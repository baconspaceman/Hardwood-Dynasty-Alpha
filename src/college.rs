//! College basketball: programs, coaches, recruiting, a season, and a national tournament.
//!
//! The college world is fictional (names and conferences) but behaves like the real thing:
//! programs have prestige, facilities and coaches; high-school seniors are ranked and recruited;
//! players move through four class years (freshman to senior); the best declare for the draft
//! (with era-appropriate eligibility rules); and a tournament crowns a champion.
//!
//! The same model is used whether you are watching from the pro side, scouting, playing as a
//! created player, or running a program as a head coach or athletic director.

use crate::contract::*;
use crate::fastsim::*;
use crate::generate::*;
use crate::league::*;
use crate::names;
use crate::player::*;
use crate::rng::Rng;
use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoachInfo {
    pub name: String,
    pub born: i32,
    pub recruiting: f64,
    pub development: f64,
    pub tactics: f64,
    pub offense: f64,
    pub defense: f64,
    pub reputation: f64,
    pub wins: u32,
    pub losses: u32,
    pub titles: u16,
    pub years: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollegeSeason {
    pub season: Season,
    pub w: u16,
    pub l: u16,
    pub result: String,
    pub coach: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct College {
    pub id: CollegeId,
    pub name: String,
    pub nickname: String,
    pub conf: u8,
    /// 0-100 how attractive the program is to recruits.
    pub prestige: f64,
    pub facilities: f64,
    /// Fan base / market 0-1.
    pub market: f64,
    pub roster: Vec<PlayerId>,
    pub coach: CoachInfo,
    /// Annual NIL / booster collective budget (modern era).
    pub nil_budget: Money,
    pub nil_spent: Money,
    pub record: crate::team::Record,
    pub history: Vec<CollegeSeason>,
    pub titles: Vec<Season>,
    /// Athletic director's patience 0-100.
    pub ad_patience: f64,
    pub starters: Vec<PlayerId>,
    pub tactics_tempo: f64,
    pub tactics_three: f64,
    /// Recruiting points to spend on the user's recruiting board this offseason.
    pub recruiting_points: f64,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CFixture {
    pub day: u16,
    pub home: CollegeId,
    pub away: CollegeId,
    pub done: bool,
    pub home_pts: u16,
    pub away_pts: u16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CollegeState {
    pub schedule: Vec<CFixture>,
    pub field: Vec<CollegeId>,
    /// Current tournament round: pairs still to play, by tournament day.
    pub pending: Vec<(CollegeId, CollegeId)>,
    pub winners: Vec<CollegeId>,
    pub round: u8,
    pub champion: Option<CollegeId>,
    pub done: bool,
    pub tournament_started: bool,
    pub last_champion_text: String,
}

pub fn era_college_games(year: Season) -> u16 {
    crate::era::interp(
        &[(1946, 22.0), (1975, 27.0), (2000, 31.0), (2024, 33.0)],
        year as f64,
    ) as u16
}

pub fn tournament_size(year: Season) -> usize {
    match year {
        ..=1950 => 8,
        1951..=1974 => 16,
        1975..=1978 => 32,
        1979..=1984 => 48,
        _ => 64,
    }
}

pub fn class_of(p: &Player) -> u8 {
    p.custom.get("class").copied().unwrap_or(1.0) as u8
}

pub fn set_class(p: &mut Player, c: u8) {
    p.custom.insert("class".into(), c as f64);
}

impl League {
    // ------------------------------------------------------------------ creation

    pub fn init_amateur_world(&mut self, rng: &mut Rng) {
        let n_setting = self.settings.num("college.programs") as usize;
        let era_factor =
            crate::era::interp(&[(1946, 0.55), (1970, 0.8), (1985, 1.0)], self.year as f64);
        let n = ((n_setting as f64 * era_factor).round() as usize).clamp(8, 200);
        self.create_colleges(rng, n);
        self.fill_college_rosters(rng);
        self.generate_hs_class(rng, 12);
        self.generate_hs_class(rng, 11);
        self.recruit_to_open_slots(rng);
        self.create_clubs(rng);
        self.fill_club_rosters(rng);
    }

    fn create_colleges(&mut self, rng: &mut Rng, n: usize) {
        let mut base = names::college_names();
        rng.shuffle(&mut base);
        let dirs = [
            "North", "South", "East", "West", "Central", "Upper", "Lower", "New",
        ];
        let confs = names::conference_names();
        let n_conf = (n / 10).clamp(2, confs.len());
        for i in 0..n {
            let (nm, nick) = if i < base.len() {
                base[i].clone()
            } else {
                (
                    format!(
                        "{} {}",
                        dirs[(i / base.len() - 1) % dirs.len()],
                        base[i % base.len()].0
                    ),
                    base[i % base.len()].1.clone(),
                )
            };
            let prestige = (rng.gauss(48.0, 19.0)).clamp(8.0, 98.0);
            let id = self.colleges.len() as CollegeId;
            let coach = new_coach(rng, self.year, prestige);
            self.colleges.push(College {
                id,
                name: nm,
                nickname: nick,
                conf: (i % n_conf) as u8,
                prestige,
                facilities: (prestige * 0.6 + rng.gauss(20.0, 10.0)).clamp(10.0, 98.0),
                market: (prestige / 120.0 + rng.gauss(0.1, 0.08)).clamp(0.05, 0.95),
                roster: vec![],
                coach,
                nil_budget: 0,
                nil_spent: 0,
                record: Default::default(),
                history: vec![],
                titles: vec![],
                ad_patience: rng.gauss(55.0, 15.0).clamp(20.0, 95.0),
                starters: vec![],
                tactics_tempo: rng.gauss(0.5, 0.15).clamp(0.1, 0.9),
                tactics_three: rng.gauss(0.5, 0.15).clamp(0.1, 0.9),
                recruiting_points: 100.0,
                active: true,
            });
        }
    }

    fn fill_college_rosters(&mut self, rng: &mut Rng) {
        let flow = self.settings.num("realism.international_flow");
        let ids: Vec<CollegeId> = self.colleges.iter().map(|c| c.id).collect();
        for cid in ids {
            let prestige = self.colleges[cid as usize].prestige;
            let name = self.colleges[cid as usize].name.clone();
            for class in 1..=4u8 {
                let count = if class == 1 {
                    3
                } else {
                    3 + (rng.chance(0.4) as usize)
                };
                for _ in 0..count {
                    let p = self.make_college_player(rng, prestige, class, flow, &name);
                    let id = self.add_player(p);
                    self.pm(id).affiliation = Affiliation::College(cid);
                    self.colleges[cid as usize].roster.push(id);
                }
            }
        }
    }

    fn make_college_player(
        &self,
        rng: &mut Rng,
        prestige: f64,
        class: u8,
        flow: f64,
        school: &str,
    ) -> Player {
        let base = 31.0 + prestige * 0.10 + class as f64 * 2.3;
        let cur = (rng.gauss(base, 5.0) + rng.exp(2.0)).clamp(30.0, 74.0);
        let age = 17 + class as i32 + if rng.chance(0.2) { 1 } else { 0 };
        let pot =
            (cur + rng.uniform(1.0, 16.0) * (1.0 + (4 - class) as f64 * 0.12)).clamp(cur, 96.0);
        let spec = GenSpec::new(self.year, age, cur, pot, OriginKind::College);
        let mut p = generate_player(&self.content, rng, 0, &spec, flow);
        p.affiliation = Affiliation::College(0);
        p.origin.school = school.to_string();
        set_class(&mut p, class);
        p.fitness = 90.0;
        p
    }

    /// A new high-school class (one grade). Grade 12 = seniors.
    pub fn generate_hs_class(&mut self, rng: &mut Rng, grade: u8) {
        let n = ((self.colleges.len() as f64) * 2.4 + 70.0) as usize;
        let strength = 1.0 + rng.gauss(0.0, 0.25 * self.settings.num("draft.class_variance"));
        let flow = self.settings.num("realism.international_flow");
        let generational = rng
            .chance((0.09 * self.settings.num("draft.generational_rate")).clamp(0.0, 0.9))
            || rng.chance((0.08 * self.settings.num("draft.generational_rate")).clamp(0.0, 0.9));
        let (hs_a, hs_b) = names::school_words();
        for k in 0..n {
            // top-heavy talent: a few stars, many role players
            let q = rng.exp(1.0);
            let cur = (27.0 + q * 5.2 * strength + rng.gauss(0.0, 3.0)).clamp(24.0, 58.0);
            let pot = (cur
                + 8.0
                + rng.exp(7.0) * strength
                + if k == 0 && generational { 14.0 } else { 0.0 })
            .clamp(cur, 98.0);
            let age = if grade == 12 {
                17 + rng.chance(0.5) as i32
            } else {
                16 + rng.chance(0.5) as i32
            };
            let spec = GenSpec::new(
                self.year,
                age,
                cur + if k == 0 && generational { 5.0 } else { 0.0 },
                pot,
                OriginKind::HighSchool,
            );
            let mut p = generate_player(&self.content, rng, 0, &spec, flow * 0.4);
            p.affiliation = Affiliation::HighSchool;
            p.origin.school = format!("{} {}", rng.pick(&hs_a), rng.pick(&hs_b));
            p.custom.insert("class".into(), grade as f64);
            p.fitness = 92.0;
            self.add_player(p);
        }
        self.rank_hs_class();
    }

    /// Recompute national HS rankings for each grade.
    pub fn rank_hs_class(&mut self) {
        for grade in [11u8, 12u8] {
            let mut v: Vec<(PlayerId, f64)> = self
                .players
                .iter()
                .filter(|p| {
                    p.affiliation == Affiliation::HighSchool
                        && p.custom.get("class").copied().unwrap_or(0.0) as u8 == grade
                })
                .map(|p| {
                    (
                        p.id,
                        p.ovr as f64 * 0.45 + p.potential as f64 * 0.55 + p.id as f64 * 1e-9,
                    )
                })
                .collect();
            v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            for (rank, (id, _)) in v.into_iter().enumerate() {
                self.pm(id).custom.insert("rank".into(), (rank + 1) as f64);
            }
        }
    }

    // ------------------------------------------------------------------ recruiting

    /// How much a recruit likes a program (higher = more interested).
    pub fn recruit_interest(&self, cid: CollegeId, id: PlayerId, user_bonus: f64) -> f64 {
        let c = &self.colleges[cid as usize];
        let p = self.p(id);
        let rank = p.custom.get("rank").copied().unwrap_or(200.0);
        // playing time: would he crack the top 8 of this roster?
        let mut vals: Vec<f64> = c.roster.iter().map(|&r| self.p(r).ovr as f64).collect();
        vals.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let eighth = vals.get(7).copied().unwrap_or(35.0);
        let pt = ((p.ovr as f64 - eighth) * 1.5).clamp(-20.0, 25.0);
        let star_wants_prestige = if rank <= 30.0 { 1.4 } else { 0.9 };
        let nil = if self.settings.bool("college.nil") && self.year >= 2021 {
            ((c.nil_budget - c.nil_spent).max(0) as f64 / 1_000_000.0).min(15.0) * 1.2
        } else {
            0.0
        };
        c.prestige * 0.45 * star_wants_prestige
            + c.facilities * 0.15
            + c.coach.recruiting * 0.2
            + c.market * 10.0
            + pt
            + nil
            + user_bonus
    }

    /// Fill open roster slots from the senior HS class (and commit the stars).
    pub fn recruit_to_open_slots(&mut self, rng: &mut Rng) {
        let mut seniors: Vec<PlayerId> = self
            .players
            .iter()
            .filter(|p| {
                p.affiliation == Affiliation::HighSchool
                    && p.custom.get("class").copied().unwrap_or(0.0) as u8 == 12
                    && !p.user_controlled
                    && !p.flags.contains("committed")
            })
            .map(|p| p.id)
            .collect();
        seniors.sort_by(|&a, &b| {
            self.p(a)
                .custom
                .get("rank")
                .copied()
                .unwrap_or(999.0)
                .partial_cmp(&self.p(b).custom.get("rank").copied().unwrap_or(999.0))
                .unwrap()
        });
        let limit = if self.year < 1972 { 13 } else { 13 };
        for id in seniors {
            let open: Vec<CollegeId> = self
                .colleges
                .iter()
                .filter(|c| c.active && c.roster.len() < limit && !self.user_college_controls(c.id))
                .map(|c| c.id)
                .collect();
            if open.is_empty() {
                break;
            }
            // consider a shortlist of programs
            let mut best: Option<(f64, CollegeId)> = None;
            let k = open.len().min(8);
            let mut pool = open.clone();
            rng.shuffle(&mut pool);
            // top programs always get a look at stars
            let rank = self.p(id).custom.get("rank").copied().unwrap_or(999.0);
            if rank < 40.0 {
                let mut top: Vec<CollegeId> = open.clone();
                top.sort_by(|&a, &b| {
                    self.colleges[b as usize]
                        .prestige
                        .partial_cmp(&self.colleges[a as usize].prestige)
                        .unwrap()
                });
                for &c in top.iter().take(10) {
                    if !pool.contains(&c) {
                        pool.push(c);
                    }
                }
            }
            for &c in pool.iter().take(k + 10) {
                let s = self.recruit_interest(c, id, 0.0) + rng.gauss(0.0, 6.0);
                if best.map(|b| s > b.0).unwrap_or(true) {
                    best = Some((s, c));
                }
            }
            if let Some((_, c)) = best {
                self.commit_recruit(id, c);
            }
        }
    }

    pub fn commit_recruit(&mut self, id: PlayerId, c: CollegeId) {
        let name = self.colleges[c as usize].name.clone();
        let p = self.pm(id);
        p.affiliation = Affiliation::College(c);
        p.origin.kind = OriginKind::HighSchool;
        set_class(p, 1);
        p.custom.remove("rank");
        p.flags.remove("committed");
        p.origin.school = name;
        self.colleges[c as usize].roster.push(id);
    }

    pub fn user_college_controls(&self, c: CollegeId) -> bool {
        self.user.college == Some(c)
            && (self.user.has(Role::CollegeCoach) || self.user.has(Role::CollegeAd))
    }

    // ------------------------------------------------------------------ season

    pub fn college_rating(&self, cid: CollegeId) -> f64 {
        let t = self.college_team(cid, false);
        t.rating()
    }

    pub fn college_team(&self, cid: CollegeId, for_game: bool) -> FastTeam {
        let c = &self.colleges[cid as usize];
        let _ = for_game;
        let ps: Vec<FastPlayer> = c
            .roster
            .iter()
            .filter(|&&id| {
                let p = self.p(id);
                !p.is_injured() && p.life.as_ref().map(|l| l.eligible).unwrap_or(true)
            })
            .map(|&id| FastPlayer::from_player(self.p(id)))
            .collect();
        let mut t = FastTeam::build(ps);
        t.tempo = c.tactics_tempo;
        t
    }

    fn college_style_ppg(&self) -> f64 {
        // 40-minute games: scale the pro era's points down and add the slower college tempo.
        self.style.ppg * 0.78 * (40.0 / 48.0) / 0.8
    }

    pub fn generate_college_schedule(&mut self, rng: &mut Rng) {
        let ids: Vec<CollegeId> = self
            .colleges
            .iter()
            .filter(|c| c.active)
            .map(|c| c.id)
            .collect();
        let n_games = era_college_games(self.year) as usize;
        let mut sched = vec![];
        let mut day = 8u16;
        let last_nba = self.schedule.iter().map(|f| f.day).max().unwrap_or(150);
        let span = (last_nba as f64 * 0.85).max(60.0);
        let step = (span / n_games as f64).max(1.5);
        for round in 0..n_games {
            let mut pool = ids.clone();
            rng.shuffle(&mut pool);
            // conference rounds: pair within conferences in the second 2/3 of the schedule
            if round * 3 >= n_games {
                let mut keyed: Vec<(u8, i64, CollegeId)> = pool
                    .iter()
                    .map(|&c| (self.colleges[c as usize].conf, rng.range(0, 1000), c))
                    .collect();
                keyed.sort();
                pool = keyed.into_iter().map(|k| k.2).collect();
            }
            let mut i = 0;
            while i + 1 < pool.len() {
                let (a, b) = (pool[i], pool[i + 1]);
                // if sorted by conference and pair spans conferences, still fine
                let (h, aw) = if rng.chance(0.5) { (a, b) } else { (b, a) };
                sched.push(CFixture {
                    day: day + (rng.range(0, 1) as u16),
                    home: h,
                    away: aw,
                    done: false,
                    home_pts: 0,
                    away_pts: 0,
                });
                i += 2;
            }
            day = (day as f64 + step).round() as u16;
        }
        self.college_state = CollegeState {
            schedule: sched,
            ..Default::default()
        };
        for c in self.colleges.iter_mut() {
            c.record = Default::default();
        }
    }

    /// Simulate college games scheduled for `day`; handles the tournament after the schedule ends.
    pub fn sim_college_day(&mut self, day: u32) {
        if self.colleges.is_empty() || self.college_state.done {
            return;
        }
        let mut rng = Rng::from_label(self.seed, &format!("college-{}-{}", self.year, day));
        let idxs: Vec<usize> = self
            .college_state
            .schedule
            .iter()
            .enumerate()
            .filter(|(_, f)| f.day as u32 <= day && !f.done)
            .map(|(i, _)| i)
            .collect();
        for i in idxs {
            let (h, a) = (
                self.college_state.schedule[i].home,
                self.college_state.schedule[i].away,
            );
            let (hp, ap) = self.play_college_game(h, a, false, &mut rng);
            let f = &mut self.college_state.schedule[i];
            f.done = true;
            f.home_pts = hp;
            f.away_pts = ap;
        }
        let reg_done = self.college_state.schedule.iter().all(|f| f.done);
        let last_day = self
            .college_state
            .schedule
            .iter()
            .map(|f| f.day)
            .max()
            .unwrap_or(0) as u32;
        if reg_done && day >= last_day + 2 {
            self.college_tournament_step(&mut rng);
        }
    }

    /// Finish the entire college season right now (used when the NBA season moves on).
    pub fn finish_college_season(&mut self) {
        if self.colleges.is_empty() {
            return;
        }
        let mut guard = 0;
        while !self.college_state.done && guard < 400 {
            guard += 1;
            let day = self
                .college_state
                .schedule
                .iter()
                .map(|f| f.day)
                .max()
                .unwrap_or(0) as u32
                + 3
                + guard;
            self.sim_college_day(day);
        }
    }

    fn play_college_game(
        &mut self,
        h: CollegeId,
        a: CollegeId,
        neutral: bool,
        rng: &mut Rng,
    ) -> (u16, u16) {
        let ppg = self.college_style_ppg();
        let three = self.style.three_rate * 0.9;
        let ht = self.college_team(h, true);
        let at = self.college_team(a, true);
        let res = simulate(&ht, &at, ppg, if neutral { 0.0 } else { 3.2 }, three, rng);
        let year = self.year;
        for (side, cid) in [(0usize, h), (1usize, a)] {
            let name = self.colleges[cid as usize].name.clone();
            for (pid, s) in &res.lines[side] {
                let p = self.pm(*pid);
                let need = !matches!(p.seasons.last(), Some(r) if r.season == year && r.level == Level::College && r.team == name);
                if need {
                    let age = year - p.birth_year;
                    p.seasons.push(SeasonRecord {
                        season: year,
                        level: Level::College,
                        team: name.clone(),
                        team_id: None,
                        age: age.max(0) as u8,
                        ovr: p.ovr,
                        stats: StatLine::default(),
                        playoffs: StatLine::default(),
                        salary: 0,
                    });
                }
                if let Some(r) = p.seasons.last_mut() {
                    r.stats.add(s);
                }
            }
        }
        let (hp, ap) = (res.pts[0], res.pts[1]);
        let rec = |c: &mut College, win: bool, pf: u16, pa: u16| {
            if win {
                c.record.w += 1;
            } else {
                c.record.l += 1;
            }
            c.record.pf += pf as u32;
            c.record.pa += pa as u32;
        };
        rec(&mut self.colleges[h as usize], hp > ap, hp, ap);
        rec(&mut self.colleges[a as usize], ap > hp, ap, hp);
        (hp, ap)
    }

    fn college_tournament_step(&mut self, rng: &mut Rng) {
        if !self.settings.bool("college.tournament") {
            self.college_state.done = true;
            self.college_state.champion = self
                .colleges
                .iter()
                .filter(|c| c.active)
                .max_by(|a, b| a.record.pct().partial_cmp(&b.record.pct()).unwrap())
                .map(|c| c.id);
            self.finish_college_year();
            return;
        }
        if !self.college_state.tournament_started {
            let size = tournament_size(self.year)
                .min(self.colleges.len().next_power_of_two())
                .min(self.colleges.len());
            let mut field: Vec<CollegeId> = self
                .colleges
                .iter()
                .filter(|c| c.active)
                .map(|c| c.id)
                .collect();
            field.sort_by(|&a, &b| {
                let (ca, cb) = (&self.colleges[a as usize], &self.colleges[b as usize]);
                (cb.record.pct() + cb.prestige / 400.0)
                    .partial_cmp(&(ca.record.pct() + ca.prestige / 400.0))
                    .unwrap()
            });
            let mut pow2 = 1;
            while pow2 * 2 <= size {
                pow2 *= 2;
            }
            field.truncate(pow2.max(2));
            self.college_state.field = field.clone();
            // 1 v N, 2 v N-1...
            let n = field.len();
            self.college_state.pending = (0..n / 2).map(|i| (field[i], field[n - 1 - i])).collect();
            self.college_state.tournament_started = true;
            self.add_news(
                "college",
                format!(
                    "The {} college tournament begins with {} teams.",
                    self.year, n
                ),
            );
            return;
        }
        // play one round
        let pairs = std::mem::take(&mut self.college_state.pending);
        let mut winners: Vec<CollegeId> = vec![];
        for (a, b) in pairs {
            let (pa, pb) = self.play_college_game(a, b, true, rng);
            // a tournament game must have a winner
            let w = if pa >= pb { a } else { b };
            winners.push(w);
        }
        self.college_state.round += 1;
        if winners.len() == 1 {
            let champ = winners[0];
            self.college_state.champion = Some(champ);
            self.college_state.done = true;
            self.colleges[champ as usize].titles.push(self.year);
            self.colleges[champ as usize].coach.titles += 1;
            let txt = format!(
                "{} {} win the {} college championship!",
                self.colleges[champ as usize].name,
                self.colleges[champ as usize].nickname,
                self.year
            );
            self.college_state.last_champion_text = txt.clone();
            self.add_news("college", txt);
            self.finish_college_year();
        } else {
            self.college_state.pending = winners
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| (c[0], c[1]))
                .collect();
        }
    }

    /// Record each program's season result and update prestige / coaches.
    fn finish_college_year(&mut self) {
        let year = self.year;
        let champ = self.college_state.champion;
        let field = self.college_state.field.clone();
        let mut fired = vec![];
        for c in self.colleges.iter_mut() {
            let result = if Some(c.id) == champ {
                "National champions".to_string()
            } else if field.contains(&c.id) {
                "Made the tournament".to_string()
            } else {
                "No postseason".to_string()
            };
            c.history.push(CollegeSeason {
                season: year,
                w: c.record.w,
                l: c.record.l,
                result,
                coach: c.coach.name.clone(),
            });
            c.coach.wins += c.record.w as u32;
            c.coach.losses += c.record.l as u32;
            c.coach.years += 1;
            // prestige drifts with success
            let pct = c.record.pct();
            let target = 25.0
                + pct * 55.0
                + if field.contains(&c.id) { 8.0 } else { 0.0 }
                + if Some(c.id) == champ { 10.0 } else { 0.0 };
            c.prestige = (c.prestige * 0.93 + target * 0.07).clamp(5.0, 99.0);
            c.facilities = (c.facilities + 0.4).min(98.0);
            // job security
            if c.coach.years > 2 && pct < 0.38 && c.ad_patience < 80.0 {
                fired.push(c.id);
            }
        }
        for cid in fired {
            if self.user.college == Some(cid) && self.user.has(Role::CollegeCoach) {
                continue; // handled by the career module
            }
            let prestige = self.colleges[cid as usize].prestige;
            let mut rng = Rng::from_label(self.seed, &format!("coachfire-{year}-{cid}"));
            let new = new_coach(&mut rng, year, prestige);
            let old = std::mem::replace(&mut self.colleges[cid as usize].coach, new);
            let cname = self.colleges[cid as usize].name.clone();
            self.add_news(
                "college",
                format!(
                    "{cname} fire coach {} and hire {}.",
                    old.name, self.colleges[cid as usize].coach.name
                ),
            );
        }
    }

    // ------------------------------------------------------------------ year rollover

    /// After the draft: seniors leave, underclassmen advance, recruits arrive, transfers move.
    pub fn college_rollover(&mut self, rng: &mut Rng) {
        if self.colleges.is_empty() {
            return;
        }
        // Players removed (drafted or declared and signed) already left via the draft.
        let mut leaving: Vec<PlayerId> = vec![];
        for c in &self.colleges {
            for &id in &c.roster {
                let p = self.p(id);
                if class_of(p) >= 4 {
                    leaving.push(id);
                }
            }
        }
        for id in leaving {
            self.graduate_college_player(id, rng);
        }
        // advance classes
        for c in self.colleges.iter() {
            for &id in c.roster.clone().iter() {
                let p = &mut self.players[id as usize];
                let cl = class_of(p);
                set_class(p, cl + 1);
            }
        }
        // transfer portal: a few players move
        if self.settings.bool("college.transfer_portal") && self.year >= 2018 {
            self.run_transfer_portal(rng);
        }
        // HS grade 12 -> recruit; grade 11 -> 12; new grade 11 class
        self.rank_hs_class();
        self.recruit_to_open_slots(rng);
        // unsigned seniors who weren't recruited: go to prep/overseas/retire
        let stragglers: Vec<PlayerId> = self
            .players
            .iter()
            .filter(|p| {
                p.affiliation == Affiliation::HighSchool
                    && p.custom.get("class").copied().unwrap_or(0.0) as u8 == 12
                    && !p.user_controlled
            })
            .map(|p| p.id)
            .collect();
        for id in stragglers {
            let p = self.pm(id);
            // most simply stop playing organized ball; a handful go overseas
            if p.potential >= 62 && rng.chance(0.25) {
                self.send_overseas(id, rng);
                self.pm(id).origin.kind = OriginKind::HighSchool;
            } else {
                self.retire_player(id, "never turned pro");
            }
        }
        // age the HS pipeline
        for p in self.players.iter_mut() {
            if p.affiliation == Affiliation::HighSchool && !p.is_retired() {
                let c = p.custom.get("class").copied().unwrap_or(11.0) as u8;
                if c == 11 {
                    p.custom.insert("class".into(), 12.0);
                }
            }
        }
        self.generate_hs_class(rng, 11);
        // fill remaining holes from the freshly aged seniors if any slots are open
        self.recruit_to_open_slots(rng);
        // restock rosters that are too thin with walk-ons
        let flow = self.settings.num("realism.international_flow");
        let ids: Vec<CollegeId> = self.colleges.iter().map(|c| c.id).collect();
        for cid in ids {
            let prestige = self.colleges[cid as usize].prestige;
            let name = self.colleges[cid as usize].name.clone();
            while self.colleges[cid as usize].roster.len() < 10 {
                let p = self.make_college_player(rng, prestige * 0.8, 1, flow, &name);
                let id = self.add_player(p);
                self.pm(id).affiliation = Affiliation::College(cid);
                self.colleges[cid as usize].roster.push(id);
            }
            self.colleges[cid as usize].recruiting_points = 100.0;
            let year = self.year;
            if self.settings.bool("college.nil") && year >= 2021 {
                let p = self.colleges[cid as usize].prestige;
                self.colleges[cid as usize].nil_budget =
                    ((p / 100.0).powi(2) * 6_000_000.0 * crate::life::cpi(year + 1).max(0.5))
                        as i64;
                self.colleges[cid as usize].nil_spent = 0;
            }
        }
    }

    fn graduate_college_player(&mut self, id: PlayerId, rng: &mut Rng) {
        // find his college and remove him
        let cid = match self.p(id).affiliation {
            Affiliation::College(c) => c,
            _ => return,
        };
        self.colleges[cid as usize].roster.retain(|&x| x != id);
        let age = self.year + 1 - self.p(id).birth_year;
        let pot = self.p(id).ovr;
        let user = self.p(id).user_controlled;
        if user {
            // The human's player faces the 'what now?' decision elsewhere.
            self.colleges[cid as usize].roster.push(id);
            return;
        }
        // An undrafted senior: sign overseas, or a free agent camp invite, or stop.
        if pot >= 48 && rng.chance(0.55) {
            self.send_overseas(id, rng);
        } else if pot >= 42 && rng.chance(0.4) {
            let p = self.pm(id);
            p.affiliation = Affiliation::FreeAgent;
            p.origin.kind = OriginKind::Undrafted;
            self.free_agents.push(id);
        } else {
            self.retire_player(id, "graduated");
        }
        let _ = age;
    }

    fn run_transfer_portal(&mut self, rng: &mut Rng) {
        let ids: Vec<PlayerId> = self
            .colleges
            .iter()
            .flat_map(|c| c.roster.iter().copied())
            .collect();
        for id in ids {
            let p = self.p(id);
            if p.user_controlled || class_of(p) >= 4 {
                continue;
            }
            let cid = match p.affiliation {
                Affiliation::College(c) => c,
                _ => continue,
            };
            // unhappy bench players (rank below 8th man) leave sometimes
            let mut vals: Vec<f64> = self.colleges[cid as usize]
                .roster
                .iter()
                .map(|&r| self.p(r).ovr as f64)
                .collect();
            vals.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let eighth = vals.get(7).copied().unwrap_or(35.0);
            let chance = if (p.ovr as f64) < eighth - 3.0 {
                0.18
            } else {
                0.03
            };
            if rng.chance(chance) {
                // choose a better-fit program with an open slot
                let dest = self
                    .colleges
                    .iter()
                    .filter(|c| c.id != cid && c.roster.len() < 13 && c.active)
                    .map(|c| {
                        (
                            self.recruit_interest(c.id, id, 0.0) + rng.gauss(0.0, 5.0),
                            c.id,
                        )
                    })
                    .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                if let Some((_, to)) = dest {
                    self.colleges[cid as usize].roster.retain(|&x| x != id);
                    self.colleges[to as usize].roster.push(id);
                    let nm = self.colleges[to as usize].name.clone();
                    let p = self.pm(id);
                    p.affiliation = Affiliation::College(to);
                    p.origin.school = nm;
                    p.flags.insert("transfer".into());
                }
            }
        }
    }

    // ------------------------------------------------------------------ draft eligibility

    /// College / HS / overseas players who enter this year's draft.
    pub fn declared_prospects(&mut self, rng: &mut Rng) -> Vec<PlayerId> {
        let draft_year = self.year + 1;
        let rules = self.content.rules(self.rules_year_for(draft_year));
        let slots = rules.draft_rounds as usize * self.active_team_ids().len();
        let mut out: Vec<PlayerId> = vec![];
        // Rank everyone who could possibly be drafted by value.
        let mut cands: Vec<(PlayerId, f64)> = vec![];
        for p in self.players.iter() {
            if p.is_retired() || p.user_controlled {
                continue;
            }
            let age_at_draft = draft_year - p.birth_year;
            let ok = match p.affiliation {
                Affiliation::College(_) => {
                    let class = class_of(p);
                    let senior = class >= 4;
                    (senior || (rules.early_entry && age_at_draft >= rules.min_draft_age as i32))
                        && (rules.early_entry || senior)
                }
                Affiliation::HighSchool => {
                    rules.hs_allowed && p.custom.get("class").copied().unwrap_or(0.0) as u8 == 12
                }
                Affiliation::Overseas(_) => {
                    age_at_draft >= rules.min_draft_age as i32
                        && age_at_draft <= 25
                        && p.draft.is_none()
                        && p.draft_rights.is_none()
                }
                _ => false,
            };
            if ok {
                cands.push((p.id, p.ovr as f64 * 0.5 + p.potential as f64 * 0.5));
            }
        }
        cands.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (rank, (id, _)) in cands.iter().enumerate() {
            let p = self.p(*id);
            let senior = matches!(p.affiliation, Affiliation::College(_)) && class_of(p) >= 4;
            let declare = if senior || matches!(p.affiliation, Affiliation::Overseas(_)) {
                // seniors/overseas pros: always in the pool if they could be picked
                rank < (slots as f64 * 1.8) as usize
            } else if matches!(p.affiliation, Affiliation::HighSchool) {
                rank < 30 && rng.chance(0.85) || (rank < 70 && rng.chance(0.2))
            } else {
                // underclassmen: strong prospects go
                (rank < slots / 2 && rng.chance(0.85)) || (rank < slots && rng.chance(0.25))
            };
            if declare {
                out.push(*id);
            }
        }
        out
    }

    /// New head coach for a program.
    pub fn hire_college_coach(&mut self, cid: CollegeId, rng: &mut Rng) {
        let prestige = self.colleges[cid as usize].prestige;
        self.colleges[cid as usize].coach = new_coach(rng, self.year, prestige);
    }

    pub fn standings_college(&self) -> Vec<CollegeId> {
        let mut v: Vec<CollegeId> = self
            .colleges
            .iter()
            .filter(|c| c.active)
            .map(|c| c.id)
            .collect();
        v.sort_by(|&a, &b| {
            let (ca, cb) = (&self.colleges[a as usize], &self.colleges[b as usize]);
            cb.record
                .pct()
                .partial_cmp(&ca.record.pct())
                .unwrap()
                .then(cb.prestige.partial_cmp(&ca.prestige).unwrap())
        });
        v
    }

    /// Pay a college player NIL money (modern era only).
    pub fn nil_available(&self) -> bool {
        self.settings.bool("college.nil") && self.year >= 2021
    }

    /// Create a pro-style contract for a drafted college player.
    pub fn rookie_contract_for(&self, pick: u16, round: u8) -> Contract {
        let cap = self.money.cap as f64;
        let first = if self.rules.rookie_scale {
            let pct = if round == 1 {
                0.075 * (1.0 - (pick as f64 - 1.0) / 60.0 * 0.86).max(0.18)
            } else {
                0.0
            };
            if round == 1 {
                (cap * pct) as i64
            } else {
                self.money.min_salary
            }
        } else {
            let base = self.money.avg_salary as f64
                * (if round == 1 {
                    1.1 - pick as f64 * 0.012
                } else {
                    0.5
                });
            base.max(self.money.min_salary as f64) as i64
        };
        let years = if round == 1 { 3 } else { 2 };
        let kind = if round == 1 {
            ContractKind::Rookie
        } else {
            ContractKind::Standard
        };
        let mut c = Contract::rising(
            first.max(self.money.min_salary),
            years,
            0.05,
            kind,
            self.year + 1,
        );
        if round == 1 && self.rules.rookie_scale {
            c.option = OptionKind::Team;
            // team option on the last year
        }
        c
    }
}

pub fn new_coach(rng: &mut Rng, year: Season, prestige: f64) -> CoachInfo {
    let pool = &names::builtin_pools()[0];
    let (f, l) = names::random_name(rng, pool);
    let q = 45.0 + prestige * 0.12;
    let g = |rng: &mut Rng| rng.gauss(q, 11.0).clamp(15.0, 97.0);
    CoachInfo {
        name: format!("{f} {l}"),
        born: year - rng.range(34, 62) as i32,
        recruiting: g(rng),
        development: g(rng),
        tactics: g(rng),
        offense: g(rng),
        defense: g(rng),
        reputation: (q + rng.gauss(0.0, 8.0)).clamp(10.0, 95.0),
        wins: 0,
        losses: 0,
        titles: 0,
        years: 0,
    }
}
