# UE spec — schema, contract precedence, confirmation and revision protocol

`proto/ue-spec.yaml` is the prototype's design contract: which pages, for whom, in which states, mapped to which real components, wired to which mocks. It is not the repo's requirements spec.md — it is the working contract within the prototype's scope: implement the spec contract when one exists, stand in for it when not. Spec first, revisions enter the spec first: this file keeps ten pages coherent and iteration cheap.

## 1. Contract precedence (never inverted)

1. The repo spec (produced by requirement-analysis) contains a page list / interaction contracts / acceptance criteria → they are the contract; ue-spec references it (`contract: spec <section>`) and implements it item by item; conflicts go through spec revision — never silent divergence.
2. intent.md exists → its expected outcomes and design intent frame the ue-spec; where ambiguous, go back to whoever raised the requirement.
3. Only raw material (PRD, user stories, flow text, old Axure exports, a one-line requirement) → interview until pages / scenarios / users / decisions to settle / data / constraints are all clear, then write the spec.

## 2. Confirmation protocol (soft, never skipped)

After drafting: post the spec path, summarize the page list + state matrices + key assumptions, and **invite confirmation before generating pages**. User confirms → `status: confirmed`; user defers ("let it run first") → continue with `status: draft` and note it in the review package. A spec never posted is a process failure; a posted spec later deferred is not. Every review round allows revisions; a substantive revision after confirmation flips `status` back to `draft`.

## 3. YAML Schema

```yaml
version: 1
status: draft | confirmed
contract: "spec page-list section" | "intent.md" | null   # the contract this spec implements
sourceMaterial: "PRD order-center v2 / Axure order flow / interview 2026-11-03"
pages:
  - id: order-list
    route: /proto/order-list
    title: Order list                     # human-readable; in the user's language
    roles: [buyer, ops]                   # drives the no-permission state design
    states: [success, loading, empty, error, disabled, no-permission]
    edgeData: [long-text, overflow, max-rows]   # the edges the mock data must stress
    responsive: desktop                   # the product's own scope, not an aspiration
    components:                           # component mapping, ladder-resolved, source recorded per entry
      - ref: "order status badge"         # UE element / business semantics
        component: NTag                   # the chosen real component
        source: naive-ui                  # origin (ladder rung)
        props: "type=statusEnum mapping"  # key props / variants
        note: pending-payment/paid/cancelled → default/info/success enum mapping
      - ref: "order table"
        component: NDataTable
        source: naive-ui
        props: "remote pagination + row-key; column render NButton via h()"
      - ref: "advanced filter bar"
        component: ProtoFilterBar         # ladder rung 3 → must appear in gaps.md
        source: proto/components
        props: ""
        note: temporary self-build until the design system adds the component
    interactions:
      - trigger: click row
        action: navigate /proto/order-detail/:id
      - trigger: click cancel-order
        action: n-popconfirm double-confirm → mock DELETE → refresh list (feedback via the project's feedback channel)
    flows: ["place-order main flow", "cancel-order branch"]
    mock:
      endpoint: /api/orders
      handler: proto/mocks/order-list.ts
      toggles: "?mockError=1 error state; delay=600ms"
flows:                                # cross-page flows (if any)
  - id: place-order
    steps: [order-list → order-detail → submit → message.success → back]
openQuestions:                        # design decisions awaiting a human call
  - "default list order: placement time or status?"
```

Field discipline:

- `states` enumerates the fixed set `success, loading, empty, error, disabled, no-permission` — per-page subsets are allowed, with the omission justified in `note`; every listed state must be demonstrable (hard rule "state matrix complete"). Every list page has at least success / loading / empty / error; every form page has at least default / validation-failed / submitting / saved.
- `components.source` values are the ladder rungs: the project business-component namespace, `naive-ui`, `proto/components` (rung 3 — every entry needs a matching gaps.md line; this is the audit chain).
- A new visual direction explicitly requested by the user (palette / style / density) is recorded in a spec-level or page-level `designDirection` field; adopted values must land through the theme mechanism (`themeOverrides` / `useThemeVars()`), and be mirrored into the manifest — authority order, default knobs, and the landing mechanism are in `reference/design-decisions.md`.
- `openQuestions` is the parking lot for decisions the prototype must not make on the user's behalf; they resurface in the review package.

## 4. Component mapping table (human-readable view)

Generate this table from the YAML when delivering the review package — reviewers read tables faster than YAML:

| UE element | Business semantics | Component | Source | variant/props | Note |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Primary button | Submit order | NButton | naive-ui | type=primary, loading | feedback via the feedback channel |
| Order status | Status badge | NTag | naive-ui | type=enum mapping | pending-payment/paid/cancelled |
| Data list | Paginated table | NDataTable | naive-ui | remote + row-key | column render uses h() |
| Advanced filter | Multi-condition filtering | ProtoFilterBar | proto/components | — | temporary self-build, see gaps.md |

## 5. Revision discipline

- **Rule**-class conclusions from a review round (interactions, state transitions, component choices) revise the matching ue-spec rows and the pages in the same batch of changes — spec and pages never fork unknowingly.
- **New requirement**-class conclusions never enter as "a spec line added in passing"; go back to spec / intent or get an explicit scope confirmation from the user first — only then does a spec row exist.
- When a repo spec contract exists and a review conclusion contradicts it, ue-spec does not absorb the contradiction quietly — send it through the spec revision channel and log it in `openQuestions` meanwhile.
- Keep `status` honest: a substantive revision after confirmation flips it back to `draft` until it is confirmed again.
