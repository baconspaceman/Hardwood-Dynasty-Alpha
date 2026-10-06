//! The game engine: a possession-by-possession basketball simulation.
//!
//! One call to [`simulate_game`] plays a whole game: 4 quarters (or era-appropriate length),
//! substitutions driven by fatigue / foul trouble / the coach's minutes plan, shot selection by
//! skill, defense, rebounding, free throws, turnovers, steals, blocks, assists, technical and
//! flagrant fouls, late-game fouling, overtime, and (optionally) in-game injuries.
//!
//! The *era* decides how the league plays (pace, 3-point volume, shooting percentages, whether
//! a shot clock or three-point line exists). The *players* decide who does what and how well.
//! Every tunable constant is a named parameter listed by [`default_tuning`] so modders and
//! calibration tests can change it without touching code.

use crate::era::{interp, EraStyle, Rules};
use crate::player::*;
use crate::rng::Rng;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// -------------------------------------------------------------------------------------------
// Tuning parameters
// -------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TuneDef {
    pub key: String,
    pub description: String,
    pub default: f64,
    pub min: f64,
    pub max: f64,
}

fn t(key: &str, desc: &str, default: f64, min: f64, max: f64) -> TuneDef {
    TuneDef { key: key.into(), description: desc.into(), default, min, max }
}

/// Every named engine parameter with its plain-English meaning.
pub fn default_tuning() -> Vec<TuneDef> {
    vec![
        t("game.pace_scale", "Multiplies game tempo. Used to calibrate possessions per game.", 1.0, 0.5, 1.5),
        t("game.skill_slope", "How much player skill moves shooting percentages (1 = normal).", 1.0, 0.3, 2.0),
        t("game.defense_slope", "How much defense matters for shot results.", 1.0, 0.2, 2.0),
        t("game.usage_exponent", "How concentrated shots are on the best scorers. Higher = more star-driven.", 3.0, 1.0, 6.0),
        t("game.three_rate_scale", "Scales how much shooters' skill changes their three-point volume.", 1.0, 0.0, 2.0),
        t("game.fatigue_rate", "How fast energy drains while playing.", 1.0, 0.0, 3.0),
        t("game.fatigue_penalty", "How much tiredness hurts skills.", 1.0, 0.0, 3.0),
        t("game.turnover_scale", "Multiplier on turnover frequency.", 1.0, 0.3, 2.0),
        t("game.foul_scale", "Multiplier on foul frequency (also set by 'Foul frequency' setting).", 1.0, 0.3, 2.0),
        t("game.block_rate", "Share of missed rim shots that are blocked before skill adjustments.", 0.085, 0.0, 0.3),
        t("game.steal_share", "Share of turnovers that are steals.", 0.5, 0.0, 1.0),
        t("game.assist_scale", "Multiplier on how often baskets are assisted.", 1.0, 0.3, 1.8),
        t("game.hot_hand_size", "Maximum shooting boost/penalty from hot/cold streaks.", 0.03, 0.0, 0.1),
        t("game.night_variance", "Team 'shooting night' randomness, in shooting percentage points.", 0.022, 0.0, 0.08),
        t("game.clutch_slope", "How much the Clutch rating matters late in close games.", 1.0, 0.0, 3.0),
        t("game.three_point_bonus", "Flat bonus (or penalty) to every three-point attempt's chance of going in.", 0.0, -0.1, 0.1),
        t("game.tech_rate", "Technical fouls per possession.", 0.0009, 0.0, 0.01),
        t("game.flagrant_rate", "Flagrant fouls per possession.", 0.0004, 0.0, 0.01),
        t("game.sub_hysteresis", "Minutes of imbalance before the coach substitutes. Lower = more substitutions.", 1.6, 0.3, 6.0),
        t("game.zone_rim_penalty", "How much a zone defense hurts rim finishing against it.", 0.02, 0.0, 0.1),
        t("game.shot_clock_violation_rate", "Share of turnovers that are shot-clock violations (when a clock exists).", 0.05, 0.0, 0.3),
    ]
}

/// Parameters resolved into plain fields for speed.
#[derive(Clone, Debug)]
pub struct Tune {
    pub pace_scale: f64,
    pub skill_slope: f64,
    pub defense_slope: f64,
    pub usage_exp: f64,
    pub three_rate_scale: f64,
    pub fatigue_rate: f64,
    pub fatigue_penalty: f64,
    pub turnover_scale: f64,
    pub foul_scale: f64,
    pub block_rate: f64,
    pub steal_share: f64,
    pub assist_scale: f64,
    pub hot_hand: f64,
    pub night_var: f64,
    pub clutch_slope: f64,
    pub three_bonus: f64,
    pub tech_rate: f64,
    pub flagrant_rate: f64,
    pub sub_hyst: f64,
    pub zone_rim_pen: f64,
    pub sc_violation: f64,
}

impl Tune {
    pub fn from_map(map: &BTreeMap<String, f64>) -> Tune {
        let g = |k: &str| -> f64 {
            map.get(k).copied().unwrap_or_else(|| default_tuning().iter().find(|d| d.key == k).map(|d| d.default).unwrap_or(1.0))
        };
        Tune {
            pace_scale: g("game.pace_scale"),
            skill_slope: g("game.skill_slope"),
            defense_slope: g("game.defense_slope"),
            usage_exp: g("game.usage_exponent"),
            three_rate_scale: g("game.three_rate_scale"),
            fatigue_rate: g("game.fatigue_rate"),
            fatigue_penalty: g("game.fatigue_penalty"),
            turnover_scale: g("game.turnover_scale"),
            foul_scale: g("game.foul_scale"),
            block_rate: g("game.block_rate"),
            steal_share: g("game.steal_share"),
            assist_scale: g("game.assist_scale"),
            hot_hand: g("game.hot_hand_size"),
            night_var: g("game.night_variance"),
            clutch_slope: g("game.clutch_slope"),
            three_bonus: g("game.three_point_bonus"),
            tech_rate: g("game.tech_rate"),
            flagrant_rate: g("game.flagrant_rate"),
            sub_hyst: g("game.sub_hysteresis"),
            zone_rim_pen: g("game.zone_rim_penalty"),
            sc_violation: g("game.shot_clock_violation_rate"),
        }
    }
}

// -------------------------------------------------------------------------------------------
// Inputs
// -------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefScheme {
    Man,
    Zone,
    /// Full-court press: forces turnovers, costs energy, can be beaten.
    Press,
}

/// How a coach wants the team to play.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Strategy {
    /// -1 (slow) to +1 (run and gun).
    pub tempo: f64,
    /// -1 (avoid threes) to +1 (shoot lots of threes).
    pub three_emphasis: f64,
    /// 0..1 how much to feed the post / paint.
    pub inside_focus: f64,
    pub defense: DefScheme,
    /// 0..1 crash the offensive glass (costs transition defense).
    pub crash_glass: f64,
    /// Intentionally foul poor free-throw shooters.
    pub hack_a: bool,
    /// Coach quality: tactics 0-100 (better in-game adjustments and synergy).
    pub tactics: f64,
}

impl Default for Strategy {
    fn default() -> Self {
        Strategy { tempo: 0.0, three_emphasis: 0.0, inside_focus: 0.5, defense: DefScheme::Man, crash_glass: 0.3, hack_a: false, tactics: 50.0 }
    }
}

/// Badge effects summed into fixed fields (fast and `Copy`). Keys match `BadgeDef::effects`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Fx {
    pub rim: f64,
    pub post: f64,
    pub mid: f64,
    pub three: f64,
    pub assisted_three: f64,
    pub contest_resist: f64,
    pub clutch: f64,
    pub ft: f64,
    pub draw_foul: f64,
    pub assist_boost: f64,
    pub team_boost: f64,
    pub tov_reduce: f64,
    pub iso: f64,
    pub steal: f64,
    pub block: f64,
    pub oreb: f64,
    pub dreb: f64,
    pub perim_d: f64,
    pub rim_d: f64,
    pub stamina: f64,
    pub injury: f64,
    pub foul_avoid: f64,
}

impl Fx {
    pub fn add(&mut self, key: &str, v: f64) {
        match key {
            "rim" => self.rim += v,
            "post" => self.post += v,
            "mid" => self.mid += v,
            "three" => self.three += v,
            "assisted_three" => self.assisted_three += v,
            "contest_resist" => self.contest_resist += v,
            "clutch" => self.clutch += v,
            "ft" => self.ft += v,
            "draw_foul" => self.draw_foul += v,
            "assist_boost" => self.assist_boost += v,
            "team_boost" => self.team_boost += v,
            "tov_reduce" => self.tov_reduce += v,
            "iso" => self.iso += v,
            "steal" => self.steal += v,
            "block" => self.block += v,
            "oreb" => self.oreb += v,
            "dreb" => self.dreb += v,
            "perim_d" => self.perim_d += v,
            "rim_d" => self.rim_d += v,
            "stamina" => self.stamina += v,
            "injury" => self.injury += v,
            "foul_avoid" => self.foul_avoid += v,
            _ => {}
        }
    }
}

/// One player ready for the game, with composite skills already computed.
#[derive(Clone, Debug)]
pub struct GamePlayer {
    pub id: PlayerId,
    pub name: String,
    pub pos: Position,
    pub ovr: f64,
    pub height: f64,
    pub fin: f64,
    pub mid: f64,
    pub three: f64,
    pub post: f64,
    pub ft: f64,
    pub handle: f64,
    pub pass: f64,
    pub orb: f64,
    pub drb: f64,
    pub perim: f64,
    pub rim: f64,
    pub steal: f64,
    pub block: f64,
    pub stamina: f64,
    pub clutch: f64,
    pub discipline: f64,
    pub draw_foul: f64,
    pub hustle: f64,
    pub speed: f64,
    pub scoring: f64,
    pub usage: f64,
    pub starter: bool,
    /// Minutes the coach wants this player to play.
    pub target_min: f64,
    pub fitness: f64,
    /// Chance per 100 minutes of an in-game injury (0 disables).
    pub injury_per_100: f64,
    /// Badge strengths by effect key.
    pub fx: Fx,
}

