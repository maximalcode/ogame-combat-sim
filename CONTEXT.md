# OGame Combat Simulator Context

The domain language for fleet combat, combat evidence, and the future flight and
missile surfaces. This glossary describes the concepts callers and reports
exchange; it is not an implementation specification.

## Combat and evidence

**Combat report**:
A record of one battle's outcome, losses, debris, and derived results.
_Avoid_: Result blob, battle output

**Exact fleet**:
A fleet whose ship composition comes directly from a user-provided combat or
espionage report. Stored fixtures retain combat evidence rather than player
names, coordinates, or reusable report keys.
_Avoid_: Known fleet when the provenance is only an estimate

**Estimated fleet**:
A fleet composition inferred from public information and presented with its
uncertainty and provenance.
_Avoid_: Exact fleet, observed fleet; estimates are outside the current scope

**Provenance**:
The source and context that explain where a fleet or combat observation came
from, such as its universe, date, version, and report origin.
_Avoid_: Metadata when it affects trust in an observation

**Observed battle**:
A battle result recorded by the live OGame server and preserved in a combat
report.
_Avoid_: Synthetic fixture, simulator comparison

**Reference simulation**:
Output from another combat simulator used to expose disagreements in aggregate
results; it is supporting evidence rather than an observed battle.
_Avoid_: Ground truth, observed battle

**Evidence hierarchy**:
The order used to judge combat behavior: sanitized live-server observations
with permission to retain them, official rules and versioned in-game Techinfo,
then dated manual comparisons with other simulators. A public report page may
be inspected to find disagreements, but publication alone is not permission to
copy it into this repository.
_Avoid_: Treating a third-party simulator or public archive as canonical

**Report import**:
The conversion of a user-provided OGame combat or espionage report into
sanitized simulation input and, when applicable, a regression fixture.
_Avoid_: Account synchronization, report scraping

**Attack wave**:
One separate attack mission in a sequence against the same target. The
attacker's survivors return after each wave; the target's remaining state may
become the next wave's starting state.
_Avoid_: Combat round; six rounds end one battle, not a wave sequence

**Tactical retreat**:
An automatic pre-combat withdrawal of an eligible defending fleet when the
configured attacker-to-defender fleet ratio is exceeded. Its player-score
cutoff comes from universe settings: current universes normally use 500,000
points, while legacy universes may use 50,000.
_Avoid_: Attacker withdrawal, draw, combat timeout

## Fields and mechanics

**Debris field**:
Recoverable resources created by destroyed ships and defenses according to the
universe's debris rules.
_Avoid_: Wreck field

**Wreck field**:
Repairable destroyed ships that may return after a battle; it is separate
from the debris field.
_Avoid_: Debris, salvage

**Defence rebuild**:
The post-battle recovery of destroyed defensive structures. Each structure has
a 70% recovery chance, increased to 85% by the Engineer officer.
_Avoid_: Ship repair, debris recovery

**Missile attack**:
An attack that targets defenses directly, with anti-ballistic interception and
no fleet-combat rounds.
_Avoid_: Fleet battle, missile round

**Flight estimate**:
A structured calculation of a fleet's flight time, fuel consumption, and cargo
capacity under the selected universe and drive settings.
_Avoid_: Combat result, travel guess

**Universe metadata**:
Public per-universe information such as names, coordinates, settings, and
scores that provides context for a simulation.
_Avoid_: Fleet composition; metadata does not reveal exact fleets
