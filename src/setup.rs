//! Building a brand-new league for any starting year.
//!
//! The starting league is generated from the era's rules and style: the right number of teams
//! (franchises are founded, move and fold across history), players of the right kinds
//! (more big men and post scorers in the 1950s, more shooters today), era-appropriate payrolls
//! and a calendar that begins in the preseason.

use crate::content::Content;
use crate::contract::*;
use crate::economy::*;
use crate::era;
use crate::franchise;
use crate::game::{Cal, Strategy};
use crate::generate::*;
use crate::league::*;
use crate::names;
use crate::player::*;
use crate::progression::age_shift;
use crate::rng::{seed_from_text, Rng};
use crate::settings::{SettingValue, Settings};
use crate::team::*;
use crate::types::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct NewLeagueOptions {
    pub name: String,
    pub year: Season,
    pub seed: String,
    pub preset: Option<String>,
    /// Mod packs as JSON text.
    pub mods: Vec<String>,
    pub overrides: BTreeMap<String, SettingValue>,
}

impl Default for NewLeagueOptions {
    fn default() -> Self {
        NewLeagueOptions {
            name: "My League".into(),
            year: 1996,
            seed: "hardwood".into(),
            preset: None,
            mods: vec![],
            overrides: BTreeMap::new(),
        }
    }
}

const SLOT_MEAN: [f64; 15] = [
    67.0, 63.5, 60.5, 58.0, 56.0, 53.5, 51.5, 49.5, 48.0, 46.5, 45.5, 44.5, 43.5, 42.5, 41.5,
];

pub(crate) fn sample_age(rng: &mut Rng) -> i32 {
    let w = [
        2.0, 3.5, 4.5, 5.0, 6.0, 7.0, 7.5, 8.0, 8.0, 7.5, 6.5, 5.5, 4.5, 3.5, 2.5, 1.8, 1.2, 0.6,
    ];
    19 + rng.weighted(&w) as i32
}

