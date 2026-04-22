# Contributing

## Working Rules

- Keep game rules and simulation logic inside `src/game/engine`.
- Keep historical data mapping concerns inside `src/game/data`.
- Keep UI components dumb whenever possible.
- Keep saveable state serializable and versioned.

## Local Workflow

1. Install dependencies with `npm install`.
2. Run the app with `npm run dev`.
3. Before opening a PR, run `npm run check`.

## Pull Requests

- Keep the scope narrow.
- Explain the gameplay impact clearly.
- Call out save-state changes and data migrations.
- Include screenshots when the interface changes.

## Branch Naming

- `feature/*` for gameplay or engine work
- `data/*` for historical data packs and mappings
- `docs/*` for documentation
- `chore/*` for tooling and CI
