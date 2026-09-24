# Conditional comparison from reported combat stats

`combat-cli report compare-stats --file /private/path/input.json` runs an offline
comparison using explicitly supplied effective unit statistics. It is separate
from `report complete` and `report compare`: it does **not** create a verified
battle input or verify API imports, researched technology, classes, lifeforms,
or historical universe settings. No public HTTP endpoint accepts these inputs.

This is useful when a public report exposes starting fleets and combat stats
but does not explain how those stats were obtained. The existing combat-core
slot round loop consumes the supplied stats directly; no residual bonus is
reverse-engineered or called lifeform research. Each participant keeps its own
stats, even when several participants field the same entity type.

## Artifact

This is an invented, deterministic shield-bounce example, not a public report:

```json
{
  "battle": {
    "attackers": [{"units": {
      "204": {"count": 250, "weapon": 100, "shield": 100, "hull": 400}
    }}],
    "defenders": [{"units": {
      "408": {"count": 1, "weapon": 1, "shield": 10000, "hull": 10000}
    }}],
    "rapid_fire": false
  },
  "rapid_fire_basis": "assumed",
  "source": {"archive_report": null, "battle_timestamp": null},
  "observed": {
    "winner": "attacker",
    "rounds": 1,
    "attacker_remaining": [{"204": 250}],
    "defender_remaining": [{"408": 0}]
  }
}
```

- Weapon is effective damage per shot; shield and hull are effective hitpoints.
  `hull` is **not structural integrity**. If the source reports structural
  integrity, divide by ten and floor to the engine’s integer hull convention at
  the evidence boundary. If the source's units are
  ambiguous, do not run that report. The format rejects an `armour` field.
  Display rounding remains a limitation, not proof of exact starting stats.
- Counts are positive integers; zero-count types should be omitted from opening
  fleets. Shield must be finite and nonnegative, hull finite and positive.
  Unknown entity types, missing stats, and extra fields are rejected.
- Each side contains 1–255 ordered slots, named A1… / D1… in diagnostics.
  `*_remaining` contains pre-repair final counts in that same slot order,
  including explicit zeros for destroyed types. Use `null` if attribution is
  unknown; never reorder participants merely to fit a result. An incomplete or
  contradictory supplied final fleet is rejected.
- Winner is `attacker`, `defender`, `draw`, or `null`; rounds is 0–6 or `null`.
  Missing observations remain unassessable.
- Rapid fire is required explicitly. Its basis is `reported` or `assumed`.
  The latter produces a conditional comparison, never historical verification.
  The engine uses its built-in rapid-fire relationships. Neither choice proves
  version-specific behavior, special General perks, or report authenticity.
- Provenance is limited to a public archive report number and optional Unix
  battle timestamp in seconds. Null explicitly records missing context. Keep
  source URLs, universe/version evidence, and extraction notes in your separate
  local evidence ledger. Do not put player details or reusable keys here.
- Exact runs are capped at five million starting units across both sides and
  2 MiB of CLI input. Larger inputs fail explicitly; there is no downscaling.

## Results and limits

Both comparison modes use the same cumulative sampling and statistical helpers:
50 runs, 150 more if any metric remains uncertain, then 800 more if needed.
Earlier samples and stage assessments are retained. Suspicion alone does not
trigger extra sampling. Results include per-metric counts, empirical spread,
inclusive-tail probability, Wilson interval, the 5% rarity boundary, and status.
Multiple related metrics are diagnostic views, not a global correctness test.

The supported metrics are outcome, round count, per-slot per-entity losses,
aggregate side losses, and resource-value construction losses calculated from
those counts using the engine's stat table. Debris, loot, moon chance, repairs,
wreck fields, round firepower and shield absorption are explicitly unassessable
in this mode. Defence losses mean destruction during combat, before rebuilding.

Diagnostics retain the exact starting-stat artifact, rapid-fire assumption,
source context, model limitations, package version, and statistical method.
Keep the tested Git revision alongside local run results for reproducibility.
Random results are not bit-for-bit reproducible. A discrepancy is evidence to
investigate, not authorization to change combat mechanics.

Keep archive captures, extracted inputs and results outside the repository.
Public availability does not grant fixture-publication consent. Normal tests
are synthetic and offline; this command neither fetches reports nor publishes
fixtures. Redirect its private diagnostics to a local file rather than CI logs.
