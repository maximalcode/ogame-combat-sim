# CLAUDE.md

Working notes for this repository. Everything here was verified against the
code — if a claim below and the code disagree, the code is right and this file
needs fixing.

## What this is

An OGame v7 fleet-combat simulator. `combat-core` is the engine, `combat-types`
the shared data model, `combat-api` a small stateless axum server.

This repository was started fresh in August 2026. Its predecessor began life as
the combat engine for a private OGame-clone game, and roughly three quarters of
that repo's HTTP layer existed to serve that game's backend rather than any
public user. None of it came across: no auth middleware, no shared-secret API
key, no game-to-game endpoint, no report-key store, no Postgres. The engine
itself was carried over intact — it was never contaminated.

## Crate map

| Path | Contents |
| --- | --- |
| `combat-types/src/lib.rs` | `CombatRequest`, `PartyData`, `Technology`, `CombatResults`, `SimulationResult` |
| `combat-types/src/entities.rs` | `load_entity_stats()` — the full ship and defence stat table |
| `combat-types/src/combat_report.rs` | `CombatReport`, debris, moon chance, recycler maths |
| `combat-core/src/combat.rs` | One battle: rounds, shots, rapid fire, explosions. `MAX_ROUNDS = 6` |
| `combat-core/src/simulator.rs` | `Simulator::simulate_multiple` — runs N battles in parallel via rayon |
| `combat-core/src/scaling.rs` | Downscaling above `DOWNSCALE_THRESHOLD` (10M ships) |
| `combat-core/src/economics.rs` | Debris, loot, plunder |
| `combat-core/src/report_builder.rs` | `ReportBuilder::build_summary_report` |
| `combat-api/src/main.rs` | The whole server: two routes, no state |

## Running a simulation

```rust
let results = Simulator::new().simulate_multiple(&request);
let report = ReportBuilder::new().build_summary_report(&request, &results);
```

`CombatRequest` deserializes from exactly the JSON `POST /api/simulate`
accepts, so the easiest way to build one in a test or tool is
`serde_json::from_str`. It has no `Default` impl, so a struct literal must name
every field — prefer the JSON route.

`Simulator::new()` installs a **global** rayon thread pool (75% of cores). A
second call silently does nothing, but it is process-wide state.

## Commands

```bash
cargo test --workspace            # ~25s including compile
cargo run -p combat-api           # server on :3000
```

CI gate, worth running before pushing:

```bash
cargo fmt --check && cargo clippy --workspace --all-targets && cargo test --workspace
```

## Things that will surprise you

- **Tests are compiled optimised** via `[profile.test] opt-level = 3`. This is
  not a micro-optimisation: `downscaling_accuracy` simulates 20M ships without
  downscaling to prove the approximation holds, and it takes 135s at
  `opt-level = 0` versus 9.5s at 3. Do not remove that profile section.
- **Three request fields are inert.** `universe_settings`, `attacker_bonuses`
  and `defender_bonuses` round-trip through JSON but no engine code reads them.
  Tracked in the issues.
- **`/api/simulate` overrides the request.** It caps `simulations` at
  `MAX_SIMULATIONS` (default 1000) and forces `enable_downscaling = None`.
  Both are HTTP-layer server protection; the library has no limits.
- **Edition 2024 reserves `gen`.** That is why this uses rand 0.9
  (`random`, `random_range`, `from_os_rng`) rather than rand 0.8's `gen`.
- **`enable_round_compositions`** was called `enable_ogmem_metrics` in the old
  repo. "OGMem" was private jargon with no definition; the data — per-round,
  per-ship-type snapshots — is a real OGame report feature and was kept.

## Conventions

- Branches: work on `develop`, merge to `main` when reviewed.
- Toolchain pinned in CI; `-Dwarnings` is applied there rather than in the
  manifest, so local `cargo check` warns instead of failing.
- Planning lives in GitHub issues and milestones, not in checked-in documents.

## Provenance

Unofficial fan tool, not affiliated with Gameforge. Combat mechanics were
reimplemented from scratch after analysing TrashSim's behaviour; no TrashSim
code or assets are in this repository. Keep it that way — game mechanics are
not copyrightable, assets are. See the README disclaimer.
