# Triage labels

The engineering skills (`triage` and friends) speak in canonical roles. This table maps them to the label strings in this repo's tracker (created by `scripts/sync-labels.sh` in presets from `labels.json`).

| Role | Label in this tracker | Meaning |
| --- | --- | --- |
| category: bug | `bug` | Something is broken |
| category: enhancement | `enhancement` | New feature or improvement |
| `needs-triage` | `needs-triage` | Maintainer needs to evaluate this issue |
| `needs-info` | `needs-info` | Waiting on the reporter for more information |
| `ready-for-agent` | `ready-for-agent` | Fully specified, ready for an external agent to pick up |
| `ready-for-bot` | `ready-for-bot` | Ready for an in-house bot (Maintainer) to pick up |
| `ready-for-qa` | `ready-for-qa` | Ready for QA review and independent testing |
| `ready-for-human` | `ready-for-human` | Needs a human to implement or decide |
| `wontfix` | `wontfix` | Will not be actioned |

Pickup labels: `ready-for-bot` is for in-house bots (Maintainer), `ready-for-agent` is always for an external agent, and `ready-for-qa` is for QA.