impl League {
    /// Create a new league starting in the preseason of `opts.year`.
    pub fn new(opts: NewLeagueOptions) -> Result<League, String> {
        let mut content = Content::default();
        for m in &opts.mods {
            let warnings = content.apply_mod_json(m)?;
            let _ = warnings;
        }
        let seed = seed_from_text(&opts.seed);
        let mut rng = Rng::new(seed);
        let mut settings = Settings::with_defs(content.settings.clone());
        if let Some(pid) = &opts.preset {
            let preset = content
                .presets
                .iter()
                .find(|p| &p.id == pid)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "There is no preset called '{pid}'. Options: {}.",
                        content
                            .presets
                            .iter()
                            .map(|p| p.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;
            settings.apply_preset(&preset);
        }
        for e in settings.import_values(&opts.overrides) {
            return Err(e);
        }
        if opts.year < 1946 {
            return Err(
                "Professional basketball starts in 1946. Choose a year from 1946 on.".into(),
            );
        }
        if opts.year > 2200 {
            return Err("That's a bit far into the future! Choose a year before 2200.".into());
        }

        let year = opts.year;
        let rules_year = match settings.text("realism.rules_mode").as_str() {
            "modern" => 2024,
            "custom_year" => settings.num("realism.frozen_rules_year") as i32,
            _ => year,
        };
        let mut rules = content.rules(rules_year);
        rules.year = year;
        let style = content.style(rules_year);
        if settings.bool("realism.cap_before_1984") && rules.cap_type == era::CapType::None {
            rules.cap_type = era::CapType::Soft;
        }
        let cap = content.economy.table_cap(year);
        let money = content.economy.season_money(&rules, cap, true);

        let mut league = League {
            version: SAVE_VERSION,
            name: opts.name.clone(),
            seed,
            rng: rng.fork("league"),
            settings,
            mods: opts.mods.clone(),
            content,
            start_year: year,
            year,
            phase: Phase::Preseason,
            day: 0,
            rules,
            style,
            money,
            cap_history: vec![(year, cap)],
            revenue_index: 1.0,
            cal: Cal::default(),
            refs: Default::default(),
            teams: vec![],
            players: vec![],
            people: vec![],
            free_agents: vec![],
            schedule: vec![],
            playoffs: None,
            draft: None,
            history: vec![],
            awards: vec![],
            news: vec![],
            transactions: vec![],
            stories: vec![],
            user: UserProfile::default(),
            decisions: vec![],
            next_decision: 1,
            box_log: vec![],
            hall_of_fame: vec![],
            colleges: vec![],
            clubs: vec![],
            records: Default::default(),
            events_fired: vec![],
            season_mods: BTreeMap::new(),
            games_this_season: 0,
            trade_deadline_day: 0,
            all_star_day: 0,
            playoff_results_cache: BTreeMap::new(),
            fa_day_index: 0,
            college_state: Default::default(),
            overseas_state: Default::default(),
        };
        league.games_this_season = league.rules.games;

        league.create_teams(&mut rng)?;
        league.create_staff_pool(&mut rng);
        league.create_players(&mut rng);
        league.create_picks();
        league.init_amateur_world(&mut rng);
        league.begin_season_prep();
        let champ_text = format!(
            "{} begins: {} teams, {} games each, in the {} era.",
            league.rules.league_name,
            league.active_team_ids().len(),
            league.rules.games,
            era_label(year)
        );
        league.add_news("major", champ_text);
        Ok(league)
    }

    // ------------------------------------------------------------------ teams

    fn create_teams(&mut self, rng: &mut Rng) -> Result<(), String> {
        let include_spec = self.settings.bool("realism.historical_events") && self.year >= 2026;
        let active = franchise::active_in(&self.content.franchises, self.year, include_spec);
        if active.len() < 4 {
            return Err("The content set has fewer than 4 franchises for that year. Check your franchise mods.".into());
        }
        let mut idents: Vec<(usize, franchise::Identity)> = active
            .iter()
            .map(|&i| (i, self.content.franchises[i].identity_in(self.year).clone()))
            .collect();
        // Conference: eastern half vs western half.
        idents.sort_by(|a, b| b.1.lon.partial_cmp(&a.1.lon).unwrap());
        let n = idents.len();
        let east_n = n.div_ceil(2);
        let mut conf_of: Vec<u8> = vec![0; n];
        for (i, c) in conf_of.iter_mut().enumerate() {
            *c = if i < east_n { 0 } else { 1 };
        }
        // Divisions within each conference by latitude.
        let mut div_of: Vec<u8> = vec![0; n];
        for conf in 0..2u8 {
            let mut members: Vec<usize> = (0..n).filter(|&i| conf_of[i] == conf).collect();
            members.sort_by(|&a, &b| idents[b].1.lat.partial_cmp(&idents[a].1.lat).unwrap());
            let ndiv = if n >= 20 {
                (members.len() as f64 / 5.0).round().max(1.0) as usize
            } else {
                1
            };
            let per = members.len().div_ceil(ndiv.max(1));
            for (k, &m) in members.iter().enumerate() {
                div_of[m] = conf * 3 + (k / per.max(1)) as u8;
            }
        }
        for (k, (fi, ident)) in idents.iter().enumerate() {
            self.build_team_shell(rng, *fi, ident.clone(), conf_of[k], div_of[k]);
        }
        Ok(())
    }

    /// Create an empty team (owner, budget, arena, finances) for a franchise identity.
    pub fn build_team_shell(
        &mut self,
        rng: &mut Rng,
        fi: usize,
        ident: franchise::Identity,
        conf: u8,
        div: u8,
    ) -> TeamId {
        let money = self.money.clone();
        let id = self.teams.len() as TeamId;
        let fr = &self.content.franchises[fi];
        let owner = make_owner(rng, ident.market);
        let budget = default_budget(rng, ident.market, &money, owner.wealth);
        let cap_arena = era::interp(
            &[
                (1946, 6500.0),
                (1960, 9000.0),
                (1975, 14000.0),
                (1990, 17500.0),
                (2000, 19000.0),
                (2024, 19500.0),
            ],
            self.year as f64,
        );
        let arena = Arena {
            name: format!(
                "{} {}",
                ident.city,
                if rng.chance(0.5) { "Arena" } else { "Coliseum" }
            ),
            capacity: (cap_arena * (0.88 + 0.22 * ident.market) * rng.uniform(0.95, 1.05)) as u32,
            built: self.year - rng.range(0, 35) as i32,
            quality: rng.gauss(55.0, 15.0).clamp(15.0, 95.0),
        };
        let strat = random_strategy(rng, self.year);
        let team = Team {
            id,
            franchise: fr.key.clone(),
            city: ident.city.clone(),
            nickname: ident.nickname.clone(),
            abbr: ident.abbr.clone(),
            lon: ident.lon,
            lat: ident.lat,
            market: ident.market,
            conf,
            div,
            roster: vec![],
            gleague: vec![],
            picks: vec![],
            gm: None,
            head_coach: None,
            assistants: vec![],
            scouts: vec![],
            trainer: None,
            owner,
            budget,
            finance: Financials {
                season: self.year,
                ..Default::default()
            },
            finance_history: vec![],
            arena,
            hype: rng.gauss(55.0, 15.0).clamp(10.0, 95.0),
            fan_loyalty: rng.gauss(50.0, 12.0).clamp(15.0, 90.0),
            chemistry: 55.0,
            direction: Direction::Retool,
            record: Record::default(),
            history: vec![],
            titles: vec![],
            retired_numbers: vec![],
            strategy: strat,
            starters_override: vec![],
            minutes_override: BTreeMap::new(),
            dev_focus_default: DevFocus::Balanced,
            dead_money: vec![],
            exceptions: Exceptions::default(),
            tax_years: 0,
            active: true,
        };
        self.teams.push(team);
        id
    }

    // ------------------------------------------------------------------ people (staff)

    pub fn new_person(
        &mut self,
        rng: &mut Rng,
        role: StaffRole,
        quality: f64,
        team: Option<TeamId>,
    ) -> PersonId {
        let id = self.people.len() as PersonId;
        let pool = &self.content.pools[0];
        let (first, last) = names::random_name(rng, pool);
        let g = |rng: &mut Rng, m: f64| rng.gauss(m, 11.0).clamp(15.0, 98.0);
        let age = match role {
            StaffRole::HeadCoach => rng.range(36, 62),
            StaffRole::GeneralManager => rng.range(38, 64),
            _ => rng.range(28, 62),
        } as i32;
        let salary_scale = self.money.avg_salary as f64 / 1_000_000.0 * 1.4;
        let salary = match role {
            StaffRole::HeadCoach => 0.45 + quality / 100.0 * 1.6,
            StaffRole::GeneralManager => 0.35 + quality / 100.0 * 1.2,
            StaffRole::AssistantCoach => 0.10 + quality / 100.0 * 0.3,
            StaffRole::Scout => 0.07 + quality / 100.0 * 0.15,
            StaffRole::Medical => 0.08 + quality / 100.0 * 0.2,
        } * salary_scale
            * 1_000_000.0;
        let person = Person {
            id,
            name: format!("{first} {last}"),
            born: self.year - age,
            role,
            offense: g(rng, quality),
            defense: g(rng, quality),
            development: g(rng, quality),
            tactics: g(rng, quality),
            motivation: g(rng, quality),
            evaluation: g(rng, quality),
            negotiating: g(rng, quality),
            medical: g(rng, quality),
            salary: salary as i64,
            years_left: rng.range(1, 4) as u8,
            reputation: quality.clamp(10.0, 95.0),
            team,
            former_player: None,
            philosophy: random_strategy(rng, self.year),
            wins: 0,
            losses: 0,
            titles: 0,
            years_in_role: rng.range(0, 8) as u16,
            retired: false,
        };
        self.people.push(person);
        id
    }

    /// Hire a full front office for one team.
    pub fn create_staff_for_team(&mut self, rng: &mut Rng, t: TeamId) {
        let mkt = self.team(t).market;
        let wealth = self.team(t).owner.wealth;
        let q = |rng: &mut Rng| {
            (rng.gauss(52.0, 11.0) + (mkt - 0.5) * 8.0 + (wealth - 50.0) * 0.08).clamp(25.0, 90.0)
        };
        let qv = q(rng);
        let gm = self.new_person(rng, StaffRole::GeneralManager, qv, Some(t));
        let qv = q(rng);
        let hc = self.new_person(rng, StaffRole::HeadCoach, qv, Some(t));
        let mut asst = vec![];
        for _ in 0..3 {
            let qv = q(rng);
            asst.push(self.new_person(rng, StaffRole::AssistantCoach, qv, Some(t)));
        }
        let mut scouts = vec![];
        for _ in 0..3 {
            let qv = q(rng);
            scouts.push(self.new_person(rng, StaffRole::Scout, qv, Some(t)));
        }
        let qv = q(rng);
        let trainer = self.new_person(rng, StaffRole::Medical, qv, Some(t));
        let mut s = self.person(hc).philosophy.clone();
        s.tactics = self.person(hc).tactics;
        let tm = self.team_mut(t);
        tm.gm = Some(gm);
        tm.head_coach = Some(hc);
        tm.assistants = asst;
        tm.scouts = scouts;
        tm.trainer = Some(trainer);
        tm.strategy = s;
    }

    fn create_staff_pool(&mut self, rng: &mut Rng) {
        let ids = self.active_team_ids();
        for t in ids {
            self.create_staff_for_team(rng, t);
        }
        // Coach strategy follows the head coach's philosophy.
        let ids = self.active_team_ids();
        for t in ids {
            if let Some(hc) = self.team(t).head_coach {
                let mut s = self.person(hc).philosophy.clone();
                s.tactics = self.person(hc).tactics;
                self.team_mut(t).strategy = s;
            }
        }
        // Unattached pool.
        for _ in 0..14 {
            let q = rng.gauss(48.0, 11.0).clamp(25.0, 85.0);
            self.new_person(rng, StaffRole::HeadCoach, q, None);
        }
        for _ in 0..8 {
            let q = rng.gauss(48.0, 11.0).clamp(25.0, 85.0);
            self.new_person(rng, StaffRole::GeneralManager, q, None);
        }
        for _ in 0..24 {
            let q = rng.gauss(48.0, 11.0).clamp(25.0, 85.0);
            self.new_person(rng, StaffRole::AssistantCoach, q, None);
        }
        for _ in 0..16 {
            let q = rng.gauss(48.0, 11.0).clamp(25.0, 85.0);
            self.new_person(rng, StaffRole::Scout, q, None);
        }
        for _ in 0..6 {
            let q = rng.gauss(48.0, 11.0).clamp(25.0, 85.0);
            self.new_person(rng, StaffRole::Medical, q, None);
        }
    }

    // ------------------------------------------------------------------ players

    pub fn add_player(&mut self, mut p: Player) -> PlayerId {
        let id = self.players.len() as PlayerId;
        p.id = id;
        self.players.push(p);
        id
    }

    /// Sample how a pro player entered the game, by era.
    pub fn sample_origin(&self, rng: &mut Rng) -> OriginKind {
        let y = self.year;
        let hs = if self.rules.hs_allowed {
            era::interp(&[(1975, 0.01), (1995, 0.05), (2005, 0.07)], y as f64)
        } else {
            0.0
        };
        let intl = era::international_share(y) * self.settings.num("realism.international_flow");
        let g = if self.rules.g_league { 0.04 } else { 0.0 };
        let r = rng.f64();
        if r < hs {
            OriginKind::HighSchool
        } else if r < hs + intl {
            OriginKind::International
        } else if r < hs + intl + g {
            OriginKind::GLeague
        } else if r < hs + intl + g + 0.04 {
            OriginKind::Undrafted
        } else {
            OriginKind::College
        }
    }

    /// Generate a full roster for one team. `quality` shifts every player's talent (0 = average).
    pub fn create_roster_for_team(
        &mut self,
        rng: &mut Rng,
        t: TeamId,
        quality: f64,
        roster_n: usize,
    ) {
        let flow = self.settings.num("realism.international_flow");
        let money = self.money.clone();
        let mut positions: Vec<Position> = vec![];
        while positions.len() < roster_n {
            let mut cycle = Position::ALL.to_vec();
            rng.shuffle(&mut cycle);
            positions.extend(cycle);
        }
        for slot in 0..roster_n {
            let mut peak = SLOT_MEAN[slot.min(14)] + quality + rng.gauss(0.0, 5.0);
            if slot == 0 {
                peak += rng.exp(6.0);
            } else if slot == 1 {
                peak += rng.exp(2.5);
            }
            let age = sample_age(rng);
            let ovr_now = peak + age_shift(age);
            let pot = if age < 26 {
                peak + rng.uniform(-2.0, 4.0)
            } else {
                ovr_now
            };
            let origin = self.sample_origin(rng);
            let mut spec = GenSpec::new(
                self.year,
                age,
                ovr_now.clamp(30.0, 97.0),
                pot.clamp(30.0, 99.0),
                origin,
            );
            spec.position = Some(positions[slot]);
            let mut p = generate_player(&self.content, rng, 0, &spec, flow);
            p.years_pro = (age - 20 - rng.range(0, 1) as i32).max(0) as u8;
            p.affiliation = Affiliation::Nba(t);
            p.origin.school = self.school_name_for(rng, origin);
            p.fitness = rng.uniform(80.0, 100.0) as f32;
            let mut c = market_contract(&self.content, &money, &p, self.year, rng);
            if p.years_pro <= 2 && age <= 23 {
                c.kind = ContractKind::Rookie;
                for s in c.salaries.iter_mut() {
                    *s = (*s as f64 * 0.65) as i64;
                }
            }
            p.contract = Some(c);
            let id = self.add_player(p);
            self.team_mut(t).roster.push(id);
        }
    }

    fn create_players(&mut self, rng: &mut Rng) {
        let ids = self.active_team_ids();
        let roster_n = (self.rules.roster_max as usize)
            .saturating_sub(1)
            .max(self.rules.roster_min as usize)
            .min(15);
        let parity = self.settings.num("sim.parity");
        let flow = self.settings.num("realism.international_flow");
        let quality: Vec<f64> = ids
            .iter()
            .map(|_| rng.gauss(0.0, 1.0) * (1.0 - parity * 0.9) * 2.4)
            .collect();
        for (ti, &t) in ids.iter().enumerate() {
            self.create_roster_for_team(rng, t, quality[ti], roster_n);
        }
        // Scale payrolls so the league average sits near the cap.
        let teams = self.active_team_ids();
        let total: i64 = teams.iter().map(|&t| self.payroll(t)).sum();
        let avg = total / teams.len().max(1) as i64;
        let target = (self.money.cap as f64 * 0.97) as i64;
        let f = target as f64 / avg.max(1) as f64;
        let (minsal, maxsal) = (
            self.money.min_salary,
            self.content.economy.max_salary(&self.money, 10),
        );
        for &t in &teams {
            let roster = self.team(t).roster.clone();
            for id in roster {
                let rules_max = self.rules.max_contract;
                let p = self.pm(id);
                if let Some(c) = &mut p.contract {
                    for s in c.salaries.iter_mut() {
                        let mut v = (*s as f64 * f) as i64;
                        v = v.max(minsal);
                        if rules_max {
                            v = v.min(maxsal);
                        }
                        *s = v;
                    }
                }
            }
        }
        // A few free agents.
        for _ in 0..(teams.len() * 3 / 2) {
            let age = rng.range(23, 34) as i32;
            let ovr = rng.gauss(44.0, 4.0).clamp(33.0, 56.0);
            let origin = self.sample_origin(rng);
            let spec = GenSpec::new(self.year, age, ovr, ovr, origin);
            let mut p = generate_player(&self.content, rng, 0, &spec, flow);
            p.years_pro = (age - 21).max(0) as u8;
            p.affiliation = Affiliation::FreeAgent;
            let id = self.add_player(p);
            self.free_agents.push(id);
        }
        // Initial team strategies/chemistry/direction
        for &t in &teams {
            let r = self.team_rating(t);
            let avg = self.avg_team_rating();
            let dir = if r > avg + 2.0 {
                Direction::Contend
            } else if r < avg - 2.5 {
                Direction::Rebuild
            } else {
                Direction::Retool
            };
            self.team_mut(t).direction = dir;
        }
    }

    pub fn school_name_for(&self, rng: &mut Rng, origin: OriginKind) -> String {
        match origin {
            OriginKind::College => {
                let cn = names::college_names();
                let (n, _) = rng.pick(&cn).clone();
                n
            }
            OriginKind::HighSchool => {
                let (a, b) = names::school_words();
                format!("{} {}", rng.pick(&a), rng.pick(&b))
            }
            OriginKind::International => {
                let suf = names::club_suffixes();
                format!(
                    "{} {}",
                    rng.pick(&self.content.countries)
                        .towns
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "Athens".into()),
                    rng.pick(&suf)
                )
            }
            OriginKind::GLeague => "Development League".into(),
            OriginKind::Undrafted => "Undrafted".into(),
        }
    }

