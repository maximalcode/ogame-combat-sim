# ogame-combat-sim

A fast OGame v7 fleet combat simulator, written in Rust.

Combat in OGame is stochastic — the same fleets sent twice give different
results. A single battle tells you almost nothing, so this runs the battle
hundreds of times and reports the distribution: how often you win, what it
costs you when you do, and whether the debris field pays for the trip.

> **Unofficial fan project.** Not affiliated with, endorsed by, or connected to
> Gameforge. OGame is a trademark of Gameforge. See [Provenance](#provenance).

## What it models

The engine implements OGame v7 combat as it actually behaves:

- **Rounds** — up to six, with shields regenerating between them
- **Rapid fire** — the full cross-table, including chained re-rolls
- **Shield bounce** — shots under 1% of shield strength are absorbed entirely
- **Explosions** — hull-integrity roll after each round
- **Debris fields** — with configurable metal/crystal recovery, moon chance and
  the recycler count needed to collect
- **Loot** — plunder at 50/75/100%, capped by surviving cargo capacity
- **Downscaling** — battles above ten million ships are simulated at reduced
  scale and extrapolated, so a fleet of any size resolves in reasonable time

## Quick start

Run the server:

```bash
cargo run -p combat-api
```

Then send it a battle — 100 cruisers against 1000 light fighters:

```bash
curl -s -X POST localhost:3000/api/simulate -H 'Content-Type: application/json' -d '{
  "attacker": {"technology": {"weapon": 10, "shield": 10, "armour": 10}, "entities": {"206": 100}},
  "defender": {"technology": {"weapon": 10, "shield": 10, "armour": 10}, "entities": {"204": 1000}},
  "use_rapid_fire": true,
  "simulations": 100
}'
```

The response has two halves: `results` (win counts, average rounds, per-run
detail) and `report` (fleet snapshots, losses, debris, loot, profit).

There is no database. The server keeps nothing between requests — a simulation
is a pure function of its input — so there is nothing to install, migrate or
back up.

| Endpoint | Method | Purpose |
| --- | --- | --- |
| `/` | GET | Version and liveness |
| `/api/simulate` | POST | Run a battle, get results and a report |

`PORT` (default 3000) and `MAX_SIMULATIONS` (default 1000) are read from the
environment. The cap is server protection only — the library has no limit.

## Using it as a library

`combat-core` is usable on its own, with no HTTP involved:

```rust
use combat_core::{ReportBuilder, Simulator};
use combat_types::CombatRequest;

let request: CombatRequest = serde_json::from_str(
    r#"{
    "attacker": { "technology": {"weapon": 10, "shield": 10, "armour": 10},
                  "entities": {"204": 100} },
    "defender": { "technology": {"weapon": 8, "shield": 8, "armour": 8},
                  "entities": {"401": 50} },
    "use_rapid_fire": true,
    "simulations": 1000
}"#,
)?;

let results = Simulator::new().simulate_multiple(&request);
println!("attacker wins {:.1}%", results.attacker_win_rate() * 100.0);

let report = ReportBuilder::new().build_summary_report(&request, &results);
```

`CombatRequest` deserializes from exactly the JSON the HTTP endpoint accepts,
so anything you can curl you can also feed straight to the library. That
snippet is compiled and run by `combat-core/tests/readme_example.rs`, so it
cannot quietly stop working.

## Entity IDs

Fleets are keyed by OGame's own entity IDs.

| ID | Ship | ID | Ship | ID | Defence |
| --- | --- | --- | --- | --- | --- |
| 202 | Small Cargo | 211 | Bomber | 401 | Rocket Launcher |
| 203 | Large Cargo | 212 | Solar Satellite | 402 | Light Laser |
| 204 | Light Fighter | 213 | Destroyer | 403 | Heavy Laser |
| 205 | Heavy Fighter | 214 | Deathstar | 404 | Gauss Cannon |
| 206 | Cruiser | 215 | Battlecruiser | 405 | Ion Cannon |
| 207 | Battleship | 217 | Crawler | 406 | Plasma Turret |
| 208 | Colony Ship | 218 | Reaper | 407 | Small Shield Dome |
| 209 | Recycler | 219 | Pathfinder | 408 | Large Shield Dome |
| 210 | Espionage Probe | | | | |

## Layout

| Crate | Contents |
| --- | --- |
| `combat-types` | The data model: requests, results, reports, ship and defence stats |
| `combat-core` | The engine: rounds, rapid fire, explosions, debris, loot, downscaling |
| `combat-api` | A small stateless HTTP server over the engine |

A command-line interface, benchmarks, and a web UI are planned but not yet
here; see the [open issues](https://github.com/maximalcode/ogame-combat-sim/issues)
for what is actually in progress. This README describes only what exists today.

## Development

```bash
cargo test --workspace
```

Tests are compiled optimised (`[profile.test] opt-level = 3`). The engine is a
Monte Carlo loop, and an unoptimised one is roughly fourteen times slower — the
suite takes seconds this way and minutes without it.

Before pushing, the same three gates CI runs:

```bash
cargo fmt --check && cargo clippy --workspace --all-targets && cargo test --workspace
```

Work happens on `develop`; `main` is the reviewed branch.

## Provenance

This is an independent reimplementation. OGame's combat mechanics were worked
out by observing behaviour and comparing results against
[TrashSim](https://trashsim.universeview.be/) by Klaas, whose simulator was the
reference for correctness. No TrashSim code, assets, or styling are used here —
game mechanics are not copyrightable, but assets and code are, and none were
copied. Ship and defence statistics are OGame game data.

If you represent Gameforge and want something changed, open an issue.

## License

MIT — see [LICENSE](LICENSE).
