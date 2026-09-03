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
- **Player and alliance classes** — a General fights two effective technology
  levels above his research, a Warrior alliance one, and the two stack
- **Lifeform research** — per-ship-type percentages added to hull, shield and
  firepower in the same bracket as technology. A request carries the resolved
  percentages; the library can work them out from researched levels for you
- **Debris fields** — per-universe recovery rates, set separately for ships and
  defences, with deuterium debris where the universe enables it, plus moon
  chance and the recycler count needed to collect
- **Loot** — plunder at 50/75/100%, capped by surviving cargo capacity
- **Downscaling** — battles above ten million ships are simulated at reduced
  scale and extrapolated, so a fleet of any size resolves in reasonable time
- **Instant calculation** — v13's entry rule: a side with more than 10,000 times
  the opposition's combined attack power can win the battle without it being
  fought, provided the engine can also show the six rounds would have produced
  the same result. A fleet of espionage probes is not an automatic loss — it
  cannot win against an armed opponent, but a hundred of them against a single
  Rocket Launcher is a draw, not a defeat, because the launcher only fires once
  a round

## Accuracy — what is and is not modelled

Combat resolution itself has been stable for years: rounds, rapid fire, the
bounce rule, shield regeneration, the explosion roll, the `+10% per level`
technology scaling and the ship stat table are unchanged from v7 through the
current v13. All of that is implemented and tested here.

What is **not** yet applied:

| Missing | Since | Effect |
| --- | --- | --- |
| The lifeform empire model | v9 (2022) | Bonuses apply, but which planets, buildings and species experience produced them is the caller's arithmetic — the engine takes researched levels, or the resolved percentages |

v13's instant calculation is implemented, and on considerably less than the game
applies it to. The 10,000× ratio is the gate; a battle only skips its rounds if
the engine can also show that the losing side cannot take a single unit with it,
that the winning side's fire actually gets through, that the winning side's
firepower clears the losing side's total hitpoints by the same 10,000× margin,
and that the winning side fires enough *shots* to have aimed at every unit the
loser brought — six rounds' worth at one shot per armed unit, against the same
margin. The last two are what decline most battles. The hitpoint margin catches
a Solar Satellite against a lone Espionage Probe: the ratio is met by any armed
opponent at all, but six rounds of the satellite's single point of damage leave
the probe short of destroyed. The shot count catches the opposite shape — a
handful of Deathstars at maximum Weapons against a thousand probes, which is
overwhelming on paper and, with rapid fire switched off, is a thousand targets
being shot at about once each. Everything else is simulated — slower, and the
same answer. The one visible consequence is the round count: a battle decided
this way honestly reports zero rounds fought.

One thing outside combat itself is missing too, and it changes what an attack
costs: destroyed defences are never rebuilt here, where the game gives each one
a 70% chance of coming back free — 85% with an Engineer. Defence losses in this
simulator are therefore the worst case for the defender. The Engineer flag a
request can carry is read by nothing for that reason.

The General's other perk — a small chance for a Light Fighter to destroy a
Deathstar outright — is not implemented either. Sources split between 1-in-1000
and 1-in-10000 and none of them are official, so there is no number here worth
committing to. It is recorded rather than quietly dropped.

In practice: results should be sound for a 2026 account, classes and lifeform
research included, as long as the lifeform figures handed in are right. The
lifeform table shipped here is hardcoded and goes stale whenever Gameforge
rebalances; Gameforge publishes the live configuration per universe in
`serverData.xml`, and reading it is a second implementation of the same
interface rather than a rewrite. The rest is tracked in the issues.

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

A file is also the only way to give either side a class: the shorthand has no
flag for it, and `attacker_bonuses` / `defender_bonuses` in the JSON take a
`player_class` of `general` and an `alliance_class` of `warrior`.

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

Lifeform research goes in beside a side's technology, as percentages added to
the base stats of individual ship types — this attacker's Cruisers hit and
survive 6% harder, and nothing else in the fleet moves:

