# Review loop — activation, shepherding, export, consumption, apply, wrap-up

The review runtime (`assets/ue-inline-review.js`, vendored to `proto/review/`) replaces "describing that button in the top-left corner in chat" with "click the button, write down what is wrong". Comments anchor to machine-precise elements and export as structured JSON for precise AI consumption. The runtime is a zero-dependency IIFE with UI class names prefixed `uer-`; it takes no screenshots, makes no network calls, and does not conflict with Naive UI's Teleport overlays (elements mount directly at the body top level).

## 1. Activation (the only injection path)

- The runtime lives in `proto/review/` (vendored, see reference/fence-mocks.md); ProtoLayout dynamically imports it under `?uer=1` — the review bootstrap never enters the business entry.
- SPA note: **the review session is global** — the runtime listens to the History API (the substrate beneath page switches in both vue-router and react-router), so client-side route changes automatically reload the current page's comments, pins, and export naming; pick mode and panel state persist across pages with no refresh. Page identity resolves first via the `__UE_REVIEW_META__.pages` registry baked into ProtoLayout (pageSource and ueControlId anchor naming thereby align per page), with unregistered routes falling back to the last route segment.
- Export landing: after the browser downloads `comments-<page>.json`, place it in `proto/feedback/` (or hand it back to the AI to place).

## 2. Shepherding a review session (walk it through in the user's language)

1. Open the page (dev URL + `?uer=1`, or click the "Review" link on the page banner; to demo the error state, click the banner's "mockError" link).
2. Press **M** to enter pick mode (the review pill defaults to the top right, **draggable anywhere** — when it covers a target element, just drag it aside; the position is remembered in localStorage and survives refresh / page switches). The page turns inert: clicks are intercepted, links stop navigating, and keystrokes in inputs do not leak to page hotkeys.
3. Click any element → write the comment → **Enter** saves, **Esc** exits.
4. Sidebar (pill menu): list / edit / resolve / wontfix / delete comments.
5. Reviewing can continue across pages (page switches follow automatically, panel title shows the current page name); when done, **export the JSON page by page** — the browser downloads `comments-<page>.json`; place it in `proto/feedback/` (or hand it back to the AI to place).

localStorage is only a refresh-protection draft; the exported JSON is the single source of truth — never "merge from localStorage" across sessions.

## 3. Export format (self-describing; fields add-only)

File `proto/feedback/comments-<page>.json`, schema id `ue-inline-review/comments@1` (frozen format):

```json
{
  "schema": "ue-inline-review/comments@1",
  "page": "order-list",
  "pageSource": "proto/pages/order-list.vue",
  "exportedAt": "2026-11-03T08:00:00.000Z",
  "comments": [
    {
      "id": "c-xxx",
      "status": "open | resolved | wontfix",
      "category": "unclassified | visual | rule | new-requirement | question",
      "text": "the reviewer's exact words",
      "createdAt": "...",
      "anchor": {
        "css": ".n-data-table tbody tr:nth-of-type(2) td.amount",
        "xpath": "//*[@id='order-table']/tbody/tr[2]/td[4]",
        "unique": true,
        "tag": "TD", "classes": ["amount"], "textSnippet": "≤80 chars",
        "boundingBox": { "x": 0, "y": 0, "width": 0, "height": 0 },
        "ueControlId": "UE-TD-003"
      }
    }
  ]
}
```

Anchoring rules: authored `id` → unique class-name combination → structural `nth-of-type` path from the nearest authored ancestor; CSS and XPath are recorded symmetrically; **uniqueness verified at capture time** (`querySelectorAll` count === 1). A non-unique anchor exports `unique: false` — never silent ambiguity; fall back to xpath / textSnippet / boundingBox or ask the reviewer again. `ueControlId` is best-effort; missing metadata never blocks export.

## 4. Consuming comments — triage tree (strict order)

Start from the JSON export (diff against the previous round's export when one exists). Every comment must be triaged and have its `status` updated; no comment may be silently dropped.

1. **visual** — single-element look: color / spacing / size / copy → change the prototype page. With a theme variable available, go through the theme variable (`themeOverrides` key or `useThemeVars()`) — never hardcode around the Token; merge multiple edits to the same element before acting.
2. **rule** — validation, interactions, state transitions → change the interaction code inside `proto/**` (this is what the prototype exists to test), and **revise the matching ue-spec rows in the same batch of changes** — spec and pages never fork unknowingly; when a repo spec contract exists and contradicts, take the spec revision channel and log it in `openQuestions` meanwhile.
3. **new-requirement** — beyond the confirmed spec's scope → back to spec / intent, or an explicit scope confirmation with the user when there is no artifact chain; only after confirmation does it get a spec row. Never absorb a feature request "in passing inside the file".
4. **question** — clarify with the reviewer first, then re-triage into 1–3 or close.

Ambiguous case: a comment reads visual but implies a rule (e.g. "this destructive button is too small"), triage by the deeper need and say so in the report.

## 5. Apply, verify, close the round

- Apply edits at the exact spot the anchor bundle points to; many small edits beat one big reshuffle.
- For comments involving visual changes, spot-check against the `reference/design-decisions.md` checklist after applying (contrast / icon discipline / no scattered tokens) — the reviewer's "make it look good" must not come at the cost of the floors.
- Re-verify after applying: dev server still clean, pages still complete (state matrix), footprint guard still passing (see SKILL.md verification commands).
- After re-verification, have the reviewer refresh (`?uer=1`) to confirm; update each `status`, and bump the manifest's `reviewRounds` +1.
- The round closes when every comment is `resolved` or `wontfix` (reason stated in the body); when a repo spec contract exists, rule-class conclusions must have reached their spec entry point.

## 6. Wrap-up checklist

- [ ] The review bootstrap lives only inside `proto/**`; zero pastes, zero script tags in business artifacts
- [ ] Every exported comment carries css + xpath + the unique marker + fingerprint (ueControlId when metadata exists)
- [ ] Every comment triaged (in 1→4 order), applied or answered, `status` updated
- [ ] Every rule-class conclusion revised the matching ue-spec rows; substantive revisions flipped `status` back to `draft` pending re-confirmation
- [ ] manifest `reviewRounds` bumped; page list updated (if pages were added); new rung-3 components registered in `gaps.md`
- [ ] Footprint guard and project gates (lint / typecheck / build) still all green
