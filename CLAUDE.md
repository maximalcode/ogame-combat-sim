# CLAUDE.md

Working notes for this repository. Everything here was verified against the
code — if a claim below and the code disagree, the code is right and this file
needs fixing.

## What this is

An OGame fleet-combat simulator. `combat-core` is the engine, `combat-types`
the shared data model, `combat-api` a small stateless axum server, `combat-cli`
a clap binary over the same library.

**On versions.** The old repo called this "OGame v7" everywhere. That label was
stale but the code mostly is not: combat resolution — rounds, rapid fire, the
bounce rule, shield regen, the explosion roll, `+10%`-per-level tech scaling,
the whole ship stat table — is unchanged from v7 through the current v13. What
is genuinely missing is deuterium debris (v9.2) and v13's instant-calc
short-circuit, both tracked in the issues. Player classes (v7), alliance
classes (v8) and lifeform research (v9) used to be on that list and are
modelled now — see `Technology::effective_levels` and
`combat-types/src/lifeforms.rs`. Do not reintroduce a version number into the
docs; state what is modelled instead.

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
| `combat-types/src/entities.rs` | `entity_stats()` / `load_entity_stats()` — the full ship and defence stat table |
| `combat-types/src/lifeforms.rs` | `LifeformBonuses` (what combat reads) and `LifeformTechTable` / `BuiltinLifeformTechs` (which research is worth what) |
| `combat-types/src/names.rs` | `ENTITY_INFO`, `resolve()`, `name_of()` — names and aliases, hand-written, kept in sync by test |
| `combat-types/src/combat_report.rs` | `CombatReport`, debris, moon chance, recycler maths |
| `combat-core/src/combat.rs` | One battle: rounds, shots, rapid fire, explosions. `MAX_ROUNDS = 6` |
| `combat-core/src/simulator.rs` | `Simulator::simulate_multiple` — runs N battles in parallel via rayon |
| `combat-core/src/scaling.rs` | Downscaling above `DOWNSCALE_THRESHOLD` (10M ships) |
| `combat-core/src/economics.rs` | Debris, loot, plunder |
| `combat-core/src/report_builder.rs` | `ReportBuilder::build_summary_report` |
| `combat-api/src/main.rs` | The whole server: two routes, no state |
| `combat-cli/src/cli.rs` | clap definitions, `build_request`, `parse_request_json`, `validate` |
| `combat-cli/src/args.rs` | `parse_fleet` / `parse_tech` / `parse_resources` — the shorthand parsers |
| `combat-cli/src/render.rs` | Human-readable output. Returns `String`, never prints |

## Running a simulation

```rust
let results = Simulator::new().simulate_multiple(&request);
let report = ReportBuilder::new().build_summary_report(&request, &results);
```

`CombatRequest` deserializes from exactly the JSON `POST /api/simulate`
accepts, so `serde_json::from_str` is the easiest way to build one from a
literal. In Rust code, `CombatRequest { attacker, defender, ..Default::default() }`
works too — the `Default` impl is hand-written so every field matches what
serde would fill in, and a test asserts that rather than trusting it.

`Simulator::new()` installs a **global** rayon thread pool (75% of cores). A
second call silently does nothing, but it is process-wide state.

## Commands

```bash
cargo test --workspace            # ~25s including compile
cargo run -p combat-api           # server on :3000
cargo run -p combat-cli -- sim -a "cruiser:100" -d "lf:1000" --tech 10
cargo run -p combat-cli -- entities
cargo bench --bench engine        # criterion; first compile is slow, see below
```

CI gate, worth running before pushing:

```bash
cargo fmt --check \
  && cargo clippy --workspace --all-targets \
  && cargo deny check advisories bans licenses \
  && cargo test --workspace
```

`cargo deny` is not in the toolchain — `cargo install cargo-deny` (CI pins
0.20.2). Skipping it locally only defers the failure to CI.

## Things that will surprise you

