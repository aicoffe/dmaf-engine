---
name: documentation
description: Documentation writing. Document type matrix, README template, documentation maintenance and staleness inspection. Use when the user asks to "write docs", "write a README", "fill in documentation", "documentation standards". Docs update in the same repository and the same PR as the code.
---

# Documentation

Docs share the code's life cycle: changing behavior without updating the docs = an incomplete change.

## Workflow

### 1. Document type matrix
| Document | Location | When to update |
| :--- | :--- | :--- |
| Service README | service root directory | whenever information needed for onboarding changes |
| ADR | `architecture/adr/` | every architecture decision |
| API contract | `openapi.yaml` + CHANGELOG | every API change |
| Data dictionary | `docs/data-dictionary.md` | every table structure change |
| Runbook | linked from the alerting platform | whenever an alert response plan changes |

### 2. README template (service level)
```markdown
# {service name}
One line: what it does, which business domain it belongs to

## Quick start
Environment requirements → local startup command → how to run the tests

## Architecture at a glance
What I depend on, who depends on me (link to the architecture doc)

## Common operations
Release, rollback, entry points for common questions

## Contacts
Owner, on-call channel
```
Pass bar: a new hire can get it running locally within half a day by following the README.

### 3. Writing requirements
- Write for the reader: the README for newcomers, the Runbook for the on-call engineer at night (operable by following it step by step)
- Markdown; commands directly copy-paste runnable; links preferred over copies (single source of truth)
- Primary language per the language policy (deliverables follow the original task statement's language); code/commands/identifiers stay verbatim

### 4. Maintenance
- Docs update in the same PR as the code
- Quarterly inspection: spot-check README usability and ADR status; stale docs get a stale marker with a deadline (a stale doc is treated as a defect)

## Boundaries (not in this skill)

- When to write an ADR, writing points, and the state-transition source of truth live in `rules/guides/adr-authoring-guide`; the skeleton template is in the hld-design skill's `assets/adr-template.md`; the data dictionary structure's source of truth is `rules/standards/data-standard` — this skill governs "when to update which document" and general writing requirements
- API contract content changes go through `detailed-design-backend` (design phase); this skill only covers keeping contract documents in sync

## Reference files

- `rules/guides/adr-authoring-guide.md` — when to write an ADR, writing points, and state transitions (must-read when writing or inspecting ADRs)
- hld-design skill `assets/adr-template.md` — the ADR skeleton (apply it when drafting)
- `rules/standards/data-standard.md` — the data dictionary structure's source of truth (check against it when updating the data dictionary)
- `rules/standards/engineering-standard.md` — higher-level constraints