    pub fn create_picks(&mut self) {
        let ids = self.active_team_ids();
        for &t in &ids {
            for y in (self.year + 1)..=(self.year + 4) {
                self.add_picks_for(t, y);
            }
        }
    }

    pub fn add_picks_for(&mut self, t: TeamId, draft_year: Season) {
        let rounds = self
            .content
            .rules(self.rules_year_for(draft_year))
            .draft_rounds;
        for r in 1..=rounds {
            self.team_mut(t).picks.push(DraftPick {
                year: draft_year,
                round: r,
                original: t,
                owner: t,
                protection: 0,
            });
        }
    }
}

pub(crate) fn make_owner(rng: &mut Rng, market: f64) -> Owner {
    let first = rng.pick(&names::builtin_pools()[0].first).clone();
    let last = rng.pick(&names::builtin_pools()[0].last).clone();
    Owner {
        name: format!("{first} {last}"),
        wealth: (rng.gauss(45.0, 20.0) + market * 25.0).clamp(5.0, 100.0),
        patience: rng.gauss(55.0, 20.0).clamp(10.0, 95.0),
        win_now: rng.gauss(0.55, 0.2).clamp(0.05, 0.98),
        meddling: rng.gauss(40.0, 22.0).clamp(0.0, 95.0),
        approval: 62.0,
        mandate: "Make the playoffs".into(),
        mandate_wins: 0,
        years_owned: rng.range(0, 25) as u16,
    }
}

