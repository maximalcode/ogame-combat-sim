# ogame-combat-sim

A fast OGame fleet combat simulator, written in Rust.

Combat in OGame is stochastic — the same fleets sent twice give different
results. A single battle tells you almost nothing, so this runs the battle
hundreds of times and reports the distribution: how often you win, what it
costs you when you do, and whether the debris field pays for the trip.

> **Unofficial fan project.** Not affiliated with, endorsed by, or connected to
> Gameforge. OGame is a trademark of Gameforge. See [Provenance](#provenance).

## What it models

The engine implements OGame's combat resolution as it actually behaves:

- **Rounds** — up to six, with shields regenerating between them
- **Rapid fire** — the full cross-table, including chained re-rolls
- **Shield bounce** — shots under 1% of shield strength are absorbed entirely
- **Explosions** — hull-integrity roll after each round
- **Debris fields** — per-universe recovery rates, set separately for ships and
  defences, with deuterium debris where the universe enables it, plus moon
  chance and the recycler count needed to collect
- **Loot** — plunder at 50/75/100%, capped by surviving cargo capacity
- **Downscaling** — battles above ten million ships are simulated at reduced
  scale and extrapolated, so a fleet of any size resolves in reasonable time

## Accuracy — what is and is not modelled

Combat resolution itself has been stable for years: rounds, rapid fire, the
bounce rule, shield regeneration, the explosion roll, the `+10% per level`
technology scaling and the ship stat table are unchanged from v7 through the
current v13. All of that is implemented and tested here.

What is **not** yet applied is everything added since that injects per-ship stat
modifiers:

| Missing | Since | Effect |
| --- | --- | --- |
| Lifeform research bonuses | v9 (2022) | Per-ship-type bonus to hull, shield and firepower. The largest gap by far. |
| Player class bonuses | v7 | General grants +2 effective Weapons/Shielding/Armour levels |
| Alliance class bonuses | v8 (2021) | Warrior grants +1 effective level to all three |
| v13 instant-calc rule | v13 (2026) | Battles short-circuit above a 10,000× attack-power ratio |

In practice: for a battle with no lifeforms and no classes, results should be
sound. For a developed 2026 account they will be optimistic or pessimistic
depending on who holds the bonuses, because the engine currently sees none of
them. All four are tracked in the issues, and the fix is one additive term per
stat rather than anything structural.

Stating this plainly matters more than the gaps do — every simulator in this
space advertises accuracy and none of them publish what they get wrong.

## Quick start

100 cruisers against 1000 light fighters and 200 rocket launchers, a thousand
times:

```bash
cargo run -p combat-cli -- sim \
  -a "cruiser:100" \
  -d "lf:1000,rocketlauncher:200" \
  --tech 10 -n 1000
```

Ships can be named, abbreviated or given by ID — `cruiser`, `cr` and `206` are
the same ship, and matching ignores case and punctuation. `combat-cli entities`
prints the whole table with every alias.

A battle can also come from a file, using exactly the JSON the HTTP endpoint
accepts:

```bash
cargo run -p combat-cli -- sim --file battle.json
```

Useful flags: `-n` for the number of simulations (uncapped — it is your CPU),
`--rounds` for the round-by-round breakdown of one battle, `--planet
M,C,D` to give the defender something worth looting, and `--no-rapid-fire` to
turn rapid fire off. `--help` lists the rest.

## Running it as a server

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

Fleets are keyed by OGame's own entity IDs over JSON and in the library. The CLI
also accepts names and aliases; `combat-cli entities` prints them alongside each
unit's stats.

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
| `combat-cli` | The command-line interface: `sim` and `entities` |

A web UI is planned but not yet here; see the
[open issues](https://github.com/maximalcode/ogame-combat-sim/issues) for what
is actually in progress. This README describes only what exists today.

## Development

```bash
cargo test --workspace
```

Tests are compiled optimised (`[profile.test] opt-level = 3`). The engine is a
Monte Carlo loop, and an unoptimised one is roughly fourteen times slower — the
suite takes seconds this way and minutes without it.

```bash
cargo bench
```

A criterion suite over four cases: the stat table loaded on its own, and small,
medium and downscaled-large battles. `[profile.bench]` mirrors the release
profile, so the first compile is slow and the numbers describe the binary that
actually ships. This README quotes none of them — run it on your own hardware,
because a benchmark figure from someone else's machine is decoration.

Before pushing, the same gates CI runs:

```bash
cargo fmt --check \
  && cargo clippy --workspace --all-targets \
  && cargo deny check advisories bans licenses \
  && cargo test --workspace
```

`cargo deny` needs installing separately (`cargo install cargo-deny`); the rest
ship with the toolchain. Formatting, lint policy and the supply-chain rules come
from [maxi-quality](https://github.com/maximalcode/maxi-quality), which also runs
Semgrep, Gitleaks and OSV-Scanner over every push.

Work happens on `develop`; `main` is the reviewed branch.

## Provenance

This is an independent reimplementation. OGame's combat mechanics were worked
out by observing behaviour and comparing results against TrashSim by Klaas,
which was the reference for correctness. TrashSim went offline in 2026 and its
forum thread is archived; its engine survives as
[MIT-licensed source](https://github.com/klaasvp/trashsim-public). No TrashSim
code, assets, or styling are used here — game mechanics are not copyrightable,
but assets and code are, and none were copied. Ship and defence statistics are
OGame game data.

If you represent Gameforge and want something changed, open an issue.

## License

MIT — see [LICENSE](LICENSE).
