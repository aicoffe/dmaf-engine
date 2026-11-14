---
name: frontend-development
description: Frontend implementation. Component development (component API design, controlled/uncontrolled, composition patterns, Story and component tests, design system reuse), state management (server/client state separation, request caching, global store organization), interaction patterns (loading/empty/error three states, success feedback, confirmation modals, form validation, pagination), render performance (profiling-based diagnosis, memoization decisions, list virtualization). Use when the user asks to "write a component", "wrap a component", "design a component", "state management", "store", "request caching", "implement interactions", "three states", "form validation", "pagination", "render jank", or "performance optimization".
---

# Frontend Implementation (Frontend Development)

Components, state, and interactions are executed as one: classify states before picking tools, reuse components first, style with tokens; the three states are part of the feature.

## Stack binding (executed before this skill's workflow)

Read the project's tech-stack declaration and take exactly one of the four declaration states (treat as "no declaration" when none is provided):

- **Binding present**: load the stack skill directory the "binding" section points to — read `SKILL.md` first as the entry point, then load the `reference/` docs for the domains the current task touches, following the trigger conditions in its "reference files" routing table (one task reads only its own domain; never dump the whole directory); when a stack rule conflicts with this skill, the more specific stack rule wins, and unresolvable conflicts escalate to a human
- **Binding is "none" (generalized mode)**: execute with the neutral spec plus model generalization, based on the stack named in the `backend_stack` / `frontend_stack` declaration; the verification loop follows the declaration's "verification commands" section; label output "generalized mode (<stack name>, no binding skill)"
- **No declaration**: execute per this skill's neutral spec; label output "the project declares no tech stack"
- **Declaration unparseable**: report it as-is; never guess

## Workflow

### A. State management

#### 1. State classification (before component and interaction design)
| Type | Examples | Where it lives |
| :--- | :--- | :--- |
| Server state | lists, details, dictionaries | The request library's cache (TanStack Query-like, per the stack binding) — never copied into a store |
| Global client state | auth session, theme, permissions | Global store |
| Local UI state | modal open/closed, input drafts | Component-local state — never global |

- Never manually copy API response data into a global store (two sources of truth will always drift)
- Never use global state to pass values used only between parent and child