pub(crate) fn default_budget(
    rng: &mut Rng,
    market: f64,
    money: &SeasonMoney,
    wealth: f64,
) -> Budget {
    let cap = money.cap as f64;
    Budget {
        payroll_target: (cap * (0.88 + 0.22 * market + rng.gauss(0.0, 0.05))) as i64,
        tax_tolerance: (cap * 0.05 * (0.5 + wealth / 60.0)) as i64,
        coaching: (0.8 + 0.4 * market + rng.gauss(0.0, 0.08)).clamp(0.5, 1.8),
        medical: (0.8 + 0.4 * market + rng.gauss(0.0, 0.08)).clamp(0.5, 1.8),
        scouting: (0.8 + 0.4 * market + rng.gauss(0.0, 0.08)).clamp(0.5, 1.8),
        facilities: (0.8 + 0.4 * market + rng.gauss(0.0, 0.08)).clamp(0.5, 1.8),
        marketing: (0.8 + 0.4 * market + rng.gauss(0.0, 0.08)).clamp(0.5, 1.8),
        ticket_price: 1.0,
        concession_price: 1.0,
    }
}

pub(crate) fn random_strategy(rng: &mut Rng, year: Season) -> Strategy {
    use crate::game::DefScheme;
    let def = if year >= 2001 && rng.chance(0.12) {
        DefScheme::Zone
    } else if rng.chance(0.07) {
        DefScheme::Press
    } else {
        DefScheme::Man
    };
    Strategy {
        tempo: rng.gauss(0.0, 0.45).clamp(-1.0, 1.0),
        three_emphasis: rng.gauss(0.0, 0.45).clamp(-1.0, 1.0),
        inside_focus: rng.gauss(0.5, 0.2).clamp(0.0, 1.0),
        defense: def,
        crash_glass: rng.gauss(0.3, 0.15).clamp(0.0, 1.0),
        hack_a: rng.chance(0.08),
        tactics: 50.0,
    }
}

pub fn era_label(year: Season) -> &'static str {
    match year {
        ..=1953 => "dawn-of-the-league",
        1954..=1966 => "big-man shot-clock",
        1967..=1978 => "ABA-merger",
        1979..=1991 => "Showtime-and-rivalries",
        1992..=2003 => "physical isolation",
        2004..=2013 => "pace-and-space dawn",
        _ => "three-point revolution",
    }
}
