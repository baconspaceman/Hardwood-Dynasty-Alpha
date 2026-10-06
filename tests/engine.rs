use hardwood_dynasty::calendar::Goal;
use hardwood_dynasty::import::*;
use hardwood_dynasty::league::*;
use hardwood_dynasty::player::*;
use hardwood_dynasty::setup::NewLeagueOptions;
use hardwood_dynasty::types::Phase;

fn make(year: i32, seed: &str) -> League {
    League::new(NewLeagueOptions { year, seed: seed.into(), ..Default::default() }).unwrap()
}

#[test]
fn same_seed_same_world() {
    let mut a = make(1990, "det");
    let mut b = make(1990, "det");
    a.advance(Goal::UntilPhase(Phase::PostSeason));
    b.advance(Goal::UntilPhase(Phase::PostSeason));
    assert_eq!(a.save_json().unwrap(), b.save_json().unwrap(), "identical seeds must give identical leagues");
    let c = make(1990, "other");
    assert_ne!(make(1990, "det").save_json().unwrap(), c.save_json().unwrap());
}

#[test]
fn save_and_load_continue_identically() {
    let mut a = make(2001, "saveload");
    a.advance(Goal::Days(40));
    let json = a.save_json().unwrap();
    let mut b = League::load_json(&json).unwrap();
    a.advance(Goal::Days(25));
    b.advance(Goal::Days(25));
    assert_eq!(a.save_json().unwrap(), b.save_json().unwrap());
}

#[test]
fn rules_apply_by_year() {
    // No three-pointers before 1979.
    let mut l = make(1960, "rules");
    l.advance(Goal::Days(30));
    let tpa: u32 = l.players.iter().flat_map(|p| p.seasons.iter()).map(|s| s.stats.tpa).sum();
    assert!(l.rules.three_point == false);
    assert_eq!(tpa, 0, "no threes attempted in 1960");
    assert_eq!(l.rules.shot_clock, 24);
    let m = make(1950, "rules2");
    assert_eq!(m.rules.shot_clock, 0, "no shot clock in 1950");
    // Play-in only from 2020.
    assert!(make(2021, "r3").rules.play_in);
    assert!(!make(2010, "r3").rules.play_in);
    // Cap exists from 1984.
    assert!(!make(1975, "r4").money.cap_enforced);
    assert!(make(1990, "r4").money.cap_enforced);
}

#[test]
fn injuries_are_plausible_and_career_ending_happens() {
    let mut l = make(2005, "inj");
    l.advance(Goal::Years(8));
    let (mut seasons, mut inj, mut ce) = (0usize, 0usize, 0usize);
    for p in l.players.iter().filter(|p| p.seasons.iter().any(|s| s.level == Level::Pro)) {
        seasons += p.seasons.iter().filter(|s| s.level == Level::Pro).count();
        inj += p.injury_history.len();
        if p.flags.iter().any(|f| f.contains("career-ending")) {
            ce += 1;
        }
    }
    let rate = inj as f64 / seasons.max(1) as f64;
    println!("injuries per player-season {rate:.2}, career-ending {ce}");
    assert!(rate > 0.15 && rate < 2.5, "rate {rate}");
    assert!(ce < 40);
}

#[test]
fn injury_slider_to_zero_means_no_injuries() {
    let mut l = League::new(NewLeagueOptions { year: 2010, seed: "noinj".into(), preset: Some("sandbox".into()), ..Default::default() }).unwrap();
    l.advance(Goal::Years(1));
    assert!(l.players.iter().all(|p| p.injury_history.is_empty()), "sandbox preset has no injuries");
}

