# AGR handoff into the Dock planner

Verified against AntiGameReborn **13.1.4** (2026-09-13), downloaded from the
[official Chrome Web Store package](https://chromewebstore.google.com/detail/antigamereborn/mhfbpacbhjchkjeopjfgdhckepclcfll).
Only AGR's launch construction and lifeform aggregation were inspected; no
extension or simulator code is included here.

## Launch contract

Configure the simulator base URL in AGR. It appends a language path and
`#prefill=<base64 JSON>`. `/de` and `/pt-BR` therefore reach the same German
planner. An optional `SR_KEY` query value refers to an opponent report, **not**
the own fleet. The report importer offers a separate explicit transfer action;
see [report import](report-import.md). Both parameters may also share the
fragment in either order. Raw base64 plus signs must survive key removal.

The [published prefill contract](https://battlesim.logserver.org/en/userprojects)
uses root arrays `0` (attackers), `1` (defenders), and `settings`. Unit entries
are `{ "count": integer }`, research entries `{ "level": integer }`, keyed by
OGame technology IDs. This reader supports one participant per side. The own
`ships` object is required; an explicit empty object is a supplied empty fleet.
A missing defender leaves the existing opponent unchanged.

## Current AGR modifier profile

AGR 13.1.4 sends `class`, `allianceClass`, and `lifeformBonuses` together.
Its attacking research already includes `floor(2 * (1 + General booster))`
for a General, plus one for the Warrior alliance. The reader removes those
ordinary class levels before supplying the engine's separate class fields.
A General booster producing more than two levels is rejected because the
engine does not represent that enhancement. Class IDs are player 1 Collector,
2 General, 3 Discoverer; alliance 1 Warrior, 2 Trader, 3 Researcher.

AGR's `BaseStatsBooster` is freshly accumulated lifeform research, not the
similarly named report-proxy observation. `weapon`, `shield`, and `armor` are
fractions, multiplied by 100 for this engine. Omitted units in a supplied
sparse booster table have zero bonuses. A missing table does not establish zero.
Class-bearing older/partial payloads and unknown modifier formats are rejected
rather than guessing researched versus effective levels.

## Application behavior and limits

The 128 KiB fragment limit applies before decoding. Parsing is atomic; failure
preserves the plan, its available fleet, and the last successful result. Initial
load and `hashchange` import immediately, without simulating. Selection edits
leave the imported quantities available to Wiederherstellen. A subsequent valid
handoff replaces that snapshot. Snapshots last for this open page, not across
reloads independently of the launch URL.

The source disclosure lists imported fields and assumptions, and describes the
original handoff; subsequent edits are manual. Missing numerical inputs remain
blank until request construction assumes zero. Missing classes use no class;
missing Rapidfire uses the planner's enabled default and missing deuterium
debris uses its disabled default, disclosed explicitly.

Fleet debris fractions become percentages. Enabled defence debris does **not**
supply its percentage: the field stays blank with an explanation. Inactive
defence debris supplies zero. Drive research and supported universe dimensions,
speed, donut flags and fuel factor are passed through with provenance; they do
not introduce flight calculations. Holding speed, cargo, fuel bonus, resources,
repair, plunder and requested simulation count do not change the planner's
existing calculation or 100/1,000-run choice. Stationary own units and missiles
are omitted from combat selection. Unknown combat unit IDs are rejected.

No raw payload, identity, position, server identity or report key enters planner
state, logging, storage or the simulation request. The launch URL itself remains
in the browser; the application does not persist or publish it.

## Synthetic example

This invented fixture contains no identities, positions or report capabilities.
Encode its JSON with UTF-8/base64 and append it to `/de#prefill=`. It starts with
101 cruisers; editing the selection to 50 sends 50 while restoring returns 101.
The outgoing researched weapon level is 12 and the separate class fields add
three levels once. Per-ship lifeform weapon bonus is 12.5 percent.

```json
{"0":[{"ships":{"206":{"count":101}},"research":{"109":{"level":15},"110":{"level":13},"111":{"level":14}},"class":2,"allianceClass":1,"lifeformBonuses":{"BaseStatsBooster":{"206":{"armor":0.125,"shield":0.125,"weapon":0.125,"cargo":0.1,"speed":0.1}},"CharacterClassBooster":{"1":0,"2":0,"3":0},"ShipFuelConsumption":0}}],"1":[{"ships":{"204":{"count":1000}}}],"settings":{"rapid_fire":"1","def_to_tF":"0","debris_factor":"0.7","deuterium_in_debris":"1"}}
```

`frontend/tests/agr.spec.ts` exercises this example through the browser and HTTP
boundary, along with missing, corrupt, excessive, ambiguous and multi-party
handoffs. No live player captures are used.
