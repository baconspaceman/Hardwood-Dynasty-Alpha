use hardwood_dynasty::calendar::Goal;
use hardwood_dynasty::league::*;
use hardwood_dynasty::setup::NewLeagueOptions;
use hardwood_dynasty::types::Phase;

fn make(year: i32) -> League {
    League::new(NewLeagueOptions {
        year,
        seed: "smoke".into(),
        ..Default::default()
    })
    .expect("league")
}

#[test]
fn a_full_season_runs() {
    let mut l = make(1996);
    assert_eq!(l.active_team_ids().len(), 29);
    let rep = l.advance(Goal::UntilPhase(Phase::PostSeason));
    println!("stopped: {:?}, steps {}", rep.stopped, rep.steps);
    assert!(rep.stopped.is_none(), "{:?}", rep.stopped);
    assert_eq!(l.phase, Phase::PostSeason);
    let h = l.history.last().expect("history");
    println!(
        "{} champions {} | MVP {} | avg ppg {:.1}",
        h.season, h.champion, h.mvp, h.avg_ppg
    );
    assert!(!h.champion.is_empty());
    assert!(h.avg_ppg > 85.0 && h.avg_ppg < 115.0, "ppg {}", h.avg_ppg);
}

#[test]
fn offseason_cycle_runs() {
    let mut l = make(2005);
    let rep = l.advance(Goal::EndOfSeason);
    println!("stopped: {:?}", rep.stopped);
    assert!(rep.stopped.is_none(), "{:?}", rep.stopped);
    assert_eq!(l.year, 2006);
    assert_eq!(l.phase, Phase::Preseason);
    for t in l.active_team_ids() {
        let n = l.team(t).roster.len();
        assert!(n >= 8, "{} has {} players", l.team(t).name(), n);
    }
}
