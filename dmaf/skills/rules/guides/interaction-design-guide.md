---
name: interaction-design-guide
level: L2
domain: interaction-design
description: Interaction pattern implementation-level conventions covering the concrete modes and parameters for loading/empty/error/success states, confirmation dialogs, forms, and pagination. Load when implementing or reviewing interactions.
owner: Design Team
based_on: [ux-standard, frontend-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: every six months
---

# Interaction Design Work Specification

> Color / typography tokens are carried by the design system.

## 1. Loading State

- **Standard practice**: use a skeleton screen; ❌ a full-screen spinner is forbidden
- **Loading duration**:
  - < 200 ms: no loading state
  - 200 ms – 1 s: show the skeleton screen
  - \> 1 s: skeleton screen + loading copy (e.g., "Loading, please wait")
- Local operations use an in-button loading state, not a full-page skeleton

## 2. Empty State

- **Elements**: an icon (optional) + a one-line explanation + a primary action button (e.g., "Create your first item")
- ❌ Blank pages are forbidden
- Distinguish "no data yet" from "no results for the current filter": the latter offers a "clear filters" action

## 3. Error State

- **Elements**: an error icon + friendly copy (no technical details) + a retry button or a back button
- ❌ Showing raw technical stack traces or error codes is forbidden; error codes exist to support troubleshooting (an entry point showing the trace_id is enough)
- Form validation errors display below the input, in the `error-500` color

## 4. Success State

- **Standard practice**: a toast + auto-dismiss
- **Duration**: 2 seconds; **position**: top center or top right
- ❌ Interrupting the user with a dialog to announce success is forbidden

## 5. Confirmation Dialogs

- **Use case**: destructive operations (delete, clear, irreversible)
- **Elements**: an explicit statement of the consequence + a primary action button (danger color) + a cancel button (focused by default)
- ❌ Frequent confirmation dialogs are forbidden (for frequent dangerous operations, replace them with secondary verification or a recycle-bin mechanism)

## 6. Form Interactions

- **Validation timing**: validate a single field on blur (no real-time interruption); validate everything on submit
- **Error display**: below the input; **required marker**: `*` after the label
- Show a success icon when validation passes; disable the submit button while submitting to prevent double clicks

## 7. Navigation

- Breadcrumbs: shown when depth > 2; second-level pages must have a way back
- Frequent navigation uses a sidebar; the current item is highlighted (`primary-500`)

## 8. Pagination

- Data volume < 20: no pagination; 20–100: frontend pagination; > 100: backend pagination (pagination parameter names follow `rules/guides/api-design-guide`)
- Pagination controls: previous / page numbers / next; support changing the page size (10 / 20 / 50)
- After a pagination change, scroll back to the top of the list
