use hardwood_dynasty::content::Content;
use hardwood_dynasty::game::*;
use hardwood_dynasty::generate::*;
use hardwood_dynasty::player::*;
use hardwood_dynasty::rng::Rng;

fn make_team(content: &Content, rng: &mut Rng, season: i32, id: u16) -> GameTeam {
    let mut players = vec![];
    for i in 0..13u32 {
        let target = (rng.gauss(
            if i < 5 {
                66.0
            } else if i < 9 {
                56.0
            } else {
                48.0
            },
            5.0,
        ))
        .clamp(35.0, 92.0);
        let spec = GenSpec::new(season, 26, target, target, OriginKind::College);
        let p = generate_player(content, rng, id as u32 * 100 + i, &spec, 1.0);
        players.push(GamePlayer::from_player(&p, &content.badges, 1.0, 0.0));
    }
    let rules = content.rules(season);
    assign_rotation(&mut players, false, &rules);
    GameTeam {
        id,
        name: format!("T{id}"),
        players,
        strategy: Strategy::default(),
    }
}

fn main() {
    let content = Content::default();
    let years: Vec<i32> = std::env::args()
        .skip(2)
        .filter_map(|a| a.parse().ok())
        .collect();
    for season in years {
        let mut rng = Rng::new(season as u64);
        let teams: Vec<GameTeam> = (0..30)
            .map(|i| make_team(&content, &mut rng, season, i))
            .collect();
        let refs = Refs::from_players(teams.iter().flat_map(|t| t.players.iter()));
        let mut ctx = GameContext {
            refs,
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
        };
        calibrate(&mut ctx, &teams, &mut rng, 150, 3);
        println!("cal {:?}", ctx.cal);
        let n = 600;
        let (
            mut pts,
            mut poss,
            mut fga,
            mut tpa,
            mut tpm,
            mut fgm,
            mut fta,
            mut ftm,
            mut tov,
            mut orb,
            mut drb,
            mut ast,
            mut stl,
            mut blk,
            mut pf,
            mut ot,
            mut hw,
        ) = (
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        );
        let mut margins = vec![];
        let mut mins = 0.0;
        for g in 0..n {
            let a = (g * 7) % 30;
            let b = (a + 1 + (g * 3) % 29) % 30;
            let bs = simulate_game(&ctx, &teams[a], &teams[b], &mut rng);
            for tb in [&bs.home, &bs.away] {
                pts += tb.pts as f64;
                poss += tb.possessions as f64;
                for p in &tb.players {
                    let s = &p.stats;
                    fga += s.fga as f64;
                    fgm += s.fgm as f64;
                    tpa += s.tpa as f64;
                    tpm += s.tpm as f64;
                    fta += s.fta as f64;
                    ftm += s.ftm as f64;
                    tov += s.tov as f64;
                    orb += s.orb as f64;
                    drb += s.drb as f64;
                    ast += s.ast as f64;
                    stl += s.stl as f64;
                    blk += s.blk as f64;
                    pf += s.pf as f64;
                    mins += s.min;
                }
            }
            ot += bs.overtimes as f64;
            hw += if bs.winner_is_home() { 1.0 } else { 0.0 };
            margins.push((bs.home.pts as f64 - bs.away.pts as f64).abs());
        }
        let tg = (n * 2) as f64;
        let st = content.style(season);
        let mm: f64 = margins.iter().sum::<f64>() / n as f64;
        println!("== {season}: target ppg {:.1} pace {:.1} 3PAr {:.3} fg2 {:.3} fg3 {:.3} ftr {:.3} ft% {:.3} tov {:.3} orb {:.3}", st.ppg, st.pace, st.three_rate, st.fg2, st.fg3, st.ft_rate, st.ft_pct, st.tov, st.orb);
        println!("   got    ppg {:.1} pace {:.1} 3PAr {:.3} fg2 {:.3} fg3 {:.3} ftr {:.3} ft% {:.3} tov {:.3} orb {:.3}",
            pts / tg, (fga - orb + tov + 0.44 * fta) / tg, tpa / fga, (fgm - tpm) / (fga - tpa), if tpa > 0.0 { tpm / tpa } else { 0.0 }, fta / fga, ftm / fta, tov / poss, orb / (orb + drb) * 0.0 + orb / (fga - fgm + 0.44 * 0.0 + (fta - ftm) * 0.0));
        println!("   per game: fga {:.1} fta {:.1} ast {:.1} stl {:.1} blk {:.1} pf {:.1} reb {:.1} min {:.0} OT {:.3} home win {:.3} avg margin {:.1}", fga / tg, fta / tg, ast / tg, stl / tg, blk / tg, pf / tg, (orb + drb) / tg, mins / tg, ot / n as f64, hw / n as f64, mm);
    }
}
