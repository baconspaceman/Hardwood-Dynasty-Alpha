//! Storylines and league-wide events. (Data first; narrative engine added below.)

use crate::events::*;

fn c(var: &str, op: &str, value: f64) -> Cond {
    Cond {
        var: var.into(),
        op: op.into(),
        value,
    }
}

#[allow(clippy::too_many_arguments)]
fn lev(
    id: &str,
    title: &str,
    text: &str,
    chance: f64,
    min_year: i32,
    max_year: i32,
    conds: Vec<Cond>,
    effects: Vec<Effect>,
) -> EventDef {
    EventDef {
        id: id.into(),
        title: title.into(),
        text: text.into(),
        category: "league".into(),
        chance,
        stages: vec!["league".into()],
        min_year,
        max_year,
        conditions: conds,
        once: min_year == max_year,
        cooldown: 0,
        effects,
        choices: vec![],
    }
}

/// League-wide events. Scheduled history uses chance 1.0 with a fixed year; random ones have a chance per season.
pub fn builtin_league_events() -> Vec<EventDef> {
    let a = Effect::add;
    vec![
        lev(
            "lockout_1998",
            "Lockout shortens the season",
            "Owners and players cannot agree on a labor deal. The season is cut to 50 games.",
            1.0,
            1998,
            1998,
            vec![],
            vec![
                Effect::set("league.games", 50.0),
                a("league.revenue_pct", -12.0),
            ],
        ),
        lev(
            "lockout_2011",
            "Lockout again",
            "A second lockout costs the league part of the schedule. 66 games are played.",
            1.0,
            2011,
            2011,
            vec![],
            vec![
                Effect::set("league.games", 66.0),
                a("league.revenue_pct", -8.0),
            ],
        ),
        lev(
            "pandemic_2019",
            "A pandemic halts the sport",
            "A global pandemic suspends play. The season resumes in a sealed bubble with no fans.",
            1.0,
            2019,
            2019,
            vec![],
            vec![
                Effect::set("league.games", 72.0),
                a("league.revenue_pct", -25.0),
                a("league.attendance_pct", -90.0),
            ],
        ),
        lev(
            "pandemic_2020",
            "A second shortened season",
            "The next season starts late and runs 72 games with limited crowds.",
            1.0,
            2020,
            2020,
            vec![],
            vec![
                Effect::set("league.games", 72.0),
                a("league.revenue_pct", -12.0),
                a("league.attendance_pct", -45.0),
            ],
        ),
        lev(
            "tv_deal_1995",
            "Landmark television deal",
            "A huge new national TV contract is signed.",
            1.0,
            1995,
            1995,
            vec![],
            vec![a("league.revenue_pct", 25.0)],
        ),
        lev(
            "tv_deal_2015",
            "The TV money boom",
            "A massive new TV deal sends the salary cap soaring.",
            1.0,
            2015,
            2015,
            vec![],
            vec![a("league.revenue_pct", 38.0)],
        ),
        lev(
            "recession",
            "Economic recession",
            "A recession bites into ticket sales and sponsorship.",
            0.015,
            1950,
            9999,
            vec![],
            vec![a("league.revenue_pct", -7.0)],
        ),
        lev(
            "boom",
            "Business boom",
            "A strong economy lifts revenue league-wide.",
            0.03,
            1950,
            9999,
            vec![],
            vec![a("league.revenue_pct", 5.0)],
        ),
        lev(
            "new_tv_random",
            "New media rights deal",
            "A new media rights deal boosts league revenue.",
            0.04,
            2000,
            9999,
            vec![c("year", ">=", 2000.0)],
            vec![a("league.revenue_pct", 10.0)],
        ),
        lev(
            "referee_scandal",
            "Officiating controversy",
            "A controversy over officiating dominates headlines.",
            0.01,
            1950,
            9999,
            vec![],
            vec![a("league.credibility", -3.0)],
        ),
        lev(
            "star_retirement_wave",
            "Generational changing of the guard",
            "Several legends announce their retirement in the same offseason.",
            0.01,
            1950,
            9999,
            vec![],
            vec![Effect::news("A generation of stars is stepping away.")],
        ),
    ]
}

// -------------------------------------------------------------------------------------------
// Narrative engine
// -------------------------------------------------------------------------------------------

use crate::league::League;
use crate::player::{Level, Position};
use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chapter {
    pub season: Season,
    pub day: u32,
    pub text: String,
}

