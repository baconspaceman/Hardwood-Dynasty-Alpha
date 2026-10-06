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
pub mod content;
pub mod contract;
pub mod economy;
pub mod era;
pub mod events;
pub mod franchise;
pub mod game;
pub mod generate;
pub mod injury;
pub mod life;
pub mod names;
pub mod player;
pub mod rng;
pub mod settings;
pub mod story;
pub mod types;
