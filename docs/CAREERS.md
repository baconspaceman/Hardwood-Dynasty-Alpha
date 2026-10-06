# Careers and the life sim

The same world runs under every role; your role decides which controls you have.

## Playing as a created player

`create-player` with no arguments prints every option. Example:

```text
create-player name="Alex Rivers" pos=SG height=77 arch=two_way_wing talent=4 start=hs9 points=three:20,mid:15,athletic:10
```

* **talent 1-5** sets your ceiling (and your skill-point budget: 40 to 80).
* **start**: `hs9`-`hs12`, `college`, `overseas`, `pro`. The era is whatever league you started.
* Your **time budget** (`allocate study=25 skill_work=35 gym=10 social=10 family=10 rest=10`) changes
  every month: study protects grades and eligibility, skill work and gym drive development, rest
  restores energy, friends/family move happiness and stress, media builds fame, a job earns money.
* **Life events** appear with choices (a party invite before a big game, a runner offering cash, a
  coach conflict, an endorsement offer, burnout...). `events` lists them; `event <id> <option>` answers.
  `setauto on` lets the game decide small choices.
* **Milestone decisions** (`decisions` / `decide`): after senior year choose a college from the offers
  you earned (or the draft, overseas, a pathway program); each college offseason stay/declare/transfer;
  draft night (or undrafted: camp deal / overseas); free-agent offers; when to retire.
* **Money**: salary, taxes (era-specific!), agent fees, endorsements, lifestyle, investments, debt.
* Your player is simulated in the same engine as everyone else: stats, injuries (including
  career-ending ones), awards, the Hall of Fame.

## GM

Trades (`trade BOS give "Name", 2027r1 get "Other"` then add `confirm`), free agency (`fa`, `sign`),
`resign`, the draft (`draftboard`, `scout`, `pick`), staff (`people`, `hire`, `fire`), cap exceptions
(minimum, mid-level) and the era's trade rules (salary matching, apron limits, the Stepien rule).
The owner grades you; get fired and offers arrive.

## Head coach

`starters`, `minutes`, `strategy tempo|threes|inside|glass|hack|defense`, `focus <player> <skill>`.
Coach quality (yours or the AI's) shapes development and late-game decisions.

## Owner

`money` is a one-page dashboard with traffic-light health. `budget ticket 1.2`, `budget facilities 1.5`,
`budget payroll 150M`, `budget tax 20M`. `chase 25M` shows the exact luxury-tax bill, profit hit and
title-odds change before you commit. Owners may delegate the GM or coach jobs.

## Scouts, assistants and college roles

Scouts reduce prospect uncertainty (`scout <player>`) and earn promotions. Assistant coaches set practice
focus and climb to head coach. College coaches recruit (`college recruits`, `recruit <name> <points>`),
manage NIL (modern era) and chase the national title; ADs control budget and hiring.
