use hardwood_dynasty::calendar::Goal;
use hardwood_dynasty::career::CreatePlayerSpec;
use hardwood_dynasty::league::*;
use hardwood_dynasty::player::Affiliation;
use hardwood_dynasty::setup::NewLeagueOptions;
use hardwood_dynasty::types::Phase;

#[test]
fn a_created_player_lives_a_whole_career() {
    let mut l = League::new(NewLeagueOptions { year: 2008, seed: "career".into(), ..Default::default() }).unwrap();
    let spec = CreatePlayerSpec { first: "Marcus".into(), last: "Hale".into(), talent: 5, start: "hs12".into(), points: vec![("playmaking".into(), 30), ("three".into(), 25), ("athletic".into(), 25)], ..Default::default() };
    let pid = l.create_player(&spec).unwrap();
    l.user.auto_decisions = true;
    let mut seen_pro = false;
    for _ in 0..30 {
        let rep = l.advance(Goal::EndOfSeason);
        assert!(rep.stopped.is_none(), "{:?}", rep.stopped);
        if matches!(l.p(pid).affiliation, Affiliation::Nba(_)) {
            seen_pro = true;
        }
        if l.p(pid).is_retired() {
            break;
        }
    }
    println!("final: {:?} retired {:?} seasons {}", l.p(pid).affiliation, l.p(pid).retired, l.p(pid).seasons.len());
    assert!(seen_pro, "a generational prospect should reach the NBA");
    assert!(l.p(pid).seasons.len() >= 5);
    assert!(l.phase == Phase::Preseason || l.phase == Phase::PostSeason);
}
