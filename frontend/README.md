# Frontend (web UI)

A React + Vite + Tailwind app that talks to `combat-api`. This directory holds
the app shell, build tooling and typed API client; the actual fleet, technology
and results surfaces are tracked in sibling issues and slot into the
placeholder regions established here.

## Stack

React 18, Vite 5, Tailwind 3, recharts. The predecessor's stack, chosen for
continuity with the archived reference — see issue #22.

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

Then open http://localhost:5173, click **Simulate**, and the shell will call
`POST /api/simulate` with a demo request and render a typed summary of the
response.

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

## Layout

```
src/
├── api/
│   ├── client.ts     # typed fetch wrapper; surfaces every failure as ApiError
│   ├── types.ts      # request/response models mirroring combat-types
│   └── index.ts      # barrel
├── components/
│   ├── FleetEntry.tsx       # fleet-entry region (placeholder)
│   ├── TechnologyInput.tsx  # technology-input region (placeholder)
│   └── ResultsPanel.tsx     # results region (typed summary + error state)
├── config.ts         # API base URL resolution
├── App.tsx           # shell: layout + the one piece of shared state
├── main.tsx          # React root
└── index.css         # Tailwind entry
```

Each of the three regions is its own component file and owns none of the
others' logic — that seam is the point of this issue. Sibling issues fill them
in without growing one file.