/// A multi-chapter storyline the league is living through.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Story {
    pub id: u32,
    /// `dynasty`, `rivalry`, `comeback`, `lastdance`, `superteam`, `cinderella`, `drought`, `tragedy`, `feud`, `rookie`, `tank`...
    pub kind: String,
    pub title: String,
    pub season: Season,
    pub major: bool,
    pub chapters: Vec<Chapter>,
    pub players: Vec<PlayerId>,
    pub teams: Vec<TeamId>,
    pub active: bool,
}

impl League {
    fn intensity(&self) -> f64 {
        self.settings.num("story.intensity")
    }

    /// Start a story, or add a chapter if a story of the same kind and key already exists this era.
    fn story_upsert(
        &mut self,
        kind: &str,
        key_players: &[PlayerId],
        key_teams: &[TeamId],
        title: &str,
        chapter: &str,
        major: bool,
    ) -> u32 {
        if self.intensity() <= 0.0 {
            return 0;
        }
        let (season, day) = (self.year, self.day);
        let existing = self.stories.iter_mut().find(|s| {
            s.active && s.kind == kind && s.players == key_players && s.teams == key_teams
        });
        if let Some(s) = existing {
            if s.chapters.last().map(|c| c.text.as_str()) != Some(chapter) {
                s.chapters.push(Chapter {
                    season,
                    day,
                    text: chapter.to_string(),
                });
            }
            return s.id;
        }
        let id = self.stories.len() as u32 + 1;
        self.stories.push(Story {
            id,
            kind: kind.into(),
            title: title.into(),
            season,
            major,
            chapters: vec![Chapter {
                season,
                day,
                text: chapter.to_string(),
            }],
            players: key_players.to_vec(),
            teams: key_teams.to_vec(),
            active: true,
        });
        if major {
            self.add_news("story", format!("STORYLINE: {title}"));
        }
        id
    }

    pub fn story_weekly(&mut self) {
        if self.intensity() <= 0.0 {
            return;
        }
        let games_played = self.day;
        if games_played < 14 {
            return;
        }
        let ids = self.active_team_ids();
        // winning / losing streaks
        for &t in &ids {
            let (streak, name) = (self.team(t).record.streak, self.team(t).name());
            if streak >= 11 {
                self.story_upsert(
                    "streak",
                    &[],
                    &[t],
                    &format!("{name}'s winning streak"),
                    &format!("{name} have won {streak} straight."),
                    false,
                );
            } else if streak <= -12 {
                self.story_upsert(
                    "streak",
                    &[],
                    &[t],
                    &format!("{name}'s skid"),
                    &format!("{name} have lost {} straight.", -streak),
                    false,
                );
            }
        }
        // players on pace for something special
        let cands = self.award_candidates(false);
        if let Some(c) = cands
            .iter()
            .filter(|c| c.games >= 14)
            .max_by(|a, b| a.ppg.partial_cmp(&b.ppg).unwrap())
        {
            let p = self.p(c.id);
            if c.ppg >= self.style.ppg * 0.32 {
                let name = p.name();
                let txt = format!("{name} is averaging {:.1} points per game.", c.ppg);
                self.story_upsert(
                    "scoring",
                    &[c.id],
                    &[],
                    &format!("{name} on a scoring tear"),
                    &txt,
                    false,
                );
            }
        }
        // trade demands
        let wants: Vec<PlayerId> = self
            .players
            .iter()
            .filter(|p| p.mood.wants_trade && p.is_active_pro() && p.ovr >= 66)
            .map(|p| p.id)
            .collect();
        for id in wants {
            let p = self.p(id);
            let (n, t) = (p.name(), p.team_id());
            if let Some(t) = t {
                let tn = self.team(t).name();
                self.story_upsert(
                    "feud",
                    &[id],
                    &[t],
                    &format!("{n} wants out of {tn}"),
                    &format!("{n} has requested a trade from {tn}."),
                    false,
                );
            }
        }
    }

    pub fn story_comeback(&mut self, id: PlayerId, games: u16) {
        let p = self.p(id);
        if p.ovr < 66 {
            return;
        }
        let (n, t) = (p.name(), p.team_id());
        let tn = t.map(|t| self.team(t).name()).unwrap_or_default();
        self.story_upsert(
            "comeback",
            &[id],
            &[],
            &format!("{n}: the long road back"),
            &format!("{n} returns for {tn} after missing {games} games."),
            games > 60,
        );
    }

