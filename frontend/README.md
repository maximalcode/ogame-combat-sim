# Frontend (web UI)

A React + Vite + Tailwind app that talks to `combat-api`. This directory holds
the app shell, build tooling, typed API client, fleet-entry surface (multi-slot
ACS composition), technology and planet-resource surface, and results display.

## Stack

React 18, Vite 6, Tailwind 3, recharts. The predecessor's stack, chosen for
continuity with the archived reference — see issue #22.

Issue #22 named Vite **5**; this is Vite 6 and the bump was forced, not
preferred. Every 5.x release including the last one (5.4.21) carries three
advisories — GHSA-fx2h-pf6j-xcff (8.2), GHSA-4w7w-66w2-5vf9, GHSA-v6wh-96g9-6wx3
— and drags in esbuild 0.21.5 with a fourth. All four are fixed only from
6.4.2/6.4.3 onward, so staying on 5 means either a red OSV-Scanner gate or
suppressing a High, and neither is worth it for a build tool that touches no
application code. `@vitejs/plugin-react` already supported `^6.0.0`.

## Requirements

Node 18+ (developed against Node 24). npm is the only package manager
configured; a `package-lock.json` is committed once dependencies are installed.

## Getting started

From this directory:

```bash
npm install
npm run dev      # http://localhost:5173
```

The dev server proxies `/api` to `http://localhost:3000`, so start the API
alongside it:

```bash
# from the repo root, in another terminal
cargo run -p combat-api
```

Then open http://localhost:5173, compose the two fleets (each side supports
multiple ACS slots), set each side's combat technology levels — and, if the
defending planet is known, its resources — and click **Simulate**. The results
surface renders the outcome distribution alongside its economics, ship losses,
debris, loot and profit. Opening **Round detail** runs one representative
battle with per-round composition tracking; aggregate simulations do not pay
that extra cost.

## Configuration

The API base URL is read from `VITE_API_BASE_URL` at build time. When unset
(or empty) the client uses same-origin requests, which the Vite dev server
proxies to `http://localhost:3000` — see `vite.config.ts`. Copy `.env.example`
to `.env.local` to point at a different API:

```
VITE_API_BASE_URL=https://my-api.example.com
```

## Scripts

| Script | What it does |
| --- | --- |
| `npm run dev` | Vite dev server with HMR |
| `npm run build` | `tsc --noEmit` then `vite build` — the production build |
| `npm run preview` | Serve the production build locally |
| `npm run typecheck` | `tsc --noEmit` |
| `npm run lint` | `eslint src --max-warnings 0` — the CI gate |

## Linting

CI runs `npm ci && npm run lint` in this directory and nothing else, so `lint`
is the whole TypeScript gate — `--max-warnings 0` is load-bearing, because
`no-console` is a warning and without the flag it never fails.

`eslint.base.mjs` and `tsconfig.base.json` are **copied from maxi-quality by its
`adopt.sh`** and must not be hand-edited — same rule as the clippy block in the
root `Cargo.toml` (see `CLAUDE.md`). Repo-specific choices go in
`eslint.config.mjs` and `tsconfig.json`, which extend them. `tsconfig.json`
overrides only what a browser app genuinely needs to differ on (DOM lib, bundler
resolution, JSX, no emit); none of the baseline's strict family is relaxed.

## Layout

```
src/
├── api/
│   ├── client.ts     # typed fetch wrapper; surfaces every failure as ApiError
│   ├── types.ts      # request/response models mirroring combat-types
│   └── index.ts      # barrel
├── combat/
│   └── input.ts      # technology levels + optional defender resources
├── components/
│   ├── FleetEntry.tsx       # fleet-entry region: two party columns, slot tabs
│   ├── fleet/
│   │   ├── PartyColumn.tsx  # one side: slot tabs + the active slot's editor
│   │   └── SlotEditor.tsx   # one slot's composition rows and add-picker
│   ├── results/
│   │   ├── EconomicsSummary.tsx      # debris, loot, harvest and net profit
│   │   ├── LossesTable.tsx           # average losses by entity type
│   │   ├── OutcomeDistribution.tsx   # rates + per-outcome economics
│   │   └── RoundCompositionView.tsx  # opt-in representative round detail
│   ├── TechnologyInput.tsx  # technology levels + defender planet resources
│   └── ResultsPanel.tsx     # results region state + aggregate/round tabs
├── fleet/
│   ├── catalog.ts    # entity ids and names the pickers offer
│   └── types.ts      # FleetState, slot helpers, buildCombatRequest
├── results/
│   └── model.ts      # typed outcome/composition derivation + formatting
├── config.ts         # API base URL resolution
├── App.tsx           # shell: layout + the one piece of shared state
├── main.tsx          # React root
└── index.css         # Tailwind entry
```

Each of the three regions is its own component file and owns none of the
others' logic. Deeper component folders keep each region's rendering details
behind that established seam.
