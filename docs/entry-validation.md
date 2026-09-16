# Combined entry acceptance and combat evidence

Issue #76 keeps three different claims separate: the attack-planning journey
works, a comparison with an observed battle is assessable, and a particular
combat mechanic is supported. None implies the next.

## Reproducible user journey

`frontend/tests/entry.spec.ts` opens `/de` with an AGR base64 `prefill` and
`SR_KEY` together. It checks the available fleet, edits the selected quantity,
explicitly transfers the report, verifies the opponent and retained selection,
changes universe rules, explicitly simulates twice, compares the two successful
attempts, and restores the original available fleet. Imports and edits must
send no simulation requests. The report's unknown shielding and ambiguous
booster remain unknown/Assumed zero, never verified evidence.

Additional cases cover either fragment parameter order, raw base64 containing
`+`, report-before-AGR, and delayed reports during own selection changes,
new defender handoffs and manual defender edits. Universe settings remain the
planner's explicit values; the report disclosure asks the user to reconcile
those with its universe. A report cannot silently replace those settings.
The existing `agr.spec.ts`, `report.spec.ts` and `planner.spec.ts` retain malformed
input, replacement-import races, failures between successes, stale simulation
responses, resource rounding, keyboard and image-failure coverage.

All browser fixtures are invented from the documented AGR and sanitized
report contracts. The zero-filled report token is synthetic and every successful
report fetch in the browser suite is intercepted. No real report, player name,
coordinate or reusable key is stored. Synthetic numerical responses test UI
arithmetic and state transitions; they are not observations of combat accuracy.

Run from `frontend/`:

```sh
npm ci
npx playwright install chromium
npm run lint
npm run build
npm test
```

CI's `frontend-behavior` job runs the deterministic browser suite, lint and
build (including test typechecking). Rust's existing router test,
`reports::tests::router_uses_real_client_and_sanitizer_without_simulating`,
exercises the real import adapter, HTTP client and sanitizer with a local
provider stub, including visibility precedence and redacted failures.

For the opt-in local browser/API smoke, start `cargo run -p combat-api` at the
repository root, then run from `frontend/`:

```sh
LIVE_API=1 npm test -- --grep 'real local API smoke'
```

The combined smoke uses the same full journey with **real simulation requests**
through Vite to the Rust API and engine. Report transport is still intercepted;
this proves the productive browser/simulation connection, not live proxy
availability. The separate Rust adapter test covers that report connection
against a controlled provider. No live service is a CI dependency. For an
explicitly consented private live-proxy check, use the existing procedure in
[report-import.md](report-import.md); successful retrieval still grants no
publication permission. `PLAYWRIGHT_CHROME` can select an installed Chromium
when the bundled browser download is unavailable.

## Observed-battle comparison gate

Reviewed #17 and #63–#68 against the integrated code. Reuse
`combat-ogame-api::reports::complete_candidate` / `combat-cli report complete` and the
universe resolver for independently supplied completion evidence. They preserve
unknowns and return field issues instead of a request when required evidence is
missing or contradictory. Existing completion tests cover missing lifeform
percentages, ambiguous technology/class bases, starting-stat mismatches,
universe identity and temporal provenance, and the unchanged publication-consent
boundary. The Dock's exploratory scenario is not accepted as proof of those
inputs. Editing until a result matches would invalidate the comparison.

The #66 empirical-distribution comparison is **not implemented in this base**.
#63/#67/#68 are not completed by this acceptance work. Until a verified input,
separate observation and supported comparison path all exist, an observed
battle is **not assessable**, not an accuracy pass. Do not build another
statistical engine in the browser or reinterpret the attempt-comparison table
as an observed-battle comparison.

The existing `combat-fixtures` path remains available for approved public cases:

```sh
cargo run -p combat-cli -- fixture check combat-core/tests/fixtures
cargo run -p combat-cli -- fixture run combat-core/tests/fixtures
```

Its envelope requires publication consent for observed battles and a written
per-fixture tolerance justification; `blocked_on` is an explicit skip. The
shipped case is a deterministic **synthetic self-consistency placeholder** with
zero tolerance. Its pass is not live-server evidence. Do not convert an
incomplete import into a fixture by accepting serde defaults as verification.