    pub fn story_series_done(&mut self, winner: TeamId, loser: TeamId, round: usize) {
        if self.intensity() <= 0.0 {
            return;
        }
        let (wn, ln) = (self.team(winner).name(), self.team(loser).name());
        // rivalry: count playoff meetings in the last 4 seasons
        let mut meetings = 1;
        for s in self.stories.iter() {
            if s.kind == "rivalry" && s.teams.contains(&winner) && s.teams.contains(&loser) {
                meetings += s.chapters.len();
            }
        }
        let mut teams = vec![winner.min(loser), winner.max(loser)];
        teams.sort();
        if meetings >= 2 {
            self.story_upsert(
                "rivalry",
                &[],
                &teams,
                &format!("{wn} vs. {ln}: a playoff rivalry"),
                &format!("{} {}: {wn} beat {ln} again.", self.year, round + 1),
                meetings >= 3,
            );
        } else {
            self.story_upsert(
                "rivalry",
                &[],
                &teams,
                &format!("{wn} vs. {ln}: a playoff rivalry"),
                &format!("{} playoffs: {wn} eliminate {ln}.", self.year),
                false,
            );
        }
        // Cinderella: big upset by seed
        if let Some(st) = &self.playoffs {
            if let Some(s) = st.rounds.get(round).and_then(|r| {
                r.iter()
                    .find(|s| s.winner == Some(winner) && (s.low == winner || s.high == winner))
            }) {
                if s.low == winner && s.low_seed >= s.high_seed + 4 && s.best_of > 1 {
                    let title = format!("{wn}'s Cinderella run");
                    let txt = format!(
                        "The {}-seed {wn} knock off the {}-seed {ln}.",
                        s.low_seed, s.high_seed
                    );
                    self.story_upsert("cinderella", &[], &[winner], &title, &txt, false);
                }
            }
        }
    }

    pub fn story_champion(&mut self, c: TeamId) {
        let name = self.team(c).name();
        let titles: Vec<Season> = self.team(c).titles.clone();
        let recent = titles.iter().filter(|&&y| y > self.year - 6).count();
        let consecutive = titles
            .iter()
            .rev()
            .enumerate()
            .take_while(|(i, &y)| y == self.year - *i as i32)
            .count();
        if consecutive == 2 {
            self.story_upsert(
                "dynasty",
                &[],
                &[c],
                &format!("{name}: back-to-back champions"),
                &format!("{} repeat as champions in {}.", name, self.year),
                false,
            );
        }
        if consecutive >= 3 {
            self.story_upsert(
                "dynasty",
                &[],
                &[c],
                &format!("The {name} dynasty"),
                &format!(
                    "{name} win a {} title in a row ({}).",
                    ordinal_word(consecutive),
                    self.year
                ),
                true,
            );
        } else if recent >= 3 {
            self.story_upsert(
                "dynasty",
                &[],
                &[c],
                &format!("The {name} dynasty"),
                &format!("{name} collect another ring: {recent} titles in six years."),
                true,
            );
        }
        if titles.len() == 1 {
            let longest = self.year
                - self
                    .team(c)
                    .history
                    .first()
                    .map(|h| h.season)
                    .unwrap_or(self.year);
            self.story_upsert(
                "drought",
                &[],
                &[c],
                &format!("{name} finally win it all"),
                &format!("{name} win their first title after {longest} seasons of waiting."),
                longest >= 10,
            );
        }
        // Hall-of-fame-calibre star's first ring
        if let Some(best) = self
            .team(c)
            .roster
            .iter()
            .copied()
            .max_by_key(|&id| self.p(id).ovr)
        {
            let p = self.p(best);
            if p.ovr >= 78
                && !p
                    .awards
                    .iter()
                    .any(|a| a.award == "Champion" && a.season != self.year)
            {
                let n = p.name();
                self.story_upsert(
                    "redemption",
                    &[best],
                    &[c],
                    &format!("{n} gets his ring"),
                    &format!("{n} wins his first championship with {name}."),
                    p.age_now_hint(self.year) >= 30,
                );
            }
        }
    }

    pub fn story_offseason(&mut self) {
        if self.intensity() <= 0.0 {
            return;
        }
        for t in self.active_team_ids() {
            let stars: Vec<PlayerId> = self
                .team(t)
                .roster
                .iter()
                .copied()
                .filter(|&id| self.p(id).ovr >= 78)
                .collect();
            if stars.len() >= 3 {
                let n = self.team(t).name();
                self.story_upsert(
                    "superteam",
                    &[],
                    &[t],
                    &format!("{n}: a superteam forms"),
                    &format!("{n} have {} players rated 78 or higher.", stars.len()),
                    true,
                );
            }
            // last dance
            for &id in &self.team(t).roster.clone() {
                let p = self.p(id);
                if p.ovr >= 72 && self.year + 1 - p.birth_year >= 35 {
                    let n = p.name();
                    let tn = self.team(t).name();
                    self.story_upsert(
                        "lastdance",
                        &[id],
                        &[t],
                        &format!("{n}'s last dance"),
                        &format!(
                            "At {}, {n} returns to {tn} for what may be a final season.",
                            self.year + 1 - p.birth_year
                        ),
                        p.hof_hint(),
                    );
                }
            }
        }
        // retire inactive stories older than 6 seasons
        let y = self.year;
        for s in self.stories.iter_mut() {
            if y - s.season > 6 {
                s.active = false;
            }
        }
    }

