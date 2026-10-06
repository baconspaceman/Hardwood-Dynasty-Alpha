//! The League: the whole living world in one struct.
//!
//! A `League` owns every team, player, coach, schedule, history book and the human's career
//! profile. Other files add behaviour to it with more `impl League` blocks:
//! * `setup.rs`    - building a brand-new league for any starting year
//! * `season.rs`   - schedules, daily simulation, standings
//! * `playoffs.rs` - play-in and playoff brackets
//! * `offseason.rs`- progression, retirement, contracts, free agency
//! * `draft.rs`    - draft classes, lottery and the draft itself
//! * `trade.rs`    - trade AI and cap rules
//! * `finance.rs`  - revenue, expenses, owner budgets
//! * ...and more (college, overseas, career, story, import).

use crate::awards::AwardResult;
use crate::content::Content;
use crate::economy::SeasonMoney;
use crate::era::{EraStyle, Rules};
use crate::game::{Cal, Refs};
use crate::player::*;
use crate::rng::Rng;
use crate::settings::Settings;
use crate::team::*;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SAVE_VERSION: u32 = 1;

/// What the human can be. A career may hold several roles at once (an Owner can also act as GM).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Player,
    Gm,
    HeadCoach,
    AssistantCoach,
    Scout,
    Owner,
    CollegeCoach,
    CollegeAd,
    CollegeScout,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Player => "Player",
            Role::Gm => "General Manager",
            Role::HeadCoach => "Head Coach",
            Role::AssistantCoach => "Assistant Coach",
            Role::Scout => "Pro Scout",
            Role::Owner => "Owner",
            Role::CollegeCoach => "College Head Coach",
            Role::CollegeAd => "College Athletic Director",
            Role::CollegeScout => "College Scout",
        }
    }
    pub fn parse(s: &str) -> Option<Role> {
        match s.trim().to_lowercase().replace([' ', '-'], "_").as_str() {
            "player" => Some(Role::Player),
            "gm" | "general_manager" => Some(Role::Gm),
            "coach" | "head_coach" => Some(Role::HeadCoach),
            "assistant" | "assistant_coach" => Some(Role::AssistantCoach),
            "scout" | "pro_scout" => Some(Role::Scout),
            "owner" => Some(Role::Owner),
            "college_coach" | "college_head_coach" => Some(Role::CollegeCoach),
            "ad" | "college_ad" | "athletic_director" | "college_owner" => Some(Role::CollegeAd),
            "college_scout" => Some(Role::CollegeScout),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub name: String,
    pub roles: Vec<Role>,
    pub team: Option<TeamId>,
    pub college: Option<CollegeId>,
    /// The human's own created player (Player role).
    pub player: Option<PlayerId>,
    /// 0-100: how the league sees you; drives job offers.
    pub reputation: f64,
    pub log: Vec<String>,
    /// Owner mode: let the AI handle the GM / coaching jobs.
    pub delegate_gm: bool,
    pub delegate_coach: bool,
    pub auto_decisions: bool,
    /// Prospects the user scouts (for scouting familiarity): id -> level 0-100.
    pub scouted: BTreeMap<PlayerId, u8>,
    pub scouting_points: f64,
}

impl Default for UserProfile {
    fn default() -> Self {
        UserProfile {
            name: "You".into(),
            roles: vec![],
            team: None,
            college: None,
            player: None,
            reputation: 40.0,
            log: vec![],
            delegate_gm: false,
            delegate_coach: false,
            auto_decisions: false,
            scouted: BTreeMap::new(),
            scouting_points: 0.0,
        }
    }
}

impl UserProfile {
    pub fn has(&self, r: Role) -> bool {
        self.roles.contains(&r)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsItem {
    pub season: Season,
    pub day: u32,
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fixture {
    pub day: u16,
    pub home: TeamId,
    pub away: TeamId,
    pub done: bool,
    pub home_pts: u16,
    pub away_pts: u16,
    pub overtimes: u8,
    /// "" regular season; other tags for special games (e.g. "cup").
    pub tag: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SeasonSummary {
    pub season: Season,
    pub league_name: String,
    pub champion: String,
    pub champion_id: Option<TeamId>,
    pub runner_up: String,
    pub finals_result: String,
    pub finals_mvp: String,
    pub mvp: String,
    pub dpoy: String,
    pub roy: String,
    pub scoring_leader: String,
    pub best_record: String,
    pub teams: usize,
    pub games: u16,
    pub cap: Money,
    pub avg_ppg: f64,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub season: Season,
    pub day: u32,
    pub kind: String,
    pub text: String,
    pub teams: Vec<TeamId>,
}

/// Something the human must decide before the calendar can move on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    pub id: u32,
    pub kind: String,
    pub title: String,
    pub text: String,
    pub options: Vec<DecisionOption>,
    /// Optional payload (player ids etc.).
    pub subject: Option<PlayerId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionOption {
    pub id: String,
    pub label: String,
    pub explain: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct League {
    pub version: u32,
    pub name: String,
    pub seed: u64,
    pub rng: Rng,
    pub settings: Settings,
    /// Mod packs (JSON text) applied on top of the built-in content. Saved so the game can rebuild.
    pub mods: Vec<String>,
    #[serde(skip)]
    pub content: Content,
    pub start_year: Season,
    pub year: Season,
    pub phase: Phase,
    /// Day within the season (regular season day number; playoffs continue counting).
    pub day: u32,
    pub rules: Rules,
    pub style: EraStyle,
    pub money: SeasonMoney,
    /// Economic history: cap-equivalent by season.
    pub cap_history: Vec<(Season, Money)>,
    /// League revenue index (1.0 at start). Drives simulated cap growth.
    pub revenue_index: f64,
    pub cal: Cal,
    pub refs: Refs,
    pub teams: Vec<Team>,
    pub players: Vec<Player>,
    pub people: Vec<Person>,
    pub free_agents: Vec<PlayerId>,
    pub schedule: Vec<Fixture>,
    pub playoffs: Option<crate::playoffs::PlayoffState>,
    pub draft: Option<crate::draft::DraftState>,
    pub history: Vec<SeasonSummary>,
    pub awards: Vec<AwardResult>,
    pub news: Vec<NewsItem>,
    pub transactions: Vec<Transaction>,
    pub stories: Vec<crate::story::Story>,
    pub user: UserProfile,
    pub decisions: Vec<Decision>,
    pub next_decision: u32,
    /// Recent box scores (league-wide, trimmed).
    pub box_log: Vec<crate::game::BoxScore>,
    pub hall_of_fame: Vec<PlayerId>,
    pub colleges: Vec<crate::college::College>,
    pub clubs: Vec<crate::overseas::Club>,
    pub records: crate::records::RecordBook,
    /// Leagues events that already fired (ids).
    pub events_fired: Vec<String>,
    /// Season-level modifiers set by events (cleared each season).
    pub season_mods: BTreeMap<String, f64>,
    /// Games per team this season (can be changed by events).
    pub games_this_season: u16,
    pub trade_deadline_day: u16,
    pub all_star_day: u16,
    pub playoff_results_cache: BTreeMap<TeamId, String>,
    pub fa_day_index: u32,
    pub college_state: crate::college::CollegeState,
    pub overseas_state: crate::overseas::OverseasState,
}

impl League {
    // ------------------------------------------------------------------ lookups

    pub fn p(&self, id: PlayerId) -> &Player {
        &self.players[id as usize]
    }
    pub fn pm(&mut self, id: PlayerId) -> &mut Player {
        &mut self.players[id as usize]
    }
    pub fn team(&self, id: TeamId) -> &Team {
        &self.teams[id as usize]
    }
    pub fn team_mut(&mut self, id: TeamId) -> &mut Team {
        &mut self.teams[id as usize]
    }
    pub fn person(&self, id: PersonId) -> &Person {
        &self.people[id as usize]
    }
    pub fn active_team_ids(&self) -> Vec<TeamId> {
        self.teams
            .iter()
            .filter(|t| t.active)
            .map(|t| t.id)
            .collect()
    }
    pub fn find_team(&self, key: &str) -> Option<TeamId> {
        let k = key.trim().to_lowercase();
        self.teams
            .iter()
            .filter(|t| t.active)
            .find(|t| {
                t.abbr.to_lowercase() == k
                    || t.city.to_lowercase() == k
                    || t.nickname.to_lowercase() == k
                    || t.name().to_lowercase() == k
            })
            .or_else(|| {
                self.teams
                    .iter()
                    .filter(|t| t.active)
                    .find(|t| t.name().to_lowercase().contains(&k))
            })
            .map(|t| t.id)
    }
    pub fn find_player(&self, q: &str) -> Option<PlayerId> {
        if let Ok(id) = q.trim().parse::<u32>() {
            if (id as usize) < self.players.len() {
                return Some(id);
            }
        }
        let k = q.trim().to_lowercase();
        let exact = self
            .players
            .iter()
            .find(|p| p.name().to_lowercase() == k && !p.is_retired());
        exact
            .or_else(|| self.players.iter().find(|p| p.name().to_lowercase() == k))
            .or_else(|| {
                self.players
                    .iter()
                    .find(|p| p.last.to_lowercase() == k && !p.is_retired())
            })
            .or_else(|| {
                self.players
                    .iter()
                    .find(|p| p.name().to_lowercase().contains(&k) && !p.is_retired())
            })
            .map(|p| p.id)
    }
    pub fn age_of(&self, id: PlayerId) -> i32 {
        self.year - self.p(id).birth_year
    }
    pub fn user_team(&self) -> Option<TeamId> {
        self.user.team
    }
    pub fn is_user_team(&self, t: TeamId) -> bool {
        self.user.team == Some(t)
    }

    /// Does the human control this team's roster decisions (trades, signings)?
    pub fn user_controls_roster(&self, t: TeamId) -> bool {
        self.user.team == Some(t)
            && (self.user.has(Role::Gm) || (self.user.has(Role::Owner) && !self.user.delegate_gm))
    }
    /// Does the human control this team's lineup/minutes/strategy?
    pub fn user_controls_lineup(&self, t: TeamId) -> bool {
        self.user.team == Some(t)
            && (self.user.has(Role::HeadCoach)
                || (self.user.has(Role::Owner) && !self.user.delegate_coach))
    }

    // ------------------------------------------------------------------ news & logs

    pub fn add_news(&mut self, kind: &str, text: impl Into<String>) {
        if !self.settings.bool("story.media") && kind != "major" {
            return;
        }
        self.news.push(NewsItem {
            season: self.year,
            day: self.day,
            kind: kind.into(),
            text: text.into(),
        });
        if self.news.len() > 4000 {
            self.news.drain(0..1000);
        }
    }

    pub fn add_transaction(&mut self, kind: &str, text: impl Into<String>, teams: Vec<TeamId>) {
        self.transactions.push(Transaction {
            season: self.year,
            day: self.day,
            kind: kind.into(),
            text: text.into(),
            teams,
        });
        if self.transactions.len() > 6000 {
            self.transactions.drain(0..1500);
        }
    }

    pub fn push_decision(
        &mut self,
        kind: &str,
        title: &str,
        text: &str,
        options: Vec<DecisionOption>,
        subject: Option<PlayerId>,
    ) -> u32 {
        let id = self.next_decision;
        self.next_decision += 1;
        self.decisions.push(Decision {
            id,
            kind: kind.into(),
            title: title.into(),
            text: text.into(),
            options,
            subject,
        });
        id
    }

    // ------------------------------------------------------------------ team numbers

    pub fn payroll(&self, t: TeamId) -> Money {
        let team = self.team(t);
        let mut sum: Money = team
            .roster
            .iter()
            .chain(team.gleague.iter())
            .map(|&id| self.p(id).current_salary())
            .sum();
        sum += team.dead_money_total(self.year);
        sum
    }

    /// Healthy rotation players sorted best first.
    pub fn rotation(&self, t: TeamId) -> Vec<PlayerId> {
        let mut v: Vec<PlayerId> = self
            .team(t)
            .roster
            .iter()
            .copied()
            .filter(|&id| !self.p(id).is_injured())
            .collect();
        v.sort_by(|&a, &b| self.p(b).ovr.cmp(&self.p(a).ovr));
        v
    }

    /// A single number for team quality: weighted average of the best ten healthy players.
    pub fn team_rating(&self, t: TeamId) -> f64 {
        self.rating_of(&self.rotation(t))
    }

    pub fn team_rating_full_health(&self, t: TeamId) -> f64 {
        let mut v: Vec<PlayerId> = self.team(t).roster.clone();
        v.sort_by(|&a, &b| self.p(b).ovr.cmp(&self.p(a).ovr));
        self.rating_of(&v)
    }

    fn rating_of(&self, ids: &[PlayerId]) -> f64 {
        const W: [f64; 10] = [
            0.185, 0.16, 0.135, 0.115, 0.1, 0.085, 0.07, 0.055, 0.0575, 0.0375,
        ];
        let mut total = 0.0;
        for (i, w) in W.iter().enumerate() {
            let o = ids.get(i).map(|&id| self.p(id).ovr as f64).unwrap_or(38.0);
            total += o * w;
        }
        // Star bonus: a superstar lifts a team more than the weighted average says.
        let best = ids.first().map(|&id| self.p(id).ovr as f64).unwrap_or(40.0);
        total + (best - 70.0).max(0.0) * 0.12
    }

    pub fn avg_team_rating(&self) -> f64 {
        let ids = self.active_team_ids();
        if ids.is_empty() {
            return 55.0;
        }
        ids.iter().map(|&t| self.team_rating(t)).sum::<f64>() / ids.len() as f64
    }

    /// Approximate win probability for the home team over a neutral average opponent rating gap.
    pub fn win_prob(&self, home: f64, away: f64) -> f64 {
        let diff = (home - away) * 0.62 + self.settings.num("sim.home_court") * 0.4;
        // 1 rating point ≈ 0.62 net-rating points; 1 point of margin ≈ 3% win probability.
        let x = diff / 11.5 / (self.settings.num("sim.randomness").max(0.2));
        1.0 / (1.0 + (-1.702 * x).exp())
    }

    pub fn coach_of(&self, t: TeamId) -> Option<&Person> {
        self.team(t).head_coach.map(|id| self.person(id))
    }

    pub fn gm_of(&self, t: TeamId) -> Option<&Person> {
        self.team(t).gm.map(|id| self.person(id))
    }

    pub fn medical_quality(&self, t: TeamId) -> f64 {
        let tm = self.team(t);
        let base = tm.trainer.map(|id| self.person(id).medical).unwrap_or(45.0);
        (base * (0.85 + 0.15 * tm.budget.medical)).clamp(10.0, 100.0)
    }

    pub fn coaching_quality(&self, t: TeamId) -> f64 {
        let tm = self.team(t);
        let head = tm
            .head_coach
            .map(|id| self.person(id).development)
            .unwrap_or(45.0);
        let asst: f64 = if tm.assistants.is_empty() {
            45.0
        } else {
            tm.assistants
                .iter()
                .map(|&a| self.person(a).development)
                .sum::<f64>()
                / tm.assistants.len() as f64
        };
        ((0.55 * head + 0.45 * asst) * (0.9 + 0.1 * tm.budget.coaching)).clamp(10.0, 100.0)
    }

    pub fn scouting_quality(&self, t: TeamId) -> f64 {
        let tm = self.team(t);
        if tm.scouts.is_empty() {
            return 45.0;
        }
        let avg = tm
            .scouts
            .iter()
            .map(|&s| self.person(s).evaluation)
            .sum::<f64>()
            / tm.scouts.len() as f64;
        let gm = tm.gm.map(|g| self.person(g).evaluation).unwrap_or(45.0);
        ((0.65 * avg + 0.35 * gm) * (0.85 + 0.15 * tm.budget.scouting)).clamp(10.0, 100.0)
    }

    /// Standings position helpers.
    pub fn standings(&self, conf: Option<u8>) -> Vec<TeamId> {
        let mut v: Vec<TeamId> = self
            .teams
            .iter()
            .filter(|t| t.active && conf.map(|c| t.conf == c).unwrap_or(true))
            .map(|t| t.id)
            .collect();
        v.sort_by(|&a, &b| {
            let (ra, rb) = (&self.team(a).record, &self.team(b).record);
            rb.pct()
                .partial_cmp(&ra.pct())
                .unwrap()
                .then(rb.diff_pg().partial_cmp(&ra.diff_pg()).unwrap())
        });
        v
    }

    pub fn conference_name(&self, c: u8) -> &'static str {
        if c == 0 {
            "East"
        } else {
            "West"
        }
    }

    pub fn rebuild_content(&mut self) {
        let mut c = Content::default();
        for m in &self.mods {
            let _ = c.apply_mod_json(m);
        }
        self.settings.set_defs(c.settings.clone());
        self.content = c;
    }

    /// Resolve which year's rules apply to a season, honouring the "rule set" setting.
    pub fn rules_year_for(&self, season: Season) -> Season {
        match self.settings.text("realism.rules_mode").as_str() {
            "modern" => 2024.max(season.min(2024)),
            "custom_year" => self.settings.num("realism.frozen_rules_year") as i32,
            _ => season,
        }
    }

    pub fn save_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Could not save: {e}"))
    }

    pub fn load_json(json: &str) -> Result<League, String> {
        let mut l: League =
            serde_json::from_str(json).map_err(|e| format!("Could not read the save file: {e}"))?;
        if l.version > SAVE_VERSION {
            return Err(format!(
                "This save is from a newer version of the game (save v{}, game v{}).",
                l.version, SAVE_VERSION
            ));
        }
        l.rebuild_content();
        Ok(l)
    }

    pub fn player_stats_line(&self, id: PlayerId, season: Season) -> Option<&SeasonRecord> {
        self.p(id)
            .seasons
            .iter()
            .rev()
            .find(|s| s.season == season && s.level == Level::Pro)
    }
}
