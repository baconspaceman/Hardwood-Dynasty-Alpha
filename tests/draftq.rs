use hardwood_dynasty::calendar::Goal;
use hardwood_dynasty::league::*;
use hardwood_dynasty::setup::NewLeagueOptions;
use hardwood_dynasty::types::Phase;

#[test]
fn draft_class_quality() {
    let mut l = League::new(NewLeagueOptions {
        year: 2012,
        seed: "dq".into(),
        ..Default::default()
    })
    .unwrap();
    for _ in 0..3 {
        l.advance(Goal::UntilPhase(Phase::Draft));
        let d = l.draft.as_ref().unwrap();
        let mut v: Vec<(f64, f64, i32, String)> = d
            .pool
            .iter()
            .map(|&id| {
                let p = l.p(id);
                (
                    p.ovr as f64,
                    p.potential as f64,
                    d.year - p.birth_year,
                    format!("{:?}", p.affiliation).chars().take(4).collect(),
                )
            })
            .collect();
        v.sort_by(|a, b| {
            (b.0 * 0.5 + b.1 * 0.5)
                .partial_cmp(&(a.0 * 0.5 + a.1 * 0.5))
                .unwrap()
        });
        println!("{} class: {} prospects", d.year, v.len());
        for i in [0usize, 2, 4, 9, 14, 29, 44, 59] {
            if let Some(x) = v.get(i) {
                println!(
                    "  #{:<3} ovr {:.0} pot {:.0} age {} {}",
                    i + 1,
                    x.0,
                    x.1,
                    x.2,
                    x.3
                );
            }
        }
        // top NBA player ovr and rookies mean
        let best: f64 = l
            .players
            .iter()
            .filter(|p| p.is_active_pro())
            .map(|p| p.ovr as f64)
            .fold(0.0, f64::max);
        println!("  best pro ovr {best:.0}");
        l.advance(Goal::EndOfSeason);
    }
}
