use hardwood_dynasty::game::*;
use hardwood_dynasty::league::*;
use hardwood_dynasty::rng::Rng;
use hardwood_dynasty::setup::NewLeagueOptions;

#[test]
fn league_matches_era_style() {
    for year in [1962, 1985, 2004, 2024] {
        let l = League::new(NewLeagueOptions {
            year,
            seed: "cal".into(),
            ..Default::default()
        })
        .unwrap();
        let ids = l.active_team_ids();
        let teams: Vec<GameTeam> = ids
            .iter()
            .map(|&t| l.build_game_team(t, false, None, false))
            .collect();
        let mut ctx = l.game_context(false);
        ctx.in_game_injuries = false;
        let mut rng = Rng::new(5);
        let mut m = LeagueMeasure::default();
        for g in 0..600usize {
            let a = (g * 7) % ids.len();
            let b = (a + 1 + (g * 3) % (ids.len() - 1)) % ids.len();
            m.add(&simulate_game(&ctx, &teams[a], &teams[b], &mut rng));
        }
        let st = l.style;
        println!("{year}: ppg {:.1}/{:.1} pace {:.1}/{:.1} 3PAr {:.3}/{:.3} fg2 {:.3}/{:.3} fg3 {:.3}/{:.3} ftr {:.3}/{:.3} tov {:.3}/{:.3} cal {:?}", m.ppg(), st.ppg, m.pace(), st.pace, m.three_rate(), st.three_rate, m.fg2(), st.fg2, m.fg3(), st.fg3, m.ft_rate(), st.ft_rate, m.tov_rate(), st.tov, l.cal);
        assert!(
            (m.ppg() / st.ppg - 1.0).abs() < 0.06,
            "{year} ppg {} vs {}",
            m.ppg(),
            st.ppg
        );
    }
}
