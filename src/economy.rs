//! Money: the salary cap, luxury tax, aprons, minimum/maximum contracts and the pay scale.
//!
//! Real cap numbers are embedded for 1984-2025 (a modder can replace the whole table).
//! Before 1984 there was no cap, so we use an "average salary" curve instead, and treat the
//! total of one team's payroll at average pay as the "cap-equivalent" for sizing everything.
//! After the last known season the cap keeps moving using league revenue growth (see `league`).
//!
//! All money in the engine is whole US dollars stored in `i64`.

use crate::era::{interp, CapType, Rules};
use serde::{Deserialize, Serialize};

pub use crate::types::Money;

pub const M: i64 = 1_000_000;

/// (season start, salary cap, luxury tax line) in millions of dollars. Tax 0 = no tax that year.
pub fn builtin_cap_table() -> Vec<CapRow> {
    let rows: &[(i32, f64, f64)] = &[
        (1984, 3.6, 0.0),
        (1985, 4.233, 0.0),
        (1986, 4.945, 0.0),
        (1987, 6.164, 0.0),
        (1988, 7.232, 0.0),
        (1989, 9.802, 0.0),
        (1990, 11.871, 0.0),
        (1991, 12.5, 0.0),
        (1992, 14.0, 0.0),
        (1993, 15.175, 0.0),
        (1994, 15.964, 0.0),
        (1995, 15.964, 0.0),
        (1996, 24.363, 0.0),
        (1997, 26.9, 0.0),
        (1998, 30.0, 0.0),
        (1999, 34.0, 0.0),
        (2000, 35.5, 0.0),
        (2001, 42.5, 52.9),
        (2002, 40.271, 52.8),
        (2003, 43.84, 54.6),
        (2004, 43.87, 61.7),
        (2005, 49.5, 61.7),
        (2006, 53.135, 65.4),
        (2007, 55.63, 67.865),
        (2008, 58.68, 71.15),
        (2009, 57.7, 69.92),
        (2010, 58.044, 70.307),
        (2011, 58.044, 70.307),
        (2012, 58.044, 70.307),
        (2013, 58.679, 71.748),
        (2014, 63.065, 76.829),
        (2015, 70.0, 84.74),
        (2016, 94.143, 113.287),
        (2017, 99.093, 119.266),
        (2018, 101.869, 123.733),
        (2019, 109.14, 132.627),
        (2020, 109.14, 132.627),
        (2021, 112.414, 136.606),
        (2022, 123.655, 150.267),
        (2023, 136.021, 165.294),
        (2024, 140.588, 170.814),
        (2025, 154.647, 187.895),
    ];
    rows.iter()
        .map(|r| CapRow {
            year: r.0,
            cap: (r.1 * M as f64) as i64,
            tax: (r.2 * M as f64) as i64,
        })
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CapRow {
    pub year: i32,
    pub cap: Money,
    pub tax: Money,
}

/// Average player salary before the cap era (dollars). Rough history of pro basketball pay.
pub fn builtin_avg_salary_curve() -> Vec<(i32, f64)> {
    vec![
        (1946, 4_000.0),
        (1950, 7_500.0),
        (1955, 9_500.0),
        (1960, 13_000.0),
        (1965, 20_000.0),
        (1970, 45_000.0),
        (1975, 120_000.0),
        (1980, 215_000.0),
        (1984, 330_000.0),
    ]
}

/// The economy tables a league uses. Mods may replace any of it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomyTables {
    pub cap_table: Vec<CapRow>,
    pub avg_salary_pre_cap: Vec<(i32, f64)>,
    /// Max salary as % of cap by years of service: (min_years, pct).
    pub max_pct: Vec<(u8, f64)>,
    /// Revenue per team in "cap multiples" (how many caps of revenue a typical club earns).
    pub revenue_cap_multiple: f64,
    /// Tax rate per $1 over the line (incremental brackets are approximated by this plus bracket step).
    pub tax_rate_base: f64,
    pub tax_rate_repeater: f64,
    /// First and second apron as ratio of the tax line.
    pub first_apron_ratio: f64,
    pub second_apron_ratio: f64,
}