- **`[profile.bench]` mirrors `[profile.release]`**, `lto = "fat"` and
  `codegen-units = 1` included, so `cargo bench` recompiles the world the first
  time and measures the binary that ships. It keeps symbols where release
  strips them, because a stripped benchmark profiles as hex addresses. The
  suite lives in `combat-core/benches/engine.rs` and every case runs the same
  fixed `SIMULATIONS` count — change that constant and old numbers stop being
  comparable.
- **Tests are compiled optimised** via `[profile.test] opt-level = 3`. This is
  not a micro-optimisation: `downscaling_accuracy` simulates 20M ships without
  downscaling to prove the approximation holds, and it takes 135s at
  `opt-level = 0` versus 9.5s at 3. Do not remove that profile section.
- **`universe_settings` is inert**, and so is `PlayerBonuses::has_engineer`.
  The settings block round-trips through JSON and no engine code reads it.
  `has_engineer` does the same, deliberately: the Engineer's combat effect is on
  the post-battle defence *rebuild* roll and this engine has no rebuild step to
  attach it to. Both are tracked in the issues; neither is an oversight to be
  fixed by inventing a number. `PlayerBonuses::lifeform_bonus` used to be a
  third — it is gone, widened into `PartyData::lifeform`, because one flat
  percentage cannot describe a per-ship-type bonus and reinterpreting it as a
  global multiplier would have been wrong for every mixed fleet.
- **Class bonuses are levels, resolved before combat starts.** A General is
  worth +2 Weapons/Shielding/Armour and a Warrior alliance +1, they add, and +3
  is the ceiling. `Technology::effective_levels` folds a side's
  `PlayerBonuses` into its `Technology`, and everything that needs the result
  goes through `CombatRequest::effective_attacker` / `effective_defender` —
  there are two consumers, the simulator (which fights the battle) and the
  report builder (which states what it was fought at), and pairing a side with
  the *other* side's bonuses is a bug neither one's tests would catch. Below
  that seam the engine, the stat cache and downscaling all go on seeing a plain
  `PartyData` and cannot tell a researched level from a granted one. That is
  the whole design: **do not** add a class multiplier inside
  `ModifiedStats::calculate`, because `+10%` per level is applied once to the
  total, and a General with Weapons 10 must compute as Weapons 12 rather than
  as Weapons 10 times something.
- **Lifeform bonuses are percentages, and they *are* in `ModifiedStats::
  calculate`.** They are the other kind of stat modifier and take the other
  seam, because a lifeform research names one ship type and cannot be expressed
  as levels that apply to a whole fleet. So `PartyData` carries a
  `LifeformBonuses` beside its `Technology`, `StatsCache::new` takes the whole
  party rather than its levels, and the two terms are added inside one
  expression: `base * (1 + 0.10*level + lifeform)`. Additive is the game's rule
  and is the point — a Destroyer at Weapons 25 with `+50%` fires at 8000, not
  10,500, and computing it as two multiplications gets that wrong. `cargo` and
  `speed` are in `LifeformBonus` and read by nothing: every lifeform research
  moves all five stats at one rate, so carrying them costs nothing and leaving
  them out would make the flight model a schema change rather than a reader.
- **The lifeform table is hardcoded behind a trait, on purpose.**
  `BuiltinLifeformTechs` is the only implementation of `LifeformTechTable`
  today. Gameforge publishes the whole per-universe configuration in
  `serverData.xml`'s `<lifeformSettings>`, so a loader for it is the intended
  second implementation — add a source, do not rewrite the seam. Three
  researches named like combat techs (`12217` Rune Shields, `13217`
  Experimental Weapons Technology, `14217` Psionic Shield Matrix) reduce
  research cost and time and grant **zero** combat power; a test asserts they
  stay out of the table.
- **A report says nothing about lifeforms.** `Participant` carries effective
  technology levels and no per-ship-type bonuses, so a battle decided by
  lifeform research reads in the report as though it were not. There is nowhere
  in the report shape to put a per-ship table, and inventing one was out of
  scope; the CLI has no flag for lifeforms either, so the JSON body is the only
  way in today.
