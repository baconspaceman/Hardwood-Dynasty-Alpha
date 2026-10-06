//! Teams, front-office staff, owners and finances.
//!
//! A team is more than a roster: it has an owner with a personality and a budget, a general
//! manager, head coach and assistants, scouts and a medical staff (each with ratings that matter),
//! an arena, fans, a payroll, and a financial statement.

use crate::game::{DefScheme, Strategy};
use crate::player::DevFocus;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaffRole {
    GeneralManager,
    HeadCoach,
    AssistantCoach,
    Scout,
    Medical,
}

impl StaffRole {
    pub fn name(self) -> &'static str {
        match self {
            StaffRole::GeneralManager => "General Manager",
            StaffRole::HeadCoach => "Head Coach",
            StaffRole::AssistantCoach => "Assistant Coach",
            StaffRole::Scout => "Scout",
            StaffRole::Medical => "Head Trainer",
        }
    }
}

/// A non-player person in the league: coaches, executives, scouts, trainers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub name: String,
    pub born: i32,
    pub role: StaffRole,
    /// Ratings 0-100.
    pub offense: f64,
    pub defense: f64,
    pub development: f64,
    pub tactics: f64,
    pub motivation: f64,
    pub evaluation: f64,
    pub negotiating: f64,
    pub medical: f64,
    pub salary: Money,
    pub years_left: u8,
    pub reputation: f64,
    /// Team employing him (None = available).
    pub team: Option<TeamId>,
    /// If this person was a player, the original player id.
    pub former_player: Option<PlayerId>,
    pub philosophy: Strategy,
    /// Wins/losses/titles as head coach.
    pub wins: u32,
    pub losses: u32,
    pub titles: u16,
    pub years_in_role: u16,
    pub retired: bool,
}

