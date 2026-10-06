//! Contracts. A contract is a list of yearly salaries plus special terms.

use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    Standard,
    /// First-round rookie scale (1995+) or negotiated draft contract.
    Rookie,
    Minimum,
    /// Splits time between the pro team and its minor league affiliate (2017+).
    TwoWay,
    TenDay,
    /// Overseas club contract.
    Overseas,
    /// Re-signed using Bird rights (can exceed the cap).
    BirdRights,
    /// Designated veteran "supermax" extension.
    SuperMax,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionKind {
    None,
    /// The player may opt out after the penultimate season.
    Player,
    /// The team may decline the last season.
    Team,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Contract {
    /// Remaining yearly salaries, first = this season.
    pub salaries: Vec<Money>,
    pub kind: ContractKind,
    /// Option attached to the LAST year.
    pub option: OptionKind,
    /// Fully guaranteed? (Non-guaranteed deals can be waived for free before the deadline.)
    pub guaranteed: bool,
    pub no_trade: bool,
    /// Trade kicker bonus % (0 = none).
    pub trade_kicker: f32,
    pub signed: Season,
    /// Total years when signed (for display).
    pub total_years: u8,
}

impl Contract {
    pub fn new(salary_by_year: Vec<Money>, kind: ContractKind, signed: Season) -> Contract {
        let n = salary_by_year.len() as u8;
        Contract {
            salaries: salary_by_year,
            kind,
            option: OptionKind::None,
            guaranteed: true,
            no_trade: false,
            trade_kicker: 0.0,
            signed,
            total_years: n,
        }
    }

    /// Flat or rising contract: first-year salary, `years`, annual raise fraction (0.08 = 8%).
    pub fn rising(
        first: Money,
        years: u8,
        raise: f64,
        kind: ContractKind,
        signed: Season,
    ) -> Contract {
        let mut v = vec![];
        let mut s = first as f64;
        for _ in 0..years.max(1) {
            v.push(s as i64);
            s *= 1.0 + raise;
        }
        Contract::new(v, kind, signed)
    }

    pub fn salary(&self) -> Money {
        self.salaries.first().copied().unwrap_or(0)
    }
    pub fn years_left(&self) -> u8 {
        self.salaries.len() as u8
    }
    pub fn total_remaining(&self) -> Money {
        self.salaries.iter().sum()
    }
    pub fn expiring(&self) -> bool {
        self.salaries.len() <= 1
    }
    /// Advance one season. Returns `false` if the contract has run out.
    pub fn tick(&mut self) -> bool {
        if !self.salaries.is_empty() {
            self.salaries.remove(0);
        }
        !self.salaries.is_empty()
    }
    /// Money still owed if released now (guaranteed deals owe everything).
    pub fn dead_money(&self) -> Money {
        if self.guaranteed {
            self.total_remaining()
        } else {
            0
        }
    }
}

/// A free-agent contract request.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ask {
    pub first_year: Money,
    pub years: u8,
}