impl Default for EconomyTables {
    fn default() -> Self {
        EconomyTables {
            cap_table: builtin_cap_table(),
            avg_salary_pre_cap: builtin_avg_salary_curve(),
            max_pct: vec![(0, 0.25), (7, 0.30), (10, 0.35)],
            revenue_cap_multiple: 1.65,
            tax_rate_base: 1.5,
            tax_rate_repeater: 2.5,
            first_apron_ratio: 1.0425,
            second_apron_ratio: 1.106,
        }
    }
}

/// The money numbers for one season, resolved from the tables and the league's simulated state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SeasonMoney {
    pub year: i32,
    /// "Cap-equivalent": the cap if there is one, otherwise the era's typical payroll.
    pub cap: Money,
    /// Is a cap actually enforced this year?
    pub cap_enforced: bool,
    pub tax_line: Money,
    pub first_apron: Money,
    pub second_apron: Money,
    pub min_salary: Money,
    pub max_salary_base: Money,
    /// Typical (average) salary of a rotation player.
    pub avg_salary: Money,
}

impl EconomyTables {
    /// Cap-equivalent from tables only (no simulation). After the table's end, grow at a default 6%.
    pub fn table_cap(&self, year: i32) -> Money {
        if let Some(first) = self.cap_table.first() {
            if year >= first.year {
                if let Some(r) = self.cap_table.iter().find(|r| r.year == year) {
                    return r.cap;
                }
                let last = self.cap_table.last().unwrap();
                if year > last.year {
                    return (last.cap as f64 * 1.06f64.powi(year - last.year)) as i64;
                }
            }
        }
        // Before the cap era: payroll scale = average salary × 12 players.
        (interp(&self.avg_salary_pre_cap, year as f64) * 12.0) as i64
    }

    pub fn table_tax(&self, year: i32) -> Money {
        if let Some(r) = self.cap_table.iter().find(|r| r.year == year) {
            if r.tax > 0 {
                return r.tax;
            }
        }
        (self.table_cap(year) as f64 * 1.215) as i64
    }

    /// Resolve the money picture for a year given the (possibly simulated) cap-equivalent.
    pub fn season_money(&self, rules: &Rules, cap: Money, tax_from_table: bool) -> SeasonMoney {
        let year = rules.year;
        let cap_enforced = rules.cap_type == CapType::Soft;
        let tax_line = if rules.luxury_tax {
            if tax_from_table {
                self.table_tax(year)
            } else {
                (cap as f64 * 1.215) as i64
            }
        } else {
            i64::MAX / 4
        };
        let (first_apron, second_apron) = if rules.aprons {
            (
                (tax_line as f64 * self.first_apron_ratio) as i64,
                (tax_line as f64 * self.second_apron_ratio) as i64,
            )
        } else {
            (i64::MAX / 4, i64::MAX / 4)
        };
        // Minimum salary as % of cap: 2.0% at 1984 → 1.0% by 1996 → 0.82% modern.
        let min_pct = interp(
            &[
                (1984, 0.020),
                (1996, 0.0100),
                (2010, 0.0085),
                (2024, 0.0082),
            ],
            year as f64,
        );
        let (min_salary, avg_salary) = if year >= 1984 {
            ((cap as f64 * min_pct) as i64, (cap as f64 / 11.5) as i64)
        } else {
            let avg = interp(&self.avg_salary_pre_cap, year as f64);
            ((avg * 0.4) as i64, avg as i64)
        };
        let max_base = if rules.max_contract {
            (cap as f64 * self.max_pct[0].1) as i64
        } else {
            (cap as f64 * 0.38) as i64
        };
        SeasonMoney {
            year,
            cap,
            cap_enforced,
            tax_line,
            first_apron,
            second_apron,
            min_salary,
            max_salary_base: max_base,
            avg_salary,
        }
    }