impl GamePlayer {
    pub fn from_player(p: &Player, badges: &[BadgeDef], perf_mod: f64, injury_per_100: f64) -> GamePlayer {
        let a = &p.attrs;
        let g = |x: Attr| a.get(x);
        let h = p.height_in as f64;
        let hadj = h - 78.0;
        let pm = perf_mod * crate::injury::performance_penalty(p);
        let s = |v: f64| (v * pm).clamp(1.0, 110.0);
        let fin = s(0.40 * g(Attr::Layup) + 0.20 * g(Attr::DrivingDunk).max(g(Attr::StandingDunk)) + 0.25 * g(Attr::CloseShot) + 0.15 * g(Attr::Vertical));
        let mid = s(0.75 * g(Attr::MidRange) + 0.25 * g(Attr::ShotIq));
        let three = s(0.80 * g(Attr::ThreePoint) + 0.20 * g(Attr::ShotIq));
        let post = s(0.5 * g(Attr::PostControl) + 0.3 * g(Attr::CloseShot) + 0.2 * g(Attr::Strength));
        let ft = s(0.9 * g(Attr::FreeThrow) + 0.1 * g(Attr::OffConsistency));
        let handle = s(0.5 * g(Attr::BallHandle) + 0.2 * g(Attr::Hands) + 0.15 * g(Attr::SpeedWithBall) + 0.15 * g(Attr::PassIq));
        let pass = s(0.35 * g(Attr::PassAccuracy) + 0.35 * g(Attr::PassVision) + 0.3 * g(Attr::PassIq));
        let orb = s(0.55 * g(Attr::OffRebound) + 0.2 * g(Attr::Hustle) + 0.15 * g(Attr::Strength) + 0.1 * g(Attr::Vertical) + hadj * 0.8);
        let drb = s(0.6 * g(Attr::DefRebound) + 0.15 * g(Attr::Hustle) + 0.15 * g(Attr::Strength) + 0.1 * g(Attr::Vertical) + hadj * 0.8);
        let perim = s(0.45 * g(Attr::PerimeterDef) + 0.25 * g(Attr::LateralQuickness) + 0.15 * g(Attr::Speed) + 0.15 * g(Attr::DefConsistency));
        let rim = s(0.5 * g(Attr::InteriorDef) + 0.25 * g(Attr::Block) + 0.15 * g(Attr::HelpDefIq) + 0.1 * g(Attr::Strength) + hadj * 0.8);
        let steal = s(0.6 * g(Attr::Steal) + 0.25 * g(Attr::PassPerception) + 0.15 * g(Attr::Hands));
        let block = s(0.75 * g(Attr::Block) + 0.25 * g(Attr::Vertical) + hadj * 0.8);
        // Scoring talent: best skills count most.
        let mut sk = [fin.max(post), mid, three, handle * 0.9];
        sk.sort_by(|x, y| y.partial_cmp(x).unwrap());
        let scoring = 0.45 * sk[0] + 0.30 * sk[1] + 0.25 * sk[2];
        let mut fx = Fx::default();
        for b in &p.badges {
            if let Some(d) = badges.iter().find(|d| d.id == b.id) {
                for (k, v) in &d.effects {
                    fx.add(k, v * b.tier as i32 as f64);
                }
            }
        }
        GamePlayer {
            id: p.id,
            name: p.short_name(),
            pos: p.position,
            ovr: p.ovr as f64,
            height: h,
            fin,
            mid,
            three,
            post,
            ft,
            handle,
            pass,
            orb,
            drb,
            perim,
            rim,
            steal,
            block,
            stamina: g(Attr::Stamina),
            clutch: s(g(Attr::Clutch)),
            discipline: g(Attr::Discipline),
            draw_foul: g(Attr::DrawFoul),
            hustle: g(Attr::Hustle),
            speed: g(Attr::Speed),
            scoring,
            usage: (scoring / 60.0).max(0.2) * (0.92 + 0.16 * p.hidden.ego as f64 / 100.0),
            starter: false,
            target_min: 0.0,
            fitness: p.fitness as f64,
            injury_per_100,
            fx,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameTeam {
    pub id: TeamId,
    pub name: String,
    pub players: Vec<GamePlayer>,
    pub strategy: Strategy,
}

/// League-average composite skills (minutes-weighted). Ratings are applied *relative* to these,
/// so a league of average players shoots exactly the era's league-average percentages.
#[derive(Clone, Copy, Debug)]
pub struct Refs {
    pub fin: f64,
    pub mid: f64,
    pub three: f64,
    pub post: f64,
    pub ft: f64,
    pub handle: f64,
    pub pass: f64,
    pub orb: f64,
    pub drb: f64,
    pub perim: f64,
    pub rim: f64,
}

impl Default for Refs {
    fn default() -> Self {
        Refs { fin: 60.0, mid: 58.0, three: 55.0, post: 56.0, ft: 58.0, handle: 55.0, pass: 55.0, orb: 55.0, drb: 55.0, perim: 55.0, rim: 55.0 }
    }
}

impl Refs {
    /// Minutes-weighted average over a set of players.
    pub fn from_players<'a>(players: impl Iterator<Item = &'a GamePlayer>) -> Refs {
        let mut r = Refs { fin: 0.0, mid: 0.0, three: 0.0, post: 0.0, ft: 0.0, handle: 0.0, pass: 0.0, orb: 0.0, drb: 0.0, perim: 0.0, rim: 0.0 };
        let mut w = 0.0;
        for p in players {
            let m = p.target_min.max(0.0);
            w += m;
            r.fin += p.fin * m;
            r.mid += p.mid * m;
            r.three += p.three * m;
            r.post += p.post * m;
            r.ft += p.ft * m;
            r.handle += p.handle * m;
            r.pass += p.pass * m;
            r.orb += p.orb * m;
            r.drb += p.drb * m;
            r.perim += p.perim * m;
            r.rim += p.rim * m;
        }
        if w <= 0.0 {
            return Refs::default();
        }
        Refs { fin: r.fin / w, mid: r.mid / w, three: r.three / w, post: r.post / w, ft: r.ft / w, handle: r.handle / w, pass: r.pass / w, orb: r.orb / w, drb: r.drb / w, perim: r.perim / w, rim: r.rim / w }
    }
}

/// Calibration offsets found by [`calibrate`] so that a league of these players reproduces the
/// era's league-average pace, shooting, fouls and turnovers.
#[derive(Clone, Copy, Debug)]
pub struct Cal {
    pub two_off: f64,
    pub three_off: f64,
    pub ft_off: f64,
    pub pace_mul: f64,
    pub foul_mul: f64,
    pub tov_mul: f64,
    pub orb_mul: f64,
    pub three_mul: f64,
}

impl Default for Cal {
    fn default() -> Self {
        Cal { two_off: 0.0, three_off: 0.0, ft_off: 0.0, pace_mul: 1.0, foul_mul: 1.0, tov_mul: 1.0, orb_mul: 1.0, three_mul: 1.0 }
    }
}

#[derive(Clone, Debug)]
pub struct GameContext {
    pub refs: Refs,
    pub cal: Cal,
    pub season: Season,
    pub rules: Rules,
    pub style: EraStyle,
    pub tune: Tune,
    /// Home-court advantage in points per game.
    pub home_court: f64,
    /// 1.0 = normal randomness.
    pub randomness: f64,
    pub star_power: f64,
    pub fatigue: f64,
    pub foul_rate: f64,
    pub hot_hand: bool,
    pub clutch: bool,
    pub possession_detail: f64,
    pub in_game_injuries: bool,
    pub playoffs: bool,
    pub neutral_site: bool,
    pub play_by_play: bool,
}

// -------------------------------------------------------------------------------------------
// Outputs
// -------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerBox {
    pub id: PlayerId,
    pub name: String,
    pub stats: StatLine,
    pub started: bool,
    /// Left the game hurt (the league creates the actual injury afterwards).
    pub injured_in_game: bool,
    pub fouled_out: bool,
    pub ejected: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TeamBox {
    pub team: TeamId,
    pub name: String,
    pub pts: u16,
    pub players: Vec<PlayerBox>,
    pub team_fouls: u16,
    pub possessions: u16,
    pub fast_break_pts: u16,
    pub points_in_paint: u16,
    pub biggest_lead: i16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BoxScore {
    pub season: Season,
    pub home: TeamBox,
    pub away: TeamBox,
    /// Points by period for (home, away), including overtimes.
    pub periods: Vec<(u16, u16)>,
    pub overtimes: u8,
    pub playoffs: bool,
    pub pbp: Vec<String>,
}

impl BoxScore {
    pub fn winner_is_home(&self) -> bool {
        self.home.pts > self.away.pts
    }
}

// -------------------------------------------------------------------------------------------
// Internal state
// -------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shot {
    Rim,
    Post,
    Mid,
    Three,
}

struct SimPlayer {
    p: GamePlayer,
    energy: f64,
    fouls: u8,
    secs: f64,
    on: bool,
    out: bool,
    heat: f64,
    stats: StatLine,
    pm: i32,
    started: bool,
    injured: bool,
    fouled_out: bool,
    ejected: bool,
    techs: u8,
    stint_start: f64,
}

struct SimTeam {
    id: TeamId,
    name: String,
    strat: Strategy,
    pl: Vec<SimPlayer>,
    on: [usize; 5],
    pts: u16,
    q_pts: u16,
    fouls_q: u16,
    fouls_total: u16,
    poss: u16,
    night: f64,
    fb_pts: u16,
    paint_pts: u16,
    best_lead: i16,
}

struct Sim<'a> {
    ctx: &'a GameContext,
    rng: &'a mut Rng,
    tm: [SimTeam; 2],
    /// 0 = home, 1 = away
    off: usize,
    clock: f64,
    period: u8,
    elapsed: f64,
    total_len: f64,
    pbp: Vec<String>,
    mid_share: f64,
    post_share: f64,
    ast_rate: f64,
    shoot_foul_per_fga: f64,
    seconds_per_poss: f64,
    periods: Vec<(u16, u16)>,
    /// Is this the last possession of the period (clock expired during it)?
    transition: bool,
}

fn expo(x: f64) -> f64 {
    x.exp()
}

impl SimTeam {
    fn new(gt: &GameTeam, rng: &mut Rng, ctx: &GameContext) -> SimTeam {
        let mut pl: Vec<SimPlayer> = gt
            .players
            .iter()
            .map(|p| SimPlayer {
                energy: (100.0 - (100.0 - p.fitness) * 0.4).clamp(40.0, 100.0),
                p: p.clone(),
                fouls: 0,
                secs: 0.0,
                on: false,
                out: false,
                heat: 0.0,
                stats: StatLine::default(),
                pm: 0,
                started: false,
                injured: false,
                fouled_out: false,
                ejected: false,
                techs: 0,
                stint_start: 0.0,
            })
            .collect();
        // Starters: those flagged, else the first five.
        let mut on = [0usize; 5];
        let mut k = 0;
        for (i, p) in pl.iter().enumerate() {
            if p.p.starter && k < 5 {
                on[k] = i;
                k += 1;
            }
        }
        for i in 0..pl.len() {
            if k >= 5 {
                break;
            }
            if !on[..k].contains(&i) {
                on[k] = i;
                k += 1;
            }
        }
        for &i in &on {
            pl[i].on = true;
            pl[i].started = true;
            pl[i].stats.gs = 1;
        }
        SimTeam {
            id: gt.id,
            name: gt.name.clone(),
            strat: gt.strategy.clone(),
            pl,
            on,
            pts: 0,
            q_pts: 0,
            fouls_q: 0,
            fouls_total: 0,
            poss: 0,
            night: rng.gauss(0.0, ctx.tune.night_var * ctx.randomness),
            fb_pts: 0,
            paint_pts: 0,
            best_lead: 0,
        }
    }
}

fn mid_share_of_twos(year: i32) -> f64 {
    interp(&[(1946, 0.80), (1960, 0.75), (1980, 0.62), (1995, 0.52), (2005, 0.46), (2012, 0.40), (2016, 0.30), (2020, 0.24), (2030, 0.21)], year as f64)
}

fn post_share_of_rim(year: i32) -> f64 {
    interp(&[(1946, 0.42), (1990, 0.33), (2010, 0.16), (2024, 0.07)], year as f64)
}

fn assist_rate(year: i32) -> f64 {
    interp(&[(1946, 0.36), (1960, 0.44), (1975, 0.55), (1990, 0.58), (2010, 0.57), (2024, 0.62)], year as f64)
}

/// Play one full game.
pub fn simulate_game(ctx: &GameContext, home: &GameTeam, away: &GameTeam, rng: &mut Rng) -> BoxScore {
    let tm = [SimTeam::new(home, rng, ctx), SimTeam::new(away, rng, ctx)];
    let r = &ctx.rules;
    let total_len = 4.0 * r.quarter_minutes * 60.0;
    let tempo = (home.strategy.tempo + away.strategy.tempo) / 2.0;
    let pace = ctx.style.pace * (1.0 + 0.045 * tempo) * ctx.tune.pace_scale * ctx.cal.pace_mul;
    let poss_per_team = pace * (4.0 * r.quarter_minutes / 48.0);
    let seconds_per_poss = total_len / (2.0 * poss_per_team);
    let mid_share = mid_share_of_twos(ctx.season);
    // Expected shooting fouls per field-goal attempt (shot-mix weighted multipliers are normalised in `shot_foul_mult`).
    let shoot_foul_per_fga = ctx.style.ft_rate / 2.15 * ctx.cal.foul_mul * ctx.tune.foul_scale * ctx.foul_rate * r.foul_multiplier;
    let mut sim = Sim {
        ctx,
        rng,
        tm,
        off: 0,
        clock: r.quarter_minutes * 60.0,
        period: 1,
        elapsed: 0.0,
        total_len,
        pbp: vec![],
        mid_share,
        post_share: post_share_of_rim(ctx.season),
        ast_rate: assist_rate(ctx.season) * ctx.tune.assist_scale,
        shoot_foul_per_fga,
        seconds_per_poss,
        periods: vec![],
        transition: false,
    };
    sim.run()
}

impl<'a> Sim<'a> {
    fn def(&self) -> usize {
        1 - self.off
    }

