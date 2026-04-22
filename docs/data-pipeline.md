# Data Pipeline

## Goal

The engine needs a way to turn a historical NBA season into a fictionalized but mechanically faithful league state.

## Recommended Pipeline

1. Import historical player and team data by year.
2. Normalize ratings, contracts, ages, and team quality.
3. Map each real franchise slot to one fictional franchise slot.
4. Generate fictional player identities while preserving role, age, peak level, and style.
5. Write the processed year pack into a stable game-data format.

## Important Rules

- Never expose real player or team names in the shipped game layer.
- Preserve statistical shape and era context.
- Keep the fictional identity layer deterministic so the same year generates the same league.

## First Useful Data Deliverable

Build one complete year pack first. `1996` is a strong candidate because it gives you a recognizable championship-level wing anchor, clear title contenders, and a strong era identity.
