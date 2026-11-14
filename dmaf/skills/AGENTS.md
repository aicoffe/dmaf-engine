# dmaf/skills/ — SDLC skill assets

The sample skill library: grouped by capability domain with **one level of categories** (the category
system comes from section 2 of `02-角色及skill清单.md`); a skill lives at `<category>/<name>/SKILL.md`,
and installation flattens by skill name (names must be globally unique).
Two common mechanism skills stay at the top level, outside the categories: `dmaf/` (the orchestration
manual) and `rules/` (the common norms library).

## Categories

| Category directory | Capability domain | Skills |
|---|---|---|
| `requirements/` | Requirements | requirement-analysis |
| `architecture/` | Architecture | architecture-analysis, hld-design, tech-selection |
| `backend/` | Backend development | backend-development, db-optimization, detailed-design-backend |
| `frontend/` | Frontend development | frontend-development, accessibility, frontend-architecture, detailed-design-frontend |
| `testing/` | Testing | test-design, automated-testing, contract-testing |
| `engineering/` | Code quality and engineering practices | code-review, git-workflow, documentation |
| `operations/` | Observability | observability |
| `security/` | Security | secure-coding |
| `governance/` | Design governance | design-system |
| `stacks/` | Stack binding | backend-rust, backend-go, backend-java, frontend-vue3-naiveui, frontend-react-mantine, ue-prototyper-react-mantine, ue-prototyper-vue3-naiveui |

**Off-process shelved skills**: the 15 skills not touched by the current sample process have been moved
to `skills-backup/` (repo root, gitignored, category structure preserved) — operations 5 (alerting /
capacity-planning / dashboard-design / incident-response / resilience), governance 4 (ai-usage-governance /
cost-optimization / data-governance / high-fidelity-design), security 3 (auth-design / secrets-management /
vulnerability-management), engineering 2 (code-style / dependency-management), testing 1
(performance-testing). Restoring = move the directory back to `dmaf/skills/<category>/` and re-run
`dmaf install`. The 6 skills referenced by merged skills' Boundaries sections (db-optimization /
secure-coding / observability / accessibility / frontend-architecture / design-system) were intentionally
kept in place — moving them would break the pointers. (contract-testing and documentation were once
shelved, and returned with the addition of the system-testing stage and the pr-preparation wiring.)

## Conventions

- **Common norms are centralized in the `rules/` skill**: L0 principles (`rules/principles.md`), L1 domain
  standards (`rules/standards/`, 11 `*-standard` documents), L2 working guides (`rules/guides/`, 67
  `*-guide` documents); the system index is `rules/SKILL.md`; skill text always cites norms by the
  `rules/standards/<name>` and `rules/guides/<name>` paths;
- **Stack binding**: `stacks/<name>/` holds self-contained stack skills (`SKILL.md` entry point + reference/
  routing; reference files must carry trigger conditions); the project's tech-stack declaration (the
  sample lands it at `dmaf-output/tech-stack.md`) is produced by the architecture-analysis three modes
  through the approval gate, and neutral skills downstream of architecture carry a unified "stack binding"
  preamble in their header that loads stack skills per the declaration; stack knowledge converges at a
  single point — neutral norms/skills must not scatter stack-binding sentences;
- A skill describes only "how": inputs (artifact paths / scope / timing) and outputs (artifact paths /
  form / when to write to disk) are given by the delegation payload — never hard-code them in skill text;
  the body must not use fixed artifact paths or stage/gate/engine protocol words (revision_feedback,
  reporting success, skipped, etc.), nor meta-language like "specified by the agent" or "when to write to
  disk" — the shape of a decoupled body is silence: technical-domain-facing and usable independently of
  the process;
- Artifacts are written to the project-root-relative full paths specified in the stage `outputs` (with
  `{SESSION_ID}` bucketing supported); never into `.dmaf/`;
- After changes, sync to the harness via `dmaf install --harness <name>`; never hand-edit the harness
  directory;
- For the orchestration mechanism see `dmaf/skills/dmaf/SKILL.md` (the conductor manual, mounted from the
  same text as `dmaf/agents/dmaf-conductor.md`).
