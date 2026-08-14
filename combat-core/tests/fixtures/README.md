# Real-world regression fixtures

Every `.json` file in this directory tree is discovered by
`regression_corpus.rs`, parsed, simulated, and compared with its recorded OGame
result. A fixture is deliberately also a reproduction case: its `request` is a
`CombatRequest` in exactly the JSON shape accepted by `POST /api/simulate`.

The corpus is for observed battles. Do not invent values that look observed:
that only proves the engine agrees with itself. The sole fixture currently in
this directory is explicitly marked as a synthetic self-consistency placeholder
so the format and runner can be exercised before issue #17 supplies real data.

## Format (schema version 1)

```json
{
  "schema_version": 1,
  "name": "short, unique description",
  "provenance": {
    "observed_battle": true,
    "source": "where the report came from, without private player data",
    "universe": "universe name or identifier",
    "approximate_date": "YYYY-MM or another honest approximation",
    "game_version": "version shown or known at the time",
    "publication_consent": true
  },
  "blocked_on": null,
  "request": { "CombatRequest": "exactly as the API accepts it" },
  "observed": {
    "outcome": "AttackersWin | DefendersWin | Draw",
    "attacker_losses": { "entity-id": 0 },
    "defender_losses": { "entity-id": 0 },
    "debris": { "metal": 0, "crystal": 0, "deuterium": 0 }
  },
  "tolerance": {
    "minimum_observed_outcome_rate": 0.8,
    "losses": { "absolute": 2.0, "relative": 0.05 },
    "debris": { "absolute": 1000.0, "relative": 0.05 },
    "justification": "why this many simulations and these margins are suitable"
  }
}
```

All fixture-envelope fields shown are required except `blocked_on`, which may
be omitted or `null`. Entity IDs are JSON object keys and therefore strings,
just as they are in an API request.

The embedded `request` deliberately keeps `CombatRequest`'s API-compatible
deserialization behavior, so a fixture doubles as a `POST /api/simulate` body:
serde ignores unknown keys inside it rather than rejecting them. A misspelled
request field would therefore fall back to a default and silently change the
battle. Validation closes that hole separately, by parsing the request,
serializing it back, and reporting any key that did not survive the round trip
— so `weapons` for `weapon` fails with its full path instead of quietly
fighting at level zero.

## Writing one

```bash
cargo run -p combat-cli -- fixture template > combat-core/tests/fixtures/my-battle.json
cargo run -p combat-cli -- fixture check combat-core/tests/fixtures/my-battle.json
cargo run -p combat-cli -- fixture run combat-core/tests/fixtures/my-battle.json
```

`template` prints this format with every field a human must supply marked
`FILL IN`; validation rejects that marker, so a half-edited template cannot
reach review claiming a battle nobody observed. `check` applies exactly the
rules the corpus test applies — they are one implementation, in
`combat-fixtures` — so a fixture that passes here passes in CI. `run` also
simulates it and prints observed against simulated, with the range each metric
covered across individual battles. That range is the evidence a `justification`
is supposed to rest on; write it from what `run` prints rather than from a
guess.

`observed_battle` must be `true` for a real report. A real report is rejected
unless `publication_consent` is also `true`. In particular, never add another
player's report without their permission. A synthetic format example must set
both fields to `false`, say that it is synthetic in `source` and `name`, and
must not contain data presented as a live observation.

## Tolerances and failures

The request's `simulations` controls the sample size. The expected outcome
passes when it occurs in at least `minimum_observed_outcome_rate` of those
runs. Losses are averaged per entity type; debris is averaged per resource.
Each numeric comparison allows:

```text
absolute + abs(observed value) * relative
```

Both numeric tolerance values must be finite and non-negative; `relative` and
the outcome rate are fractions from `0.0` to `1.0`. Tolerances live in each
fixture because fleet size and variance differ by battle. The written
`justification` is mandatory. When a comparison fails, the harness reports the
fixture, metric, observed and simulated values, allowed difference, and the
amount by which the result exceeded its tolerance.

## Missing engine features

Keep a valuable report even when the engine cannot model it yet. Set:

```json
"blocked_on": {
  "feature": "stable feature or issue name",
  "reason": "why this feature materially changes this battle"
}
```

The harness validates the fixture, skips its simulation, and writes the feature
and reason as an explicit `SKIP` line that remains visible under an ordinary
`cargo test` run. Remove `blocked_on` when the feature is implemented; do not
delete the fixture or disguise the mismatch with a wider tolerance.
