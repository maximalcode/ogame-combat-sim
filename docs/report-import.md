# Community-proxy report import

`combat-cli report` imports one combat (`cr-…`) or espionage (`sr-…`) report ID.
Recycler reports are not supported. This is separate from offline pasted-text
import (#21), and adds neither a public HTTP route nor browser UI.

Store the ID in a private local text file outside version control, then run:

```sh
cargo run -p combat-cli -- report --file /absolute/private/report-id.txt --allow-proxy-transfer
```

Without `--file`, the command reads one ID from stdin until EOF. Keep the ID
out of command arguments, shell history, public issues, screenshots and logs.
Do not paste it as a positional argument. File/open/input errors are redacted.

## Transfer and retention

Explicit consent allows a request to
`https://ogapi.faw-kes.de/v1/report/{api_id}/1`. The proxy advertises caching:
local non-retention does not imply that the proxy deletes its copy. The importer
does not write raw responses or IDs to disk. Redirects and automatic retries are
disabled. Requests time out after 20 seconds; responses are limited to 2 MiB.
No private developer key, account session, cookie or gameplay action is needed.

The client allows at most ten request starts in any rolling 60 seconds across
all client instances within one process. Failures consume a start too. Local or
provider rate limits return a redacted error with retry guidance, without an
automatic retry. Independent processes still share the provider's quota.

## Candidate versus simulation request

The JSON output is a review artifact, intentionally not a `CombatRequest`.
`review_required` names incomplete or uncertain inputs. Null means unknown;
an empty revealed composition means no units were reported in that category.
Espionage visibility flags take precedence over arrays: hidden fleet, defence
or research information never becomes zero. Fill in the attacking fleet for
espionage imports and review universe debris rules for either kind.

Combat participants receive local labels (`A1`, `D1`, etc.), not player IDs.
Round entries have a null slot when the owner-to-slot association is absent or
ambiguous (for example several ACS fleets belonging to one player). Observed
rounds, unit losses, repaired defences, resource losses, loot and debris remain
under `observed`, along with reported moon chance/creation/existence/size;
no simulation results are substituted. The provider's
`units_lost_*` totals are resource-value totals, not ship counts. Per-type losses
remain in each observed round. Total debris and remaining debris are separate.

Espionage technology is labelled researched. Combat technology is labelled as
the reported bonus divided by ten; verify class treatment before using it as
researched technology. `reported_base_stats_booster` preserves provider numeric
coefficients, **not simulator lifeform percentages**. The sampled combat and
espionage variants differ in whether these values include other modifiers.
Compare them with `reported_unit_stats` where present; do not blindly multiply
by 100 or add them on top of researched technology and classes.

Provenance retains community/universe from the validated ID and an available
event timestamp. The sampled endpoint supplies no game version, so it remains
null. An optional `generic.game_version` compatibility field is retained when
present as two to four dot-separated numeric components (maximum 32 bytes).
This extension was tested synthetically, not observed in the private samples;
malformed versions produce a redacted field error. Player names, coordinates,
message IDs, access tokens and unknown fields
are discarded through an explicit output allowlist. Output still contains
battle evidence: review privacy and obtain publication consent before sharing
it or creating a regression fixture. No fixture is published automatically.

To complete a single combat candidate offline, pass a structured artifact to
`combat-cli report complete --file`. The artifact supplies explicit participant
evidence and a fully pinned universe. Completion reports every missing or
unsupported field together; it never fills absent technology, class, lifeform,
composition, or temporal status with a default. In particular, participant
evidence must include a `lifeform` object even when it is `{}`: an omitted or
`null` value leaves the lifeform state unknown. A verified result keeps the
battle provenance and universe snapshot provenance in a separate evidence
ledger, while the observed outcome stays outside the `CombatRequest`.

When a lifeform object names an entity, its `weapon`, `shield`, and `armour`
percentages are required individually. An omitted or `null` combat percentage
produces a targeted completion issue; it is never treated as an explicit zero.
An empty entity object therefore remains incomplete, while the explicitly empty
whole `lifeform` map confirms that no lifeform modifiers apply. Optional
`cargo` and `speed` values are retained when supplied and are not invented
when absent.

Supplied lifeform percentages must be finite and non-negative. Completion also
checks that the resulting weapon, shield, and armour starting statistics remain
valid for the simulator's numeric representation; it does not impose a
game-specific maximum where none is established.

## Validation

Normal Rust tests are offline: synthetic schema examples test parsing and
sanitization, and a local HTTP stub tests transport failures, size limits,
redirect rejection, timeouts and the shared rolling quota.

The command above is also the explicit opt-in live check. Run it separately for
one privately supplied combat ID and one espionage ID; a successful command
prints a sanitized candidate with review requirements. Do not redirect raw
reports into tracked files. Library callers with captured JSON can use
`reports::parse_report` without networking. Consent to fetch does not authorize
publishing real test fixtures; committed examples are invented, not copied
from private reports.

## Upstream procedure

The official [API access procedure](https://forum.origin.ogame.gameforge.com/forum/thread/319-api-access-applications-required-procedure-updated/)
directs new standalone tools to the community proxy. Its own
[documentation](https://ogapi.faw-kes.de/) describes the route and quota.
The [tool guidelines](https://forum.origin.ogame.gameforge.com/forum/thread/318-read-first-do-i-need-to-submit-my-tool-submission-guidelines/)
distinguish standalone calculators from scraping or game-interface tools;
the [submission template](https://forum.origin.ogame.gameforge.com/forum/thread/332-tool-submission-template/)
describes disclosures needed if that scope changes. This importer does not
enumerate reports, scrape accounts or run background refreshes.
