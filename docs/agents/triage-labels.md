# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Edit the right-hand column to match whatever vocabulary you actually use.

All five exist in the repo's GitHub labels.

## Two local conventions

**`needs-triage` is also the resting state for the backlog.** The canonical
machine treats it as "evaluation in progress", which on a repo with one
maintainer would mean the queue is permanently mid-thought. Here it carries a
second meaning: *evaluated, and deliberately not scheduled*. Issues in the
Backlog milestone sit here with their triage notes attached and no further
action pending. Several of them use the `needs-info` template in those notes —
that is a formatting choice, not a claim that someone is waiting on a reply.

**`needs-info` is therefore near-unusable here**, because the reporter and the
maintainer are the same person and an issue waiting on yourself never resolves.
Prefer recording the open questions in a `needs-triage` comment.

**An umbrella issue rests at `ready-for-human` and closes when its children do.**
It is not work, so it can never be `ready-for-agent`; the human part is the
decomposition and the sequencing. #6 and #7 are the worked examples — both were
split during triage, and both stay open purely as the thing their children roll
up into.