```json
"attacker": {
  "technology": {"weapon": 10, "shield": 10, "armour": 10},
  "entities": {"206": 100},
  "lifeform": {"206": {"weapon": 6.0, "shield": 6.0, "armour": 6.0}}
}
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
environment. Invalid, empty, out-of-range, and non-Unicode values retain the
legacy fallback to those defaults. A configured `MAX_SIMULATIONS=0` makes
simulation work unavailable and requests return HTTP 503; the server never
executes above that configured cap. The cap is server protection only — the
library has no limit.

## Release containers

The API and frontend have separate multi-stage release images. Both runtime
stages run as an unprivileged user, and the frontend bundle is built without an
API address compiled into it. The frontend's static server proxies same-origin
`/api` requests to `API_UPSTREAM` when it starts.

Run the local pair with host-facing ports bound to loopback:

```bash
docker compose -f compose.yaml build
docker compose -f compose.yaml up
curl -fsS http://127.0.0.1:3000/
open http://127.0.0.1:8080/
```

The API image can be built directly from the repository root. Buildx selects
the requested Linux architecture and tags the image locally:

```bash
docker buildx build --load --platform linux/arm64 \
  -f combat-api/Dockerfile -t ogame-combat-api:local-arm64 .
docker buildx build --load --platform linux/amd64 \
  -f combat-api/Dockerfile -t ogame-combat-api:local-amd64 .
docker buildx build --load --platform linux/arm64 \
  -f frontend/Dockerfile -t ogame-combat-frontend:local-arm64 frontend
docker buildx build --load --platform linux/amd64 \
  -f frontend/Dockerfile -t ogame-combat-frontend:local-amd64 frontend
```

On an arm64 Docker Desktop host, the arm64 build and run are native. The
amd64 commands may use emulation; record their build result separately from
native runtime performance. Inspect the selected architecture and immutable
image digest with `docker image inspect --format '{{.Id}} {{.Architecture}}'`
and `docker image inspect --format '{{json .RepoDigests}}'`.

The API accepts `MAX_CONCURRENT_SIMULATIONS` as a positive integer (default 1).
When all permits are in use, simulation requests return 503 immediately while
`GET /` remains available. The CPU-bound computation runs on an owned worker
thread, and its permit is released only after that computation returns, even if
the HTTP client disconnects. `SHUTDOWN_GRACE_SECONDS` (default 10) is the
finite SIGTERM drain budget. SIGTERM stops new admission and lets admitted
requests finish within the budget; work that exceeds it is interrupted when
the process exits because blocking CPU work cannot be cancelled by aborting an
HTTP future. A normal `docker compose up` reuses container state; use explicit
container/image removal commands when a destructive reset is intended.

The admission count covers API simulation workers, not the engine's Rayon
threads. `Simulator::new()` explicitly builds the process-wide Rayon pool with
`max(1, floor(num_cpus::get() * 3 / 4))`; `RAYON_NUM_THREADS` therefore does not
override this pool. To inspect the runtime constraint and total process thread
count, run a container with `--cpus=1`, exercise `POST /api/simulate`, and then
run `docker exec <container> sh -c 'grep -E "Cpus_allowed_list|Threads:" /proc/1/status'`
(the cgroup quota is also visible in `/sys/fs/cgroup/cpu.max`). In a release
smoke container started with `--cpus=1`, this reported `cpu.max: 100000 100000`,
`Cpus_allowed_list: 0-7`, and `Threads: 3` after a request. Those three threads
include the binary and runtime support; the explicit Rayon pool contributes one
worker under that quota. Raising
`MAX_CONCURRENT_SIMULATIONS` adds one owned API worker per admitted computation,
so it increases total threads independently of the Rayon pool.

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

## Import a report

Import one privately supplied combat or espionage report ID through the
community proxy with `combat-cli report --file /private/report-id.txt
--allow-proxy-transfer`. The result is a sanitized review candidate, not an
automatically completed simulation request. Missing data and uncertain modifier
semantics are flagged. IDs and raw responses are not saved by the importer.
See [report import](docs/report-import.md) for transfer/privacy details and the
opt-in live check. No private developer API key is required.

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
| `combat-cli` | Simulation, entities, corpus fixtures and report import |
| `combat-ogame-api` | Public per-universe XML metadata and on-demand community-proxy report import |

A web UI lives under `frontend/` (see its README): fleet entry with multi-slot
ACS, technology levels and planet resources. Rendering the results is still in
progress — see the
[open issues](https://github.com/maximalcode/ogame-combat-sim/issues) for the
remaining work. This README describes only what exists today.

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