    /// Max salary as a % of the cap depending on years of service.
    pub fn max_salary(&self, money: &SeasonMoney, years_service: u8) -> Money {
        let mut pct = self.max_pct[0].1;
        for (y, p) in &self.max_pct {
            if years_service >= *y {
                pct = *p;
            }
        }
        (money.cap as f64 * pct) as i64
    }

    /// Luxury tax owed for a payroll. Incremental brackets per $5M over (modern style),
    /// with a repeater surcharge.
    pub fn luxury_tax(&self, money: &SeasonMoney, payroll: Money, repeater: bool) -> Money {
        if payroll <= money.tax_line {
            return 0;
        }
        let over = (payroll - money.tax_line) as f64;
        let rate = if repeater {
            self.tax_rate_repeater
        } else {
            self.tax_rate_base
        };
        // bracket step: every 5/136 of cap over adds +0.25 to the rate (scales with the cap)
        let bracket = money.cap as f64 * (5.0 / 136.0);
        let mut tax = 0.0;
        let mut remaining = over;
        let mut r = rate;
        while remaining > 0.0 {
            let slice = remaining.min(bracket);
            tax += slice * r;
            remaining -= slice;
            r += 0.25;
        }
        tax as i64
    }
}

/// How much a player of a given overall is "worth" as a fraction of the cap-equivalent.
/// This is the *market value curve* that everything (contracts, AI, owner budgets) hangs from.
/// Calibrated so that a typical 15-man league payroll lands near the cap.
pub fn market_value_pct(ovr: f64, age: f64, potential: f64) -> f64 {
    // Value blends current ability with potential for the young.
    let youth = ((24.0 - age) / 6.0).clamp(0.0, 1.0);
    let eff = ovr + (potential - ovr).max(0.0) * 0.35 * youth;
    // Aging discount for older players.
    let old = ((age - 31.0) / 7.0).clamp(0.0, 1.0);
    let eff = eff - old * 6.0;
    let x = (eff - 50.0) / 28.0;
    // 0.8% (min) at 50 up to ~35% at 95+ via a convex curve.
    let pct = 0.008 + 0.34 * x.max(0.0).powf(2.3);
    pct.clamp(0.008, 0.40)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::era::rules_for;

    #[test]
    fn cap_table_known_values() {
        let t = EconomyTables::default();
        assert_eq!(t.table_cap(1984), 3_600_000);
        assert_eq!(t.table_cap(2024), 140_588_000);
        assert!(t.table_cap(2030) > t.table_cap(2025));
        // pre-cap payroll scale is far below the 1984 cap
        assert!(t.table_cap(1950) < 150_000);
    }

    #[test]
    fn tax_and_aprons_follow_rules() {
        let t = EconomyTables::default();
        let r = rules_for(2024);
        let m = t.season_money(&r, t.table_cap(2024), true);
        assert!(m.tax_line > m.cap);
        assert!(m.first_apron > m.tax_line && m.second_apron > m.first_apron);
        assert_eq!(t.luxury_tax(&m, m.tax_line - 1, false), 0);
        assert!(t.luxury_tax(&m, m.tax_line + 10 * M, false) > 15 * M);
        assert!(
            t.luxury_tax(&m, m.tax_line + 10 * M, true)
                > t.luxury_tax(&m, m.tax_line + 10 * M, false)
        );
        let old = t.season_money(&rules_for(1990), t.table_cap(1990), true);
        assert!(old.tax_line > old.cap * 100); // no tax in 1990
    }

    #[test]
    fn value_curve_is_monotone() {
        let mut last = 0.0;
        for o in 40..99 {
            let v = market_value_pct(o as f64, 27.0, o as f64);
            assert!(v >= last);
            last = v;
        }
        assert!(market_value_pct(95.0, 27.0, 95.0) > 0.25);
        assert!(market_value_pct(45.0, 27.0, 45.0) < 0.012);
    }
}
