# Contributing

## Working rules

- Game rules and simulation logic live in the library (`src/`); `src/bin/hwd.rs` only reads input and prints.
- Prefer **data over code**: if a number or list could be a mod, put it in the content registry (`content.rs`).
- Keep everything deterministic: use the league's RNG, never the clock or hash-map order.
- Keep saves loadable; if you change a saved struct, add `#[serde(default)]` for new fields.
- Settings need a plain-English description and low/high notes (a test enforces this).

## Local workflow

1. Install Rust from https://rustup.rs
2. `cargo run --release` to play.
3. Before a PR: `cargo fmt`, `cargo clippy`, `cargo test --release`.
4. If you touched settings, tuning, rules or content: `cargo run --release -- gen-docs docs`.

## Pull requests

- Keep the scope narrow and explain the gameplay impact.
- Call out save-format changes.
- Include a short transcript when the text interface changes.

## Branch naming

- `feature/*` gameplay or engine work, `data/*` content and history, `docs/*`, `chore/*`.
