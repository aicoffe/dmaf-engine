---
name: hld-design
description: 'Architecture design (hld). Expands the design points in the architecture impact statement into a complete hld review draft: identify complexity, design alternatives & trade-offs (when real trade-offs exist), 360-degree evaluation and selection, detailed solution, five views output, evolution planning and FMEA. Use when the user asks to "do architecture design", "update the hld", "decompose modules", "design service boundaries", "draw architecture diagrams", "write ADRs", "design high-concurrency/high-availability solutions", or "estimate capacity". Produces architecture decision and design documents; does not write feature code.'
---

# Architecture Design (hld authoring)

Expand the design points in the architecture impact statement into a complete hld review draft. Core ideas of the methodology:

1. **The purpose of architecture design is to resolve software complexity**, not to chase technical sophistication. Discussing solutions without complexity: the better the execution, the further off the mark.
2. **The three principles**: fitness beats industry-leading, simplicity beats complexity, evolution beats one-step completion. Use them to calibrate every trade-off.
3. **The 4R architecture definition**: architecture = top-level structure (Rank) + roles (Role) + relations (Relation) + rules (Rule). All architecture diagrams and documents are organized around the 4R.

## Workflow

### 1. Read inputs and make the execution determination
- Read the spec (NFRs/scope/constraints) and the architecture impact statement: conclusion is design needed → expand per Phases 1–6 below; conclusion is no design needed → report with a reason and produce no placeholder document
- The upstream delivery index and existing hlds are soft-input references (may be absent); when conflicting with existing repository constraints, escalate rather than silently bypassing
- Make **reasonable assumptions** for missing key information using common industry values, and mark them explicitly in the artifact as "Assumption:"; re-verify conclusions when assumptions change
- The impact statement already provides the hld revision points and the ADR draft — expand point by point; do not start a fresh design from scratch

### 2. Phase 1: Identify complexity

**Read `reference/complexity.md` first**, and use the "screening method" to analyze the six dimensions (high performance / high availability / extensibility / low cost / security / scale) one by one, qualitatively + quantitatively. Worksheet: `assets/complexity-worksheet.md`.

Key rules (violating them derails the whole solution):

- **Quantified estimation**: target TPS/QPS = daily average per second × peak factor (≈3) × headroom factor (2–4: 4 when the base is low, 2 when high; >10× not recommended). Argue in "per second", not "100 million per day"
- **There are usually only 1–2 primary complexities**. All six dimensions being problems = the system was previously mis-designed or this is a misjudgment; you must rank them and solve the most significant first — do not fantasize about solving everything at once
- Output the complexity list: per dimension "primary complexity or not + evidence (quantified derivation) + priority"

### 3. Phase 2: Design alternatives & trade-offs (only when real trade-offs exist)

Alternatives are **not mandatory**. For each primary complexity, first make the **trade-off determination**:

- **A best practice or de facto standard exists** (the mature industry pattern is the only reasonable option, or a standard catalog hit): adopt it directly and record the evidence (the specific practice/standard entries cited and their applicability conditions) — **no need to design alternatives**
- **A real architectural trade-off exists** (multiple viable directions, quality attributes traded against each other, no recognized de facto standard): only then design **2–3** alternatives (template: `assets/option-evaluation.md`)

Discipline when real trade-offs exist (the three most common mistakes):

