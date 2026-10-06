//! # Hardwood Dynasty
//!
//! A deep basketball simulation engine. The library is UI-agnostic: the `hwd` program in
//! `src/bin/hwd.rs` is one front end, but anything (a GUI, a web page, a bot) can drive it.
//!
//! Module map (read in this order if you are new):
//! * [`rng`] - seedable random numbers (everything is reproducible)
//! * [`settings`] - every setting with plain-English help, presets
//! * [`era`] - the rules of basketball by year, and how each era plays
//! * [`economy`] - salary cap, tax, aprons, pay scales
//! * [`player`], [`generate`] - players, attributes, archetypes, badges
//! * [`injury`] - injuries from sprains to career-ending
//! * [`life`], [`events`] - the life sim and the data-driven event engine
//! * [`game`] - the possession-by-possession game engine
//! * [`content`] - the content registry and mod system

pub mod awards;
pub mod calendar;
pub mod career;
pub mod college;
pub mod content;
pub mod contract;
pub mod draft;
pub mod economy;
pub mod era;
pub mod events;
pub mod fastsim;
pub mod finance;
pub mod franchise;
pub mod game;
pub mod generate;
pub mod import;
pub mod injury;
pub mod league;
pub mod leagueevents;
pub mod life;
pub mod names;
pub mod offseason;
pub mod overseas;
pub mod player;
pub mod playoffs;
pub mod progression;
pub mod records;
pub mod rng;
pub mod season;
pub mod settings;
pub mod setup;
pub mod story;
pub mod team;
pub mod trade;
pub mod types;
