# Frontend (web UI)

A React + Vite + Tailwind app that talks to `combat-api`. This directory holds
the production Dock planner, build tooling, and typed API client. Older ACS
and detailed-report components remain available in the source tree.

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

Then open http://localhost:5173. The production entry is the German Dock planner
selected in issue #72, based on local prototype commits `89cd311` and `3cac91b`.
Add only the units you want to plan with through the manual picker. Your own
available fleet is a snapshot; changing selected quantities never changes it.
Restore affects only quantities. Empty numeric fields remain blank and visibly
marked **Assumed zero**; conversion to zero happens when building an attack
scenario for the API. They are not verified observed inputs.

Combat research, player/alliance classes, per-unit lifeform combat percentages,
rapid fire, and universe debris rules are supported. Only **Simulieren** sends a
request (100 or 1,000 runs). Failed requests retain inputs and the last success;
changed inputs mark that result as outdated. The result is a partial profit:
all debris minus attacker losses, without loot, fuel, or rebuild. Fleet count
averages in the API summary truncate units, so the displayed loss average uses
the per-run economic identity instead. No ACS, import, account sync, or flight
controls are exposed in this manual flow.

The fleet marker is original local vector artwork. See
[art provenance](public/art/README.md); no external image host is contacted.

## Browser checks

```sh
npm ci
npx playwright install chromium
npm test
npm run lint
npm run build
# With combat-api running on port 3000:
LIVE_API=1 npm test -- --grep 'real local API smoke'
```

The deterministic browser suite intercepts the HTTP simulation route, verifying
input bounds, request contents, explicit starts, errors, keyboard navigation,
and layouts at 1440px and 1024px with image loading blocked. The separate live
smoke checks transport and rendered completion without random outcome assertions.
Set `PLAYWRIGHT_CHROME` to an installed Chromium executable if bundled browser
downloads are unavailable. CI runs the deterministic suite, lint, and build.

## Configuration

The API base URL is read from `VITE_API_BASE_URL` at build time. When unset
(or empty) the client uses same-origin requests, which the Vite dev server
proxies to `http://localhost:3000` — see `vite.config.ts`. Copy `.env.example`
to `.env.local` to point at a different API:

```
VITE_API_BASE_URL=https://my-api.example.com
```

## Release container

`Dockerfile` builds the bundle in a clean Node image and serves it with the
small runtime proxy in `server.mjs`. Leave `VITE_API_BASE_URL` unset so the
browser uses same-origin `/api`; set `API_UPSTREAM` when starting the runtime
container (the default is `http://api:3000`). `PORT` defaults to 8080. The
runtime uses the non-root `node` user.

From the repository root:

```bash
docker buildx build --load --platform linux/arm64 \
  -f frontend/Dockerfile -t ogame-combat-frontend:local-arm64 frontend
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

CI runs lint, build/typechecking, and browser tests. In lint,
`--max-warnings 0` is load-bearing, because
`no-console` is a warning and without the flag it never fails.

`eslint.base.mjs` and `tsconfig.base.json` are **copied from maxi-quality by its
`adopt.sh`** and must not be hand-edited — same rule as the clippy block in the
root `Cargo.toml` (see `CLAUDE.md`). Repo-specific choices go in
`eslint.config.mjs` and `tsconfig.json`, which extend them. `tsconfig.json`
overrides only what a browser app genuinely needs to differ on (DOM lib, bundler
resolution, JSX, no emit); none of the baseline's strict family is relaxed.

## Source layout

The production entry is `App.tsx` and `planner/` (scenario model, manual fleet
editor, numeric fields, and results). The older components listed below remain
in the source tree but are not mounted by the Dock entry.

```
src/
├── api/
│   ├── client.ts     # typed fetch wrapper; surfaces every failure as ApiError
│   ├── types.ts      # request/response models mirroring combat-types
│   └── index.ts      # barrel
├── combat/
│   └── input.ts      # technology, optional modifiers + defender resources
├── components/
│   ├── ClassInput.tsx       # per-side player and alliance class selectors
│   ├── FleetEntry.tsx       # fleet-entry region: two party columns, slot tabs
│   ├── LifeformInput.tsx    # per-side, per-entity lifeform percentages
│   ├── UniverseSettingsInput.tsx # optional universe debris override
│   ├── fleet/
│   │   ├── PartyColumn.tsx  # one side: slot tabs + the active slot's editor
│   │   └── SlotEditor.tsx   # one slot's composition rows and add-picker
│   ├── results/
│   │   ├── EconomicsSummary.tsx      # debris, loot, harvest and net profit
│   │   ├── LossesTable.tsx           # average losses by entity type
│   │   ├── OutcomeDistribution.tsx   # rates + per-outcome economics
│   │   └── RoundCompositionView.tsx  # opt-in representative round detail
│   ├── TechnologyInput.tsx  # composes technology, modifiers + resources
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

The planner owns its scenario in `App.tsx`; presentation components receive
values and callbacks. The API request is built at the scenario boundary.
