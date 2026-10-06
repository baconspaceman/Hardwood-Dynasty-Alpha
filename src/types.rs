//! Small shared types used everywhere.

use serde::{Deserialize, Serialize};

pub type PlayerId = u32;
pub type TeamId = u16;
pub type CollegeId = u16;
pub type ClubId = u16;
pub type PersonId = u32;
/// A season is named by the year it starts: 1996 means 1996-97.
pub type Season = i32;
/// Money is always whole US dollars.
pub type Money = i64;

/// Where in the yearly calendar the league is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Offseason after the playoffs: awards, retirements, contract expiry decisions.
    PostSeason,
    /// Draft lottery and draft.
    Draft,
    /// Free agency period.
    FreeAgency,
    /// Training camp and preseason; rosters must be cut down.
    Preseason,
    RegularSeason,
    PlayIn,
    Playoffs,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::PostSeason => "Post-season",
            Phase::Draft => "Draft",
            Phase::FreeAgency => "Free agency",
            Phase::Preseason => "Preseason",
            Phase::RegularSeason => "Regular season",
            Phase::PlayIn => "Play-in",
            Phase::Playoffs => "Playoffs",
        }
    }
}

pub fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Format dollars compactly: $1.2M, $850K, $4,500.
pub fn fmt_money(m: Money) -> String {
    let a = m.abs() as f64;
    let sign = if m < 0 { "-" } else { "" };
    if a >= 1e9 {
        format!("{sign}${:.2}B", a / 1e9)
    } else if a >= 1e6 {
        format!("{sign}${:.1}M", a / 1e6)
    } else if a >= 1e4 {
        format!("{sign}${:.0}K", a / 1e3)
    } else {
        format!("{sign}${}", a as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn money_format() {
        assert_eq!(fmt_money(1_250_000), "$1.2M");
        assert_eq!(fmt_money(850_000), "$850K");
        assert_eq!(fmt_money(4_500), "$4500");
        assert_eq!(fmt_money(-3_000_000), "-$3.0M");
    }
}
