# Architecture Diagram Drawing Guide

> When to read: Phase 5 five-views output, drawing any architecture diagram, or reviewing diagrams that communicate poorly
>
> Many designs are done well but explained poorly — the problem is in the diagram: not knowing what to draw or how much. The fix is to use the 4R architecture definition to fix content and boundaries.

## 1. Guiding idea: the 4R four-step drawing method

Software architecture = top-level structure (Rank) + roles (Role) + relations (Relation) + rules (Rule). Drawing maps to four steps:

1. **Fix the Rank**: first decide which level this diagram covers (L0 system-wide → L4 inside a module). One diagram covers one level; an all-in-one, everything-included diagram is as good as no diagram.
2. **Draw the Roles**: decompose the system from the current angle; roles map to blocks/icons/nodes in the diagram.
3. **Draw the Relations**: connecting lines between roles; different line styles represent different relations (sync call/async notification/data dependency).
4. **Draw the Rules**: pick core scenarios and draw system sequence diagrams expressing how roles collaborate to complete business.

Role + Relation diagrams = **static architecture diagrams** (generally 1–2; split into two when complex: one "roles + core functions", one "role-to-role relations"); Rule sequence diagrams = **dynamic architecture diagrams** (one or more per core scenario).

## 2. Relation to the 4+1 views

The 4+1 views (1995, Kruchten: logical/process/development/physical/scenarios) are conceptually right but rarely landed: in distributed systems the development view is hard to express, the UML binding is unattractive, and logical/development/process views blur together. Actual work uses the common architecture-diagram system in the table below instead; it is organized by "decomposition angle" and covers all of the 4+1 concerns (this skill's five views map onto it too):

| Common diagram | Describes | 4+1 counterpart | This skill's counterpart |
|---|---|---|---|
| Business architecture diagram | Which business functions the system provides | Scenario view | Logical view, functional-view section |
| Client/frontend architecture diagram | Client-side domain logic decomposition | Development view | Supplement to logical/development views |
| System architecture diagram | Backend logical architecture (Role+Relation), a.k.a. technical architecture | Logical view | Logical view, system-architecture section |
| Application architecture diagram | Which deployable applications compose it | Development view | Development view |
| Deployment architecture diagram | Datacenter/network/hardware deployment | Physical view | Physical view |
| System sequence diagram | How roles collaborate in a scenario | Process view | Runtime view |

Other systems exist — TOGAF (business/data/application/technical architecture; enterprise-leaning) and C4 (Context/Container/Component/Code; limited expressiveness) — with no unified industry standard; draw with the validated system above.

## 3. Drawing essentials per diagram type

| Diagram | Use | Techniques |
|---|---|---|
| Business architecture diagram | Product planning reviews, briefing senior staff on the business, onboarding training | Color marks business status (≤3 colors); business groups as dashed boxes/backgrounds; align blocks neatly — but lengths serve alignment only, not magnitude; magnitude is expressed by color |
| Client/frontend architecture diagram | Overall in-domain design, training | Color distinguishes module types; multiple relation types get different line styles; layered grouping |
| System architecture diagram | Overall architecture design, training | Color distinguishes roles; lines carry relations; logical grouping. Split into two when complex: roles+functions / relations |
| Application architecture diagram | Project development and testing, ops deployment, subdomain design | Complex systems draw per subdomain — one diagram per domain |
| Deployment architecture diagram | Overall design, ops planning | Icons instead of blocks; datacenters/AZs as partition frames |
| System sequence diagram | Used alongside the three static diagrams | UML sequence diagram; participant order = call chain; failure branches must be expressed |

## 4. General conventions (what reviewers pick at first glance)

1. One diagram, one Rank level; start another when drilling down.
2. Legend first: line styles, colors, and icon meanings explained beside the diagram.
3. ≤3 colors; color carries information (status/category), never decoration.
4. Explainable lines: every line answers "what relation, what direction, sync or async".
5. No circular-dependency arrows (a back edge must be explained — usually event subscription).
6. Information density: roles irrelevant to this communication are resolutely hidden into lower-level views.
7. Delivery format: Mermaid first (versionable, renderable); the final version for executive briefings may be polished with drawing tools, but the structure stays unchanged.

## 5. Common pitfalls

- Cramming 5 levels into one diagram; the reviewer's first question is "what level is this".
- Drawing system architecture with UML class diagrams — all effort spent on notation.
- Static only, no dynamic: without sequence diagrams, collaboration rules (Rule) have nowhere to live.
- Happy path only: sequence diagrams without failure branches equal no reliability design.
- Missing legend: solid vs dashed meaning in the same diagram is left to guessing.