#### 2. Server state (request cache)
- Cache key: `{resource}:{parameter digest}`, e.g. `claims:list:status=approved&page=1`
- Invalidation: after a write succeeds, invalidate precisely the related keys (edit an order → invalidate that order's detail + the list); full cache clears are forbidden
- Set `staleTime` by data volatility (dictionaries long, transactions short)
- Optimistic updates only for low-risk operations; on failure, roll back and notify
- Request deduplication + automatic cancellation on unmount

#### 3. Global state
- Split slices by business domain; giant single objects are forbidden
- Normalize: store entities as byId maps + id arrays, not deep nested copies
- Mutate only through defined actions/methods; compute derived data with selectors — never store it redundantly

### B. Component development

#### 4. Reuse priority
```
1. The UI library's ready-made components (library and version bound by the project's stack declaration)
2. Shared component library (shared/)
3. New domain components (promotable to shared after review)
```
Search existing components before writing a new one; reinventing the wheel requires a stated reason.

#### 5. Component API design
- Typed props (exported TypeScript interfaces); required/optional explicit
- Event callbacks named `onXxx`; optional props have explicit defaults
- Form components support controlled usage (value + onChange) while also offering an uncontrolled default
- Allow pass-through of className/style/data-*
- More than 15 props → redesign (split or composition)
- Single responsibility: list/card families use compound components (`<Table><Table.Column /></Table>`); slots via children / slot props

#### 6. Styling and accessibility
- Styles reference design tokens only; hardcoded colors/spacing are forbidden
- Interactive components carry aria attributes and keyboard support; focus styles must never be removed

#### 7. Story and tests
- Shared components must have Stories: covering default/hover/focus/disabled/error states
- Core interactions (click, input, submit) have component tests: Testing Library behavior assertions, not implementation details

#### 8. Promotion and deprecation
- Source of truth: the "component library maintenance" section of the `design-system` skill (promotion: used by ≥ 2 domains + API review + Stories/tests complete; deprecation: `@deprecated` + migration notes, kept for one release cycle)
- Execution: before promoting, search existing shared/ components (see reuse priority) to avoid promoting a duplicate of something that already exists

### C. Interaction patterns

#### 9. Three states and feedback (parameter source of truth: `rules/guides/interaction-design-guide`)

The three states are part of the feature. The **patterns and parameters** (duration thresholds, pagination thresholds, element checklists, prohibitions) of the four states loading/empty/error/success, of confirmation modals, form validation, and pagination take the clause table as the single source of truth; implement and self-check against it item by item — inventing parameters from memory is forbidden; when the stack-binding skill's interaction conventions conflict, the more specific stack rule wins.

In-implementation decision heuristics (coverage judgments beyond the guide's clauses):

- **Four-question coverage**: every data display surface answers four questions first — what shows while loading / what shows with no data / what shows on failure / what feedback on success; any missing answer = incomplete; fill it in before building
- **One feedback channel**: one operation's success feedback goes through exactly one channel (toast or page state); stacking is forbidden
- **Empty-state split**: design "no data" and "no results for this filter" as two different needs; the latter must offer a clear-filters entry (the landing judgment of the clause table's empty-state clause)
- **Self-check against the table**: check item by item against the clause table before submitting

### D. Render performance (measure first, then optimize; never memoize on gut feeling)

#### 10. Diagnosing render problems
- Three-step triage: record with the framework's profiler → identify the components re-rendering unnecessarily → trace the state-change source; caching/memoization optimization without profiling data is forbidden
- Classify the jank: heavy single-frame computation (slow render) / unrelated components re-rendering repeatedly / long lists without virtualization — the three classes have different remedies; classify before acting

#### 11. Caching and memoization decisions
- Component caching and derived-computation caching apply only when one of these holds: profiling confirms unnecessary re-renders with measurable render cost; unstable props/dependency references cause cascading subtree re-renders
- Blanket memoization is forbidden (caching itself carries comparison and maintenance costs); cheap computations are not cached
- Concrete APIs and idioms follow the stack-binding skill's "performance" section (when a binding exists)

#### 12. Large-list virtualization
- Long lists rendering ≥ 200 rows (default threshold; lower it if measurements show jank) or rows containing heavy components → virtualized rendering (pick per the stack-binding ecosystem)
- Lists already volume-controlled by pagination/scroll loading do not need virtualization — control volume first, virtualize second

#### 13. Treating re-render sources
- Push state down: mutable state lives in the smallest subtree that needs it (aligns with §1 local UI state)
- Cache expensive derived computation (sorting/filtering large arrays); hoist constants unrelated to business state out of the component

## Boundaries (not in this skill)

- Accessibility-specific work (WCAG baseline audit) goes to `accessibility`; frontend project structure/routing/build goes to `frontend-architecture`; new design-token values go through the `design-system` list review (this skill only consumes tokens already on the list)
- The backend's companion exception architecture (error code definitions, retryability classification) belongs to the backend implementation skill; this skill only owns the user-side three-state presentation

## Hard rules

- State changes are traceable (DevTools wired in development; actions carry origin annotations); the global store's core mutation logic must have unit tests
- Bypassing state management to mutate internal objects directly is forbidden
- Copy-pasting components without extraction is forbidden; bypassing the component library to hand-assemble styles is forbidden; untyped props (any) are forbidden

## Reference files

- `rules/guides/component-design-guide.md` — full component design spec (primary document)
- `rules/guides/interaction-design-guide.md` — full interaction patterns and parameters (primary document)
- `rules/standards/frontend-standard.md` / `rules/standards/ux-standard.md` — upper-level constraints

## Default values

The ≤ 15 props redesign threshold and the ≥ 2 domains promotion rule are organization defaults (adjustable per project with a recorded note); interaction parameters such as three-state durations, pagination thresholds, and toast durations follow the shared clause table `rules/guides/interaction-design-guide` (see §C).