    fn lead(&self, team: usize) -> i32 {
        self.tm[team].pts as i32 - self.tm[1 - team].pts as i32
    }

    fn log(&mut self, s: String) {
        if self.ctx.play_by_play {
            let q = if self.period <= 4 { format!("Q{}", self.period) } else { format!("OT{}", self.period - 4) };
            let m = (self.clock / 60.0).floor() as i32;
            let sec = (self.clock as i32) % 60;
            self.pbp.push(format!("{q} {m}:{sec:02} [{}-{}] {s}", self.tm[0].pts, self.tm[1].pts));
        }
    }

    fn run(&mut self) -> BoxScore {
        // Tip-off: random team gets the first possession.
        let tip_winner = if self.rng.chance(0.5) { 0 } else { 1 };
        self.off = tip_winner;
        let mut period_start_off = [tip_winner, 0, 0, 0];
        period_start_off[1] = 1 - tip_winner;
        period_start_off[2] = 1 - tip_winner;
        period_start_off[3] = tip_winner;
        let ot_len = self.ctx.rules.overtime_minutes * 60.0;
        loop {
            let ps = if self.period <= 4 { period_start_off[self.period as usize - 1] } else { (tip_winner + self.period as usize) % 2 };
            self.off = ps;
            self.play_period();
            self.periods.push((self.tm[0].q_pts, self.tm[1].q_pts));
            for t in self.tm.iter_mut() {
                t.q_pts = 0;
                t.fouls_q = 0;
            }
            if self.period >= 4 && self.tm[0].pts != self.tm[1].pts {
                break;
            }
            if self.period >= 12 {
                // Absurdly long game: settle it.
                if self.tm[0].pts == self.tm[1].pts {
                    self.tm[0].pts += 1;
                    if let Some(p) = self.tm[0].pl.first_mut() {
                        p.stats.pts += 1;
                    }
                }
                break;
            }
            self.period += 1;
            if self.period > 4 {
                self.clock = ot_len;
                self.total_len += ot_len;
            } else {
                self.clock = self.ctx.rules.quarter_minutes * 60.0;
            }
        }
        self.finish()
    }

    fn play_period(&mut self) {
        // Between periods players get a little rest (more at halftime).
        let rest = if self.period == 3 { 14.0 } else { 3.0 };
        for t in self.tm.iter_mut() {
            for p in t.pl.iter_mut() {
                if p.on {
                    p.energy = (p.energy + rest * 0.5).min(100.0);
                }
            }
        }
        while self.clock > 0.0 {
            self.substitutions();
            self.possession();
            self.off = self.def();
        }
    }

    // ---------------------------------------------------------------------------------------
    // Substitutions
    // ---------------------------------------------------------------------------------------

    fn clutch_time(&self) -> bool {
        self.period >= 4 && self.clock <= 300.0 && self.lead(0).abs() <= 8
    }

    fn garbage_time(&self) -> bool {
        self.period >= 4 && self.clock <= 360.0 && self.lead(0).abs() >= 22
    }

    fn substitutions(&mut self) {
        let hyst = self.ctx.tune.sub_hyst * 60.0;
        let clutch = self.clutch_time();
        let garbage = self.garbage_time();
        let elapsed = self.elapsed;
        let period = self.period as usize;
        let foul_limit = self.ctx.rules.foul_limit;
        for ti in 0..2 {
            let team_lead = self.lead(ti);
            let t = &mut self.tm[ti];
            // Mandatory removals first.
            for slot in 0..5 {
                let idx = t.on[slot];
                let must = t.pl[idx].out || t.pl[idx].fouls >= foul_limit;
                let tired = t.pl[idx].energy < 28.0;
                if must || tired {
                    if let Some(r) = Self::best_replacement(t, slot, elapsed, period, true, clutch, garbage, team_lead) {
                        Self::swap(t, slot, r, elapsed);
                    }
                }
            }
            // Rotation control law.
            let mut guard = 0;
            loop {
                guard += 1;
                if guard > 4 {
                    break;
                }
                // find on-court player most over his minutes budget
                let mut worst_slot = None;
                let mut worst = -hyst;
                for slot in 0..5 {
                    let idx = t.on[slot];
                    let d = Self::deficit(&t.pl[idx], elapsed, period, clutch, garbage, team_lead, false);
                    if d < worst {
                        worst = d;
                        worst_slot = Some(slot);
                    }
                }
                let Some(slot) = worst_slot else { break };
                if let Some(r) = Self::best_replacement(t, slot, elapsed, period, false, clutch, garbage, team_lead) {
                    let d_r = Self::deficit(&t.pl[r], elapsed, period, clutch, garbage, team_lead, false);
                    if d_r > -hyst * 0.2 {
                        Self::swap(t, slot, r, elapsed);
                        continue;
                    }
                }
                break;
            }
            // Closing lineup: in clutch time, make sure the best available players are on the floor.
            if clutch {
                let mut order: Vec<usize> = (0..t.pl.len()).filter(|&i| !t.pl[i].out && t.pl[i].fouls < foul_limit && t.pl[i].energy > 35.0).collect();
                order.sort_by(|&a, &b| t.pl[b].p.ovr.partial_cmp(&t.pl[a].p.ovr).unwrap());
                for &best in order.iter().take(5) {
                    if !t.pl[best].on {
                        // swap out the weakest on-court player not in the top 5
                        let top5: Vec<usize> = order.iter().take(5).copied().collect();
                        if let Some(slot) = (0..5).filter(|s| !top5.contains(&t.on[*s])).min_by(|a, b| t.pl[t.on[*a]].p.ovr.partial_cmp(&t.pl[t.on[*b]].p.ovr).unwrap()) {
                            Self::swap(t, slot, best, elapsed);
                        }
                    }
                }
            }
        }
    }

