---
name: requirement-analysis
description: Requirement analysis. Turns ideas, tickets, alerts, and incidents into intent.md, then turns the intent into spec.md with acceptance criteria and user stories. Use when the user describes a business problem or asks to "organize requirements", "write a spec", "split user stories", or "define acceptance criteria". Produces requirements only, never design.
---

# Requirement Analysis

Turn business language into verifiable requirement specifications. Answers only "what to build and why", never "how to build it".

## Workflow

### Intent capture → intent.md
1. From the material the user provides (idea / ticket / alert / incident), extract: problem, expected outcomes, hard constraints, open questions, sources
2. Capture principle: capture only "what problem to solve", in business language, introducing no solution words ("add a cache" is a solution, not an intent); keep information minimal; expected outcomes must be falsifiable (one can judge "was it achieved or not")
3. Fill gaps with follow-up questions; focus per source: idea → whose problem, of what, and what happens if left unsolved; ticket → the underlying request (descriptions are often solutions rather than problems); alert → the impact behind the alarm, prevent recurrence or just stop the bleeding; incident → retrospective improvement items, blast radius, and priority
4. Hard constraints must be explicit; undecided matters go into "open questions" — hiding them is forbidden

### intent → spec.md
1. Answer the open questions in the intent, or move them into Out of Scope
2. Write the scope: In Scope and Out of Scope are equally important; vague catch-alls like "etc." are forbidden
3. Write user stories: `As a {role}, I want {capability}, so that {business value}` — all three elements required; roles are real user/system roles, never generic; a story that cannot state "so that" is usually a feature list, not a story
4. Give each story 3–7 Given-When-Then acceptance criteria: the happy path + main exceptions + boundaries; acceptance criteria convert directly into implementation verification and tests
5. Derive NFRs (performance / security / reliability), marked "derived, pending confirmation" for human confirmation — derivation methods: performance is estimated from business scale (quantitative model in the hld-design Phase 1 complexity screening), reliability/security are derived from business criticality grading; the UE contract is part of the requirements: write only "what feedback the user expects", never the implementation (skeleton screens and toasts are design)
6. Prioritize the stories with MoSCoW labels (Must / Should / Could / Won't this time) — Must = a direct dependency of this cycle's acceptance; scope pruning starts from Could
7. Run specification-conflict detection while generating; write discovered conflicts into open questions for design-time adjudication to resolve

### INVEST self-check

A story is accepted only after passing all six dimensions; how to judge the three most easily misjudged ones:

- **Independent**: remove any story and the rest can still ship independently. Violation: technology-layered stories ("create tables" / "wire the API") → fix: re-slice by user-visible behavior, folding the technical work into behavior stories as tasks, never as standalone stories
- **Small**: completable and demonstrable by one team within one iteration. Violation: "manage the entire claims process" → fix: slice by user goal (submit / review / pay out), not by data table or page
- **Testable**: every acceptance criterion can be written Given-When-Then and verified automatically. Violation: "the system should be easy to use" → fix: translate into observable behavior (operation steps, response time limits, verifiable results)
- Check the remaining three dimensions (negotiable / valuable / estimable) against their definitions; if any fails, split or rewrite. Splitting stories by technical layer is forbidden

### Splitting methods

By business process step (submit → review → notify) / by data dimension (single-record operations → batch operations) / by rule variant (standard flow → special-case rules) / core first, exceptions later (main path → exception-prompt refinement). Stories produced by splitting must still pass INVEST; technical tasks disguised as stories ("set up the XX tables" — no role, no value) and stories without acceptance criteria must not enter review.

## Hard rules

- No architecture, technology selection, table schemas, component names, or concrete interaction implementations — remove on sight
- NFR numbers without a source must be marked pending confirmation
- The capture phase must not give feasibility conclusions or solutions
- An acceptance criterion you cannot write means the story is not thought through — go back and clarify at the intent level

## Boundaries (not in this skill)

- Architecture and implementation design (hld / lld) is handled by `hld-design`, `detailed-design-backend` / `detailed-design-frontend` — this skill produces requirements only and never answers "how to build it"
