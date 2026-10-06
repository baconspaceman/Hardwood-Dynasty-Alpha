//! Careers: be a player (with a full life), GM, head coach, owner, scout, or college coach / AD.
//!
//! The same living world runs underneath every role. Your role decides which controls you have
//! and which decisions the game stops to ask you about. A career can change roles over time: a
//! player can retire and become a coach, a coach a GM, a GM an owner.

use crate::contract::*;
use crate::generate::*;
use crate::league::*;
use crate::life::*;
use crate::player::*;
use crate::rng::Rng;
use crate::season::DayReport;
use crate::team::*;
use crate::types::*;

/// How a created player is built. Everything has a sensible default.
#[derive(Clone, Debug)]
pub struct CreatePlayerSpec {
    pub first: String,
    pub last: String,
    /// Height in inches.
    pub height_in: u8,
    pub position: Position,
    pub archetype: String,
    pub country: String,
    pub hometown: String,
    /// 1 (role player ceiling) to 5 (generational).
    pub talent: u8,
    /// Extra skill points to spend: (family key, points). Total is capped by `talent`'s budget.
    pub points: Vec<(String, u8)>,
    /// Where the story starts: "hs9", "hs10", "hs11", "hs12", "college", "pro".
    pub start: String,
    pub work_ethic: u8,
    pub personality: Vec<String>,
}

impl Default for CreatePlayerSpec {
    fn default() -> Self {
        CreatePlayerSpec {
            first: "Alex".into(),
            last: "Rivers".into(),
            height_in: 77,
            position: Position::SG,
            archetype: "two_way_wing".into(),
            country: "USA".into(),
            hometown: "Chicago".into(),
            talent: 3,
            points: vec![],
            start: "hs9".into(),
            work_ethic: 70,
            personality: vec![],
        }
    }
}

/// Skill points available at each talent level (beyond the archetype's natural shape).
pub fn point_budget(talent: u8) -> u32 {
    match talent {
        1 => 40,
        2 => 50,
        3 => 60,
        4 => 70,
        _ => 80,
    }
}

pub fn talent_label(talent: u8) -> &'static str {
    match talent {
        1 => "Solid contributor (ceiling: bench/role player)",
        2 => "Promising (ceiling: rotation player)",
        3 => "High-level prospect (ceiling: starter / occasional all-star)",
        4 => "Elite prospect (ceiling: all-star)",
        _ => "Generational (ceiling: all-time great)",
    }
}

impl League {
    // ------------------------------------------------------------------ roles

    /// Take a job on a team. Returns a plain-English summary.
    pub fn take_role(
        &mut self,
        role: Role,
        team: Option<TeamId>,
        college: Option<CollegeId>,
    ) -> Result<String, String> {
        match role {
            Role::Gm | Role::HeadCoach | Role::AssistantCoach | Role::Scout | Role::Owner => {
                let t = team.ok_or("Pick a team for that role.")?;
                if !self.team(t).active {
                    return Err("That franchise isn't active.".into());
                }
                self.user.team = Some(t);
                if !self.user.roles.contains(&role) {
                    self.user.roles.push(role);
                }
                if role == Role::Owner {
                    // owners hold the GM and coach jobs unless delegated
                    self.user.delegate_gm = false;
                    self.user.delegate_coach = false;
                    self.team_mut(t).owner.name = self.user.name.clone();
                    self.team_mut(t).owner.approval = 100.0;
                    self.team_mut(t).owner.patience = 100.0;
                }
                let tn = self.team(t).name();
                self.user.log.push(format!(
                    "{}: became {} of the {tn}.",
                    self.year,
                    role.name()
                ));
                Ok(format!("You are now the {} of the {tn}.", role.name()))
            }
            Role::CollegeCoach | Role::CollegeAd | Role::CollegeScout => {
                let c = college.ok_or("Pick a college for that role.")?;
                if (c as usize) >= self.colleges.len() {
                    return Err("There is no such college.".into());
                }
                self.user.college = Some(c);
                if !self.user.roles.contains(&role) {
                    self.user.roles.push(role);
                }
                let cn = self.colleges[c as usize].name.clone();
                if role == Role::CollegeCoach {
                    self.colleges[c as usize].coach.name = self.user.name.clone();
                }
                self.user
                    .log
                    .push(format!("{}: became {} at {cn}.", self.year, role.name()));
                Ok(format!("You are now {} at {cn}.", role.name()))
            }
            Role::Player => Err("Use create-player to start a player career.".into()),
        }
    }

    pub fn leave_role(&mut self, role: Role) {
        self.user.roles.retain(|r| *r != role);
        if !self.user.roles.iter().any(|r| {
            matches!(
                r,
                Role::Gm | Role::HeadCoach | Role::AssistantCoach | Role::Scout | Role::Owner
            )
        }) {
            self.user.team = None;
        }
        if !self
            .user
            .roles
            .iter()
            .any(|r| matches!(r, Role::CollegeCoach | Role::CollegeAd | Role::CollegeScout))
        {
            self.user.college = None;
        }
    }

    // ------------------------------------------------------------------ coaching controls

    pub fn set_minutes(&mut self, player: PlayerId, minutes: f64) -> Result<String, String> {
        let t = self.user.team.ok_or("You don't run a team.")?;
        if !self.user_controls_lineup(t) {
            return Err(
                "Only the Head Coach (or an Owner who runs the bench) sets minutes.".into(),
            );
        }
        if self.p(player).team_id() != Some(t) {
            return Err("He isn't on your team.".into());
        }
        if !(0.0..=48.0).contains(&minutes) {
            return Err("Minutes must be between 0 and 48.".into());
        }
        if minutes <= 0.0 {
            self.team_mut(t).minutes_override.remove(&player);
        } else {
            self.team_mut(t).minutes_override.insert(player, minutes);
        }
        Ok(format!(
            "{} will play about {:.0} minutes when healthy.",
            self.p(player).name(),
            minutes
        ))
    }

    pub fn set_starters(&mut self, ids: Vec<PlayerId>) -> Result<String, String> {
        let t = self.user.team.ok_or("You don't run a team.")?;
        if !self.user_controls_lineup(t) {
            return Err(
                "Only the Head Coach (or an Owner who runs the bench) sets the lineup.".into(),
            );
        }
        if ids.len() != 5 {
            return Err("Pick exactly five starters.".into());
        }
        for id in &ids {
            if self.p(*id).team_id() != Some(t) {
                return Err(format!("{} isn't on your team.", self.p(*id).name()));
            }
        }
        self.team_mut(t).starters_override = ids;
        Ok("Starting lineup set.".into())
    }

    pub fn set_strategy_field(&mut self, field: &str, value: &str) -> Result<String, String> {
        let t = self.user.team.ok_or("You don't run a team.")?;
        if !self.user_controls_lineup(t) {
            return Err(
                "Only the Head Coach (or an Owner who runs the bench) sets strategy.".into(),
            );
        }
        let num = || {
            value
                .parse::<f64>()
                .map_err(|_| format!("'{value}' isn't a number."))
        };
        let s = &mut self.teams[t as usize].strategy;
        match field {
            "tempo" => s.tempo = num()?.clamp(-1.0, 1.0),
            "threes" | "three" => s.three_emphasis = num()?.clamp(-1.0, 1.0),
            "inside" => s.inside_focus = num()?.clamp(0.0, 1.0),
            "glass" | "crash" => s.crash_glass = num()?.clamp(0.0, 1.0),
            "hack" => s.hack_a = matches!(value, "on" | "true" | "yes" | "1"),
            "defense" => {
                s.defense = match value {
                    "man" => crate::game::DefScheme::Man,
                    "zone" => {
                        if !self.rules.zone_defense {
                            return Err("Zone defense is illegal in this era.".into());
                        }
                        crate::game::DefScheme::Zone
                    }
                    "press" => crate::game::DefScheme::Press,
                    _ => return Err("Defense must be man, zone or press.".into()),
                }
            }
            _ => return Err("Fields: tempo (-1..1), threes (-1..1), inside (0..1), glass (0..1), hack (on/off), defense (man/zone/press).".into()),
        }
        Ok(format!("Strategy updated: {field} = {value}."))
    }