    /// Positive = this player is owed court time, negative = he has played more than planned (seconds).
    fn deficit(p: &SimPlayer, elapsed: f64, period: usize, clutch: bool, garbage: bool, lead: i32, _force: bool) -> f64 {
        let mut frac = p.p.target_min / 48.0;
        if clutch {
            frac = (frac * 1.35).min(0.97);
        }
        if garbage {
            frac = if lead > 0 || lead < 0 { if p.p.target_min >= 22.0 { frac * 0.1 } else { (frac * 2.2).min(0.95) } } else { frac };
        }
        // foul trouble: fouls above (period+1) are a sit-down signal
        let trouble = (p.fouls as i32 - (period as i32 + 1)).max(0) as f64 * 150.0;
        // Tiredness pushes toward the bench.
        let tired = ((55.0 - p.energy).max(0.0)) * 5.0;
        // Starters get a head start at the beginning of the first and third quarters.
        frac * elapsed - p.secs - trouble - tired
    }

    fn best_replacement(t: &SimTeam, slot: usize, elapsed: f64, period: usize, mandatory: bool, clutch: bool, garbage: bool, lead: i32) -> Option<usize> {
        let leaving = t.on[slot];
        let lpos = t.pl[leaving].p.pos;
        let mut best: Option<(f64, usize)> = None;
        for (i, p) in t.pl.iter().enumerate() {
            if p.on || p.out || p.fouls >= 6 || p.energy < 30.0 || p.p.target_min <= 0.0 && !mandatory {
                continue;
            }
            let d = Self::deficit(p, elapsed, period, clutch, garbage, lead, false);
            // prefer owed minutes, similar position, quality
            let mut score = d / 60.0 + p.p.ovr * 0.08;
            if p.p.pos == lpos {
                score += 1.0;
            } else if (p.p.pos.idx() as i32 - lpos.idx() as i32).abs() == 1 {
                score += 0.4;
            }
            // keep a ball handler and a big on the floor
            let handlers_after = (0..5).filter(|s| *s != slot).filter(|s| t.pl[t.on[*s]].p.handle >= 52.0).count() + (p.p.handle >= 52.0) as usize;
            if handlers_after == 0 {
                score -= 4.0;
            }
            let bigs_after = (0..5).filter(|s| *s != slot).filter(|s| t.pl[t.on[*s]].p.height >= 80.0).count() + (p.p.height >= 80.0) as usize;
            if bigs_after == 0 && t.pl.iter().any(|x| x.p.height >= 80.0 && !x.out) {
                score -= 2.0;
            }
            if best.map(|b| score > b.0).unwrap_or(true) {
                best = Some((score, i));
            }
        }
        best.map(|b| b.1)
    }

    fn swap(t: &mut SimTeam, slot: usize, incoming: usize, elapsed: f64) {
        let out_idx = t.on[slot];
        t.pl[out_idx].on = false;
        t.pl[incoming].on = true;
        t.pl[incoming].stint_start = elapsed;
        t.on[slot] = incoming;
    }

    // ---------------------------------------------------------------------------------------
    // Clock & energy
    // ---------------------------------------------------------------------------------------

    fn advance(&mut self, secs: f64) {
        let secs = secs.min(self.clock).max(0.0);
        self.clock -= secs;
        self.elapsed += secs;
        let mins = secs / 60.0;
        let drain_base = 1.15 * self.ctx.fatigue * self.ctx.tune.fatigue_rate;
        let press = |s: DefScheme| if s == DefScheme::Press { 1.12 } else { 1.0 };
        let (so, sd) = (self.tm[0].strat.defense, self.tm[1].strat.defense);
        let mults = [press(so), press(sd)];
        for ti in 0..2 {
            let m = mults[ti];
            for p in self.tm[ti].pl.iter_mut() {
                if p.on {
                    p.secs += secs;
                    p.energy = (p.energy - drain_base * (1.30 - p.p.stamina / 100.0 + p.p.fx.stamina * -1.0) * mins * m).max(0.0);
                } else {
                    p.energy = (p.energy + 0.75 * mins * (0.8 + p.p.stamina / 250.0)).min(100.0);
                }
            }
        }
        // In-game injuries.
        if self.ctx.in_game_injuries {
            for ti in 0..2 {
                for slot in 0..5 {
                    let idx = self.tm[ti].on[slot];
                    let chance = self.tm[ti].pl[idx].p.injury_per_100 * mins / 100.0 * (1.0 + (100.0 - self.tm[ti].pl[idx].energy) / 150.0);
                    if chance > 0.0 && self.rng.f64() < chance {
                        let name = self.tm[ti].pl[idx].p.name.clone();
                        self.tm[ti].pl[idx].injured = true;
                        self.tm[ti].pl[idx].out = true;
                        self.log(format!("{name} is down and leaves the game with an injury."));
                    }
                }
            }
        }
    }

    fn credit_plus_minus(&mut self, scoring_team: usize, pts: i32) {
        for slot in 0..5 {
            let i = self.tm[scoring_team].on[slot];
            self.tm[scoring_team].pl[i].pm += pts;
            let j = self.tm[1 - scoring_team].on[slot];
            self.tm[1 - scoring_team].pl[j].pm -= pts;
        }
    }

    fn add_points(&mut self, team: usize, pts: u16) {
        self.tm[team].pts += pts;
        self.tm[team].q_pts += pts;
        self.credit_plus_minus(team, pts as i32);
        let lead = self.lead(team) as i16;
        if lead > self.tm[team].best_lead {
            self.tm[team].best_lead = lead;
        }
    }

    // ---------------------------------------------------------------------------------------
    // Possession
    // ---------------------------------------------------------------------------------------

    fn fatigue_pen(&self, e: f64) -> f64 {
        (100.0 - e) * 0.10 * self.ctx.fatigue * self.ctx.tune.fatigue_penalty
    }

    fn possession(&mut self) {
        let off = self.off;
        let def = self.def();
        self.tm[off].poss += 1;
        let r = &self.ctx.rules;

        // Duration of this possession's first phase.
        let mut mean = self.seconds_per_poss * if self.transition { 0.6 } else { 1.0 };
        let lead = self.lead(off);
        let late = self.period >= 4 && self.clock < 150.0;
        if late {
            if lead > 0 && lead <= 12 {
                mean *= 1.55; // protect the lead: burn clock
            } else if lead < 0 {
                mean *= 0.65; // chase
            }
        }
        // Era without a shot clock: leading teams stall in the second half.
        if r.shot_clock == 0 && self.period >= 3 && lead >= 3 {
            mean *= 1.0 + (lead as f64 * 0.03).min(0.35);
        }
        let max_dur = if r.shot_clock > 0 { r.shot_clock as f64 } else { 90.0 };
        let dur = self.rng.gauss(mean, mean * 0.33).clamp(3.0, max_dur);
        let dur = dur.min(self.clock.max(0.5));
        let was_transition = self.transition;
        self.transition = false;

        // Intentional fouling by the trailing defense late in the game.
        if self.should_hack(def, off) {
            let t = self.rng.uniform(2.0, 6.0).min(self.clock);
            self.advance(t);
            self.intentional_foul(def, off);
            return;
        }

        // Random technical / flagrant fouls.
        if self.rng.f64() < self.ctx.tune.tech_rate {
            let who = self.rng.chance(0.5) as usize;
            self.technical(who);
        }

        self.advance(dur);
        self.run_possession(off, def, was_transition, true);
    }

    fn should_hack(&mut self, def: usize, off: usize) -> bool {
        if self.ctx.possession_detail < 0.5 {
            return false;
        }
        let r = &self.ctx.rules;
        let behind = self.lead(def);
        // Late game: trailing team fouls to stop the clock.
        let late = self.period >= 4 && self.clock <= if r.intentional_foul_rule { 120.0 } else { 90.0 } && behind < 0 && behind >= -9 && self.clock > 3.0;
        let _ = off;
        if late && self.rng.chance(if self.clock < 60.0 { 0.9 } else { 0.5 }) {
            return true;
        }
        // Hack-a-player strategy (not allowed to be as aggressive under the 2016 rule except via a decision to foul on the ball).
        if self.tm[def].strat.hack_a && self.period >= 2 && self.clock > 120.0 && !r.intentional_foul_rule {
            let worst = self.tm[off].on.iter().map(|&i| self.tm[off].pl[i].p.ft).fold(100.0, f64::min);
            return worst < 45.0 && self.rng.chance(0.55);
        }
        false
    }

    fn intentional_foul(&mut self, def: usize, off: usize) {
        // Foul the on-court offensive player who is worst at free throws.
        let victim = *self.tm[off].on.iter().min_by(|&&a, &&b| self.tm[off].pl[a].p.ft.partial_cmp(&self.tm[off].pl[b].p.ft).unwrap()).unwrap();
        let fouler = self.pick_defender_for_foul(def);
        self.commit_foul(def, fouler);
        let (fname, vname) = (self.tm[def].pl[fouler].p.name.clone(), self.tm[off].pl[victim].p.name.clone());
        self.log(format!("{fname} intentionally fouls {vname}."));
        let in_bonus = self.tm[def].fouls_q >= self.ctx.rules.bonus_fouls as u16 || self.clock <= 120.0;
        // After this call the main loop flips `self.off`. Leaving it as `off` hands the ball to `def`;
        // setting it to `def` lets the offense keep the ball.
        if in_bonus {
            match self.free_throws(off, victim, 2, def, 1.0) {
                FtEnd::MissedLive => {
                    if let ShotEnd::OffensiveRebound = self.rebound(off, def, false, 0.7, None) {
                        self.off = def; // offense keeps it
                    }
                }
                FtEnd::Made => {}
            }
        } else {
            self.off = def; // side-out: the offense keeps the ball
        }
    }


