//! Eras: the rules of basketball, year by year.
//!
//! Basketball changed enormously between 1946 and today: no shot clock until 1954, no
//! three-pointer until 1979, hand-checking until 2004, a salary cap only since 1984...
//! This module holds that history as DATA (a timeline of "rule patches"), then answers the
//! question "what are the rules in season X?" by applying every patch up to X in order.
//!
//! Because the timeline is plain data, a mod can add, replace or remove patches (see
//! `content.rs`) to invent entirely new eras, e.g. "a league where the 3-pointer is worth 4".
//!
//! Season naming: a season is identified by the year it STARTS. `1996` means 1996-97.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lottery {
    /// Worst record picks first (before 1966).
    WorstFirst,
    /// Coin flip between the worst teams of each conference (1966-1984).
    CoinFlip,
    /// Original envelope lottery, 1985-1989.
    Envelope,
    /// Weighted lottery, 1990-1993.
    Weighted,
    /// Revised weighted lottery, 1994-2018.
    Weighted94,
    /// "Flattened" odds, 2019+: the three worst teams share the best odds.
    Flattened,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaRegime {
    /// Players are bound to their team forever (the "reserve clause") until traded or released.
    Reserve,
    /// Players can leave after their option year, but the old team can match any offer (right of first refusal).
    FirstRefusal,
    /// Modern free agency: unrestricted, with restricted free agency for rookies.
    Modern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapType {
    /// No salary cap: teams pay what they can afford.
    None,
    /// Soft cap with exceptions (Bird rights, mid-level...).
    Soft,
}

/// Generates the `Rules` struct, a matching `RulesPatch` (all fields optional) and `apply`.
macro_rules! rules {
    ($( $(#[$m:meta])* $name:ident : $ty:ty = $default:expr ),* $(,)?) => {
        /// The complete rule book for one season.
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
        pub struct Rules {
            /// The season these rules were resolved for (start year).
            pub year: i32,
            $( $(#[$m])* pub $name: $ty, )*
        }

        impl Rules {
            fn base(year: i32) -> Rules {
                Rules { year, $( $name: $default, )* }
            }
        }

        /// A set of changes to the rule book. Only the fields that are `Some` change.
        #[derive(Clone, Debug, Default, Serialize, Deserialize)]
        #[serde(default)]
        pub struct RulesPatch {
            $( pub $name: Option<$ty>, )*
        }

        impl RulesPatch {
            pub fn apply(&self, r: &mut Rules) {
                $( if let Some(v) = &self.$name { r.$name = v.clone(); } )*
            }
        }
    };
}

rules! {
    /// Name of the top league ("BAA", "NBA"...).
    league_name: String = "BAA".to_string(),
    /// Regular-season games per team.
    games: u16 = 60,
    quarter_minutes: f64 = 12.0,
    overtime_minutes: f64 = 5.0,
    /// Seconds on the shot clock; 0 means there is no shot clock.
    shot_clock: u8 = 0,
    /// Seconds the shot clock resets to after an offensive rebound (0 = same as full clock).
    shot_clock_reset_orb: u8 = 0,
    three_point: bool = false,
    /// Distance of the three-point line in feet.
    three_distance: f64 = 23.75,
    three_value: u8 = 3,
    hand_checking: bool = true,
    /// Zone defense legal? (Illegal-defense rules existed until 2001.)
    zone_defense: bool = false,
    defensive_three_seconds: bool = false,
    lane_width: u8 = 6,
    foul_limit: u8 = 6,
    /// Team fouls in a quarter before the other team shoots bonus free throws.
    bonus_fouls: u8 = 6,
    roster_max: u8 = 10,
    roster_min: u8 = 8,
    /// Players allowed to dress for a game.
    active_max: u8 = 10,
    two_way_slots: u8 = 0,
    /// Teams that make the playoffs (after any play-in).
    playoff_teams: u8 = 6,
    /// Best-of length for each playoff round, first round first.
    series_lengths: Vec<u8> = vec![3, 5, 7],
    play_in: bool = false,
    draft_rounds: u8 = 10,
    lottery: Lottery = Lottery::WorstFirst,
    /// "Territorial picks": teams could claim a local college star instead of drafting (until 1965).
    territorial_picks: bool = false,
    /// May a player be drafted straight out of high school?
    hs_allowed: bool = false,
    /// Minimum age to be drafted (calendar year of draft minus birth year).
    min_draft_age: u8 = 21,
    /// Underclassmen allowed to leave college early ("hardship" in the 1970s).
    early_entry: bool = false,
    fa_regime: FaRegime = FaRegime::Reserve,
    cap_type: CapType = CapType::None,
    rookie_scale: bool = false,
    max_contract: bool = false,
    luxury_tax: bool = false,
    aprons: bool = false,
    bird_rights: bool = false,
    mid_level_exception: bool = false,
    sign_and_trade: bool = false,
    g_league: bool = false,
    replay_review: bool = false,
    coaches_challenge: bool = false,
    /// Late-game intentional-foul restrictions (2016 rule).
    intentional_foul_rule: bool = false,
    flop_penalties: bool = false,
    /// "Freedom of movement" emphasis: fewer hand-check/contact fouls allowed on the perimeter.
    freedom_of_movement: bool = false,
    all_star_game: bool = false,
    in_season_tournament: bool = false,
    /// Free throw / foul / shot-clock tuning knobs (mods may change; 1.0 = normal).
    foul_multiplier: f64 = 1.0,
}

/// One entry in the timeline of rule changes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleChange {
    /// Season (start year) the change takes effect.
    pub year: i32,
    /// Unique id so mods can replace or remove it.
    pub id: String,
    pub title: String,
    pub description: String,
    pub patch: RulesPatch,
}

fn rc(year: i32, id: &str, title: &str, description: &str, patch: RulesPatch) -> RuleChange {
    RuleChange { year, id: id.into(), title: title.into(), description: description.into(), patch }
}

/// The built-in rule timeline. Years are season start years.
pub fn builtin_timeline() -> Vec<RuleChange> {
    vec![
        rc(1946, "baa-founded", "Basketball Association of America tips off",
            "The BAA begins: 48-minute games, no shot clock, no three-point line, a reserve clause binding players to teams.",
            RulesPatch { ..Default::default() }),
        rc(1947, "baa-48-games", "Short season", "The 1947-48 season is cut to 48 games.",
            RulesPatch { games: Some(48), ..Default::default() }),
        rc(1948, "baa-60-games-again", "Schedule back to 60", "Teams play 60 games.",
            RulesPatch { games: Some(60), roster_max: Some(11), active_max: Some(11), ..Default::default() }),
        rc(1949, "nba-merger", "The NBA is born", "The BAA absorbs the National Basketball League and becomes the NBA. Longer schedule, bigger rosters.",
            RulesPatch { league_name: Some("NBA".into()), games: Some(68), roster_max: Some(12), active_max: Some(12), playoff_teams: Some(8), ..Default::default() }),
        rc(1950, "all-star-game", "First All-Star Game", "The league holds its first All-Star Game (1951).",
            RulesPatch { all_star_game: Some(true), ..Default::default() }),
        rc(1951, "lane-12", "Lane widened to 12 feet", "The key widens from 6 to 12 feet to slow down towering centers.",
            RulesPatch { lane_width: Some(12), ..Default::default() }),
        rc(1954, "shot-clock", "The 24-second shot clock",
            "Games had been collapsing into stalling and fouling. The 24-second clock arrives and scoring explodes.",
            RulesPatch { shot_clock: Some(24), games: Some(72), bonus_fouls: Some(6), playoff_teams: Some(6), ..Default::default() }),
        rc(1958, "round-sizes-58", "Four-round... three-round playoffs", "Playoff formats settle into divisional rounds.",
            RulesPatch { playoff_teams: Some(8), series_lengths: Some(vec![5, 7, 7]), ..Default::default() }),
        rc(1952, "territorial-picks", "Territorial draft picks", "Teams may claim a local college star instead of using a regular pick.",
            RulesPatch { territorial_picks: Some(true), draft_rounds: Some(15), ..Default::default() }),
        rc(1961, "schedule-80", "80-game schedule", "Expansion and growth push the schedule to 79-80 games.",
            RulesPatch { games: Some(79), ..Default::default() }),
        rc(1962, "schedule-80b", "80 games", "The schedule becomes 80 games.", RulesPatch { games: Some(80), ..Default::default() }),
        rc(1964, "lane-16", "Lane widened to 16 feet", "The key is widened again, to its modern 16 feet.",
            RulesPatch { lane_width: Some(16), ..Default::default() }),
        rc(1966, "coin-flip", "Draft order by coin flip", "The worst team in each conference flips a coin for the first pick. Territorial picks are abolished.",
            RulesPatch { lottery: Some(Lottery::CoinFlip), territorial_picks: Some(false), ..Default::default() }),
        rc(1967, "schedule-82", "82-game season", "The schedule reaches the modern 82 games in 1967-68.",
            RulesPatch { games: Some(82), ..Default::default() }),
        rc(1969, "playoffs-7s", "Best-of-seven rounds", "All playoff rounds are best-of-seven.",
            RulesPatch { series_lengths: Some(vec![7, 7, 7]), all_star_game: Some(true), ..Default::default() }),
        rc(1971, "hardship", "Hardship draft", "After the Spencer Haywood decision, players can leave college early to turn pro.",
            RulesPatch { early_entry: Some(true), min_draft_age: Some(19), ..Default::default() }),
        rc(1974, "playoff-12", "Twelve-team playoffs", "Bigger league, bigger playoffs, with a short first round.",
            RulesPatch { playoff_teams: Some(12), series_lengths: Some(vec![3, 7, 7, 7]), draft_rounds: Some(10), ..Default::default() }),
        rc(1975, "hs-to-pro", "High schoolers can turn pro", "The first teenagers jump from high school straight to the league.",
            RulesPatch { hs_allowed: Some(true), min_draft_age: Some(18), ..Default::default() }),
        rc(1976, "free-agency-ofr", "Free agency begins", "The Oscar Robertson settlement ends the reserve clause: players can switch teams after their option, but the old team may match offers.",
            RulesPatch { fa_regime: Some(FaRegime::FirstRefusal), roster_max: Some(12), ..Default::default() }),
        rc(1979, "three-point-line", "The three-point line", "The three-point shot arrives in the NBA (23 ft 9 in), a gimmick that will eventually remake the sport.",
            RulesPatch { three_point: Some(true), roster_max: Some(12), active_max: Some(12), ..Default::default() }),
        rc(1981, "bonus-5", "Bonus at five fouls", "Teams go into the bonus earlier.",
            RulesPatch { bonus_fouls: Some(5), ..Default::default() }),
        rc(1983, "cap-agreement", "Salary cap agreement", "Players and owners agree to the first salary cap, starting 1984-85, in exchange for revenue sharing.",
            RulesPatch { ..Default::default() }),
        rc(1984, "salary-cap", "The salary cap", "A soft salary cap begins, with the Larry Bird exception letting teams re-sign their own stars.",
            RulesPatch { cap_type: Some(CapType::Soft), bird_rights: Some(true), playoff_teams: Some(16), series_lengths: Some(vec![5, 7, 7, 7]), all_star_game: Some(true), roster_max: Some(12), ..Default::default() }),
        rc(1985, "lottery", "The draft lottery", "An envelope lottery sets the order for non-playoff teams. The draft stretches to seven rounds.",
            RulesPatch { lottery: Some(Lottery::Envelope), draft_rounds: Some(7), ..Default::default() }),
        rc(1988, "free-agency-modern", "Modern free agency; mid-level exception", "Restricted and unrestricted free agency replace the right of first refusal. Teams get the mid-level exception and sign-and-trade deals.",
            RulesPatch { fa_regime: Some(FaRegime::Modern), mid_level_exception: Some(true), sign_and_trade: Some(true), ..Default::default() }),
        rc(1989, "draft-two-rounds", "Two-round draft", "The draft is cut to two rounds.", RulesPatch { draft_rounds: Some(2), roster_max: Some(13), ..Default::default() }),
        rc(1990, "weighted-lottery", "Weighted lottery", "Odds now depend on record: the worst teams get the best chances.",
            RulesPatch { lottery: Some(Lottery::Weighted), ..Default::default() }),
        rc(1994, "short-three", "Shorter three-point line", "The line moves in to 22 feet all around, and the lottery is revised.",
            RulesPatch { three_distance: Some(22.0), lottery: Some(Lottery::Weighted94), ..Default::default() }),
        rc(1995, "rookie-scale", "Rookie salary scale", "First-round rookie salaries are fixed by draft slot. The league adds Toronto and Vancouver.",
            RulesPatch { rookie_scale: Some(true), roster_max: Some(15), active_max: Some(12), ..Default::default() }),
        rc(1997, "line-restored", "Three-point line restored", "The line goes back to 23 ft 9 in.",
            RulesPatch { three_distance: Some(23.75), ..Default::default() }),
        rc(1999, "max-contract", "Max contracts", "A new labor deal introduces maximum salaries tied to years of service.",
            RulesPatch { max_contract: Some(true), ..Default::default() }),
        rc(2001, "zone-and-tax", "Zone defense legal; luxury tax", "Illegal-defense rules are replaced by a defensive three-second rule, so zones are allowed. The luxury tax begins. A minor league (D-League) launches.",
            RulesPatch { zone_defense: Some(true), defensive_three_seconds: Some(true), luxury_tax: Some(true), g_league: Some(true), ..Default::default() }),
        rc(2003, "bo7-first-round", "Best-of-seven first round", "Every playoff series is now best-of-seven.",
            RulesPatch { series_lengths: Some(vec![7, 7, 7, 7]), ..Default::default() }),
        rc(2004, "no-hand-checking", "Hand-checking banned", "Perimeter contact is called tightly; guards are freed to attack off the dribble.",
            RulesPatch { hand_checking: Some(false), ..Default::default() }),
        rc(2005, "age-limit", "Age limit and 13 active", "Players must be 19 and a year out of high school. Teams may dress 13.",
            RulesPatch { hs_allowed: Some(false), min_draft_age: Some(19), active_max: Some(13), ..Default::default() }),
        rc(2012, "flop-penalty", "Flopping penalties", "Fines for flopping begin.", RulesPatch { flop_penalties: Some(true), ..Default::default() }),
        rc(2014, "replay-center", "Replay center", "Referees get centralized video review.", RulesPatch { replay_review: Some(true), ..Default::default() }),
        rc(2016, "intentional-fouls", "Hack-a-player rules", "Intentional fouls away from the ball in the last two minutes are penalized.",
            RulesPatch { intentional_foul_rule: Some(true), ..Default::default() }),
        rc(2017, "two-way", "Two-way contracts", "Teams can sign two players who shuttle between the league and its minor league.",
            RulesPatch { two_way_slots: Some(2), roster_max: Some(15), ..Default::default() }),
        rc(2018, "freedom-of-movement", "Freedom of movement; 14-second reset", "Referees crack down on contact; the shot clock resets to 14 after an offensive rebound.",
            RulesPatch { freedom_of_movement: Some(true), shot_clock_reset_orb: Some(14), ..Default::default() }),
        rc(2019, "coaches-challenge", "Coach's challenge", "Coaches can challenge calls.", RulesPatch { coaches_challenge: Some(true), lottery: Some(Lottery::Flattened), ..Default::default() }),
        rc(2020, "play-in", "The play-in tournament", "Seeds 7-10 play for the final playoff spots.",
            RulesPatch { play_in: Some(true), ..Default::default() }),
        rc(2023, "aprons-and-cup", "Aprons and in-season tournament", "A new labor deal adds punitive tax 'aprons' and an in-season tournament.",
            RulesPatch { aprons: Some(true), in_season_tournament: Some(true), ..Default::default() }),
    ]
}

/// Resolve the rule book for a season from a timeline (patches applied in year order).
pub fn rules_from(timeline: &[RuleChange], year: i32) -> Rules {
    let mut sorted: Vec<&RuleChange> = timeline.iter().filter(|c| c.year <= year).collect();
    sorted.sort_by_key(|c| c.year);
    let mut r = Rules::base(year);
    for c in sorted {
        c.patch.apply(&mut r);
    }
    r.year = year;
    r
}

/// Convenience: rules from the built-in timeline.
pub fn rules_for(year: i32) -> Rules {
    rules_from(&builtin_timeline(), year)
}

/// Statistical "feel" of an era. All values are league averages that the game engine
/// is calibrated to reproduce.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct EraStyle {
    pub year: i32,
    /// Possessions per 48 minutes, per team.
    pub pace: f64,
    /// Average points per team per game (a calibration target, not an input).
    pub ppg: f64,
    /// Share of field-goal attempts that are threes.
    pub three_rate: f64,
    /// Two-point field-goal percentage.
    pub fg2: f64,
    /// Three-point percentage.
    pub fg3: f64,
    pub ft_pct: f64,
    /// Free throw attempts per field-goal attempt.
    pub ft_rate: f64,
    /// Turnovers per possession.
    pub tov: f64,
    /// Offensive rebound share of misses.
    pub orb: f64,
}

pub fn builtin_style_anchors() -> Vec<EraStyle> {
    let rows: &[(i32, f64, f64, f64, f64, f64, f64, f64, f64, f64)] = &[
        (1946, 80.0, 67.0, 0.0, 0.368, 0.28, 0.66, 0.30, 0.20, 0.38),
        (1950, 90.0, 83.0, 0.0, 0.391, 0.30, 0.73, 0.38, 0.18, 0.37),
        (1954, 77.0, 79.5, 0.0, 0.420, 0.30, 0.74, 0.40, 0.17, 0.37),
        (1955, 94.0, 93.1, 0.0, 0.420, 0.30, 0.75, 0.36, 0.17, 0.37),
        (1956, 106.0, 99.0, 0.0, 0.391, 0.30, 0.75, 0.35, 0.17, 0.37),
        (1960, 126.0, 115.0, 0.0, 0.385, 0.30, 0.74, 0.36, 0.18, 0.37),
        (1962, 125.0, 118.8, 0.0, 0.414, 0.30, 0.73, 0.36, 0.18, 0.37),
        (1964, 119.0, 115.3, 0.0, 0.431, 0.30, 0.74, 0.35, 0.18, 0.36),
        (1967, 125.0, 117.4, 0.0, 0.406, 0.30, 0.72, 0.35, 0.17, 0.36),
        (1970, 108.0, 114.0, 0.0, 0.497, 0.30, 0.74, 0.33, 0.17, 0.34),
        (1975, 102.0, 103.0, 0.0, 0.471, 0.30, 0.75, 0.32, 0.17, 0.32),
        (1980, 103.0, 109.3, 0.027, 0.506, 0.28, 0.75, 0.31, 0.16, 0.31),
        (1985, 102.0, 110.8, 0.06, 0.520, 0.29, 0.76, 0.30, 0.15, 0.30),
        (1990, 98.3, 106.3, 0.09, 0.518, 0.30, 0.75, 0.30, 0.15, 0.30),
        (1995, 92.9, 99.5, 0.16, 0.510, 0.33, 0.74, 0.30, 0.15, 0.29),
        (1996, 91.8, 96.9, 0.20, 0.483, 0.36, 0.74, 0.30, 0.15, 0.29),
        (1999, 88.9, 91.6, 0.17, 0.475, 0.34, 0.73, 0.29, 0.15, 0.29),
        (2004, 90.1, 93.4, 0.20, 0.465, 0.35, 0.75, 0.29, 0.14, 0.28),
        (2008, 91.9, 99.9, 0.22, 0.503, 0.36, 0.76, 0.30, 0.14, 0.27),
        (2012, 91.3, 96.3, 0.25, 0.470, 0.36, 0.77, 0.29, 0.14, 0.27),
        (2015, 93.9, 100.0, 0.31, 0.493, 0.35, 0.75, 0.27, 0.14, 0.26),
        (2017, 96.4, 105.6, 0.35, 0.507, 0.36, 0.77, 0.27, 0.135, 0.25),
        (2019, 100.0, 111.2, 0.38, 0.526, 0.355, 0.77, 0.27, 0.135, 0.25),
        (2022, 98.2, 110.6, 0.40, 0.529, 0.357, 0.78, 0.27, 0.13, 0.25),
        (2024, 98.5, 114.2, 0.42, 0.550, 0.366, 0.78, 0.26, 0.125, 0.25),
    ];
    rows.iter()
        .map(|r| EraStyle { year: r.0, pace: r.1, ppg: r.2, three_rate: r.3, fg2: r.4, fg3: r.5, ft_pct: r.6, ft_rate: r.7, tov: r.8, orb: r.9 })
        .collect()
}

/// Interpolate the era style for a year. Years beyond the last anchor keep the last style
/// (the game engine then lets the league evolve on its own through player skills).
pub fn style_from(anchors: &[EraStyle], year: i32) -> EraStyle {
    let mut a: Vec<&EraStyle> = anchors.iter().collect();
    a.sort_by_key(|s| s.year);
    if a.is_empty() {
        return builtin_style_anchors()[0];
    }
    if year <= a[0].year {
        let mut s = *a[0];
        s.year = year;
        return s;
    }
    if year >= a[a.len() - 1].year {
        let mut s = *a[a.len() - 1];
        s.year = year;
        return s;
    }
    let i = a.iter().position(|s| s.year > year).unwrap();
    let (lo, hi) = (a[i - 1], a[i]);
    let t = (year - lo.year) as f64 / (hi.year - lo.year) as f64;
    let l = |x: f64, y: f64| x + (y - x) * t;
    EraStyle {
        year,
        pace: l(lo.pace, hi.pace),
        ppg: l(lo.ppg, hi.ppg),
        three_rate: l(lo.three_rate, hi.three_rate),
        fg2: l(lo.fg2, hi.fg2),
        fg3: l(lo.fg3, hi.fg3),
        ft_pct: l(lo.ft_pct, hi.ft_pct),
        ft_rate: l(lo.ft_rate, hi.ft_rate),
        tov: l(lo.tov, hi.tov),
        orb: l(lo.orb, hi.orb),
    }
}

pub fn style_for(year: i32) -> EraStyle {
    style_from(&builtin_style_anchors(), year)
}

/// Share of league players born outside the US, by year (rough history).
pub fn international_share(year: i32) -> f64 {
    let pts: &[(i32, f64)] = &[(1946, 0.005), (1970, 0.01), (1980, 0.02), (1990, 0.05), (1995, 0.09), (2000, 0.12), (2005, 0.17), (2010, 0.2), (2015, 0.24), (2020, 0.27), (2030, 0.32)];
    interp(pts, year as f64)
}

/// Piecewise-linear interpolation helper over (x, y) points sorted by x.
pub fn interp(pts: &[(i32, f64)], x: f64) -> f64 {
    if x <= pts[0].0 as f64 {
        return pts[0].1;
    }
    for w in pts.windows(2) {
        let (x0, y0) = (w[0].0 as f64, w[0].1);
        let (x1, y1) = (w[1].0 as f64, w[1].1);
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    pts[pts.len() - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_milestones() {
        assert!(!rules_for(1950).three_point);
        assert_eq!(rules_for(1950).shot_clock, 0);
        assert_eq!(rules_for(1955).shot_clock, 24);
        assert!(rules_for(1979).three_point);
        assert!(!rules_for(1978).three_point);
        assert!(!rules_for(2004).hand_checking);
        assert!(rules_for(2003).hand_checking);
        assert!(rules_for(2020).play_in);
        assert!(!rules_for(2019).play_in);
        assert_eq!(rules_for(1984).cap_type, CapType::Soft);
        assert_eq!(rules_for(1983).cap_type, CapType::None);
        assert!(rules_for(2000).hs_allowed);
        assert!(!rules_for(2006).hs_allowed);
        assert!(!rules_for(1960).hs_allowed);
        assert_eq!(rules_for(2024).draft_rounds, 2);
        assert_eq!(rules_for(1996).series_lengths, vec![5, 7, 7, 7]);
        assert_eq!(rules_for(2003).series_lengths, vec![7, 7, 7, 7]);
        assert_eq!(rules_for(1996).three_distance, 22.0);
        assert_eq!(rules_for(2000).three_distance, 23.75);
    }

    #[test]
    fn style_interpolates() {
        let s = style_for(1997);
        assert!(s.pace > 88.0 && s.pace < 93.0);
        assert_eq!(style_for(1940).three_rate, 0.0);
        assert!(style_for(2030).three_rate > 0.4);
    }
}