    pub fn set_dev_focus(&mut self, player: PlayerId, focus: DevFocus) -> Result<String, String> {
        let t = self.p(player).team_id();
        let owns = t
            .map(|t| {
                self.user.team == Some(t)
                    && (self.user.has(Role::Gm)
                        || self.user.has(Role::HeadCoach)
                        || self.user.has(Role::AssistantCoach)
                        || self.user.has(Role::Owner))
            })
            .unwrap_or(false);
        if !owns && !self.p(player).user_controlled {
            return Err("You can only set practice focus for your own players.".into());
        }
        self.pm(player).dev_focus = focus;
        Ok(format!(
            "{} will work on: {}.",
            self.p(player).name(),
            focus.name()
        ))
    }

    // ------------------------------------------------------------------ scouting

    /// Scout a prospect: raises how well you know him (reduces fog).
    pub fn scout_prospect(&mut self, id: PlayerId) -> Result<String, String> {
        if !self.user.roles.iter().any(|r| {
            matches!(
                r,
                Role::Gm
                    | Role::Scout
                    | Role::CollegeScout
                    | Role::Owner
                    | Role::CollegeCoach
                    | Role::CollegeAd
                    | Role::AssistantCoach
            )
        }) {
            return Err("You need a front-office role to scout.".into());
        }
        let cost = 8.0;
        if self.user.scouting_points < cost {
            return Err(format!(
                "Not enough scouting points ({:.0} left, {cost} needed). They refill each month.",
                self.user.scouting_points
            ));
        }
        self.user.scouting_points -= cost;
        let lvl = self.user.scouted.entry(id).or_insert(0);
        *lvl = (*lvl as u32 + 25).min(100) as u8;
        let level = *lvl;
        let (o, pt, sigma) = self.scouted_view(self.user.team, id);
        let p = self.p(id);
        Ok(format!("{} ({}, {}): est. overall {:.0}, potential {:.0}, uncertainty ±{:.1}. Familiarity {}%.", p.name(), p.position.name(), p.height_str(), o, pt, sigma, level))
    }

    pub fn refill_scouting_points(&mut self) {
        let mut pts = 40.0;
        if self.user.has(Role::Scout) || self.user.has(Role::CollegeScout) {
            pts = 90.0;
        }
        self.user.scouting_points = pts;
    }

    // ------------------------------------------------------------------ staff (GM / owner)

    pub fn hire_staff(&mut self, person: PersonId, as_role: StaffRole) -> Result<String, String> {
        let t = self.user.team.ok_or("You don't run a team.")?;
        if !(self.user.has(Role::Gm) || self.user.has(Role::Owner)) {
            return Err("Only a GM or Owner hires staff.".into());
        }
        let p = self.person(person).clone();
        if p.team.is_some() {
            return Err("He is already employed.".into());
        }
        if p.role != as_role {
            return Err(format!(
                "{} is a {}, not a {}.",
                p.name,
                p.role.name(),
                as_role.name()
            ));
        }
        let team = self.team_mut(t);
        match as_role {
            StaffRole::HeadCoach => {
                if let Some(old) = team.head_coach {
                    self.people[old as usize].team = None;
                }
                self.teams[t as usize].head_coach = Some(person);
            }
            StaffRole::GeneralManager => return Err("You are the GM.".into()),
            StaffRole::AssistantCoach => {
                if team.assistants.len() >= 4 {
                    return Err("You already have 4 assistants. Fire one first.".into());
                }
                team.assistants.push(person);
            }
            StaffRole::Scout => {
                if team.scouts.len() >= 5 {
                    return Err("You already have 5 scouts. Fire one first.".into());
                }
                team.scouts.push(person);
            }
            StaffRole::Medical => {
                if let Some(old) = team.trainer {
                    self.people[old as usize].team = None;
                }
                self.teams[t as usize].trainer = Some(person);
            }
        }
        self.people[person as usize].team = Some(t);
        self.people[person as usize].years_left = 3;
        Ok(format!("{} hired as {}.", p.name, as_role.name()))
    }

    pub fn fire_staff(&mut self, person: PersonId) -> Result<String, String> {
        let t = self.user.team.ok_or("You don't run a team.")?;
        if !(self.user.has(Role::Gm) || self.user.has(Role::Owner)) {
            return Err("Only a GM or Owner fires staff.".into());
        }
        let tm = self.team_mut(t);
        let name;
        if tm.head_coach == Some(person) {
            tm.head_coach = None;
        } else if tm.trainer == Some(person) {
            tm.trainer = None;
        } else if tm.assistants.contains(&person) {
            tm.assistants.retain(|&x| x != person);
        } else if tm.scouts.contains(&person) {
            tm.scouts.retain(|&x| x != person);
        } else {
            return Err("That person doesn't work for you.".into());
        }
        name = self.people[person as usize].name.clone();
        self.people[person as usize].team = None;
        // dead money = remaining salary
        let sal = self.people[person as usize].salary;
        self.team_mut(t).finance.staff += sal / 2;
        Ok(format!(
            "{name} has been let go (you owe half a year's salary)."
        ))
    }

    // ------------------------------------------------------------------ college roles

    pub fn college_offer(
        &mut self,
        hs_player: PlayerId,
        pitch_points: f64,
    ) -> Result<String, String> {
        let c = self
            .user
            .college
            .ok_or("You don't run a college program.")?;
        if !(self.user.has(Role::CollegeCoach) || self.user.has(Role::CollegeAd)) {
            return Err("Only a college coach or AD recruits.".into());
        }
        let p = self.p(hs_player);
        if p.affiliation != Affiliation::HighSchool {
            return Err("He isn't a high-school recruit.".into());
        }
        if self.colleges[c as usize].roster.len() >= 13 {
            return Err("Your roster is full (13 scholarships).".into());
        }
        let pts = pitch_points.clamp(0.0, self.colleges[c as usize].recruiting_points);
        self.colleges[c as usize].recruiting_points -= pts;
        let bonus = pts * 0.35 + self.colleges[c as usize].coach.recruiting * 0.05;
        // Compare against his best alternative among a handful of programs.
        let mine = self.recruit_interest(c, hs_player, bonus);
        let mut rng = Rng::from_label(
            self.seed,
            &format!("recruit-{}-{}-{}", self.year, c, hs_player),
        );
        let mut best_other = 0.0f64;
        let mut other = None;
        for oc in self
            .colleges
            .iter()
            .filter(|x| x.id != c && x.roster.len() < 13)
            .map(|x| x.id)
            .collect::<Vec<_>>()
        {
            let s = self.recruit_interest(oc, hs_player, 0.0) + rng.gauss(0.0, 5.0);
            if s > best_other {
                best_other = s;
                other = Some(oc);
            }
        }
        let name = self.p(hs_player).name();
        if mine + rng.gauss(0.0, 4.0) >= best_other {
            self.commit_recruit(hs_player, c);
            Ok(format!("{name} commits to your program!"))
        } else {
            let rival = other
                .map(|o| self.colleges[o as usize].name.clone())
                .unwrap_or_default();
            Ok(format!("{name} is leaning toward {rival}. Spend more recruiting points or improve your pitch (prestige {:.0}, facilities {:.0}).", self.colleges[c as usize].prestige, self.colleges[c as usize].facilities))
        }
    }

    pub fn college_set_nil(&mut self, amount: Money) -> Result<String, String> {
        let c = self
            .user
            .college
            .ok_or("You don't run a college program.")?;
        if !self.user.has(Role::CollegeAd) {
            return Err("Only the Athletic Director sets the NIL budget.".into());
        }
        if !self.nil_available() {
            return Err("NIL deals aren't allowed in this era (or are turned off).".into());
        }
        self.colleges[c as usize].nil_budget = amount.max(0);
        Ok("NIL collective budget updated.".into())
    }