    fn pick_defender_for_foul(&mut self, def: usize) -> usize {
        let w: Vec<f64> = (0..5).map(|s| {
            let p = &self.tm[def].pl[self.tm[def].on[s]];
            (110.0 - p.p.discipline).max(5.0)
        }).collect();
        self.tm[def].on[self.rng.weighted(&w)]
    }

    fn commit_foul(&mut self, team: usize, player: usize) {
        let fl = self.ctx.rules.foul_limit;
        let t = &mut self.tm[team];
        t.pl[player].fouls += 1;
        t.pl[player].stats.pf += 1;
        t.fouls_q += 1;
        t.fouls_total += 1;
        if t.pl[player].fouls >= fl {
            t.pl[player].fouled_out = true;
            t.pl[player].out = true;
            let n = t.pl[player].p.name.clone();
            self.log(format!("{n} fouls out."));
        }
    }

    fn technical(&mut self, team: usize) {
        let w: Vec<f64> = (0..5).map(|s| 100.0 - self.tm[team].pl[self.tm[team].on[s]].p.discipline + 10.0).collect();
        let i = self.tm[team].on[self.rng.weighted(&w)];
        self.tm[team].pl[i].techs += 1;
        self.tm[team].pl[i].stats.pf += 0;
        let shooter_team = 1 - team;
        let shooter = *self.tm[shooter_team].on.iter().max_by(|&&a, &&b| self.tm[shooter_team].pl[a].p.ft.partial_cmp(&self.tm[shooter_team].pl[b].p.ft).unwrap()).unwrap();
        let name = self.tm[team].pl[i].p.name.clone();
        self.log(format!("Technical foul on {name}."));
        let _ = self.free_throws(shooter_team, shooter, 1, team, 1.0);
        if self.tm[team].pl[i].techs >= 2 {
            self.tm[team].pl[i].out = true;
            self.tm[team].pl[i].ejected = true;
            self.log(format!("{name} is ejected."));
        }
    }

    /// Run the possession until it ends (made basket, defensive rebound, turnover...).
    fn run_possession(&mut self, off: usize, def: usize, transition: bool, fresh: bool) {
        let mut transition = transition;
        let mut fresh = fresh;
        let mut second_chance = false;
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 8 {
                return;
            }
            // Turnover?
            if fresh && !second_chance && self.try_turnover(off, def, transition) {
                return;
            }
            fresh = false;
            // Non-shooting foul before the shot (reach-ins, loose-ball, off-ball).
            if !second_chance && self.rng.f64() < 0.075 * self.ctx.tune.foul_scale * self.ctx.foul_rate * if self.ctx.rules.hand_checking { 1.12 } else { 0.95 } {
                let fouler = self.pick_defender_for_foul(def);
                self.commit_foul(def, fouler);
                let bonus = self.tm[def].fouls_q >= self.ctx.rules.bonus_fouls as u16;
                if bonus {
                    let victim = self.pick_shooter(off, def, Shot::Rim);
                    let n = self.tm[def].pl[fouler].p.name.clone();
                    self.log(format!("Foul on {n}; {} to the line.", self.tm[off].pl[victim].p.name));
                    if let FtEnd::MissedLive = self.free_throws(off, victim, 2, def, 1.0) {
                        if let ShotEnd::OffensiveRebound = self.rebound(off, def, false, 0.75, None) {
                            second_chance = true;
                            transition = false;
                            continue;
                        }
                    }
                    return;
                }
                // otherwise: ball goes back to the offense; fall through and shoot
            }
            // Take the shot.
            match self.shot(off, def, transition, second_chance) {
                ShotEnd::Over => return,
                ShotEnd::OffensiveRebound => {
                    second_chance = true;
                    transition = false;
                    // time for the put-back
                    let t = self.rng.uniform(1.5, 5.5);
                    self.advance(t);
                    if self.clock <= 0.0 {
                        return;
                    }
                }
            }
        }
    }

    fn try_turnover(&mut self, off: usize, def: usize, transition: bool) -> bool {
        let ctx = self.ctx;
        let base = ctx.style.tov * ctx.tune.turnover_scale * ctx.cal.tov_mul;
        // team handling vs team pressure
        let handle_avg: f64 = self.tm[off].on.iter().map(|&i| self.tm[off].pl[i].p.handle).sum::<f64>() / 5.0;
        let pressure: f64 = self.tm[def].on.iter().map(|&i| self.tm[def].pl[i].p.steal * 0.6 + self.tm[def].pl[i].p.perim * 0.4).sum::<f64>() / 5.0;
        let press_mult = match self.tm[def].strat.defense {
            DefScheme::Press => 1.22,
            DefScheme::Zone => 0.96,
            DefScheme::Man => 1.0,
        };
        let unp: f64 = self.tm[off].on.iter().map(|&i| self.tm[off].pl[i].p.fx.tov_reduce).sum();
        let mut p = base * (1.0 + 0.011 * (self.ctx.refs.handle - handle_avg) + 0.010 * (pressure - 55.0)) * press_mult;
        p *= 1.0 - unp.min(0.5);
        if transition {
            p *= 0.8;
        }
        if !self.rng.chance(p.clamp(0.02, 0.45)) {
            return false;
        }
        // who turned it over?
        let w: Vec<f64> = (0..5).map(|s| {
            let pl = &self.tm[off].pl[self.tm[off].on[s]];
            pl.p.usage * (130.0 - pl.p.handle).max(15.0)
        }).collect();
        let who = self.tm[off].on[self.rng.weighted(&w)];
        self.tm[off].pl[who].stats.tov += 1;
        let is_steal = self.rng.chance(ctx.tune.steal_share);
        if is_steal {
            let w: Vec<f64> = (0..5).map(|s| expo(0.04 * (self.tm[def].pl[self.tm[def].on[s]].p.steal - 50.0)) * (1.0 + self.tm[def].pl[self.tm[def].on[s]].p.fx.steal * 4.0)).collect();
            let st = self.tm[def].on[self.rng.weighted(&w)];
            self.tm[def].pl[st].stats.stl += 1;
            let (n1, n2) = (self.tm[off].pl[who].p.name.clone(), self.tm[def].pl[st].p.name.clone());
            self.log(format!("{n2} steals it from {n1}."));
            self.transition = self.rng.chance(0.55);
        } else {
            let n1 = self.tm[off].pl[who].p.name.clone();
            if ctx.rules.shot_clock > 0 && self.rng.chance(ctx.tune.sc_violation) {
                self.log("Shot clock violation.".to_string());
            } else {
                self.log(format!("{n1} turns it over."));
            }
        }
        true
    }

    fn pick_shooter(&mut self, off: usize, def: usize, kind: Shot) -> usize {
        let _ = def;
        let clutch = self.clutch_time();
        let star = self.ctx.star_power;
        let exp = self.ctx.tune.usage_exp * star;
        let w: Vec<f64> = (0..5)
            .map(|s| {
                let p = &self.tm[off].pl[self.tm[off].on[s]];
                let mut skill = match kind {
                    Shot::Rim => p.p.fin * 0.7 + p.p.scoring * 0.3,
                    Shot::Post => p.p.post * 0.7 + p.p.scoring * 0.3,
                    Shot::Mid => p.p.mid * 0.7 + p.p.scoring * 0.3,
                    Shot::Three => p.p.three * 0.7 + p.p.scoring * 0.3,
                };
                skill = skill.max(5.0);
                let e = 0.75 + 0.25 * (p.energy / 100.0);
                let mut u = (p.p.usage.powf(exp / 3.0) * (skill / 60.0).powf(exp * 0.55)) * e;
                if clutch {
                    u *= 1.0 + (p.p.clutch - 55.0) / 120.0 * self.ctx.tune.clutch_slope + (p.p.ovr - 60.0) / 150.0;
                }
                u * (1.0 + p.heat * 1.2)
            })
            .collect();
        self.tm[off].on[self.rng.weighted(&w)]
    }

    fn pick_shot_type(&mut self, off: usize, shooter_hint: Option<usize>, transition: bool, second_chance: bool) -> (usize, Shot) {
        let r = &self.ctx.rules;
        let style = &self.ctx.style;
        // First choose a shooter by overall usage, then his shot type by skills.
        let shooter = match shooter_hint {
            Some(s) => s,
            None => self.pick_shooter(off, 1 - off, Shot::Mid),
        };
        let p = &self.tm[off].pl[shooter].p;
        let strat = &self.tm[off].strat;
        let mut three_p = 0.0;
        if r.three_point && !second_chance {
            let rate = style.three_rate;
            let skill = 0.03 * (p.three - self.ctx.refs.three) * self.ctx.tune.three_rate_scale;
            let size_pen = if p.height >= 82.0 { -0.35 } else if p.height >= 80.0 { -0.12 } else { 0.0 };
            three_p = rate * self.ctx.cal.three_mul * expo(skill + size_pen + 0.35 * strat.three_emphasis);
            // Positional tilt: guards and wings take more, bigs fewer.
            three_p = three_p.clamp(0.0, 0.85);
            if transition {
                three_p *= 0.8;
            }
            // end-of-game 3-point chasing
            if self.period >= 4 && self.clock < 14.0 && self.lead(off) < 0 && self.lead(off) >= -3 {
                three_p = 0.92;
            }
        }
        if self.rng.f64() < three_p {
            return (shooter, Shot::Three);
        }
        // Two-point shot: mid vs rim vs post.
        let ms = self.mid_share * if second_chance { 0.2 } else { 1.0 };
        let mid_pref = expo(0.028 * (p.mid - self.ctx.refs.mid)) * ms;
        let rim_pref = expo(0.028 * (p.fin.max(p.post * 0.9) - self.ctx.refs.fin)) * (1.0 - ms) * if transition { 2.0 } else { 1.0 } * (1.0 + 0.5 * (strat.inside_focus - 0.5));
        let mid_prob = mid_pref / (mid_pref + rim_pref);
        if self.rng.f64() < mid_prob {
            return (shooter, Shot::Mid);
        }
        // Post-up or at the rim
        let post_pref = expo(0.03 * (p.post - p.fin)) * self.post_share * (1.0 + 0.8 * (strat.inside_focus - 0.5)) * if p.height >= 79.0 { 1.5 } else { 0.4 };
        let post_prob = (post_pref / (post_pref + (1.0 - self.post_share))).clamp(0.0, 0.85);
        if !transition && self.rng.f64() < post_prob {
            (shooter, Shot::Post)
        } else {
            (shooter, Shot::Rim)
        }
    }

