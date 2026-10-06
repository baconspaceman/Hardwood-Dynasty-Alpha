//! The league record book: single-game bests and notable milestones.

use crate::game::BoxScore;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordEntry {
    pub value: f64,
    pub holder: String,
    pub team: String,
    pub season: Season,
    pub detail: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RecordBook {
    /// Category key → record.
    pub single_game: BTreeMap<String, RecordEntry>,
    pub team_game: BTreeMap<String, RecordEntry>,
}

pub const SINGLE_GAME_CATS: &[(&str, &str)] = &[
    ("pts", "Points in a game"),
    ("reb", "Rebounds in a game"),
    ("ast", "Assists in a game"),
    ("stl", "Steals in a game"),
    ("blk", "Blocks in a game"),
    ("tpm", "Threes in a game"),
];

impl RecordBook {
    /// Check a box score for new records. Returns a list of headline strings for new ones.
    pub fn check_game(&mut self, bs: &BoxScore, season: Season, playoffs: bool) -> Vec<String> {
        let mut out = vec![];
        for tb in [&bs.home, &bs.away] {
            for p in &tb.players {
                let s = &p.stats;
                let vals = [
                    ("pts", s.pts as f64),
                    ("reb", s.reb() as f64),
                    ("ast", s.ast as f64),
                    ("stl", s.stl as f64),
                    ("blk", s.blk as f64),
                    ("tpm", s.tpm as f64),
                ];
                for (k, v) in vals {
                    // Ignore tiny values so the first game of a league doesn't spam records.
                    let floor = match k {
                        "pts" => 35.0,
                        "reb" => 18.0,
                        "ast" => 14.0,
                        "stl" => 6.0,
                        "blk" => 7.0,
                        _ => 7.0,
                    };
                    if v < floor {
                        continue;
                    }
                    let key = k.to_string();
                    let better = self
                        .single_game
                        .get(&key)
                        .map(|r| v > r.value)
                        .unwrap_or(true);
                    if better {
                        let prev = self.single_game.get(&key).map(|r| r.value).unwrap_or(0.0);
                        self.single_game.insert(
                            key,
                            RecordEntry {
                                value: v,
                                holder: p.name.clone(),
                                team: tb.name.clone(),
                                season,
                                detail: format!(
                                    "{}{}",
                                    if playoffs { "playoffs, " } else { "" },
                                    s.pts
                                ),
                            },
                        );
                        if prev > 0.0 {
                            let cat = SINGLE_GAME_CATS
                                .iter()
                                .find(|(c, _)| *c == k)
                                .map(|c| c.1)
                                .unwrap_or(k);
                            out.push(format!(
                                "NEW RECORD: {} sets the {} mark with {}!",
                                p.name,
                                cat.to_lowercase(),
                                v as i64
                            ));
                        }
                    }
                }
            }
            let key = "team_pts".to_string();
            let better = self
                .team_game
                .get(&key)
                .map(|r| tb.pts as f64 > r.value)
                .unwrap_or(true);
            if better {
                self.team_game.insert(
                    key,
                    RecordEntry {
                        value: tb.pts as f64,
                        holder: tb.name.clone(),
                        team: tb.name.clone(),
                        season,
                        detail: String::new(),
                    },
                );
            }
        }
        out
    }
}