impl Person {
    pub fn overall(&self) -> f64 {
        match self.role {
            StaffRole::GeneralManager => {
                0.4 * self.evaluation + 0.3 * self.negotiating + 0.3 * self.development
            }
            StaffRole::HeadCoach => {
                0.3 * self.tactics
                    + 0.2 * self.offense
                    + 0.2 * self.defense
                    + 0.15 * self.development
                    + 0.15 * self.motivation
            }
            StaffRole::AssistantCoach => {
                0.35 * self.development
                    + 0.25 * self.offense
                    + 0.25 * self.defense
                    + 0.15 * self.motivation
            }
            StaffRole::Scout => self.evaluation,
            StaffRole::Medical => self.medical,
        }
    }
    pub fn age(&self, season: Season) -> i32 {
        season - self.born
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Owner {
    pub name: String,
    /// Net worth in billions of today's dollars equivalent (0-100 scale for simplicity).
    pub wealth: f64,
    /// 0-100: how long he tolerates losing.
    pub patience: f64,
    /// 0 = only cares about profit, 1 = only wants to win.
    pub win_now: f64,
    /// 0-100: how much he overrules the GM/coach.
    pub meddling: f64,
    /// Current approval of the front office 0-100.
    pub approval: f64,
    /// Mandate for the season, in plain English.
    pub mandate: String,
    pub mandate_wins: u16,
    pub years_owned: u16,
}

/// What the owner lets the club spend. Set by the owner (you, in Owner mode) or by the AI owner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Budget {
    /// Payroll the owner wants to stay near.
    pub payroll_target: Money,
    /// Maximum luxury-tax bill the owner will accept.
    pub tax_tolerance: Money,
    /// 0.5 - 2.0 spending multipliers vs. a typical team.
    pub coaching: f64,
    pub medical: f64,
    pub scouting: f64,
    pub facilities: f64,
    pub marketing: f64,
    /// Ticket price level: 1.0 = typical for the market.
    pub ticket_price: f64,
    pub concession_price: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Financials {
    pub season: Season,
    pub gate: Money,
    pub concessions: Money,
    pub local_tv: Money,
    pub national_tv: Money,
    pub merchandise: Money,
    pub sponsors: Money,
    pub playoffs: Money,
    pub revenue_sharing: Money,
    pub payroll: Money,
    pub luxury_tax: Money,
    pub staff: Money,
    pub facilities: Money,
    pub marketing: Money,
    pub operations: Money,
    pub minor_league: Money,
    pub home_games: u16,
    pub attendance_total: u64,
}

impl Financials {
    pub fn revenue(&self) -> Money {
        self.gate
            + self.concessions
            + self.local_tv
            + self.national_tv
            + self.merchandise
            + self.sponsors
            + self.playoffs
            + self.revenue_sharing
    }
    pub fn expenses(&self) -> Money {
        self.payroll
            + self.luxury_tax
            + self.staff
            + self.facilities
            + self.marketing
            + self.operations
            + self.minor_league
    }
    pub fn profit(&self) -> Money {
        self.revenue() - self.expenses()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Arena {
    pub name: String,
    pub capacity: u32,
    pub built: Season,
    /// 0-100 quality: amenities, atmosphere.
    pub quality: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Contend,
    Retool,
    Rebuild,
}

impl Direction {
    pub fn name(self) -> &'static str {
        match self {
            Direction::Contend => "Contending",
            Direction::Retool => "Retooling",
            Direction::Rebuild => "Rebuilding",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Record {
    pub w: u16,
    pub l: u16,
    pub home_w: u16,
    pub home_l: u16,
    pub conf_w: u16,
    pub conf_l: u16,
    pub pf: u32,
    pub pa: u32,
    pub streak: i16,
    pub last10: Vec<bool>,
}

impl Record {
    pub fn games(&self) -> u16 {
        self.w + self.l
    }
    pub fn pct(&self) -> f64 {
        if self.games() == 0 {
            0.5
        } else {
            self.w as f64 / self.games() as f64
        }
    }
    pub fn diff_pg(&self) -> f64 {
        if self.games() == 0 {
            0.0
        } else {
            (self.pf as f64 - self.pa as f64) / self.games() as f64
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TeamSeason {
    pub season: Season,
    pub name: String,
    pub w: u16,
    pub l: u16,
    pub seed: Option<u8>,
    /// Furthest playoff round reached: 0 = missed, 1 = made it, ... N = champion (see `playoff_result`).
    pub playoff_result: String,
    pub champion: bool,
    pub payroll: Money,
    pub profit: Money,
    pub coach: String,
    pub best_player: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DraftPick {
    pub year: Season,
    pub round: u8,
    pub original: TeamId,
    pub owner: TeamId,
    /// Protected if it lands in the top N (then converts to a later pick). 0 = unprotected.
    pub protection: u8,
}

/// Cap exceptions available this season (dollars). Reset every year.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Exceptions {
    pub mle_used: Money,
    pub min_signings: u8,
    pub trade_exception: Money,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Team {
    pub id: TeamId,
    pub franchise: String,
    pub city: String,
    pub nickname: String,
    pub abbr: String,
    pub lon: f64,
    pub lat: f64,
    /// Market size 0-1.
    pub market: f64,
    pub conf: u8,
    pub div: u8,
    pub roster: Vec<PlayerId>,
    pub gleague: Vec<PlayerId>,
    pub picks: Vec<DraftPick>,
    pub gm: Option<PersonId>,
    pub head_coach: Option<PersonId>,
    pub assistants: Vec<PersonId>,
    pub scouts: Vec<PersonId>,
    pub trainer: Option<PersonId>,
    pub owner: Owner,
    pub budget: Budget,
    pub finance: Financials,
    pub finance_history: Vec<Financials>,
    pub arena: Arena,
    /// 0-100 how excited the fanbase is.
    pub hype: f64,
    /// 0-100 how deep loyalty runs (slow-moving).
    pub fan_loyalty: f64,
    pub chemistry: f64,
    pub direction: Direction,
    pub record: Record,
    pub history: Vec<TeamSeason>,
    pub titles: Vec<Season>,
    pub retired_numbers: Vec<(String, u8)>,
    pub strategy: Strategy,
    /// Manual starters and minutes set by a human coach.
    pub starters_override: Vec<PlayerId>,
    pub minutes_override: BTreeMap<PlayerId, f64>,
    pub dev_focus_default: DevFocus,
    pub dead_money: Vec<(Season, Money)>,
    pub exceptions: Exceptions,
    /// Consecutive seasons over the tax line (for the repeater penalty).
    pub tax_years: u8,
    pub active: bool,
}

impl Team {
    pub fn name(&self) -> String {
        format!("{} {}", self.city, self.nickname)
    }
    pub fn dead_money_total(&self, season: Season) -> Money {
        self.dead_money
            .iter()
            .filter(|(y, _)| *y == season)
            .map(|(_, m)| *m)
            .sum()
    }
}

pub fn default_strategy_from(tempo: f64, three: f64, inside: f64, def: DefScheme) -> Strategy {
    Strategy {
        tempo,
        three_emphasis: three,
        inside_focus: inside,
        defense: def,
        crash_glass: 0.3,
        hack_a: false,
        tactics: 50.0,
    }
}