- **A report states effective levels, not researched ones.** `Participant.
  technology` in a `CombatReport` — and so the CLI header and the `report` half
  of the `/api/simulate` response — is what the side actually fought at. A
  General who researched 10 is reported as 12. Showing the researched figure
  beside a battle resolved at the higher one reads as a bug, and there is
  nowhere in the report to show both.
- **Two OGame mechanics are known and deliberately not implemented**, because
  neither could be sourced to a number worth committing to. The Light Fighter's
  chance to destroy a Deathstar outright (a General perk): sources split
  between 1-in-1000 and 1-in-10000 with nothing official either way. The
  Engineer officer: see the inert-fields bullet above. Guessing either would
  put a fabricated constant into a simulator whose whole selling point is
  stating what it gets wrong.
- **`/api/simulate` overrides the request.** It caps `simulations` at
  `MAX_SIMULATIONS` (default 1000) and forces `enable_downscaling = None`.
  Both are HTTP-layer server protection; the library has no limits. **The CLI
  deliberately does not copy either one** — a local binary spending its own CPU
  has no shared resource to protect, and `simulations_are_not_capped_at_the_api_limit`
  in `combat-cli/src/cli.rs` is there to stop someone "fixing" the discrepancy.
- **clap renders `///` into `--help`.** A doc comment on a field in
  `combat-cli/src/cli.rs` is user-facing text, not a note to the next reader.
  Internal rationale goes in a `//` comment above it.
- **`combat-core/tests/common/mod.rs` is the shared battle fixture.** 250 Light
  Fighters against one Large Shield Dome, decided entirely by whether a shot
  clears 1% of a 10,000 shield — which makes it a single-number probe for any
  stat modifier, and free of randomness so a run of five simulations all agree.
  `class_bonuses.rs` and `lifeform_bonuses.rs` both use it and each keeps its
  own `resolve`. A third mechanism that modifies stats should join them rather
  than copy the fixture again.
- **The entity name table is hand-written.** `EntityStats` has no name field, so
  `combat-types/src/names.rs` carries names and aliases separately. Two tests
  assert the two tables cover exactly the same ids in both directions; add a
  ship to one and the other fails.
- **`build_summary_report` hardcodes `round_details: None`.** It averages a run,
  and an average has no per-round narrative. `combat-cli --rounds` therefore
  reads `results.results[0]` — one battle — and its header says so.
- **Edition 2024 reserves `gen`.** That is why this uses rand 0.9
  (`random`, `random_range`, `from_os_rng`) rather than rand 0.8's `gen`.
- **The clippy config has two halves.** Everything above the
  `--- repo-specific ---` line in `Cargo.toml` is copied from maxi-quality and
  should be regenerated with its `adopt.sh`, not hand-edited. Below it is this
  repo's own policy — currently the four cast lints, allowed because ship
  counts are integers and combat maths is floating point, so the whole engine
  crosses between them. Anything narrower than a whole-repo policy belongs at
  the site as `#[allow(...)]` with a written reason. There are ten: four in
  `combat-core/src`, and `too_many_lines` on six scenario tests.
- **`enable_round_compositions`** was called `enable_ogmem_metrics` in the old
  repo. "OGMem" was private jargon with no definition; the data — per-round,
  per-ship-type snapshots — is a real OGame report feature and was kept.

## Conventions

- Branches: work on `develop`, merge to `main` when reviewed.
- Toolchain pinned in CI; `-Dwarnings` is applied there rather than in the
  manifest, so local `cargo check` warns instead of failing.
- Planning lives in GitHub issues and milestones, not in checked-in documents.

## Agent skills

### Issue tracker

GitHub issues on `maximalcode/ogame-combat-sim`, via the `gh` CLI. See
`docs/agents/issue-tracker.md`.

### Triage labels

The five canonical roles, each label string equal to its name. See
`docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` and `docs/adr/` at the repo root. See
`docs/agents/domain.md`.

## Provenance

Unofficial fan tool, not affiliated with Gameforge. Combat mechanics were
reimplemented from scratch after analysing TrashSim's behaviour; no TrashSim
code or assets are in this repository. Keep it that way — game mechanics are
not copyrightable, assets are. See the README disclaimer.
