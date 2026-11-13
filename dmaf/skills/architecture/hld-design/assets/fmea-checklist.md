# FMEA Hazard Screening Table Template

> Purpose: Phase 6. Mandatory when the primary complexities include **high availability**. FMEA (Failure Mode and Effects Analysis) is the method for eliminating architectural availability hazards: it does not ask "how low is the probability" but "if it happens, how large is the impact and is there a response". Screening conclusions (mitigations and action items) go into the hld's non-functional architecture section.

## How to use

1. Fill the table row by row per function point (one function point may have multiple failure modes — use multiple rows).
2. Generate action items for rows with "high severity + no effective mitigation currently", and fold them into detailed design.
3. FMEA is a living document: backfill on solution changes and on new failures after launch.

## FMEA table

| Function point | Failure mode | Failure impact | Severity | Failure cause | Failure probability | Existing measures | Avoidance measure | Solution measure | Solvable? |
|---|---|---|---|---|---|---|---|---|---|
| <message storage> | <MySQL primary machine crashes> | <messages temporarily unreadable> | <medium> | <hardware failure> | <low> | <one-primary-one-standby replication> | <primary-standby failover> | <failover script + drills> | <yes> |
| | | | | | | | | | |

How to fill each column:

- **Function point**: a function from the system user's perspective; granularity is yours to choose as long as coverage is complete.
- **Failure mode**: what failure can hit this function point (crash/timeout/data corruption/replication lag …); enumerate as exhaustively as possible.
- **Failure impact**: impact on users/business (unavailability duration, data loss volume, compliance risk).
- **Severity**: high/medium/low (or 1–10), corresponding to the failure impact.
- **Failure cause**: hardware, software bugs, human operations, external dependencies …
- **Failure probability**: low/medium/high — for prioritization only, **not for wishful thinking** ("low probability" does not excuse leaving severe items unhandled).
- **Existing measures**: current design/operations coverage for this failure (write "none" if there is none).
- **Avoidance measure**: means that lower the occurrence probability (redundancy, inspections, change control).
- **Solution measure**: means that reduce the impact after it happens (failover, degradation, rollback, drills).
- **Solvable?**: for failures that cannot be fully solved today, explicitly declare the residual risk and provide a monitoring/alerting compensation (e.g., alert when replication lag >30s).

## Action item output

| Severity | Failure mode without effective mitigation | Action item | Owner | Deadline |
|---|---|---|---|---|
| High | <...> | <supplementary design/drill/alerting> | | |

## Screening discipline

- Work backwards from "impact" to measures, not forwards from "I have redundancy" self-confirmation.
- High-severity items must not leave "solution measure" blank — at minimum there must be alerting + a residual-risk statement.
- After completion, focus the recheck on: single points (the only node/service/datacenter), whether the failover decision mechanism itself is a single point, and cascading failures triggered by replication lag.
