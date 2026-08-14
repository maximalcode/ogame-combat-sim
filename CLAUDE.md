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
the whole ship stat table — is unchanged from v7 through the current v13. v13's
own change to the combat entry path is modelled: there is no probe-only
auto-loss and there never was one here, and the instant-calculation
short-circuit lives in `combat-core/src/instant.rs` — narrower than the
changelog states it, deliberately, see the bullet below.
Everything else that fed *stat modifiers* in has since been modelled: player
classes (v7), alliance classes (v8) and lifeform research (v9) — see
`Technology::effective_levels` and `combat-types/src/lifeforms.rs` — and
per-universe debris rules, deuterium debris (v9.2) included, see
`CombatRequest::debris_settings`. Do not reintroduce a version number into the
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
| `combat-types/src/lib.rs` | `CombatRequest`, `PartyData`, `Technology`, `CombatResults`, `SimulationResult`, `UniverseSettings`, `DebrisSettings` |
| `combat-types/src/entities.rs` | `entity_stats()` / `load_entity_stats()` — the full ship and defence stat table |
| `combat-types/src/lifeforms.rs` | `LifeformBonuses` (what combat reads) and `LifeformTechTable` / `BuiltinLifeformTechs` (which research is worth what) |
| `combat-types/src/names.rs` | `ENTITY_INFO`, `resolve()`, `name_of()` — names and aliases, hand-written, kept in sync by test |
| `combat-types/src/combat_report.rs` | `CombatReport`, debris, moon chance, recycler maths |
| `combat-core/src/combat.rs` | One battle: rounds, shots, rapid fire, explosions. `MAX_ROUNDS = 6` |
| `combat-core/src/instant.rs` | v13's instant calculation: combined attack power, and when a battle may be decided without rounds |
| `combat-core/src/simulator.rs` | `Simulator::simulate_multiple` — runs N battles in parallel via rayon |
| `combat-core/src/scaling.rs` | Downscaling above `DOWNSCALE_THRESHOLD` (10M ships) |
| `combat-core/src/economics.rs` | Debris, loot, plunder |
| `combat-core/src/report_builder.rs` | `ReportBuilder::build_summary_report` |
| `combat-fixtures/src/lib.rs` | The regression corpus fixture format: `Fixture`, validation, `Evaluation`, `run_fixture`, `ignored_request_fields` |
| `combat-api/src/main.rs` | The whole server: two routes, no state |
| `combat-cli/src/cli.rs` | clap definitions, `build_request`, `parse_request_json`, `validate` |
| `combat-cli/src/args.rs` | `parse_fleet` / `parse_tech` / `parse_resources` — the shorthand parsers |
| `combat-cli/src/render.rs` | Human-readable output. Returns `String`, never prints |
| `combat-ogame-api/src/lib.rs` | Public OGame XML client and offline parsers; disk cache, per-host rate limit and `serverData.xml` lifeform table source |

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
cargo run -p combat-cli -- fixture template     # skeleton for a new corpus fixture
cargo run -p combat-cli -- fixture check DIR    # the checks CI applies, before CI
cargo run -p combat-cli -- fixture run DIR      # ...plus observed vs simulated
cargo bench --bench engine        # criterion; first compile is slow, see below
```

The web UI is a separate npm project under `frontend/`:

```bash
cd frontend && npm install
npm run dev                       # :5173, proxies /api to :3000
npm run lint                      # the TypeScript CI gate — see below
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
- **Some fields still round-trip and are read by nothing.**
  `PlayerBonuses::has_engineer` is one, deliberately: the Engineer's combat
  effect is on the post-battle defence *rebuild* roll and this engine has no
  rebuild step to attach it to. `universe_settings` is half of another — its
  debris fields are read now, but `galaxies`, `systems`, the two donut flags,
  `fleet_speed` and `deuterium_save_factor` are inert because nothing here
  computes flight or fuel yet, which is issue #14. Both are tracked; neither is
  an oversight to be fixed by inventing a number. `PlayerBonuses::lifeform_bonus`
  used to be a third — it is gone, widened into `PartyData::lifeform`, because
  one flat percentage cannot describe a per-ship-type bonus and reinterpreting it
  as a global multiplier would have been wrong for every mixed fleet.
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
  `BuiltinLifeformTechs` is the offline implementation of `LifeformTechTable`;
  `combat-ogame-api::ServerDataLifeformTechs` is the per-universe source backed
  by `serverData.xml`'s `<lifeformSettings>`. Keep adding sources behind that
  seam rather than rewriting it. Three
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
- **The instant calculation fires on less than the changelog says, and that is
  the feature.** v13 resolves a battle without rounds when one side has more
  than 10,000 times the other's combined attack power. `combat-core/src/
  instant.rs` implements that ratio — one definition of attack power for both
  sides, every unit's *effective* weapon damage summed, read off the `Entity`
  values the round loop shoots with so technology, class levels and lifeform
  percentages are all in it, and **defences count towards a defender's**, they
  shoot. But the ratio alone contradicts this engine's own rounds: at Weapons
  9, 250 Light Fighters are worth 23,750 attack power against a Large Shield
  Dome's 1 — 2.4 times over the 10,000 threshold — and still cannot scratch
  it. That is `tests/common/mod.rs`, the shared fixture, and a short-circuit
  on the ratio alone would report the dome destroyed. So three further
  conditions, each read off `apply_damage_fast` rather than invented, have to
  hold as well: the loser's shots must bounce off
  the winner entirely, the winner's must register on everything the loser has,
  and the winner's firepower must clear the loser's total hitpoints by the same
  10,000× margin — the changelog's own number, reused rather than a second
  constant invented for it. Anything else is simulated, which is slower and
  right. **The short-circuit is only ever allowed to be an optimisation**, so
  `Combat::simulate_single_through_the_rounds` exists purely so
  `tests/instant_calculation.rs` can fight the same battle both ways and compare
  — including over fleets nobody chose. It resolves by emptying the losing
  party and letting the ordinary code build the result, which is why `rounds` is
  an honest 0, `round_details` is an empty list, and debris, loot, profit and
  the per-slot breakdown all come out in the usual shape. Slot mode takes the
  same rule, because attack power is a property of a side and a slot is only how
  that side is reported.
- **Two OGame mechanics are known and deliberately not implemented, for two
  different reasons.** Do not collapse them into one. The Light Fighter's
  chance to destroy a Deathstar outright (a General perk) has no number worth
  committing to: sources split between 1-in-1000 and 1-in-10000 with nothing
  official either way, and guessing would put a fabricated constant into a
  simulator whose whole selling point is stating what it gets wrong. The
  Engineer officer is the other case — not an unsourced number but nowhere to
  put it. Its effect is on the post-battle defence rebuild roll and this engine
  has no rebuild step at all, so `has_engineer` waits rather than being turned
  into a stat bonus it is not; see the inert-fields bullet above. The 70% /
  85% figures quoted at `combat-types/src/lib.rs:195` are folklore until
  checked — issue #41 confirms them against a live source and adds the phase.
- **Debris rules come from two places and one wins.** A request can set
  `debris_percentage` at the top level *and* describe debris inside
  `universe_settings`. `CombatRequest::debris_settings` settles it:
  **`universe_settings` wins whenever it is present**, and the top-level field
  is the fallback for requests without one. The fallback reports fleet debris
  only — no defence debris, no deuterium — which is exactly what the engine did
  before any of this was read, so a request with `universe_settings: null`
  produces the wreck field it always did. Two tests in `combat-types/src/lib.rs`
  pin both halves of the rule; do not "simplify" it to one source. The sharp
  edge: `UniverseSettings` defaults every field, so a block that sets only
  `galaxies` still wins, and its `debris_fleet` default of 30 quietly overrides
  a `debris_percentage` of 70. `CombatResults::debris_settings` reports what was
  actually used, which is the fastest way to see it happen.
- **`DebrisField::total()` counts deuterium**, and that total feeds moon chance,
  the recycler count and both profit figures in
  `combat-types/src/combat_report.rs`. So enabling deuterium debris on a
  universe moves the moon roll and the harvest estimate too. That is correct —
  recyclers do collect it — but it means a change to the debris maths is never
  only a change to the debris maths.
- **The two profit figures are alternatives, not addends.** `attacker_profit`
  assumes the attacker harvests the entire debris field; `defender_profit`
  assumes the defender does. Summing them double-counts the field. Because both
  use `DebrisField::total()`, enabling defence debris or deuterium debris moves
  both figures at once.
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
- **The regression corpus is data, not code.** Every `.json` under
  `combat-core/tests/fixtures/` is discovered, simulated and compared by
  `combat-core/tests/regression_corpus.rs`, so adding a battle is adding a file
  — the format is documented in that directory's `README.md`. Three things
  about it are load-bearing. **Skips and the pass/skip tally bypass
  `eprintln!`**, going to the raw `std::io::stderr()` handle through
  `write_past_capture`, because libtest swallows a *passing* test's output and
  a corpus that skipped every fixture would otherwise read as a green run;
  `skip_records_remain_visible_under_libtest_capture` pins that by re-running
  itself in a child process. **The envelope is `deny_unknown_fields` and the
  `request` inside it is not** — it is a plain `CombatRequest`, deliberately,
  so that a fixture doubles as an `/api/simulate` body. That would leave a
  misspelled request field to take its default and quietly change the battle,
  so `ignored_request_fields` closes it from outside serde: parse the request,
  serialize it back, and report any key that did not survive — descending into
  slot arrays, because `PartySlot` is not `deny_unknown_fields` either. The
  carve-out is `FIELDS_SKIPPED_ON_OUTPUT`, a **list of names** rather than a
  test on the value: `"universe_setings": null` is a typo and looks exactly
  like a skipped `Option`, so excusing every null would excuse it too.
  `fields_serde_skips_on_output_are_not_mistaken_for_typos` fails if a new
  `skip_serializing_if` arrives without being added to that list. A mistyped
  **entity id** is the same defect — fleets are maps, so `"2014": 30` is
  well-formed and simply never fights — and `unknown_entity_errors` checks
  every id against `names::name_of`. And **tolerances are per
  fixture with a written justification**, never a global constant, because
  variance scales with fleet size. The one fixture there now is a labelled
  synthetic placeholder and says so in its `name`, `source` and
  `observed_battle: false`; real reports arrive via issue #17 and are rejected
  without `publication_consent`.
- **The fixture format is a crate, not a test module, and that is the point.**
  `combat-fixtures` holds the envelope, its validation and the comparison;
  `combat-core/tests/regression_corpus.rs` is only the part specific to running
  the corpus under libtest, and `combat-cli fixture template|check|run` offers a
  contributor the same checks before they open a pull request. A fixture that
  passes `check` locally and fails in CI is the one failure the arrangement
  exists to prevent, so **do not reimplement a rule in either caller** —
  including the *order* of validate, skip, compare, which is why both go
  through `evaluate_fixture` and `run_fixture` is only that reduced to
  pass/skip/fail. `the_shipped_corpus_passes_the_checks_the_cli_applies` in
  `combat-cli/src/fixture.rs` is the assertion that they agree. Note the CLI
  counts a skip apart from a pass and says so in its summary, for the same
  reason the corpus test writes skip records past libtest's capture: a fixture
  that was never compared has not agreed with anything. The crate
  deliberately does **not** depend on `combat-core`: `run_fixture` takes a
  closure that produces `CombatResults`, so validating a fixture never installs
  the process-wide rayon pool.
- **A combat report cannot be imported automatically, and it is not for want of
  code.** The `cr-en-1-<hash>` string a player can copy is an id, not data, and
  resolving it goes through `api/v1/combat/report?api_key=…` with a developer
  key Gameforge issues on application and requires be kept private.
  `combat-ogame-api` touches only the public XML endpoints. So the corpus is
  filled by hand, which is why the authoring tooling exists and why issue #17 is
  labelled `ready-for-human`.
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
- **`frontend/` is the other language, and it has the same two halves.**
  `frontend/eslint.base.mjs` and `frontend/tsconfig.base.json` are maxi-quality
  copies written by its `adopt.sh` — regenerate them, do not hand-edit. This
  repo's own choices live in `eslint.config.mjs` and `tsconfig.json`, which
  extend them; `tsconfig.json` overrides only what a browser app must differ on
  (DOM lib, bundler resolution, JSX, no emit) and relaxes none of the strict
  family. The CI gate is exactly `npm ci && npm run lint` run inside
  `frontend/` — no build, no typecheck, no tests — so **`lint` is the whole
  TypeScript gate**, and `--max-warnings 0` is what gives `no-console` teeth.
  `frontend/` is not a Cargo workspace member; `cargo test --workspace` never
  sees it.
- **The OGame XML cache has two clocks.** `combat-ogame-api` accepts a cached
  response only while both its file age and the root `timestamp` are within the
  endpoint's own cadence (hourly for highscores, daily for players and server
  data, weekly for universe and player data). Fetching stays out of every
  engine path, and the one-request-per-second limiter is process-wide per host.
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
