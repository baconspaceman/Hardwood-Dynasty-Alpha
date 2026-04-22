# Repository Checklist

## GitHub Settings

- Add repository topics such as `basketball`, `sports-sim`, `gm-sim`, `react`, and `typescript`
- Protect `main`
- Require CI before merge
- Add a short repository description in GitHub settings

## Product Setup

- Decide the first supported starting year
- Choose the historical data source and normalization plan
- Decide on a license
- Pick a deployment target

## Engineering Setup

- Keep Node 22 as the CI baseline
- Run `npm run check` before merge
- Add save migrations before shipping the first generated league