1. **Do not design only the "best" option** — pick the suitable one, not the flashy one
2. **Alternatives must differ markedly** (architecture-level differences: open source vs in-house, primary-standby vs cluster; parameter-level differences do not make two alternatives), and write only to the **technology-selection level**, with no implementation details (details are Phase 4's business)
3. **Do not force alternatives where there is no trade-off** — straw-man alternatives padding the count are worse than none; where a best practice/de facto standard exists, record the evidence and adopt directly

Composing mature patterns is the right path: combine and adjust proven architecture patterns (primary-standby/cluster/load balancing/layering/plug-in …) based on business understanding; consider innovation only when mature patterns are entirely insufficient.

### 4. Phase 3: Evaluation and selection (360-degree evaluation)

When there are no alternatives (Phase 2 determined direct adoption of a de facto standard), skip the multi-alternative evaluation table and record in the selection record the **adoption rationale** plus the drawback mitigation of item 4 below; when real trade-offs exist, follow this procedure:

1. List the quality-attribute dimensions (performance, availability, hardware cost, project investment, complexity, security, extensibility, operability, etc.), evaluate every alternative dimension by dimension, and fill in the evaluation table
2. **Do not count pros and cons** (the count-comparison method is wrong) and **do not apply weighted scores** (weights cannot be objective; it becomes a numbers game)
3. **Select by priority**: combining business stage, team size and skills, and the operations system, rank the quality attributes; satisfy the first priority first, then the second…
4. After selection you must add a "drawback mitigation" paragraph: state the selected option's drawbacks explicitly and cite the three principles to explain why they are acceptable (e.g., "the performance drawback is irrelevant to the current business and can be scaled horizontally later — evolution principle")

**The team** is a legitimate and important input among the evaluation dimensions — the same set of options yields different selections in different teams (a startup picks open source for fast launch; a large company builds in-house).

### 5. Phase 4: Detailed solution design

Refine the selected option dimension by dimension, organizing sections by the complexity list. **First read the dimension references hit by the Phase 1 conclusions**:

| Design dimension | Read on demand |
|---|---|
| High performance (storage/single machine/cluster) | `reference/high-performance.md` |
| High availability (compute/storage/multi-site/interface-level failures/FMEA) | `reference/high-availability.md` |
| Extensibility (layering/microservices/microkernel/split granularity) | `reference/extensibility.md` |
| Security / low cost / scale | `reference/security-cost-scale.md` |
| Overall architecture assembly, tailoring by business stage, open source vs in-house selection | `reference/internet-template.md` |
| Interface conventions, distributed transactions, idempotency, multi-site implementation patterns | `reference/detailed-design-techniques.md` |

Integration-pattern selection decision tree, external system integration (anti-corruption layer/credentials/sandbox/Webhook), landing module decomposition (physical organization/dependency constraints/evolution signals), microservice-split prerequisites and strangler migration: `reference/architecture-guidance.md` (required reading when integration / module decomposition / microservice splitting is involved).

Refinement principles:

- Sub-selections in detailed design (e.g., load-balancing algorithm: round robin vs ip_hash) are **lightweight** decisions — pick directly from the applicability tables in the references; do not rerun the alternatives process
- Each dimension's design must land at an implementable level: concrete parameters, component counts, failover strategies, data flows; draw a diagram whenever you can
- If analysis shows a dimension needs no design, **still write "none" with a one-line reason** — omitting it will be treated as a gap in review
- If an alternative proves infeasible at the detail level (a key technical point or quality attribute was missed): fall back, fix the option, and say so explicitly — do not soldier on

### 6. Phase 5: Five views output

Describe the architecture completely from five angles using the five views, plus a scenario view tying them together. Per-view templates and checklists: `assets/views/<view>.md`; diagramming methodology (the 4R four-step drawing method, static vs dynamic diagrams) in `reference/diagram-guide.md`; architecture diagrams default to Mermaid (fall back to ASCII box diagrams when the user's environment does not support it).

| View | Question it answers | Required content |
|---|---|---|
| **Logical view** | What functions the system provides, which logical modules compose it, and how the modules relate | Modules/bounded contexts, domain model outline, dependency directions |
| **Development view** | Which developable/deployable units compose it, how the code is organized, what it depends on | Project/package structure, inter-module interfaces, build dependencies |
| **Runtime view** | How the roles collaborate in core scenarios (dynamic flows, sequences) | Process/service decomposition, concurrency and synchronization points, message flows |
| **Physical view** | What physical infrastructure it deploys on (datacenters/network/hardware/specs) | Node/environment/middleware topology, network boundaries |
| **Data view** | How data is modeled, where it lives, how it flows/replicates/shards | Core data models, sharding and archiving |

Drawing requirements: every service must have logical + runtime views; the rest on demand (a monolith's development view may merge with the logical view); a unified legend (solid line = synchronous call, dashed line = async message, cylinder = data store, double border = external system); one diagram, one claim — do not mix deployment and logical information; update diagrams in sync with hld changes, and treat stale diagrams as defects. The scenario view ties the first four together: it must cover at least the core write path, the core read path, and one failure scenario (the degradation path), each scenario annotated with participating modules, call order, transaction boundaries, and failure points. Put diagrams in `architecture/diagrams/`; prefer text-based diagram sources committed to the repository so they are diffable.

Requirements scenarios are the five views' input and validation: every view must be able to answer the identified core scenarios. Assemble the complete document per `assets/arch-doc-template.md` (the nine-section current-state structure; views and materials attach to sections). Writing essentials: service-level decisions only (class design and algorithm details belong to the lld), link rather than copy, every decision traceable, consistent with the spec/NFRs.

### 7. Phase 6: Evolution planning and risk wrap-up

- **Architecture evolution planning** (evolution principle): output a phased implementation plan as "what phase 1 does / what phase 2 does"; the first phase is minimally viable — do not build ahead of predicted business scale; the plan goes into the hld appendix, not the body
- **FMEA hazard screening**: mandatory when the primary complexities include high availability; template `assets/fmea-checklist.md`

### 8. Stack declaration verification and self-check

- Verify item-by-item consistency with the project's tech-stack declaration (technology selections and integration patterns must not exceed the declaration); on mismatch, revise or escalate — do not silently bypass
- Self-check: every design point in the impact statement expanded, every key decision has an ADR (alternatives and rationale recorded where real trade-offs exist; evidence recorded where best practices/de facto standards are adopted), NFRs fully covered, backward compatibility assessed, observability in place, no circular dependencies

## Boundaries (not in this skill)

- Architecture impact judgment and the project stack declaration belong to `architecture-analysis` (this skill consumes its impact statement and makes no judgment of its own)
- Feature-level detailed design (interfaces/classes/table structures/components) belongs to `detailed-design-backend` / `detailed-design-frontend`

## Hard rules

- Distributed monoliths and nanoservices are forbidden; technology-choice changes without an ADR are forbidden
- Architecture changes go in their own PR, reviewed separately from feature code
- High-risk architecture decisions (style, decomposition, integration patterns) take effect only after human review passes
- The hld is a current-state document: evolution and history stay out of the document body (evolution history is carried by ADRs and adjudication records; an ADR's trade-off record — alternatives or evidence — and its rationale are design content and are kept)

## Reference files

- `rules/guides/adr-authoring-guide.md` — when to write ADRs, authoring essentials, and status transitions (required reading when writing an ADR)
- `assets/adr-template.md` — the ADR skeleton (apply when drafting an ADR)
- `rules/standards/architecture-standard.md` / `rules/standards/security-standard.md` — upper-layer constraints
- `reference/` — methodology deep reads (complexity screening / high performance / high availability / extensibility / security-cost-scale / internet template / detailed design techniques / diagramming guide / supplementary architecture techniques) — read the ones hit by trigger conditions; **dumping the whole directory is forbidden**