#[test]
fn csv_import_replaces_a_roster() {
    let mut l = make(1996, "import");
    let csv = "team,name,age,position,height_in,overall,potential,salary,years,country,ppg,rpg,apg\n\
        BOS,Johnny Example,27,SF,79,88,88,15000000,3,USA,27.5,6.1,4.0\n\
        BOS,Sam Center,30,C,84,74,74,6000000,2,SRB,14.0,10.2,1.5\n\
        BOS,Pat Passer,24,PG,74,70,76,2000000,2,USA,9.0,3.0,9.5\n\
        BOS,Dee Wing,26,SG,77,66,66,1500000,1,FRA,12.0,3.0,2.0\n\
        BOS,Ed Forward,28,PF,81,64,64,900000,1,USA,8.0,6.0,1.0\n\
        BOS,Gus Guard,25,PG,73,60,62,700000,1,USA,6.0,2.0,3.0\n\
        BOS,Hal Wing,29,SF,78,58,58,600000,1,USA,5.0,3.0,1.0\n\
        BOS,Ian Big,31,C,83,55,55,500000,1,USA,4.0,5.0,0.5\n";
    let rep = l.import_roster_csv(csv, true).unwrap();
    println!("{}", rep.summary());
    assert_eq!(rep.players_created, 8);
    assert!(rep.errors.is_empty());
    let t = l.find_team("BOS").unwrap();
    assert_eq!(l.team(t).roster.len(), 8);
    let star = l.find_player("Johnny Example").unwrap();
    assert!((l.p(star).ovr as i32 - 88).abs() <= 2, "imported overall {}", l.p(star).ovr);
    assert_eq!(l.p(star).position, Position::SF);
    assert_eq!(l.p(star).height_in, 79);
    // the stat-driven passer should be a good passer
    let pp = l.find_player("Pat Passer").unwrap();
    assert!(l.p(pp).attrs.get(Attr::PassVision) > l.p(star).attrs.get(Attr::PassVision) - 5.0);
    // bad team gives a readable error
    let bad = l.import_roster_csv("team,name\nNOPE,Someone\n", false).unwrap();
    assert!(bad.errors.iter().any(|e| e.contains("doesn't exist")));
    assert!(l.import_roster_csv("hello,world\n1,2\n", false).is_err());
}

#[test]
fn year_pack_round_trip() {
    let mut l = make(1984, "pack");
    l.advance(Goal::Days(3));
    let pack = l.export_year_pack("My Pack");
    let json = serde_json::to_string(&pack).unwrap();
    let back: YearPack = serde_json::from_str(&json).unwrap();
    let (m, rep) = League::new_from_pack(&back, &NewLeagueOptions { seed: "pack2".into(), ..Default::default() }).unwrap();
    println!("{}", rep.summary());
    assert_eq!(m.year, 1984);
    assert_eq!(m.active_team_ids().len(), l.active_team_ids().len());
    let t = m.active_team_ids()[0];
    assert!(m.team(t).roster.len() >= 8);
}

#[test]
fn mods_change_the_game() {
    let m = r#"{ "mod": {"name":"test"}, "settings_defaults": {"injuries.frequency": 0.0}, "badges": {"remove": ["iron_man"]} }"#;
    let l = League::new(NewLeagueOptions { year: 2000, seed: "mod".into(), mods: vec![m.into()], ..Default::default() }).unwrap();
    assert_eq!(l.settings.num("injuries.frequency"), 0.0);
    assert!(!l.content.badges.iter().any(|b| b.id == "iron_man"));
    assert!(League::new(NewLeagueOptions { mods: vec!["{ nope".into()], ..Default::default() }).is_err());
}

#[test]
fn chase_projection_is_consistent() {
    let mut l = make(2024, "chase");
    let t = l.active_team_ids()[3];
    l.take_role(Role::Owner, Some(t), None).unwrap();
    let extra = l.money.cap / 5;
    let p = l.chase_projection(t, extra);
    assert_eq!(p.total_extra_cost, extra + (p.luxury_tax_after - p.luxury_tax_before));
    assert!(p.title_odds_after >= p.title_odds_before);
    assert!(p.new_payroll == l.payroll(t) + extra);
}

#[test]
fn cap_moves_with_history_and_future() {
    let mut l = make(2022, "cap");
    assert_eq!(l.money.cap, 123_655_000);
    l.advance(Goal::Years(5));
    assert!(l.year >= 2027);
    assert!(l.money.cap > 150_000_000, "cap grows after 2025: {}", l.money.cap);
}

#[test]
fn settings_are_all_reachable_through_a_league() {
    let l = make(1996, "settings");
    for d in l.settings.defs() {
        assert!(l.settings.explain(&d.key).is_some());
    }
}
