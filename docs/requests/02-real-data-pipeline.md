# Request 2: Accurate real-player rosters, stats and tendencies

## Goal
Let a user build a league full of **real, accurate** players and years (names, ages, heights,
stats, contracts) and have the sim and the tendencies reflect how those players really played.

## Hard rules (read first)
- **Do not commit real player data to the repo.** The repo ships a tool, not a dataset. The user
  downloads data themselves and the tool converts it locally into the existing import formats.
- **Check every source's terms before building a fetcher.** Many stat sites forbid scraping or
  redistribution, and some data licences differ for personal use versus sharing. The tool must read
  files the user already has, and must never hammer a website. Record each source's licence in
  `docs/DATA_SOURCES.md` and show it to the user when they choose a source.
- Names and stat lines are facts, but databases and photos are protected by their owners.
  **Never include photos or likenesses** (see request 5).
- This is a hobby project: no official-league branding, logos or trademarks in shipped content.

## Candidate sources to evaluate (verify licence and current availability for each)
- Open datasets people publish (for example on Kaggle or GitHub) covering box scores, per-game and
  advanced stats, draft history, and salaries.
- Community-maintained league APIs/wrappers (for example the unofficial `nba_api` Python package):
  usable only if its terms allow it, rate-limited and cached.
- Public-domain or openly licensed historical compilations for pre-1980 eras.
- Overseas leagues (EuroLeague and others): often thinner data; fall back to the generator.
The first deliverable is a **short written comparison** of sources: coverage by year, fields
available, licence, and effort. The user picks one before any fetcher is built.

## What to build
1. **`tools/` or a `datatool` binary** (kept separate from the game library; extra dependencies are
   fine here if justified). Input: files in a folder the user points at. Output: roster JSON/CSV
   and year-pack JSON matching `docs/IMPORTING.md`.
2. **Name and identity matching** across sources and years (accents, suffixes, nicknames, players
   who changed names). Produce a report of unmatched or ambiguous rows instead of guessing.
3. **Stats to attributes.** `src/import.rs` already infers ratings from per-game stats. Upgrade
   it to use richer inputs when present: shot-zone and shot-type data, per-100 and advanced rates
   (usage, assist %, steal %, block %, rebound %, free-throw rate, turnover %), tracking-style data
   for later years, and **era normalisation** (a 20 ppg season in 1962 is not a 20 ppg season in
   2019). Validate against the tier table from request 1.
4. **Era-correct data handling.** Many stats do not exist before certain years (3-pointers before
   1979, steals/blocks before 1974, minutes before 1952). Missing data must degrade gracefully to
   position-and-era estimates, and flag the player as "estimated".
5. **Contracts.** Where salary data exists use it, scaled with `economy.rs`; otherwise generate.
6. **A quality report** per import: how many players matched, how many are estimated, how far each
   rating is from the source stats.

## Acceptance
- Running the tool on a small sample dataset (include a tiny made-up fixture in `tests/`, never
  real data) produces a file that `import roster` and `newpack` accept.
- A "backtest": simulate a historical season from imported data and compare team win totals and
  league leaders to the source. Report the error. Target: most teams within a few wins.
- Documentation for non-programmers: step by step, how to get the data and run the tool.
