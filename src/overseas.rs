//! Professional basketball overseas: domestic leagues and the continental "EuroLeague".
//!
//! Clubs are fictional (real cities, made-up names). They sign local and imported players,
//! play seasons, win titles, and hold the contracts of "draft-and-stash" prospects whose NBA
//! rights belong to a league team. Players who don't make the NBA (or who choose not to) can
//! build a whole career overseas, and stars can come back.

use crate::contract::*;
use crate::fastsim::*;
use crate::generate::*;
use crate::league::*;
use crate::names;
use crate::player::*;
use crate::rng::Rng;
use crate::team::Record;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClubSeason {
    pub season: Season,
    pub w: u16,
    pub l: u16,
    pub result: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Club {
    pub id: ClubId,
    pub name: String,
    pub city: String,
    pub country: String,
    pub league: String,
    /// 1 = top club (plays in the continental EuroLeague), 2 = strong domestic side, 3 = smaller club.
    pub tier: u8,
    pub budget: Money,
    pub roster: Vec<PlayerId>,
    pub record: Record,
    pub titles: Vec<Season>,
    pub euro_titles: Vec<Season>,
    pub history: Vec<ClubSeason>,
    pub euro_record: Record,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OFixture {
    pub day: u16,
    pub home: ClubId,
    pub away: ClubId,
    pub done: bool,
    /// true = EuroLeague game
    pub euro: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OverseasState {
    pub schedule: Vec<OFixture>,
    pub done: bool,
    pub champions: BTreeMap<String, ClubId>,
    pub euro_champion: Option<ClubId>,
}

const EURO_CODES: &[&str] = &[
    "ESP", "SRB", "CRO", "SLO", "LTU", "LAT", "GRE", "TUR", "ITA", "FRA", "GER", "RUS", "UKR",
    "ISR", "GBR", "FIN", "SWE",
];

impl League {
    pub fn create_clubs(&mut self, rng: &mut Rng) {
        let suffixes = names::club_suffixes();
        let countries = self.content.countries.clone();
        for c in countries {
            let w = names::country_weight(&c, self.year.max(1990)).max(0.0);
            // number of clubs by basketball strength of the country (use a mid-era weight so countries exist early too)
            let strength = c.weight.iter().map(|(_, v)| *v).fold(0.0, f64::max);
            let n = ((strength.sqrt() * 1.3).round() as usize).clamp(1, 6);
            let _ = w;
            let mut used: Vec<String> = vec![];
            for k in 0..n {
                let town = c.towns[k % c.towns.len()].clone();
                let suf = rng.pick(&suffixes).clone();
                let mut name = format!("{} {}", town, suf);
                if used.contains(&name) {
                    name = format!("{} {} {}", town, suf, k + 1);
                }
                used.push(name.clone());
                let id = self.clubs.len() as ClubId;
                let tier = if k == 0 && EURO_CODES.contains(&c.code.as_str()) && strength >= 3.0 {
                    1
                } else if k < (n + 1) / 2 {
                    2
                } else {
                    3
                };
                let budget_mult = match tier {
                    1 => 0.35,
                    2 => 0.12,
                    _ => 0.04,
                };
                self.clubs.push(Club {
                    id,
                    name,
                    city: town.replace('_', " "),
                    country: c.code.clone(),
                    league: c.league.clone(),
                    tier,
                    budget: (self.money.avg_salary as f64 * 12.0 * budget_mult * 3.0) as i64,
                    roster: vec![],
                    record: Record::default(),
                    titles: vec![],
                    euro_titles: vec![],
                    history: vec![],
                    euro_record: Record::default(),
                });
            }
        }
        // second top club for the biggest basketball nations
        let mut top_per_country: BTreeMap<String, u8> = BTreeMap::new();
        for c in self.clubs.iter_mut() {
            if c.tier == 1 {
                *top_per_country.entry(c.country.clone()).or_insert(0) += 1;
            }
        }
        let _ = top_per_country;
        // ensure at least 12 EuroLeague clubs
        let mut euro_n = self.clubs.iter().filter(|c| c.tier == 1).count();
        let mut idx = 0;
        while euro_n < 12 && idx < self.clubs.len() {
            if self.clubs[idx].tier == 2 && EURO_CODES.contains(&self.clubs[idx].country.as_str()) {
                self.clubs[idx].tier = 1;
                euro_n += 1;
            }
            idx += 1;
        }
    }

    pub fn fill_club_rosters(&mut self, rng: &mut Rng) {
        let flow = self.settings.num("realism.international_flow");
        let n = self.clubs.len();
        for ci in 0..n {
            let (tier, country) = (self.clubs[ci].tier, self.clubs[ci].country.clone());
            let count = 11 + rng.range(0, 2) as usize;
            for k in 0..count {
                let base = match tier {
                    1 => 54.0,
                    2 => 47.0,
                    _ => 41.0,
                };
                let age = if k < 3 {
                    rng.range(18, 21)
                } else {
                    rng.range(21, 35)
                } as i32;
                let peak = rng.gauss(base + 4.0, 6.0) + rng.exp(2.0);
                let cur = (peak + crate::progression::age_shift(age)).clamp(30.0, 80.0);
                let pot = if age < 25 {
                    (peak + rng.uniform(0.0, 14.0)).clamp(cur, 92.0)
                } else {
                    cur
                };
                // mostly locals, some imports (including Americans)
                let local = rng.chance(0.65);
                let mut spec = GenSpec::new(self.year, age, cur, pot, OriginKind::International);
                if local {
                    spec.country = Some(country.clone());
                } else if rng.chance(0.5) {
                    spec.country = Some("USA".into());
                }
                let mut p = generate_player(&self.content, rng, 0, &spec, flow);
                p.years_pro = (age - 18).max(0) as u8;
                p.affiliation = Affiliation::Overseas(ci as ClubId);
                p.origin.school = self.clubs[ci].name.clone();
                p.contract = Some(self.overseas_contract(tier, p.ovr, rng));
                let id = self.add_player(p);
                self.clubs[ci].roster.push(id);
            }
        }
    }

    pub fn overseas_contract(&self, tier: u8, ovr: u8, rng: &mut Rng) -> Contract {
        let mult = match tier {
            1 => 0.5,
            2 => 0.16,
            _ => 0.06,
        };
        let sal = (self.money.avg_salary as f64
            * mult
            * (0.4 + (ovr as f64 - 40.0).max(0.0) / 25.0)
            * rng.uniform(0.8, 1.2)) as i64;
        Contract::rising(
            sal.max(self.money.min_salary / 4),
            rng.range(1, 3) as u8,
            0.03,
            ContractKind::Overseas,
            self.year,
        )
    }

    pub fn club_rating(&self, cid: ClubId) -> f64 {
        self.club_team(cid).rating()
    }

    fn club_team(&self, cid: ClubId) -> FastTeam {
        let ps: Vec<FastPlayer> = self.clubs[cid as usize]
            .roster
            .iter()
            .filter(|&&id| !self.p(id).is_injured())
            .map(|&id| FastPlayer::from_player(self.p(id)))
            .collect();
        FastTeam::build(ps)
    }

    /// Put a player on an overseas club that fits his level (and nationality when possible).
    pub fn send_overseas(&mut self, id: PlayerId, rng: &mut Rng) {
        if self.clubs.is_empty() {
            return;
        }
        let ovr = self.p(id).ovr;
        let country = self.p(id).country.clone();
        let want_tier = if ovr >= 62 {
            1
        } else if ovr >= 52 {
            2
        } else {
            3
        };
        let mut cands: Vec<(f64, ClubId)> = self
            .clubs
            .iter()
            .filter(|c| c.roster.len() < 15)
            .map(|c| {
                let mut s =
                    -((c.tier as f64 - want_tier as f64).abs()) * 3.0 + rng.uniform(0.0, 2.0);
                if c.country == country {
                    s += 2.0;
                }
                (s, c.id)
            })
            .collect();
        cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let Some(&(_, cid)) = cands.first() else {
            return;
        };
        // remove from previous affiliations
        let t = self.p(id).team_id();
        if let Some(t) = t {
            self.team_mut(t).roster.retain(|&x| x != id);
            self.team_mut(t).gleague.retain(|&x| x != id);
        }
        self.free_agents.retain(|&x| x != id);
        let tier = self.clubs[cid as usize].tier;
        let name = self.clubs[cid as usize].name.clone();
        let contract = self.overseas_contract(tier, ovr, rng);
        let p = self.pm(id);
        p.affiliation = Affiliation::Overseas(cid);
        p.contract = Some(contract);
        p.origin.school = name;
        self.clubs[cid as usize].roster.push(id);
    }

    // ------------------------------------------------------------------ season

    pub fn generate_overseas_schedule(&mut self, rng: &mut Rng) {
        let mut sched = vec![];
        let last_nba = self.schedule.iter().map(|f| f.day).max().unwrap_or(150) as f64;
        let span = last_nba * 0.9;
        // domestic leagues
        let mut leagues: BTreeMap<String, Vec<ClubId>> = BTreeMap::new();
        for c in &self.clubs {
            leagues.entry(c.league.clone()).or_default().push(c.id);
        }
        for (_, ids) in leagues {
            let n = ids.len();
            if n < 2 {
                continue;
            }
            let rounds = if n <= 8 { (n - 1) * 2 } else { 26 };
            for r in 0..rounds {
                let mut pool = ids.clone();
                rng.shuffle(&mut pool);
                let day = (6.0 + span * (r as f64 / rounds as f64)).round() as u16;
                let mut i = 0;
                while i + 1 < pool.len() {
                    let (h, a) = if rng.chance(0.5) {
                        (pool[i], pool[i + 1])
                    } else {
                        (pool[i + 1], pool[i])
                    };
                    sched.push(OFixture {
                        day,
                        home: h,
                        away: a,
                        done: false,
                        euro: false,
                    });
                    i += 2;
                }
            }
        }
        // EuroLeague: top-tier clubs play 24 rounds
        let euro: Vec<ClubId> = self
            .clubs
            .iter()
            .filter(|c| c.tier == 1)
            .map(|c| c.id)
            .collect();
        for r in 0..24 {
            let mut pool = euro.clone();
            rng.shuffle(&mut pool);
            let day = (10.0 + span * (r as f64 / 24.0)).round() as u16 + 1;
            let mut i = 0;
            while i + 1 < pool.len() {
                let (h, a) = if rng.chance(0.5) {
                    (pool[i], pool[i + 1])
                } else {
                    (pool[i + 1], pool[i])
                };
                sched.push(OFixture {
                    day,
                    home: h,
                    away: a,
                    done: false,
                    euro: true,
                });
                i += 2;
            }
        }
        self.overseas_state = OverseasState {
            schedule: sched,
            ..Default::default()
        };
        for c in self.clubs.iter_mut() {
            c.record = Record::default();
            c.euro_record = Record::default();
        }
    }

    pub fn sim_overseas_day(&mut self, day: u32) {
        if self.clubs.is_empty() || self.overseas_state.done {
            return;
        }
        let mut rng = Rng::from_label(self.seed, &format!("overseas-{}-{}", self.year, day));
        let idxs: Vec<usize> = self
            .overseas_state
            .schedule
            .iter()
            .enumerate()
            .filter(|(_, f)| f.day as u32 <= day && !f.done)
            .map(|(i, _)| i)
            .collect();
        for i in idxs {
            let f = self.overseas_state.schedule[i].clone();
            self.play_club_game(f.home, f.away, f.euro, &mut rng);
            self.overseas_state.schedule[i].done = true;
        }
    }

    fn play_club_game(&mut self, h: ClubId, a: ClubId, euro: bool, rng: &mut Rng) {
        let ht = self.club_team(h);
        let at = self.club_team(a);
        // FIBA-style 40 minute games: slightly lower scoring than the NBA in the same era
        let ppg = self.style.ppg * 0.78 * 0.9;
        let res = simulate(&ht, &at, ppg, 3.0, self.style.three_rate * 0.8, rng);
        let year = self.year;
        for (side, cid) in [(0usize, h), (1usize, a)] {
            let name = self.clubs[cid as usize].name.clone();
            for (pid, s) in &res.lines[side] {
                let p = self.pm(*pid);
                let need = !matches!(p.seasons.last(), Some(r) if r.season == year && r.level == Level::Overseas && r.team == name);
                if need {
                    let age = year - p.birth_year;
                    p.seasons.push(SeasonRecord {
                        season: year,
                        level: Level::Overseas,
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
        for (cid, pf, pa) in [(h, hp, ap), (a, ap, hp)] {
            let c = &mut self.clubs[cid as usize];
            let rec = if euro {
                &mut c.euro_record
            } else {
                &mut c.record
            };
            if pf > pa {
                rec.w += 1;
            } else {
                rec.l += 1;
            }
            rec.pf += pf as u32;
            rec.pa += pa as u32;
        }
    }

    /// Wrap up the overseas year: crown champions and record histories.
    pub fn finish_overseas_season(&mut self) {
        if self.clubs.is_empty() {
            return;
        }
        // finish unplayed games
        let last = self
            .overseas_state
            .schedule
            .iter()
            .map(|f| f.day)
            .max()
            .unwrap_or(0) as u32
            + 1;
        self.sim_overseas_day(last);
        if self.overseas_state.done {
            return;
        }
        let mut rng = Rng::from_label(self.seed, &format!("overseas-final-{}", self.year));
        let year = self.year;
        // domestic champions: top 4 by record play off
        let mut leagues: BTreeMap<String, Vec<ClubId>> = BTreeMap::new();
        for c in &self.clubs {
            leagues.entry(c.league.clone()).or_default().push(c.id);
        }
        let mut champs = BTreeMap::new();
        for (name, mut ids) in leagues {
            ids.sort_by(|&a, &b| {
                self.clubs[b as usize]
                    .record
                    .pct()
                    .partial_cmp(&self.clubs[a as usize].record.pct())
                    .unwrap()
            });
            let top: Vec<ClubId> = ids.iter().take(4).copied().collect();
            let champ = self.mini_playoff(&top, &mut rng);
            champs.insert(name, champ);
            for (rank, &cid) in ids.iter().enumerate() {
                let c = &mut self.clubs[cid as usize];
                let res = if cid == champ {
                    "Champions".to_string()
                } else if rank < 4 {
                    "Playoffs".to_string()
                } else {
                    "Regular season".to_string()
                };
                c.history.push(ClubSeason {
                    season: year,
                    w: c.record.w,
                    l: c.record.l,
                    result: res,
                });
            }
            self.clubs[champ as usize].titles.push(year);
        }
        // EuroLeague
        let mut euro: Vec<ClubId> = self
            .clubs
            .iter()
            .filter(|c| c.tier == 1)
            .map(|c| c.id)
            .collect();
        euro.sort_by(|&a, &b| {
            self.clubs[b as usize]
                .euro_record
                .pct()
                .partial_cmp(&self.clubs[a as usize].euro_record.pct())
                .unwrap()
        });
        let top: Vec<ClubId> = euro.iter().take(4).copied().collect();
        let euro_champ = self.mini_playoff(&top, &mut rng);
        self.clubs[euro_champ as usize].euro_titles.push(year);
        let txt = format!(
            "{} win the {year} EuroLeague title!",
            self.clubs[euro_champ as usize].name
        );
        self.add_news("overseas", txt);
        self.overseas_state.champions = champs;
        self.overseas_state.euro_champion = Some(euro_champ);
        self.overseas_state.done = true;
    }

    fn mini_playoff(&mut self, top: &[ClubId], rng: &mut Rng) -> ClubId {
        if top.len() < 2 {
            return top.first().copied().unwrap_or(0);
        }
        let mut play = |a: ClubId, b: ClubId, this: &mut League, rng: &mut Rng| -> ClubId {
            let (ta, tb) = (this.club_team(a), this.club_team(b));
            let mut wa = 0;
            let mut wb = 0;
            while wa < 2 && wb < 2 {
                let r = simulate(&ta, &tb, this.style.ppg * 0.7, 2.0, 0.2, rng);
                if r.pts[0] > r.pts[1] {
                    wa += 1;
                } else {
                    wb += 1;
                }
            }
            if wa > wb {
                a
            } else {
                b
            }
        };
        if top.len() < 4 {
            return play(top[0], top[1], self, rng);
        }
        let w1 = play(top[0], top[3], self, rng);
        let w2 = play(top[1], top[2], self, rng);
        play(w1, w2, self, rng)
    }

    /// Yearly overseas upkeep: expiring contracts, youth intake, stash decisions.
    pub fn overseas_rollover(&mut self, rng: &mut Rng) {
        if self.clubs.is_empty() {
            return;
        }
        let flow = self.settings.num("realism.international_flow");
        for ci in 0..self.clubs.len() {
            // players whose club contract expired: re-sign or leave
            for id in self.clubs[ci].roster.clone() {
                let alive = self
                    .pm(id)
                    .contract
                    .as_mut()
                    .map(|c| c.tick())
                    .unwrap_or(false);
                if !alive {
                    let ovr = self.p(id).ovr;
                    let age = self.year + 1 - self.p(id).birth_year;
                    let tier = self.clubs[ci].tier;
                    if self.p(id).user_controlled {
                        continue;
                    }
                    if age >= 36 && ovr < 60 && rng.chance(0.6) {
                        self.clubs[ci].roster.retain(|&x| x != id);
                        self.retire_player(id, "age");
                    } else if rng.chance(0.82) {
                        let c = self.overseas_contract(tier, ovr, rng);
                        self.pm(id).contract = Some(c);
                    } else {
                        // move to another club
                        self.clubs[ci].roster.retain(|&x| x != id);
                        self.send_overseas(id, rng);
                    }
                }
            }
            // draft-and-stash: NBA teams call up their stashed players when ready
            for id in self.clubs[ci].roster.clone() {
                let (rights, ovr) = (self.p(id).draft_rights, self.p(id).ovr);
                if let Some(t) = rights {
                    let age = self.year + 1 - self.p(id).birth_year;
                    if (age >= 22 && ovr >= 50 || ovr >= 58)
                        && self.team(t).roster.len() < self.rules.roster_max as usize
                        && !self.user_controls_roster(t)
                        && rng.chance(0.5)
                    {
                        self.clubs[ci].roster.retain(|&x| x != id);
                        let (sal, yrs) = self.rookie_deal_for_stash(id);
                        self.pm(id).draft_rights = None;
                        self.sign_player(id, t, sal, yrs, ContractKind::Rookie, "signing");
                    }
                }
            }
            // youth intake to keep rosters full
            while self.clubs[ci].roster.len() < 10 {
                let tier = self.clubs[ci].tier;
                let country = self.clubs[ci].country.clone();
                let base = match tier {
                    1 => 52.0,
                    2 => 45.0,
                    _ => 40.0,
                };
                let age = rng.range(18, 23) as i32;
                let cur = rng.gauss(base - 4.0, 5.0).clamp(30.0, 70.0);
                let pot = (cur + rng.uniform(2.0, 18.0)).clamp(cur, 92.0);
                let mut spec =
                    GenSpec::new(self.year + 1, age, cur, pot, OriginKind::International);
                spec.country = Some(country);
                let mut p = generate_player(&self.content, rng, 0, &spec, flow);
                p.affiliation = Affiliation::Overseas(ci as ClubId);
                p.origin.school = self.clubs[ci].name.clone();
                p.contract = Some(self.overseas_contract(tier, p.ovr, rng));
                let id = self.add_player(p);
                self.clubs[ci].roster.push(id);
            }
            // trim oversized rosters (release extras to retirement)
            while self.clubs[ci].roster.len() > 15 {
                let worst = self.clubs[ci]
                    .roster
                    .iter()
                    .copied()
                    .filter(|&id| !self.p(id).user_controlled)
                    .min_by_key(|&id| self.p(id).ovr);
                match worst {
                    Some(w) => {
                        self.clubs[ci].roster.retain(|&x| x != w);
                        self.retire_player(w, "released overseas");
                    }
                    None => break,
                }
            }
        }
    }

    fn rookie_deal_for_stash(&self, id: PlayerId) -> (Money, u8) {
        let _ = id;
        (((self.money.cap as f64) * 0.03) as i64, 3)
    }
}