    /// One shot attempt, including fouls, blocks, assists, rebounds and free throws.
    fn shot(&mut self, off: usize, def: usize, transition: bool, second_chance: bool) -> ShotEnd {
        let ctx = self.ctx;
        let (shooter, kind) = self.pick_shot_type(off, None, transition, second_chance);
        let (fg2, fg3) = (ctx.style.fg2, ctx.style.fg3);
        let ms = self.mid_share;
        let mid_base = fg2 - 0.075;
        let rim_base = ((fg2 - ms * mid_base) / (1.0 - ms)).clamp(0.3, 0.85);
        let slope = ctx.tune.skill_slope;
        let dslope = ctx.tune.defense_slope;
        let rf = ctx.refs;

        // Copy everything we need about the shooter so we can mutate the sim freely below.
        let (m_fin, m_mid, m_three, m_post, m_handle, m_scoring, m_draw, m_clutch, m_energy, m_heat, fx) = {
            let me = &self.tm[off].pl[shooter];
            (me.p.fin, me.p.mid, me.p.three, me.p.post, me.p.handle, me.p.scoring, me.p.draw_foul, me.p.clutch, me.energy, me.heat, me.p.fx)
        };
        let fat = self.fatigue_pen(m_energy);

        // Defense: perimeter and rim composites.
        let (perim_d, rim_d, rim_fx, perim_fx, zone, best_block, blk_fx, def_disc) = {
            let dteam = &self.tm[def];
            let perim_d: f64 = dteam.on.iter().map(|&i| dteam.pl[i].p.perim - self.fatigue_pen(dteam.pl[i].energy) * 0.5).sum::<f64>() / 5.0;
            let mut v: Vec<f64> = dteam.on.iter().map(|&i| dteam.pl[i].p.rim - self.fatigue_pen(dteam.pl[i].energy) * 0.5).collect();
            v.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let rim_d = 0.55 * v[0] + 0.25 * v[1] + 0.2 * v[2..].iter().sum::<f64>() / 3.0;
            let rim_fx: f64 = dteam.on.iter().map(|&i| dteam.pl[i].p.fx.rim_d).sum();
            let perim_fx: f64 = dteam.on.iter().map(|&i| dteam.pl[i].p.fx.perim_d).sum();
            let best_block = dteam.on.iter().map(|&i| dteam.pl[i].p.block).fold(0.0, f64::max);
            let blk_fx: f64 = dteam.on.iter().map(|&i| dteam.pl[i].p.fx.block).sum();
            let def_disc: f64 = dteam.on.iter().map(|&i| dteam.pl[i].p.discipline).sum::<f64>() / 5.0;
            (perim_d, rim_d, rim_fx, perim_fx, dteam.strat.defense == DefScheme::Zone, best_block, blk_fx, def_disc)
        };

        // Assisted?
        let type_mult = match kind {
            Shot::Rim => 0.85,
            Shot::Post => 0.5,
            Shot::Mid => 1.0,
            Shot::Three => 1.25,
        };
        let self_create = ((m_handle - rf.handle) * 0.004 + (m_scoring - 62.0) * 0.003).clamp(-0.1, 0.25);
        let mut p_ast = (self.ast_rate * type_mult * (1.0 - self_create) * if transition { 0.9 } else { 1.0 }).clamp(0.0, 0.95);
        if second_chance {
            p_ast = 0.05;
        }
        let team_pass: f64 = self.tm[off].on.iter().map(|&i| self.tm[off].pl[i].p.pass).sum::<f64>() / 5.0;
        p_ast = (p_ast * (1.0 + 0.008 * (team_pass - rf.pass))).clamp(0.0, 0.97);
        let assisted = self.rng.f64() < p_ast;
        let assister = if assisted {
            let cand: Vec<usize> = (0..5).map(|s| self.tm[off].on[s]).filter(|&i| i != shooter).collect();
            let w: Vec<f64> = cand
                .iter()
                .map(|&i| {
                    let p = &self.tm[off].pl[i];
                    expo(0.05 * (p.p.pass - 50.0)) * (1.0 + p.p.fx.assist_boost * 6.0) * (0.6 + p.p.handle / 120.0)
                })
                .collect();
            Some(cand[self.rng.weighted(&w)])
        } else {
            None
        };
        let ast_skill = assister.map(|a| self.tm[off].pl[a].p.pass).unwrap_or(55.0);

        // Make probability
        let mut p = match kind {
            Shot::Rim => rim_base + ctx.cal.two_off + 0.0026 * slope * (m_fin - rf.fin) - 0.0021 * dslope * (rim_d - rf.rim) - rim_fx * 0.04,
            Shot::Post => rim_base + ctx.cal.two_off - 0.035 + 0.0027 * slope * (m_post - rf.post) - 0.0020 * dslope * (rim_d - rf.rim) + fx.post - rim_fx * 0.04,
            Shot::Mid => mid_base + ctx.cal.two_off + 0.0030 * slope * (m_mid - rf.mid) - 0.0018 * dslope * (perim_d - rf.perim) + fx.mid - perim_fx * 0.04,
            Shot::Three => fg3 + ctx.cal.three_off + 0.0030 * slope * (m_three - rf.three) - 0.0016 * dslope * (perim_d - rf.perim) + fx.three + ctx.tune.three_bonus - perim_fx * 0.04,
        };
        if matches!(kind, Shot::Rim | Shot::Post) {
            p += fx.rim * if kind == Shot::Rim { 1.0 } else { 0.4 };
            if zone {
                p -= ctx.tune.zone_rim_pen;
            }
        }
        // Structural bonuses are centred on their expected value so an average league
        // still produces the era's average percentages.
        {
            let (b_a, b_u) = match kind {
                Shot::Rim => (0.03, 0.0),
                Shot::Post => (0.01, 0.0),
                Shot::Mid => (0.03, -0.02),
                Shot::Three => (0.045, -0.02),
            };
            let exp_ast = (self.ast_rate * type_mult).clamp(0.0, 0.95);
            let expected = exp_ast * b_a + (1.0 - exp_ast) * b_u;
            if assisted {
                p += b_a + 0.0006 * (ast_skill - rf.pass) - expected;
                if matches!(kind, Shot::Three) {
                    p += fx.assisted_three;
                }
            } else {
                p += b_u * if matches!(kind, Shot::Mid | Shot::Three) { (1.0 - fx.contest_resist * 4.0).max(0.0) } else { 1.0 } - expected;
            }
        }
        if transition {
            p += 0.09 - 0.008;
        } else {
            p -= 0.008;
        }
        if second_chance && matches!(kind, Shot::Rim | Shot::Post) {
            p += 0.02;
        } else if matches!(kind, Shot::Rim | Shot::Post) {
            p -= 0.003;
        }
        p -= (fat - 2.0) * 0.014;
        if ctx.hot_hand {
            p += m_heat * ctx.tune.hot_hand * 4.0;
        }
        if self.clutch_time() && ctx.clutch {
            p += 0.0011 * (m_clutch - 55.0) * ctx.tune.clutch_slope + fx.clutch;
        }
        // team-level boosts: floor general etc.
        p += self.tm[off].on.iter().map(|&i| self.tm[off].pl[i].p.fx.team_boost).sum::<f64>() * 0.5;
        // Coaching tactics (small): 50 is neutral.
        p += (self.tm[off].strat.tactics - 50.0) * 0.00012;
        // Home court & shooting night
        let hc = ctx.home_court * 0.0014;
        let side = if ctx.neutral_site { 0.0 } else if off == 0 { hc } else { -hc };
        p += side + self.tm[off].night;
        let p = p.clamp(0.06, 0.93);

        // Shooting foul?
        let mut foul_mult = match kind {
            Shot::Rim => 1.9,
            Shot::Post => 1.55,
            Shot::Mid => 0.55,
            Shot::Three => 0.14,
        };
        if matches!(kind, Shot::Mid | Shot::Three) {
            foul_mult *= if ctx.rules.hand_checking { 1.12 } else if ctx.rules.freedom_of_movement { 0.95 } else { 1.0 };
        }
        let mut p_foul = self.shoot_foul_per_fga * foul_mult * (1.0 + 0.011 * (m_draw - 55.0) + fx.draw_foul) / (1.0 + 0.008 * (def_disc - 55.0));
        if second_chance {
            p_foul *= 1.1;
        }
        let p_foul = p_foul.clamp(0.0, 0.5);
        let fouled = self.rng.f64() < p_foul;
        let made = self.rng.f64() < p;

        let shooter_name = self.tm[off].pl[shooter].p.name.clone();
        let is_three = kind == Shot::Three;
        let pts_val: u16 = if is_three { ctx.rules.three_value as u16 } else { 2 };

        // Hot/cold tracking
        {
            let pl = &mut self.tm[off].pl[shooter];
            pl.heat = (pl.heat * 0.85 + if made { 0.025 } else { -0.018 }).clamp(-0.12, 0.12);
        }

        if fouled {
            let fouler = self.pick_defender_for_foul(def);
            self.commit_foul(def, fouler);
            let fouler_name = self.tm[def].pl[fouler].p.name.clone();
            if made {
                self.record_fg(off, shooter, assister, kind, pts_val, transition);
                self.log(format!("{shooter_name} {} and is fouled by {fouler_name}! And-one.", shot_desc(kind)));
                let _ = self.free_throws(off, shooter, 1, def, 1.0);
                return ShotEnd::Over;
            } else {
                let n = if is_three { 3 } else { 2 };
                self.log(format!("{fouler_name} fouls {shooter_name} on the shot."));
                return match self.free_throws(off, shooter, n, def, 1.0) {
                    FtEnd::MissedLive => self.rebound(off, def, false, 0.8, None),
                    _ => ShotEnd::Over,
                };
            }
        }

        // FGA recorded
        {
            let pl = &mut self.tm[off].pl[shooter];
            pl.stats.fga += 1;
            if is_three {
                pl.stats.tpa += 1;
            }
        }

        if made {
            self.record_fg_made(off, shooter, assister, kind, pts_val, transition);
            let assist_txt = assister.map(|a| format!(" (assist: {})", self.tm[off].pl[a].p.name)).unwrap_or_default();
            self.log(format!("{shooter_name} {}{}", shot_desc(kind), assist_txt));
            return ShotEnd::Over;
        }

        // Miss: blocked?
        let blk_base = ctx.tune.block_rate
            * match kind {
                Shot::Rim => 1.0,
                Shot::Post => 0.8,
                Shot::Mid => 0.35,
                Shot::Three => 0.08,
            };
        let p_blk = (blk_base * expo(0.035 * (best_block - 50.0)) * (1.0 + 0.5 * blk_fx)).clamp(0.0, 0.45);
        let blocked = self.rng.f64() < p_blk;
        if blocked {
            let w: Vec<f64> = (0..5).map(|s| expo(0.06 * (self.tm[def].pl[self.tm[def].on[s]].p.block - 50.0))).collect();
            let b = self.tm[def].on[self.rng.weighted(&w)];
            self.tm[def].pl[b].stats.blk += 1;
            let bn = self.tm[def].pl[b].p.name.clone();
            self.log(format!("{bn} blocks {shooter_name}'s shot!"));
        } else {
            self.log(format!("{shooter_name} misses a {}.", shot_word(kind)));
        }
        self.rebound(off, def, blocked, 1.0, None)
    }