    // ------------------------------------------------------------------ create-a-player

    /// Validate and build a new player, placing him in the world.
    pub fn create_player(&mut self, spec: &CreatePlayerSpec) -> Result<PlayerId, String> {
        if !(60..=94).contains(&spec.height_in) {
            return Err("Height must be between 5'0\" (60) and 7'10\" (94) inches.".into());
        }
        if !self
            .content
            .archetypes
            .iter()
            .any(|a| a.id == spec.archetype)
        {
            return Err(format!(
                "Unknown archetype '{}'. Options: {}.",
                spec.archetype,
                self.content
                    .archetypes
                    .iter()
                    .map(|a| a.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !(1..=5).contains(&spec.talent) {
            return Err("Talent must be 1-5.".into());
        }
        let budget = point_budget(spec.talent);
        let total: u32 = spec.points.iter().map(|(_, p)| *p as u32).sum();
        if total > budget {
            return Err(format!(
                "You spent {total} skill points but only have {budget} at this talent level."
            ));
        }
        for (k, _) in &spec.points {
            if crate::player::Family::from_key(k).is_none() {
                return Err(format!("Unknown skill family '{k}'. Options: inside, mid, three, playmaking, perimeter_d, interior_d, rebounding, athletic, mental."));
            }
        }
        let mut rng = self.rng.fork("create-player");
        // age and stage
        let (age, class, stage, origin) = match spec.start.as_str() {
            "hs9" => (14, 9u8, LifeStage::HighSchool, OriginKind::HighSchool),
            "hs10" => (15, 10, LifeStage::HighSchool, OriginKind::HighSchool),
            "hs11" => (16, 11, LifeStage::HighSchool, OriginKind::HighSchool),
            "hs12" => (17, 12, LifeStage::HighSchool, OriginKind::HighSchool),
            "college" => (18, 1, LifeStage::College, OriginKind::College),
            "pro" => (21, 0, LifeStage::Pro, OriginKind::College),
            "overseas" => (19, 0, LifeStage::Overseas, OriginKind::International),
            other => {
                return Err(format!(
                    "Unknown start '{other}'. Use hs9, hs10, hs11, hs12, college, overseas or pro."
                ))
            }
        };
        let potential: f64 = match spec.talent {
            1 => 62.0,
            2 => 70.0,
            3 => 78.0,
            4 => 86.0,
            _ => 95.0,
        };
        // current level by age (peak minus the age shift)
        // Teen talent: a future star is still well below pro level in high school.
        let ovr_now = if age < 19 {
            25.0 + (potential - 55.0).max(0.0) * 0.6 - (18 - age).max(0) as f64 * 2.5
        } else {
            potential + crate::progression::age_shift(age)
        };
        let mut gs = GenSpec::new(self.year, age, ovr_now.max(26.0), potential, origin);
        gs.height_in = Some(spec.height_in);
        gs.position = Some(spec.position);
        gs.archetype = Some(spec.archetype.clone());
        gs.country = Some(spec.country.clone());
        let mut p = generate_player(&self.content, &mut rng, 0, &gs, 1.0);
        p.first = spec.first.clone();
        p.last = spec.last.clone();
        p.hometown = spec.hometown.clone();
        p.user_controlled = true;
        p.hidden.work_ethic = spec.work_ethic;
        p.hidden.bust = 0.0;
        // spend skill points
        for (fam_key, pts) in &spec.points {
            let fam = crate::player::Family::from_key(fam_key).unwrap();
            let attrs: Vec<Attr> = Attr::ALL
                .iter()
                .copied()
                .filter(|a| a.family() == fam)
                .collect();
            let per = *pts as f64 / attrs.len() as f64 * 0.9;
            for a in attrs {
                p.attrs.add(a, per);
            }
        }
        p.recompute_ovr();
        p.potential = p.potential.max(p.ovr);
        p.badges = earned_badges(&p.attrs, &self.content.badges, 6);
        let school_name = match stage {
            LifeStage::HighSchool => {
                let (a, b) = crate::names::school_words();
                format!("{} {}", rng.pick(&a), rng.pick(&b))
            }
            _ => String::new(),
        };
        p.origin.school = school_name.clone();
        let mut life = LifeState::new(&self.content.life, stage, &school_name, class);
        // personality traits
        life.traits = spec.personality.clone();
        let fam_names = ["Mom", "Dad"];
        for n in fam_names {
            let (f, l) = crate::names::random_name(&mut rng, &self.content.pools[0]);
            let ln = if n == "Mom" || n == "Dad" {
                spec.last.clone()
            } else {
                l
            };
            life.relationships.push(Relationship {
                name: format!("{f} {ln}"),
                kind: "parent".into(),
                closeness: 60.0 + rng.gauss(0.0, 10.0),
                note: n.to_lowercase(),
            });
        }
        let (f, l) = crate::names::random_name(&mut rng, &self.content.pools[0]);
        life.relationships.push(Relationship {
            name: format!("{f} {l}"),
            kind: "friend".into(),
            closeness: 65.0,
            note: "best friend".into(),
        });
        let (f, l) = crate::names::random_name(&mut rng, &self.content.pools[0]);
        life.relationships.push(Relationship {
            name: format!("Coach {l}"),
            kind: "coach".into(),
            closeness: 50.0,
            note: format!("{f}, high-school coach"),
        });
        life.finance.cash = (crate::life::cpi(self.year) * 300.0) as i64;
        life.timeline.push(LifeEntry {
            season: self.year,
            age,
            text: format!(
                "The story begins: {} {} starts as {}.",
                spec.first,
                spec.last,
                match stage {
                    LifeStage::HighSchool => format!("a grade-{class} student"),
                    LifeStage::College => "a college freshman".into(),
                    LifeStage::Overseas => "a pro overseas".into(),
                    _ => "a pro rookie".into(),
                }
            ),
        });
        p.life = Some(Box::new(life));
        p.dev_focus = DevFocus::Balanced;
        // placement
        let id = self.add_player(p);
        match stage {
            LifeStage::HighSchool => {
                self.pm(id).affiliation = Affiliation::HighSchool;
                self.pm(id).custom.insert("class".into(), class as f64);
                self.pm(id).custom.insert("rank".into(), 9999.0);
            }
            LifeStage::College => {
                // pick the best-fitting program that has room
                let best = self
                    .colleges
                    .iter()
                    .filter(|c| c.roster.len() < 13)
                    .max_by(|a, b| {
                        self.recruit_interest(a.id, id, 0.0)
                            .partial_cmp(&self.recruit_interest(b.id, id, 0.0))
                            .unwrap()
                    })
                    .map(|c| c.id);
                if let Some(c) = best {
                    self.commit_recruit(id, c);
                    let cn = self.colleges[c as usize].name.clone();
                    if let Some(l) = &mut self.pm(id).life {
                        l.school = cn;
                    }
                }
            }
            LifeStage::Overseas => {
                let mut rng2 = rng.fork("overseas");
                self.send_overseas(id, &mut rng2);
            }
            LifeStage::Pro => {
                // signs with a team as a rookie free agent / second-rounder
                let t = self.user.team.unwrap_or_else(|| self.active_team_ids()[0]);
                let c = self.rookie_contract_for(40, 2);
                let p = self.pm(id);
                p.contract = Some(c);
                p.affiliation = Affiliation::Nba(t);
                self.team_mut(t).roster.push(id);
            }
            LifeStage::Retired => {}
        }
        self.user.player = Some(id);
        if !self.user.roles.contains(&Role::Player) {
            self.user.roles.push(Role::Player);
        }
        self.user.log.push(format!(
            "{}: created player {} {}.",
            self.year, spec.first, spec.last
        ));
        self.refill_scouting_points();
        Ok(id)
    }

    pub fn set_life_allocation(&mut self, wanted: &[(String, f64)]) -> Result<String, String> {
        let pid = self.user.player.ok_or("You don't have a player career.")?;
        let defs = self.content.life.clone();
        let p = self.pm(pid);
        let life = p
            .life
            .as_mut()
            .ok_or("Your player has no life sim (it may be turned off in settings).")?;
        life.set_allocation(&defs, wanted)?;
        let mut s = String::from("Your time is now split: ");
        let parts: Vec<String> = life
            .allocation
            .iter()
            .filter(|(_, v)| **v > 0.5)
            .map(|(k, v)| format!("{k} {v:.0}%"))
            .collect();
        s += &parts.join(", ");
        Ok(s)
    }

    // ------------------------------------------------------------------ monthly life tick

    pub fn monthly_life_tick(&mut self) {
        self.refill_scouting_points();
        let intensity = self.settings.num("life.intensity");
        let pid = match self.user.player {
            Some(p) => p,
            None => return,
        };
        if intensity <= 0.0 || self.p(pid).life.is_none() {
            return;
        }
        let year = self.year;
        let defs = self.content.life.clone();
        let income = match &self.p(pid).contract {
            Some(c)
                if matches!(
                    self.p(pid).affiliation,
                    Affiliation::Nba(_) | Affiliation::Overseas(_)
                ) =>
            {
                c.salary()
            }
            _ => {
                // college stipend / NIL
                if matches!(self.p(pid).affiliation, Affiliation::College(_))
                    && self.nil_available()
                {
                    (crate::life::cpi(year) * 20_000.0) as i64
                } else {
                    0
                }
            }
        };
        let win = self
            .p(pid)
            .team_id()
            .map(|t| self.team(t).record.pct())
            .unwrap_or(0.5);
        let inp = LifeInputs {
            defs: &defs,
            year,
            team_win_pct: win,
            avg_salary: self.money.avg_salary,
            annual_income: income,
            intensity,
            auto_decisions: self.user.auto_decisions || self.settings.bool("life.auto_decisions"),
        };
        let mut rng = self.rng.fork("life");
        let log = monthly_tick(&mut self.players[pid as usize], &inp, &mut rng);
        // accumulate development momentum for yearly progression
        {
            let tmp = &mut self.players[pid as usize];
            if let Some(l) = &tmp.life {
                let (s, p2) = (l.stat("dev_skill"), l.stat("dev_phys"));
                *tmp.custom.entry("dev_acc_skill".into()).or_insert(0.0) += s;
                *tmp.custom.entry("dev_acc_phys".into()).or_insert(0.0) += p2;
            }
        }
        for l in log {
            self.add_news("user", format!("LIFE: {l}"));
        }
    }

    pub fn after_postseason_life(&mut self) {
        self.monthly_life_tick();
        // yearly taxes reset
        if let Some(pid) = self.user.player {
            if let Some(l) = &mut self.pm(pid).life {
                l.finance.income_this_year = 0;
                l.finance.taxes_this_year = 0;
            }
        }
        self.career_review();
        self.plan_player_decisions();
    }

    /// HS games for the human's player (the amateur-level stats are generated).
    pub fn user_player_day_hook(&mut self, day: u32, notes: &mut Vec<String>) {
        let Some(pid) = self.user.player else { return };
        let (aff, injured) = (self.p(pid).affiliation.clone(), self.p(pid).is_injured());
        if aff != Affiliation::HighSchool || day % 3 != 0 || injured {
            return;
        }
        // eligibility
        if let Some(l) = &self.p(pid).life {
            if !l.eligible {
                return;
            }
        }
        let mut rng = Rng::from_label(self.seed, &format!("hs-{}-{}-{}", self.year, pid, day));
        let (line, desc) = self.hs_game(pid, &mut rng);
        let year = self.year;
        let school = self.p(pid).origin.school.clone();
        let p = self.pm(pid);
        let need = !matches!(p.seasons.last(), Some(r) if r.season == year && r.level == Level::HighSchool);
        if need {
            let age = year - p.birth_year;
            p.seasons.push(SeasonRecord {
                season: year,
                level: Level::HighSchool,
                team: school,
                team_id: None,
                age: age.max(0) as u8,
                ovr: p.ovr,
                stats: StatLine::default(),
                playoffs: StatLine::default(),
                salary: 0,
            });
        }
        if let Some(r) = p.seasons.last_mut() {
            r.stats.add(&line);
        }
        if line.pts >= 30 {
            notes.push(desc);
        }
    }

    fn hs_game(&self, pid: PlayerId, rng: &mut Rng) -> (StatLine, String) {
        let p = self.p(pid);
        let gp = crate::game::GamePlayer::from_player(p, &self.content.badges, 1.0, 0.0);
        let min = rng.gauss(26.0, 3.0).clamp(14.0, 32.0);
        // High-school scoring: a 40 plays like a decent starter (~12 ppg), a 60 like a star (~27).
        let base = 3.0 + (p.ovr as f64 - 30.0).max(0.0) * 0.65;
        let usage = (gp.scoring / 60.0).clamp(0.7, 1.3);
        let pts = (base * usage * (min / 26.0) * rng.gauss(1.0, 0.28))
            .clamp(0.0, 62.0)
            .round() as u32;
        let reb =
            ((gp.drb.max(gp.orb) - 30.0).max(0.0) * 0.18 * (min / 30.0) * rng.gauss(1.0, 0.35))
                .max(0.0)
                .round() as u32;
        let ast = ((gp.pass - 30.0).max(0.0) * 0.12 * (min / 30.0) * rng.gauss(1.0, 0.4))
            .max(0.0)
            .round() as u32;
        let stl = ((gp.steal - 30.0).max(0.0) * 0.04 * rng.gauss(1.0, 0.5))
            .max(0.0)
            .round() as u32;
        let blk = ((gp.block - 30.0).max(0.0) * 0.035 * rng.gauss(1.0, 0.5))
            .max(0.0)
            .round() as u32;
        let fga = (pts as f64 * 0.85 + rng.gauss(0.0, 1.5))
            .max(pts as f64 * 0.5)
            .round() as u32;
        let s = StatLine {
            g: 1,
            gs: 1,
            min,
            pts,
            fga,
            fgm: (pts as f64 * 0.42) as u32,
            tpa: fga / 4,
            tpm: pts / 9,
            fta: pts / 6,
            ftm: pts / 9,
            orb: reb / 3,
            drb: reb - reb / 3,
            ast,
            stl,
            blk,
            tov: 2,
            pf: 2,
            plus_minus: 0,
        };
        let d = format!(
            "{} drops {} points ({} reb, {} ast) for {}.",
            p.name(),
            pts,
            reb,
            ast,
            p.origin.school
        );
        (s, d)
    }

    // ------------------------------------------------------------------ the human's player decisions

    /// Queue decisions that apply to the human's player at the start of the offseason.
    pub fn plan_player_decisions(&mut self) {
        let Some(pid) = self.user.player else { return };
        if self.p(pid).is_retired() {
            return;
        }
        let year = self.year;
        let name = self.p(pid).name();
        let aff = self.p(pid).affiliation.clone();
        let age = year + 1 - self.p(pid).birth_year;
        match aff {
            Affiliation::HighSchool => {
                let class = self.p(pid).custom.get("class").copied().unwrap_or(9.0) as u8;
                if class >= 12 {
                    self.queue_hs_path(pid);
                } else {
                    let c = class + 1;
                    self.pm(pid).custom.insert("class".into(), c as f64);
                    if let Some(l) = &mut self.pm(pid).life {
                        l.class_year = c;
                    }
                    self.add_news("user", format!("{name} moves up to grade {c}."));
                }
            }
            Affiliation::College(c) => {
                let class = crate::college::class_of(self.p(pid));
                let rules = self.content.rules(self.rules_year_for(year + 1));
                let eligible_early = rules.early_entry
                    && (year + 1 - self.p(pid).birth_year) >= rules.min_draft_age as i32;
                let (o, pt, _) = self.scouted_view(None, pid);
                let mut opts = vec![];
                if class >= 4 {
                    opts.push(DecisionOption {
                        id: "declare".into(),
                        label: "Enter the draft".into(),
                        explain: format!(
                            "Your draft stock is roughly {} (overall ~{:.0}, potential ~{:.0}).",
                            self.draft_projection_text(pid),
                            o,
                            pt
                        ),
                    });
                    opts.push(DecisionOption {
                        id: "overseas".into(),
                        label: "Play overseas".into(),
                        explain: "Skip the draft and sign with a pro club in Europe or elsewhere."
                            .into(),
                    });
                } else {
                    if eligible_early {
                        opts.push(DecisionOption {
                            id: "declare".into(),
                            label: "Declare for the draft".into(),
                            explain: format!(
                                "Leave school early. Projected: {}.",
                                self.draft_projection_text(pid)
                            ),
                        });
                    }
                    opts.push(DecisionOption {
                        id: "stay".into(),
                        label: format!("Stay in school (class {})", class + 1),
                        explain: "Keep developing and improve your stock.".into(),
                    });
                    if self.settings.bool("college.transfer_portal") && year >= 2018 {
                        opts.push(DecisionOption {
                            id: "transfer".into(),
                            label: "Enter the transfer portal".into(),
                            explain:
                                "Move to a different program for a better role or more exposure."
                                    .into(),
                        });
                    }
                    if eligible_early {
                        opts.push(DecisionOption {
                            id: "overseas".into(),
                            label: "Go pro overseas".into(),
                            explain: "Sign with a club abroad instead of the draft.".into(),
                        });
                    }
                }
                let _ = c;
                self.push_decision("college_year", &format!("{name}: offseason decision"), &format!("The college season is over. You are in class {class} (age {age} next season). What's next?"), opts, Some(pid));
            }
            Affiliation::Nba(t) => {
                // retirement thinking for older players
                if age >= 33 {
                    self.push_decision(
                        "retire",
                        &format!("{name}: keep playing?"),
                        &format!("You are {age} years old with {} pro seasons behind you. Do you want to keep playing?", self.p(pid).pro_seasons()),
                        vec![
                            DecisionOption { id: "continue".into(), label: "Keep playing".into(), explain: "One more season.".into() },
                            DecisionOption { id: "retire".into(), label: "Retire".into(), explain: "Hang up the sneakers. You'll still manage your life, money and legacy.".into() },
                        ],
                        Some(pid),
                    );
                }
                let _ = t;
            }
            Affiliation::Overseas(_) => {
                let (o, _, _) = self.scouted_view(None, pid);
                if age >= 20 && self.p(pid).draft.is_none() {
                    self.pm(pid).flags.insert("declared".into());
                }
                let _ = o;
            }
            Affiliation::FreeAgent => {
                // handled by the free-agent offers decision
            }
            Affiliation::GLeague(_) | Affiliation::Retired => {}
        }
    }

    fn draft_projection_text(&self, pid: PlayerId) -> String {
        let rank = self.estimate_draft_rank(pid);
        let slots = self.rules.draft_rounds as usize * self.active_team_ids().len();
        if rank <= 5 {
            "a top-5 pick".to_string()
        } else if rank <= 14 {
            "a lottery pick".to_string()
        } else if rank <= 30 {
            "a first-round pick".to_string()
        } else if rank <= slots {
            "a second-round pick".to_string()
        } else {
            "likely undrafted".to_string()
        }
    }

    pub fn estimate_draft_rank(&self, pid: PlayerId) -> usize {
        let v = |id: PlayerId| {
            let (o, p, _) = self.scouted_view(None, id);
            o * 0.5 + p * 0.5
        };
        let mine = v(pid);
        let mut n = 1;
        for p in self.players.iter() {
            if p.id == pid || p.is_retired() || p.user_controlled {
                continue;
            }
            let eligible = match p.affiliation {
                Affiliation::College(_) => true,
                Affiliation::Overseas(_) => p.draft.is_none(),
                Affiliation::HighSchool => self.rules.hs_allowed,
                _ => false,
            };
            if eligible && v(p.id) > mine {
                n += 1;
            }
        }
        // only a fraction of those actually enter the draft
        (n as f64 * 0.42).ceil() as usize
    }

    fn queue_hs_path(&mut self, pid: PlayerId) {
        let name = self.p(pid).name();
        let year = self.year;
        let gpa = self
            .p(pid)
            .life
            .as_ref()
            .map(|l| l.stat("grades"))
            .unwrap_or(3.0);
        let mut opts = vec![];
        // college offers: top programs by interest
        if gpa >= 2.0 {
            let mut offers: Vec<(f64, CollegeId)> = self
                .colleges
                .iter()
                .filter(|c| c.roster.len() < 13 && c.active)
                .map(|c| (self.recruit_interest(c.id, pid, 0.0), c.id))
                .collect();
            offers.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            // quality of offers depends on his profile: rank by (ovr, potential) and fame
            let fame = self
                .p(pid)
                .life
                .as_ref()
                .map(|l| l.stat("fame"))
                .unwrap_or(10.0);
            let n_offers = (3.0 + (self.p(pid).ovr as f64 - 38.0).max(0.0) * 0.12 + fame * 0.06)
                .clamp(2.0, 8.0) as usize;
            // star offers come from the top; weaker players get mid-level offers
            let skip = if self.p(pid).potential < 60 {
                offers.len() / 3
            } else if self.p(pid).potential < 72 {
                offers.len() / 6
            } else {
                0
            };
            for (s, c) in offers.into_iter().skip(skip).take(n_offers) {
                let col = &self.colleges[c as usize];
                opts.push(DecisionOption {
                    id: format!("college:{c}"),
                    label: format!(
                        "{} {} (prestige {:.0})",
                        col.name, col.nickname, col.prestige
                    ),
                    explain: format!(
                        "Coach {} ({}). Interest score {:.0}. {}",
                        col.coach.name,
                        if col.coach.recruiting > 60.0 {
                            "great recruiter"
                        } else {
                            "solid coach"
                        },
                        s,
                        if self.nil_available() {
                            "NIL deals available."
                        } else {
                            ""
                        }
                    ),
                });
            }
        } else {
            opts.push(DecisionOption {
                id: "juco".into(),
                label: "Junior college (fix your grades)".into(),
                explain:
                    "Your GPA is below 2.0, so no top program can take you. Prove yourself first."
                        .into(),
            });
        }
        let rules = self.content.rules(self.rules_year_for(year + 1));
        if rules.hs_allowed {
            opts.push(DecisionOption {
                id: "declare".into(),
                label: "Go pro: enter the draft".into(),
                explain: format!(
                    "Skip college. Projected: {}.",
                    self.draft_projection_text(pid)
                ),
            });
        }
        if !self.clubs.is_empty() {
            opts.push(DecisionOption {
                id: "overseas".into(),
                label: "Go pro overseas".into(),
                explain: "Sign with a professional club abroad right away.".into(),
            });
        }
        if rules.g_league {
            opts.push(DecisionOption {
                id: "gleague".into(),
                label: "Pro pathway program".into(),
                explain:
                    "A development-league pathway: get paid and develop against pros for a year."
                        .into(),
            });
        }
        if opts.is_empty() {
            opts.push(DecisionOption {
                id: "overseas".into(),
                label: "Try your luck overseas".into(),
                explain: "No other options are open.".into(),
            });
        }
        self.push_decision(
            "hs_path",
            &format!("{name}: graduation day"),
            "You've finished high school. Where do you take your talents?",
            opts,
            Some(pid),
        );
    }

    /// Answer a pending decision.
    pub fn resolve_decision(
        &mut self,
        id: u32,
        option: &str,
        auto: bool,
    ) -> Result<String, String> {
        let pos = self
            .decisions
            .iter()
            .position(|d| d.id == id)
            .ok_or_else(|| format!("There is no pending decision #{id}."))?;
        let d = self.decisions[pos].clone();
        if !d.options.iter().any(|o| o.id == option) {
            return Err(format!(
                "Not a valid option. Choose one of: {}.",
                d.options
                    .iter()
                    .map(|o| o.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let msg = match d.kind.as_str() {
            "hs_path" => self.apply_hs_path(d.subject.unwrap(), option)?,
            "college_year" => self.apply_college_year(d.subject.unwrap(), option)?,
            "retire" => {
                let pid = d.subject.unwrap();
                if option == "retire" {
                    let n = self.p(pid).name();
                    self.retire_player(pid, "chose to retire");
                    self.user.log.push(format!("{}: {n} retires.", self.year));
                    format!("{n} retires from basketball. Your life goes on: manage your money, family and legacy, or start a coaching career (take-role).")
                } else {
                    "You decide to play one more year.".into()
                }
            }
            "team_option" => {
                let pid = d.subject.unwrap();
                if option == "decline" {
                    if let Some(c) = &mut self.pm(pid).contract {
                        c.salaries.pop();
                    }
                }
                format!("Option {}d.", option)
            }
            "fa_offers" => self.apply_fa_offer(d.subject.unwrap(), option)?,
            "post_draft" => self.apply_post_draft(d.subject.unwrap(), option)?,
            "job_offer" | "fired" => self.apply_job_choice(option)?,
            "career_change" => self.apply_career_change(option)?,
            _ => "Done.".into(),
        };
        self.decisions.retain(|x| x.id != id);
        let _ = auto;
        Ok(msg)
    }

    fn apply_hs_path(&mut self, pid: PlayerId, option: &str) -> Result<String, String> {
        let name = self.p(pid).name();
        let mut rng = self.rng.fork("hs-path");
        let year = self.year;
        if let Some(cid) = option.strip_prefix("college:") {
            let c: CollegeId = cid.parse().map_err(|_| "bad option")?;
            if self.colleges[c as usize].roster.len() >= 13 {
                return Err("That program just filled its last scholarship. Pick another.".into());
            }
            self.commit_recruit(pid, c);
            let cn = self.colleges[c as usize].name.clone();
            if let Some(l) = &mut self.pm(pid).life {
                l.stage = LifeStage::College;
                l.school = cn.clone();
                l.class_year = 1;
                l.log(year, 18, format!("Committed to {cn}."));
            }
            self.user
                .log
                .push(format!("{year}: {name} commits to {cn}."));
            return Ok(format!("{name} commits to {cn}. College life begins!"));
        }
        match option {
            "juco" => {
                // sit out a year in junior college, improving grades; keep in HS pool next year
                if let Some(l) = &mut self.pm(pid).life {
                    l.stats.insert("grades".into(), 2.6);
                }
                self.pm(pid).custom.insert("class".into(), 12.0);
                Ok("You spend a year in junior college rebuilding your grades.".into())
            }
            "declare" => {
                self.pm(pid).flags.insert("declared".into());
                self.pm(pid).origin.kind = OriginKind::HighSchool;
                if let Some(l) = &mut self.pm(pid).life {
                    l.stage = LifeStage::Pro;
                }
                self.user.log.push(format!(
                    "{year}: {name} declares for the draft out of high school."
                ));
                Ok(format!("{name} declares for the {} draft.", year + 1))
            }
            "overseas" => {
                self.pm(pid).origin.kind = OriginKind::HighSchool;
                self.send_overseas(pid, &mut rng);
                if let Some(l) = &mut self.pm(pid).life {
                    l.stage = LifeStage::Overseas;
                    l.log(year, 18, "Signed a pro contract overseas.");
                }
                let club = match self.p(pid).affiliation {
                    Affiliation::Overseas(c) => self.clubs[c as usize].name.clone(),
                    _ => "a club".into(),
                };
                self.user
                    .log
                    .push(format!("{year}: {name} signs with {club}."));
                Ok(format!("{name} signs with {club} and moves abroad."))
            }
            "gleague" => {
                // development pathway: modelled as an overseas-like year with a stipend
                self.pm(pid).origin.kind = OriginKind::GLeague;
                self.send_overseas(pid, &mut rng);
                Ok("You join a development pathway program for a year before the draft.".into())
            }
            _ => Err("Unknown option.".into()),
        }
    }

    fn apply_college_year(&mut self, pid: PlayerId, option: &str) -> Result<String, String> {
        let name = self.p(pid).name();
        let year = self.year;
        let mut rng = self.rng.fork("college-year");
        match option {
            "declare" => {
                self.pm(pid).flags.insert("declared".into());
                if let Some(l) = &mut self.pm(pid).life {
                    l.stage = LifeStage::Pro;
                }
                self.user
                    .log
                    .push(format!("{year}: {name} declares for the draft."));
                Ok(format!("{name} declares for the {} draft.", year + 1))
            }
            "stay" => Ok("You stay in school for another year.".into()),
            "transfer" => {
                let cid = match self.p(pid).affiliation {
                    Affiliation::College(c) => c,
                    _ => return Err("You're not in college.".into()),
                };
                let dest = self
                    .colleges
                    .iter()
                    .filter(|c| c.id != cid && c.roster.len() < 13)
                    .max_by(|a, b| {
                        self.recruit_interest(a.id, pid, 0.0)
                            .partial_cmp(&self.recruit_interest(b.id, pid, 0.0))
                            .unwrap()
                    })
                    .map(|c| c.id);
                match dest {
                    Some(to) => {
                        self.colleges[cid as usize].roster.retain(|&x| x != pid);
                        self.colleges[to as usize].roster.push(pid);
                        let nm = self.colleges[to as usize].name.clone();
                        self.pm(pid).affiliation = Affiliation::College(to);
                        self.pm(pid).origin.school = nm.clone();
                        if let Some(l) = &mut self.pm(pid).life {
                            l.school = nm.clone();
                        }
                        Ok(format!("You transfer to {nm}."))
                    }
                    None => Err("No program has room right now.".into()),
                }
            }
            "overseas" => {
                self.colleges
                    .iter_mut()
                    .for_each(|c| c.roster.retain(|&x| x != pid));
                self.send_overseas(pid, &mut rng);
                if let Some(l) = &mut self.pm(pid).life {
                    l.stage = LifeStage::Overseas;
                }
                Ok(format!("{name} signs a pro contract overseas."))
            }
            _ => Err("Unknown option.".into()),
        }
    }

    /// After the draft: the human's player learns his fate and picks a path.
    pub fn handle_user_player_after_draft(&mut self) {
        let Some(pid) = self.user.player else { return };
        let name = self.p(pid).name();
        if self.p(pid).flags.contains("drafted") {
            self.pm(pid).flags.remove("drafted");
            let d = self.p(pid).draft.clone();
            let t = self.p(pid).team_id();
            if let (Some(d), Some(t)) = (d, t) {
                let sal = self.p(pid).current_salary();
                self.user.log.push(format!(
                    "{}: drafted #{} by the {} ({}).",
                    self.year + 1,
                    d.pick,
                    self.team(t).name(),
                    fmt_money(sal)
                ));
                let yr = self.year + 1;
                if let Some(l) = &mut self.pm(pid).life {
                    l.stage = LifeStage::Pro;
                    l.log(
                        yr,
                        19,
                        format!("Drafted #{} overall by {}.", d.pick, d.team),
                    );
                }
                self.add_news(
                    "user",
                    format!(
                        "{name} is drafted #{} overall by the {}!",
                        d.pick,
                        self.team(t).name()
                    ),
                );
                // the human's team (if GM too) etc: nothing more
            }
        } else if self.p(pid).flags.contains("undrafted") {
            self.pm(pid).flags.remove("undrafted");
            self.pm(pid).flags.remove("declared");
            let opts = vec![
                DecisionOption {
                    id: "camp".into(),
                    label: "Sign a camp contract with an NBA team".into(),
                    explain: "A minimum deal and a chance to make a roster in the preseason."
                        .into(),
                },
                DecisionOption {
                    id: "overseas".into(),
                    label: "Play overseas".into(),
                    explain: "Start your pro career with a club abroad.".into(),
                },
                DecisionOption {
                    id: "stay_amateur".into(),
                    label: "Return to your amateur team (if eligible)".into(),
                    explain: "Not every era allows this. If not, you'll go overseas.".into(),
                },
            ];
            self.push_decision(
                "post_draft",
                &format!("{name} goes undrafted"),
                "Draft night passes without hearing your name. It stings, but the road isn't over.",
                opts,
                Some(pid),
            );
        }
    }

    fn apply_post_draft(&mut self, pid: PlayerId, option: &str) -> Result<String, String> {
        let mut rng = self.rng.fork("post-draft");
        match option {
            "camp" => {
                let team = self.active_team_ids();
                let t = team[rng.range_usize(team.len())];
                let ms = self.money.min_salary;
                // become a free agent signing
                self.pm(pid).affiliation = Affiliation::FreeAgent;
                if let Affiliation::College(c) = self.p(pid).affiliation.clone() {
                    self.colleges[c as usize].roster.retain(|&x| x != pid);
                }
                self.sign_player(pid, t, ms, 1, ContractKind::Minimum, "signing");
                if let Some(l) = &mut self.pm(pid).life {
                    l.stage = LifeStage::Pro;
                }
                Ok(format!(
                    "You sign a camp deal with the {}.",
                    self.team(t).name()
                ))
            }
            "overseas" | "stay_amateur" => {
                self.free_agents.retain(|&x| x != pid);
                self.colleges
                    .iter_mut()
                    .for_each(|c| c.roster.retain(|&x| x != pid));
                self.send_overseas(pid, &mut rng);
                if let Some(l) = &mut self.pm(pid).life {
                    l.stage = LifeStage::Overseas;
                }
                Ok("You sign with an overseas club.".into())
            }
            _ => Err("Unknown option.".into()),
        }
    }

    /// Offers for the human's player when his contract runs out.
    pub fn make_fa_offers(&mut self, pid: PlayerId) {
        let name = self.p(pid).name();
        let (ask, years) = self.player_ask(pid);
        let agent_boost = self
            .p(pid)
            .life
            .as_ref()
            .and_then(|l| l.agent.as_ref())
            .map(|a| 1.0 + a.skill / 100.0 * 0.08)
            .unwrap_or(1.0);
        let mut offers: Vec<(f64, TeamId, Money)> = vec![];
        for t in self.active_team_ids() {
            let payroll = self.payroll(t);
            let room = if self.money.cap_enforced {
                self.money.cap - payroll
            } else {
                self.team(t).budget.payroll_target - payroll
            };
            let mle = if self.rules.mid_level_exception {
                self.mle_amount()
            } else {
                0
            };
            let max_pay = room.max(mle).max(self.money.min_salary);
            let appeal = self.team_appeal(t, pid);
            let offer = ((ask as f64 * agent_boost * (0.8 + appeal / 250.0)) as i64)
                .min(max_pay)
                .max(self.money.min_salary);
            if offer as f64 >= ask as f64 * 0.55 || offer == self.money.min_salary {
                offers.push((appeal, t, offer));
            }
        }
        offers.sort_by(|a, b| b.2.cmp(&a.2));
        let mut opts = vec![];
        for (appeal, t, offer) in offers.into_iter().take(5) {
            opts.push(DecisionOption {
                id: format!("sign:{t}:{offer}:{years}"),
                label: format!(
                    "{}: {} per year, {} years",
                    self.team(t).name(),
                    fmt_money(offer),
                    years
                ),
                explain: format!(
                    "Appeal {:.0}/100. Team rating {:.1}, mood of the room unknown. Payroll {}.",
                    appeal,
                    self.team_rating_full_health(t),
                    fmt_money(self.payroll(t))
                ),
            });
        }
        if !self.clubs.is_empty() {
            opts.push(DecisionOption {
                id: "overseas".into(),
                label: "Sign overseas".into(),
                explain: "A pro club abroad (tax rules and lifestyle differ).".into(),
            });
        }
        opts.push(DecisionOption { id: "wait".into(), label: "Wait for a better offer".into(), explain: "Free agency has 30 days. Waiting can drive the price up... or leave you with nothing.".into() });
        self.push_decision("fa_offers", &format!("{name}: free agency"), &format!("Your contract has expired. Your market value is about {} for {} years. Offers are in.", fmt_money(ask), years), opts, Some(pid));
    }

    fn apply_fa_offer(&mut self, pid: PlayerId, option: &str) -> Result<String, String> {
        if let Some(rest) = option.strip_prefix("sign:") {
            let parts: Vec<&str> = rest.split(':').collect();
            let t: TeamId = parts[0].parse().map_err(|_| "bad option")?;
            let sal: Money = parts[1].parse().map_err(|_| "bad option")?;
            let yrs: u8 = parts[2].parse().map_err(|_| "bad option")?;
            if let Affiliation::Overseas(c) = self.p(pid).affiliation.clone() {
                self.clubs[c as usize].roster.retain(|&x| x != pid);
            }
            self.free_agents.retain(|&x| x != pid);
            let was_pro = matches!(self.p(pid).affiliation, Affiliation::Nba(_));
            let _ = was_pro;
            // release from old team first
            if let Some(old) = self.p(pid).team_id() {
                self.team_mut(old).roster.retain(|&x| x != pid);
            }
            self.sign_player(pid, t, sal, yrs, ContractKind::Standard, "free agency");
            if let Some(l) = &mut self.pm(pid).life {
                l.stage = LifeStage::Pro;
            }
            let tn = self.team(t).name();
            self.user.log.push(format!(
                "{}: signs with the {tn} ({} years, {}).",
                self.year + 1,
                yrs,
                fmt_money(sal)
            ));
            return Ok(format!("You sign with the {tn}!"));
        }
        match option {
            "overseas" => {
                let mut rng = self.rng.fork("fa-overseas");
                if let Some(old) = self.p(pid).team_id() {
                    self.team_mut(old).roster.retain(|&x| x != pid);
                }
                self.pm(pid).affiliation = Affiliation::FreeAgent;
                self.send_overseas(pid, &mut rng);
                if let Some(l) = &mut self.pm(pid).life {
                    l.stage = LifeStage::Overseas;
                }
                Ok("You sign with an overseas club.".into())
            }
            "wait" => {
                self.pm(pid).flags.insert("waiting".into());
                Ok(
                    "You wait to see who calls. (Offers will come again as free agency goes on.)"
                        .into(),
                )
            }
            _ => Err("Unknown option.".into()),
        }
    }

    // ------------------------------------------------------------------ career review, jobs

    /// After every season: update reputation, check job security, possibly offer new jobs.
    pub fn career_review(&mut self) {
        let year = self.year;
        if let Some(t) = self.user.team {
            let rank_pct = {
                let order = self.standings(None);
                let r = order.iter().position(|&x| x == t).unwrap_or(0) as f64;
                1.0 - r / order.len().max(1) as f64
            };
            let champion = self
                .history
                .last()
                .map(|h| h.champion_id == Some(t))
                .unwrap_or(false);
            let mut delta = (rank_pct - 0.5) * 8.0;
            if champion {
                delta += 12.0;
            }
            self.user.reputation = (self.user.reputation + delta).clamp(0.0, 100.0);
            let _ = year;
        }
        if let Some(c) = self.user.college {
            let pct = self.colleges[c as usize].record.pct();
            self.user.reputation = (self.user.reputation + (pct - 0.5) * 12.0).clamp(0.0, 100.0);
        }
        // Job security for GM/coach (not owners)
        if self.settings.bool("difficulty.can_be_fired") {
            if let Some(t) = self.user.team {
                let is_employee = (self.user.has(Role::Gm) || self.user.has(Role::HeadCoach))
                    && !self.user.has(Role::Owner);
                if is_employee {
                    let approval = self.team(t).owner.approval;
                    let patience = self.team(t).owner.patience
                        * self.settings.num("difficulty.owner_patience");
                    if approval < 22.0 && patience < 75.0 {
                        self.fire_user(t);
                    }
                }
            }
        }
    }

    pub fn check_user_job_security(&mut self) {}

    fn fire_user(&mut self, t: TeamId) {
        let tn = self.team(t).name();
        self.add_news(
            "major",
            format!(
                "The {tn} fire {} after a disappointing stretch.",
                self.user.name
            ),
        );
        self.user
            .log
            .push(format!("{}: fired by the {tn}.", self.year));
        self.user.reputation = (self.user.reputation - 12.0).max(0.0);
        let roles: Vec<Role> = self
            .user
            .roles
            .iter()
            .copied()
            .filter(|r| matches!(r, Role::Gm | Role::HeadCoach))
            .collect();
        self.user.team = None;
        for r in &roles {
            self.leave_role(*r);
        }
        // offers from weaker/other teams
        let mut opts = vec![];
        let mut rng = self.rng.fork("job-offers");
        let mut ids = self.active_team_ids();
        rng.shuffle(&mut ids);
        for &o in ids.iter().filter(|&&o| o != t).take(4) {
            opts.push(DecisionOption {
                id: format!(
                    "{}:{}",
                    roles.first().map(|r| role_key(*r)).unwrap_or("gm"),
                    o
                ),
                label: format!(
                    "{} {}",
                    self.team(o).name(),
                    roles.first().map(|r| r.name()).unwrap_or("GM")
                ),
                explain: format!(
                    "Team rating {:.1}. Owner patience {:.0}. {}",
                    self.team_rating_full_health(o),
                    self.team(o).owner.patience,
                    self.team(o).owner.mandate
                ),
            });
        }
        opts.push(DecisionOption {
            id: "retire".into(),
            label: "Step away from basketball".into(),
            explain: "End your front-office career.".into(),
        });
        self.push_decision("fired", "You've been fired", &format!("The {tn} owner has lost patience. Your reputation is {:.0}/100. Teams are still interested.", self.user.reputation), opts, None);
    }

    fn apply_job_choice(&mut self, option: &str) -> Result<String, String> {
        if option == "retire" {
            return Ok("You step away from the game.".into());
        }
        let parts: Vec<&str> = option.split(':').collect();
        if parts.len() != 2 {
            return Err("Unknown option.".into());
        }
        let role = Role::parse(parts[0]).ok_or("bad role")?;
        let t: TeamId = parts[1].parse().map_err(|_| "bad team")?;
        self.take_role(role, Some(t), None)
    }

    fn apply_career_change(&mut self, option: &str) -> Result<String, String> {
        let _ = option;
        Ok("Done.".into())
    }

    /// Offer the human new jobs when their reputation is high (promotions, bigger jobs).
    pub fn offer_promotions(&mut self) {
        // simple: assistant coach → head coach offer; scout → GM offer
        if self.user.reputation < 55.0 || !self.decisions.is_empty() {
            return;
        }
        let mut opts = vec![];
        if self.user.has(Role::AssistantCoach) || self.user.has(Role::Scout) {
            let role = if self.user.has(Role::AssistantCoach) {
                Role::HeadCoach
            } else {
                Role::Gm
            };
            let mut ids = self.active_team_ids();
            ids.sort_by(|&a, &b| {
                self.team(a)
                    .owner
                    .approval
                    .partial_cmp(&self.team(b).owner.approval)
                    .unwrap()
            });
            for &o in ids.iter().take(3) {
                opts.push(DecisionOption {
                    id: format!("{}:{}", role_key(role), o),
                    label: format!("{} of the {}", role.name(), self.team(o).name()),
                    explain: format!(
                        "Promotion! Their owner wants: {}.",
                        self.team(o).owner.mandate
                    ),
                });
            }
            opts.push(DecisionOption {
                id: "retire".into(),
                label: "Stay where you are".into(),
                explain: "Keep your current job.".into(),
            });
            self.push_decision(
                "job_offer",
                "A promotion is on the table",
                "Your work has been noticed around the league.",
                opts,
                None,
            );
        }
    }

    // ------------------------------------------------------------------ scenarios

    pub fn apply_scenario(&mut self, id: &str, team: TeamId) -> Result<String, String> {
        let roster = self.team(team).roster.clone();
        let mut rng = self.rng.fork("scenario");
        let shift = |l: &mut League, pid: PlayerId, delta: f64| {
            let p = l.pm(pid);
            for a in Attr::ALL {
                p.attrs.add(*a, delta);
            }
            p.recompute_ovr();
        };
        match id {
            "rebuild" => {
                for &pid in &roster {
                    shift(self, pid, -5.0);
                }
                let ids = self.active_team_ids();
                let me = team;
                for y in (self.year + 1)..=(self.year + 3) {
                    if let Some(&o) = ids
                        .iter()
                        .filter(|&&o| o != team)
                        .nth(rng.range_usize(ids.len() - 1))
                    {
                        self.move_pick(y, 1, o, o, me);
                    }
                }
                self.team_mut(team).owner.patience = 90.0;
                self.team_mut(team).direction = Direction::Rebuild;
                Ok(format!("{} are the league's weakest team with extra first-round picks and a patient owner.", self.team(team).name()))
            }
            "dynasty_end" => {
                for &pid in &roster {
                    let boost = if self.p(pid).ovr >= 60 { 4.0 } else { 0.0 };
                    shift(self, pid, boost);
                    let p = self.pm(pid);
                    p.birth_year -= 3;
                }
                self.team_mut(team).direction = Direction::Contend;
                Ok("Your champions are loaded but aging. Reload carefully.".into())
            }
            "small_market" => {
                let tm = self.team_mut(team);
                tm.market = 0.1;
                tm.budget.payroll_target = (tm.budget.payroll_target as f64 * 0.8) as i64;
                tm.owner.wealth = 20.0;
                Ok("Small market, small budget, loyal fans.".into())
            }
            "win_now" => {
                self.team_mut(team).owner.patience = 25.0;
                self.team_mut(team).owner.win_now = 0.95;
                self.team_mut(team).owner.mandate =
                    "Win the championship within two seasons".into();
                for &pid in roster.iter().take(5) {
                    shift(self, pid, 3.0);
                }
                Ok("The owner wants a title, now.".into())
            }
            "superstar" => {
                if let Some(&best) = roster.iter().max_by_key(|&&id| self.p(id).ovr) {
                    shift(self, best, 8.0);
                    let yr = self.year;
                    let b = self.pm(best);
                    b.birth_year = yr - 23;
                    b.potential = b.potential.max(95);
                    if let Some(c) = &mut b.contract {
                        c.salaries.truncate(2);
                    }
                    Ok(format!(
                        "{} is a young superstar on an expiring deal. Keep him.",
                        self.p(best).name()
                    ))
                } else {
                    Err("No players found.".into())
                }
            }
            _ => Err("Unknown scenario.".into()),
        }
    }

    pub fn day_report_hook(&mut self, _r: &DayReport) {}
}

fn role_key(r: Role) -> &'static str {
    match r {
        Role::Gm => "gm",
        Role::HeadCoach => "coach",
        Role::AssistantCoach => "assistant",
        Role::Scout => "scout",
        Role::Owner => "owner",
        Role::Player => "player",
        Role::CollegeCoach => "college_coach",
        Role::CollegeAd => "ad",
        Role::CollegeScout => "college_scout",
    }
}
