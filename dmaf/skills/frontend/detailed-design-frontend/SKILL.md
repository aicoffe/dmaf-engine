---
name: detailed-design-frontend
description: 'Frontend feature-level detailed design. Produces the frontend lld from the spec and the UE prototype: directory and routing delta, component and service layer design, state classification and request cache design, API integration conventions, interaction implementation notes, acceptance criterion mapping. Use when the user asks to "do frontend detailed design", "write the frontend lld", "design frontend components", "design frontend state", or "design page routing". Describes only this change; makes no architecture decisions.'
---

# Frontend Detailed Design (Detailed Design · Frontend)

Land architecture conclusions, the UE prototype, and the requirements into a TDD-ready frontend implementation design. Reference the hld and the UE delivery index; describe only this change.

## Stack binding (executed before this skill's workflow)

Read the project's tech-stack declaration and take exactly one of the four declaration states (treat as "no declaration" when none is provided):

- **Binding present**: load the stack skill directory the "binding" section points to — read `SKILL.md` first as the entry point, then load the `reference/` docs for the domains the current task touches, following the trigger conditions in its "reference files" routing table (one task reads only its own domain; never dump the whole directory); when a stack rule conflicts with this skill, the more specific stack rule wins, and unresolvable conflicts escalate to a human
- **Binding is "none" (generalized mode)**: execute with the neutral spec plus model generalization, based on the stack named in the `backend_stack` / `frontend_stack` declaration; the verification loop follows the declaration's "verification commands" section; label output "generalized mode (<stack name>, no binding skill)"
- **No declaration**: execute per this skill's neutral spec; label output "the project declares no tech stack"
- **Declaration unparseable**: report it as-is; never guess

## Workflow

1. **Read inputs**: spec acceptance criteria, the architecture impact statement / architecture design, the UE delivery index (address by the index when it exists)
2. **Directory and routing delta**: page list and route table delta (path ↔ page component), directory structure changes (follow the project's existing layering — no parallel structure)
3. **Component and service layer design**: component tree and split boundaries — the test: split when state ownership or change frequency differs; extract to shared when reused in ≥ 2 places or across pages; "same family, many members" (list columns / form control families) use compound components, and shells whose content the consumer decides use slots. Component API draft (typed props, event callbacks, slots, aligned with the UI library's conventions) and the service layer / request layer interfaces (aligned with the backend contract: paths / params / error codes)
4. **State design**: choose per the state classification — server state goes to the request cache (cache keys / invalidation policy / staleTime), global client state goes to the store (slice division, minimized), local UI state goes to components; copying API data into the global store is forbidden (two sources of truth will always drift)
5. **API integration conventions**: align one by one with the backend contract (field naming / error responses / pagination / idempotency); the handling boundary of the three error states — which errors the service layer handles uniformly, which render inside components
6. **Interaction implementation notes**: align with the UE prototype's mocks and interaction annotations (loading/empty/error three states, confirmation modals, form validation timing, feedback channels); interactions unclear at the implementation layer get escalated for adjudication — never unilaterally deviate from the prototype
7. **Acceptance criterion mapping table**: every spec acceptance criterion ↔ implementation point ↔ planned test — nothing missing

## Boundaries (not in this skill)

- Implementation following this design goes to `frontend-development`; the generic spec sources of truth for component APIs / interaction patterns live in `rules/guides/component-design-guide` / `interaction-design-guide` (this skill only describes this change's alignment conclusions)
- Backend feature-level design is owned by `detailed-design-backend`

**lld.md structure (frontend)**: 1 design overview (implementation approach; references the hld and the UE delivery index) → 2 directory and routing delta → 3 component and service layer design → 4 state design → 5 API integration conventions → 6 interaction implementation notes → 7 acceptance criterion mapping

## Hard rules

- Implementation strategies conflicting with the hld are forbidden; escalate conflicts for adjudication — never silently bypass
- Vacuous design is forbidden ("use a suitable component" is not design)
- Manually copying API response data into the global store is forbidden
- Visual values reference design tokens only; hardcoded colors/spacing are forbidden; conflicts with the UE prototype's mocks go back into the interaction notes
- The lld is a current-state design document: evolution and history stay out of the document body

## Pre-delivery self-check

After drafting and before delivery, check item by item:

- [ ] Conforms to the hld (no bypassing of reviewed architecture decisions) and to conflict adjudication outcomes
- [ ] Component / state / API integration design is consistent with the backend contract
- [ ] Interaction implementation notes cover the UE prototype's annotations (or the spec's interaction clauses when the prototype is absent)
- [ ] Acceptance criterion mapping has no gaps
- [ ] Actionable: can be split directly into atomic tasks

## Reference files

- `rules/guides/component-design-guide.md` / `rules/guides/interaction-design-guide.md` — component design clauses and interaction pattern parameters (shared clause tables)
- `rules/standards/frontend-standard.md` / `rules/standards/ux-standard.md` — upper-level constraints