    fn record_fg(&mut self, off: usize, shooter: usize, assister: Option<usize>, kind: Shot, pts: u16, transition: bool) {
        // FGA + FGM for an and-one
        {
            let pl = &mut self.tm[off].pl[shooter];
            pl.stats.fga += 1;
            if kind == Shot::Three {
                pl.stats.tpa += 1;
            }
        }
        self.record_fg_made(off, shooter, assister, kind, pts, transition);
    }

    fn record_fg_made(&mut self, off: usize, shooter: usize, assister: Option<usize>, kind: Shot, pts: u16, transition: bool) {
        {
            let pl = &mut self.tm[off].pl[shooter];
            pl.stats.fgm += 1;
            pl.stats.pts += pts as u32;
            if kind == Shot::Three {
                pl.stats.tpm += 1;
            }
        }
        if let Some(a) = assister {
            self.tm[off].pl[a].stats.ast += 1;
        }
        if matches!(kind, Shot::Rim | Shot::Post) {
            self.tm[off].paint_pts += pts;
        }
        if transition {
            self.tm[off].fb_pts += pts;
        }
        self.add_points(off, pts);
    }

    /// Shoot `n` free throws. Returns how the trip ended.
    fn free_throws(&mut self, off: usize, shooter: usize, n: u8, _def: usize, _scale: f64) -> FtEnd {
        let ft_base = self.ctx.style.ft_pct;
        let mut last_made = true;
        for k in 0..n {
            let sk = self.tm[off].pl[shooter].p.ft;
            let fat = self.fatigue_pen(self.tm[off].pl[shooter].energy) * 0.2;
            let mut p = ft_base + self.ctx.cal.ft_off + 0.0042 * self.ctx.tune.skill_slope * (sk - self.ctx.refs.ft) - fat * 0.01;
            p += self.tm[off].pl[shooter].p.fx.ft;
            if self.clutch_time() {
                p += 0.0006 * (self.tm[off].pl[shooter].p.clutch - 55.0);
            }
            let p = p.clamp(0.25, 0.97);
            let made = self.rng.f64() < p;
            {
                let pl = &mut self.tm[off].pl[shooter];
                pl.stats.fta += 1;
                if made {
                    pl.stats.ftm += 1;
                    pl.stats.pts += 1;
                }
            }
            if made {
                self.add_points(off, 1);
            }
            last_made = made;
            let _ = k;
        }
        // advance a few seconds of dead-ball time? (clock stops during FTs, nothing to do)
        if last_made {
            FtEnd::Made
        } else {
            FtEnd::MissedLive
        }
    }

    /// Resolve a rebound after a miss. Returns whether the offense retained the ball.
    fn rebound(&mut self, off: usize, def: usize, blocked: bool, orb_scale: f64, _hint: Option<usize>) -> ShotEnd {
        let ctx = self.ctx;
        // Team rebound / out of bounds
        if self.rng.chance(0.045) {
            // Ball goes to whoever it last touched; coin flip biased to defense.
            if self.rng.chance(0.55) {
                self.log("Out of bounds. Defense ball.".to_string());
                return ShotEnd::Over;
            } else {
                return ShotEnd::OffensiveRebound;
            }
        }
        let k = (1.0 - ctx.style.orb) / ctx.style.orb;
        let ro: f64 = self.tm[off].on.iter().map(|&i| {
            let p = &self.tm[off].pl[i];
            expo(0.045 * (p.p.orb - self.fatigue_pen(p.energy) * 0.3 - 50.0)) * (1.0 + p.p.fx.oreb * 5.0)
        }).sum();
        let rd: f64 = self.tm[def].on.iter().map(|&i| {
            let p = &self.tm[def].pl[i];
            expo(0.045 * (p.p.drb - self.fatigue_pen(p.energy) * 0.3 - 50.0)) * (1.0 + p.p.fx.dreb * 5.0)
        }).sum();
        // strategy: crash the glass raises offensive boards; sending players back lowers transition defense
        let crash = 1.0 + 0.25 * (self.tm[off].strat.crash_glass - 0.3);
        let mut p_orb = ro * crash / (ro * crash + k * rd) * orb_scale * ctx.cal.orb_mul;
        if blocked {
            p_orb *= 0.85;
        }
        if self.rng.f64() < p_orb.clamp(0.02, 0.8) {
            // offensive rebound
            let w: Vec<f64> = (0..5).map(|s| expo(0.05 * (self.tm[off].pl[self.tm[off].on[s]].p.orb - 50.0))).collect();
            let r = self.tm[off].on[self.rng.weighted(&w)];
            self.tm[off].pl[r].stats.orb += 1;
            let n = self.tm[off].pl[r].p.name.clone();
            self.log(format!("{n} grabs the offensive rebound."));
            ShotEnd::OffensiveRebound
        } else {
            let w: Vec<f64> = (0..5).map(|s| expo(0.05 * (self.tm[def].pl[self.tm[def].on[s]].p.drb - 50.0))).collect();
            let r = self.tm[def].on[self.rng.weighted(&w)];
            self.tm[def].pl[r].stats.drb += 1;
            // fast break chance after long rebound
            if self.rng.chance(0.17) {
                self.transition = true;
            }
            ShotEnd::Over
        }
    }

    fn finish(&mut self) -> BoxScore {
        let mut out = BoxScore { season: self.ctx.season, playoffs: self.ctx.playoffs, periods: self.periods.clone(), overtimes: self.period.saturating_sub(4), pbp: std::mem::take(&mut self.pbp), ..Default::default() };
        for (ti, tb) in [(0usize, &mut out.home), (1usize, &mut out.away)] {
            let t = &self.tm[ti];
            tb.team = t.id;
            tb.name = t.name.clone();
            tb.pts = t.pts;
            tb.team_fouls = t.fouls_total;
            tb.possessions = t.poss;
            tb.fast_break_pts = t.fb_pts;
            tb.points_in_paint = t.paint_pts;
            tb.biggest_lead = t.best_lead;
            for p in &t.pl {
                let mut s = p.stats.clone();
                s.min = p.secs / 60.0;
                s.g = if p.secs > 0.0 || p.started { 1 } else { 0 };
                s.gs = if p.started { 1 } else { 0 };
                s.plus_minus = p.pm;
                tb.players.push(PlayerBox { id: p.p.id, name: p.p.name.clone(), stats: s, started: p.started, injured_in_game: p.injured, fouled_out: p.fouled_out, ejected: p.ejected });
            }
        }
        out
    }
}

enum ShotEnd {
    Over,
    OffensiveRebound,
}

enum FtEnd {
    Made,
    MissedLive,
}

fn shot_desc(k: Shot) -> &'static str {
    match k {
        Shot::Rim => "scores at the rim",
        Shot::Post => "scores on the block",
        Shot::Mid => "hits a jumper",
        Shot::Three => "drains a three",
    }
}

fn shot_word(k: Shot) -> &'static str {
    match k {
        Shot::Rim => "layup",
        Shot::Post => "post move",
        Shot::Mid => "jumper",
        Shot::Three => "three",
    }
}


// -------------------------------------------------------------------------------------------
// Self-calibration
// -------------------------------------------------------------------------------------------

/// Totals gathered while calibrating.
#[derive(Default, Clone, Debug)]
pub struct LeagueMeasure {
    pub games: f64,
    pub poss: f64,
    pub fga: f64,
    pub fgm: f64,
    pub tpa: f64,
    pub tpm: f64,
    pub fta: f64,
    pub ftm: f64,
    pub tov: f64,
    pub orb: f64,
    pub drb: f64,
    pub pts: f64,
}