Before an observed comparison, record the metric, unit, semantic match and
assessment rule independently of its result:

| Metric | Unit / evidence needed | Assessment boundary |
| --- | --- | --- |
| Outcome | Categorical attacker/defender/draw observation | #66 occurrence probability, not a numeric loss |
| Rounds and per-type losses | Round count / destroyed unit count with reliable attribution | #66 inclusive empirical two-sided tail of individual samples |
| Resource losses | Metal, crystal, deuterium resource units; not a ship count | Same tail, matching pre-/post-rebuild semantics |
| Generated debris | Resource units per resource, never harvested or remaining debris | Same tail, independently verified universe debris rules |
| Loot, flight, rebuild, wreck fields | Matching inputs and implemented phase required | Otherwise not assessable; no invented zero |

The pre-agreed #66 rule uses a 95% Wilson interval around occurrence/tail
probability and a 5% diagnostic rarity boundary, retaining cumulative samples
at 50 → 200 → 1,000 only while an assessable metric remains statistically
uncertain. These are future comparison acceptance rules, **not measurements
produced here**. They account for simulation spread and finite samples; they do
not produce a global correctness pass or multiple-testing-adjusted proof.
Existing fixture mean tolerances must be independently justified for their
battle before evaluation; they are not a substitute for this distribution test.
Never adjust inputs, rerun selectively or widen tolerances to obtain a pass.

## Evidence inventory (2026-09-16)

| Evidence | Available coverage | Remaining gap |
| --- | --- | --- |
| Synthetic browser + HTTP cases | Combined imports, selection, two attempts, races, unknowns, explicit starts | No observed combat accuracy claim |
| Existing completion / universe tests | Strict evidence gate and modifier arithmetic, synthetic offline cases | No new independently verified real battle |
| Shipped regression corpus | One deterministic engine self-consistency case | Zero consented observed fixtures |
| Public OGMem inspection outside CI | Two reports inspected for suitability; no content retained | No publication consent; insufficient independent modifiers / historical settings |

On 2026-09-16, inspected the public pages
[1585453](https://ogmem.com/show/1585453) and
[1584343](https://ogmem.com/show/1584343) without downloading captures or following
private report capabilities. One exposes starting statistics and rounds, but
these do not independently establish class/lifeform decomposition and historical
universe rules. The other lacks starting modifiers and includes defence repair,
which is an unsupported post-battle phase here. Both are **not assessable** in
this acceptance run; neither is a discrepancy or accuracy pass. Public access is
not uploader permission, so no report-derived fixture, identity or battle values
are copied into tests or this document. Suitable consented evidence remains a
visible gap, not a prerequisite to invent or publish a new corpus.

Remaining mechanics include defence rebuild/Engineer, wreck fields, flight/fuel,
and the unverified General Light Fighter/Deathstar perk. Affected observations
must stay omitted or skipped with their reason. A working user journey does not
close these mechanics or the existing comparison issues.

## Local acceptance run

On 2026-09-16, at code/test revision `91b3135` (subsequent acceptance-record
edits are documentation only):

- `npm ci`, `npm run lint` and `npm run build`: passed; build includes TypeScript
  checking for application and browser tests.
- `LIVE_API=1 npm test -- --workers=2`: **31 passed, zero skipped**. Used a
  dedicated Vite port and installed Chrome for Testing 151.0.7922.34 through
  `PLAYWRIGHT_CHROME`, because the bundled browser download timed out. The API
  was built from this checkout with `cargo run --profile test -p combat-api`.
- Visually inspected the combined real-API journey at 1280px: both sides,
  unknown-field labels, universe rules, two attempt snapshots and comparison
  remained readable. Existing 1024px/1440px keyboard/image-failure checks passed.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo deny check advisories bans licenses` and `cargo test --workspace --locked`:
  passed, including the existing strict completion, universe, consent and local
  report-provider adapter tests.
- Using the CLI binary built by that workspace test run,
  `target/debug/combat-cli fixture check combat-core/tests/fixtures` and
  `target/debug/combat-cli fixture run combat-core/tests/fixtures`: one valid,
  one matched **synthetic** fixture. Zero observed battles assessed.

This accepts the functional journey. Accuracy evidence and mechanics gaps
remain as listed above; no live proxy availability or observed-battle accuracy
result is claimed.