    pub fn story_signing(&mut self, id: PlayerId, t: TeamId) {
        let p = self.p(id);
        if p.ovr < 76 {
            return;
        }
        let (n, tn) = (p.name(), self.team(t).name());
        let major = p.ovr >= 84;
        self.story_upsert(
            "signing",
            &[id],
            &[t],
            &format!("{n} picks {tn}"),
            &format!("{n} signs with {tn}, a move that reshapes the league."),
            major,
        );
    }

    pub fn story_free_agency_open(&mut self) {
        let mut top: Vec<PlayerId> = self.free_agents.clone();
        top.sort_by(|&a, &b| self.p(b).ovr.cmp(&self.p(a).ovr));
        for id in top.into_iter().take(3) {
            let p = self.p(id);
            if p.ovr >= 70 {
                let (n, o) = (p.name(), p.ovr);
                self.add_news(
                    "freeagency",
                    format!("Top free agent: {n} ({o} overall). Who will land him?"),
                );
            }
        }
    }

    pub fn story_draft_done(&mut self) {
        if let Some(d) = &self.draft {
            if let Some(first) = d.results.iter().find(|r| r.overall == 1) {
                let p = self.p(first.player);
                let (n, t) = (p.name(), self.team(first.team).name());
                let txt = format!(
                    "{n} is the face of {t}'s rebuild as the {} #1 pick.",
                    d.year
                );
                let (pid, tid, y) = (first.player, first.team, d.year);
                self.story_upsert(
                    "rookie",
                    &[pid],
                    &[tid],
                    &format!("{n}, the No. 1 pick of {y}"),
                    &txt,
                    false,
                );
            }
        }
    }

    pub fn on_career_ending_injury(&mut self, id: PlayerId) {
        let p = self.p(id);
        if p.ovr < 62 {
            return;
        }
        let n = p.name();
        let ovr = p.ovr;
        self.story_upsert(
            "tragedy",
            &[id],
            &[],
            &format!("{n}: a career cut short"),
            &format!("{n} (a {ovr}) must walk away from the game because of injury."),
            ovr >= 72,
        );
    }

    pub fn story_text(&self, s: &Story) -> String {
        let mut out = format!("{} [{}]\n", s.title, s.kind);
        for c in &s.chapters {
            out += &format!("  {}: {}\n", c.season, c.text);
        }
        out
    }

    /// Premade "storyline" starting scenarios the player can choose to begin a career.
    pub fn scenarios() -> Vec<Scenario> {
        vec![
            Scenario { id: "rebuild", name: "The Long Rebuild", text: "You inherit the league's worst roster, a patient owner, and a deep draft class. Build a champion from nothing." },
            Scenario { id: "dynasty_end", name: "End of an Era", text: "Your team just won it all, but the stars are getting old and the cap is tight. Reload without falling off a cliff." },
            Scenario { id: "small_market", name: "Small Market, Big Dreams", text: "A small-market club with a loyal owner and little cash. Compete with the giants on a budget." },
            Scenario { id: "win_now", name: "Win Now or Else", text: "A talented but underperforming team and an owner who wants a title within two years." },
            Scenario { id: "superstar", name: "Franchise Player", text: "You have the league's best young player, signed for only two more years. Build around him before he walks." },
        ]
    }
}

pub struct Scenario {
    pub id: &'static str,
    pub name: &'static str,
    pub text: &'static str,
}

fn ordinal_word(n: usize) -> String {
    match n {
        3 => "third".into(),
        4 => "fourth".into(),
        5 => "fifth".into(),
        n => format!("{n}th"),
    }
}

impl crate::player::Player {
    pub fn age_now_hint(&self, year: Season) -> i32 {
        year - self.birth_year
    }
    pub fn hof_hint(&self) -> bool {
        self.awards.len() >= 8
    }
}

#[allow(dead_code)]
fn unused(_: Position, _: Level) {}