impl LeagueMeasure {
    pub fn add(&mut self, b: &BoxScore) {
        self.games += 1.0;
        for t in [&b.home, &b.away] {
            self.poss += t.possessions as f64;
            self.pts += t.pts as f64;
            for p in &t.players {
                let s = &p.stats;
                self.fga += s.fga as f64;
                self.fgm += s.fgm as f64;
                self.tpa += s.tpa as f64;
                self.tpm += s.tpm as f64;
                self.fta += s.fta as f64;
                self.ftm += s.ftm as f64;
                self.tov += s.tov as f64;
                self.orb += s.orb as f64;
                self.drb += s.drb as f64;
            }
        }
    }
    /// Possessions per team per game using the standard box-score estimate (the same formula
    /// historical pace figures use): FGA - OREB + TOV + 0.44 FTA.
    pub fn pace(&self) -> f64 {
        (self.fga - self.orb + self.tov + 0.44 * self.fta) / (2.0 * self.games.max(1.0))
    }
    pub fn three_rate(&self) -> f64 {
        self.tpa / self.fga.max(1.0)
    }
    pub fn fg2(&self) -> f64 {
        (self.fgm - self.tpm) / (self.fga - self.tpa).max(1.0)
    }
    pub fn fg3(&self) -> f64 {
        if self.tpa > 0.0 { self.tpm / self.tpa } else { 0.0 }
    }
    pub fn ft_rate(&self) -> f64 {
        self.fta / self.fga.max(1.0)
    }
    pub fn ft_pct(&self) -> f64 {
        if self.fta > 0.0 { self.ftm / self.fta } else { 0.0 }
    }
    pub fn tov_rate(&self) -> f64 {
        self.tov / self.poss.max(1.0)
    }
    pub fn orb_rate(&self) -> f64 {
        let misses = self.fga - self.fgm;
        self.orb / (self.orb + self.drb).max(1.0) * 0.0 + self.orb / misses.max(1.0)
    }
    pub fn ppg(&self) -> f64 {
        self.pts / (2.0 * self.games.max(1.0))
    }
}

/// Play `games` sample games between random pairs of `teams` and adjust `ctx.cal` so the league
/// averages match the era style. Repeats for `rounds` rounds (2-3 is plenty).
pub fn calibrate(ctx: &mut GameContext, teams: &[GameTeam], rng: &mut Rng, games: usize, rounds: usize) -> LeagueMeasure {
    let mut last = LeagueMeasure::default();
    if teams.len() < 2 {
        return last;
    }
    for _ in 0..rounds {
        let mut m = LeagueMeasure::default();
        for _ in 0..games {
            let a = rng.range_usize(teams.len());
            let mut b = rng.range_usize(teams.len() - 1);
            if b >= a {
                b += 1;
            }
            let bs = simulate_game(ctx, &teams[a], &teams[b], rng);
            m.add(&bs);
        }
        let st = ctx.style;
        let c = &mut ctx.cal;
        c.pace_mul = (c.pace_mul * (st.pace / m.pace().max(1.0))).clamp(0.6, 1.6);
        c.two_off = (c.two_off + 0.9 * (st.fg2 - m.fg2())).clamp(-0.2, 0.2);
        if st.three_rate > 0.0 && m.tpa > 0.0 {
            c.three_off = (c.three_off + 0.9 * (st.fg3 - m.fg3())).clamp(-0.2, 0.2);
        }
        c.ft_off = (c.ft_off + 0.8 * (st.ft_pct - m.ft_pct())).clamp(-0.2, 0.2);
        c.foul_mul = (c.foul_mul * (st.ft_rate / m.ft_rate().max(0.05))).clamp(0.3, 3.0);
        c.tov_mul = (c.tov_mul * (st.tov / m.tov_rate().max(0.01))).clamp(0.4, 2.5);
        c.orb_mul = (c.orb_mul * (st.orb / m.orb_rate().max(0.05))).clamp(0.4, 2.5);
        if st.three_rate > 0.0 && m.tpa > 0.0 {
            c.three_mul = (c.three_mul * (st.three_rate / m.three_rate().max(0.005))).clamp(0.3, 3.0);
        }
        last = m;
    }
    last
}

// -------------------------------------------------------------------------------------------
// Coaching helpers
// -------------------------------------------------------------------------------------------

/// Decide starters and minutes for a roster of game players (healthy ones only).
/// `playoffs` shortens the rotation. Players who already have a positive `target_min` keep it
/// (that's how a human coach overrides minutes); everyone else is filled in to total a full game.
pub fn assign_rotation(players: &mut [GamePlayer], playoffs: bool, rules: &Rules) {
    if players.is_empty() {
        return;
    }
    let user_fixed: Vec<bool> = players.iter().map(|p| p.target_min > 0.0).collect();
    let mut order: Vec<usize> = (0..players.len()).collect();
    order.sort_by(|&a, &b| players[b].ovr.partial_cmp(&players[a].ovr).unwrap());
    // Starters: best 5, but make sure there's a ball handler and (before the positionless era) a big.
    let mut starters: Vec<usize> = order.iter().copied().take(5).collect();
    if !starters.iter().any(|&i| players[i].handle >= 52.0) {
        if let Some(&h) = order.iter().find(|&&i| !starters.contains(&i) && players[i].handle >= 52.0) {
            starters.pop();
            starters.push(h);
        }
    }
    if rules.year < 2018 && !starters.iter().any(|&i| players[i].height >= 80.0) {
        if let Some(&h) = order.iter().find(|&&i| !starters.contains(&i) && players[i].height >= 80.0) {
            let weakest = *starters.iter().min_by(|&&a, &&b| players[a].ovr.partial_cmp(&players[b].ovr).unwrap()).unwrap();
            let pos = starters.iter().position(|&x| x == weakest).unwrap();
            starters[pos] = h;
        }
    }
    for p in players.iter_mut() {
        p.starter = false;
    }
    for &i in &starters {
        players[i].starter = true;
    }
    let base: [f64; 13] = if playoffs {
        [39.0, 37.0, 35.0, 33.0, 31.0, 22.0, 18.0, 13.0, 8.0, 4.0, 0.0, 0.0, 0.0]
    } else {
        [35.0, 33.0, 31.0, 29.0, 27.0, 23.0, 20.0, 16.0, 12.0, 8.0, 4.0, 2.0, 1.0]
    };
    // Starters first (by ovr), then bench (by ovr).
    let mut ranked: Vec<usize> = starters.clone();
    ranked.sort_by(|&a, &b| players[b].ovr.partial_cmp(&players[a].ovr).unwrap());
    for &i in &order {
        if !starters.contains(&i) {
            ranked.push(i);
        }
    }
    let mut auto_total = 0.0;
    let mut fixed_total = 0.0;
    let mut auto: Vec<(usize, f64)> = vec![];
    for (rank, &i) in ranked.iter().enumerate() {
        if user_fixed[i] {
            fixed_total += players[i].target_min;
            continue;
        }
        let mut m = base.get(rank).copied().unwrap_or(0.0);
        m *= 0.88 + 0.24 * players[i].stamina / 100.0;
        m *= 1.0 + (players[i].ovr - 60.0) / 250.0;
        auto_total += m;
        auto.push((i, m));
    }
    let target_total = 5.0 * 4.0 * rules.quarter_minutes;
    let scale = if auto_total > 0.0 { ((target_total - fixed_total) / auto_total).clamp(0.6, 1.6) } else { 1.0 };
    for (i, m) in auto {
        players[i].target_min = (m * scale).min(44.0);
    }
    // Anyone left at zero who is a starter still needs minutes.
    for &i in &starters {
        if players[i].target_min <= 0.0 {
            players[i].target_min = 20.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::generate::*;

    fn make_team(content: &Content, rng: &mut Rng, season: Season, id: TeamId, boost: f64) -> GameTeam {
        let mut players = vec![];
        for i in 0..11 {
            let target = if i < 5 { 66.0 + boost } else { 54.0 + boost - i as f64 * 0.8 };
            let spec = GenSpec::new(season, 26, target, target, OriginKind::College);
            let p = generate_player(content, rng, id as u32 * 100 + i, &spec, 1.0);
            players.push(GamePlayer::from_player(&p, &content.badges, 1.0, 0.0));
        }
        let rules = content.rules(season);
        assign_rotation_simple(&mut players, false, &rules);
        GameTeam { id, name: format!("Team{id}"), players, strategy: Strategy::default() }
    }

    fn assign_rotation_simple(players: &mut [GamePlayer], playoffs: bool, rules: &Rules) {
        assign_rotation(players, playoffs, rules);
    }

    pub fn ctx_for(content: &Content, season: Season) -> GameContext {
        GameContext {
            refs: Refs::default(),
            cal: Cal::default(),
            season,
            rules: content.rules(season),
            style: content.style(season),
            tune: Tune::from_map(&content.tuning),
            home_court: 2.5,
            randomness: 1.0,
            star_power: 1.0,
            fatigue: 1.0,
            foul_rate: 1.0,
            hot_hand: true,
            clutch: true,
            possession_detail: 1.0,
            in_game_injuries: false,
            playoffs: false,
            neutral_site: false,
            play_by_play: false,
        }
    }

    #[test]
    fn a_game_produces_a_sane_box_score() {
        let content = Content::default();
        let mut rng = Rng::new(3);
        let season = 2015;
        let a = make_team(&content, &mut rng, season, 1, 0.0);
        let b = make_team(&content, &mut rng, season, 2, 0.0);
        let mut ctx = ctx_for(&content, season);
        ctx.play_by_play = true;
        let bs = simulate_game(&ctx, &a, &b, &mut rng);
        assert!(bs.home.pts > 50 && bs.away.pts > 50, "{} - {}", bs.home.pts, bs.away.pts);
        assert_ne!(bs.home.pts, bs.away.pts);
        let sum: u32 = bs.home.players.iter().map(|p| p.stats.pts).sum();
        assert_eq!(sum, bs.home.pts as u32, "player points must add up to team points");
        let mins: f64 = bs.home.players.iter().map(|p| p.stats.min).sum();
        assert!((mins - 240.0).abs() < 12.0, "minutes {mins}");
        assert!(!bs.pbp.is_empty());
    }
}
