use hardwood_dynasty::calendar::Goal;
use hardwood_dynasty::league::*;
use hardwood_dynasty::player::*;
use hardwood_dynasty::setup::NewLeagueOptions;
use hardwood_dynasty::types::Phase;

#[test]
fn played_seasons_match_style() {
    let mut l = League::new(NewLeagueOptions { year: 2012, seed: "drift".into(), ..Default::default() }).unwrap();
    for _ in 0..8 {
        l.advance(Goal::UntilPhase(Phase::Playoffs));
        let y = l.year;
        let mut s = StatLine::default();
        let mut games = 0.0;
        for p in l.players.iter() {
            for r in p.seasons.iter().filter(|r| r.season == y && r.level == Level::Pro) {
                s.add(&r.stats);
            }
        }
        for t in l.active_team_ids() {
            games += l.team(t).record.games() as f64;
        }
        let tg = games; // team-games
        let st = l.style;
        let pace = (s.fga as f64 - s.orb as f64 + s.tov as f64 + 0.44 * s.fta as f64) / tg;
        println!("{y}: ppg {:.1}/{:.1} pace {:.1}/{:.1} 3PAr {:.3}/{:.3} fg3 {:.3}/{:.3} ftr {:.3}/{:.3} tov {:.3}/{:.3} cal {:?}", s.pts as f64 / tg, st.ppg, pace, st.pace, s.tpa as f64 / s.fga as f64, st.three_rate, s.tpm as f64 / s.tpa as f64, st.fg3, s.fta as f64 / s.fga as f64, st.ft_rate, s.tov as f64 / pace, st.tov, l.cal);
        l.advance(Goal::EndOfSeason);
    }
}
