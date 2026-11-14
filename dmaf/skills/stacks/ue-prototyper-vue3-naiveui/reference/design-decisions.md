# Design decisions and quality floors (the app's visual authority, the user's direction, built-in fallbacks)

The prototype's visual authority is the product itself. This file governs design questions the app has no ready answer for: the authority order, when a new visual direction may enter, the built-in floor rules, and the pre-delivery checklist. **Verify before acting on any design decision — never by feel.**

## 1. Authority order (never inverted)

1. **The app's real design system** — Naive UI component defaults, `themeOverrides` / `useThemeVars()`, the layout shell, existing pages' composition patterns. When the product has an answer, the prototype uses it, no exceptions — this is the precondition for review conclusions to transfer to production.
2. **The user's explicit direction** — "go dark", "align with marketing's style". An explicit direction overrides defaults but **never overrides authority 1's mechanism**: new colors / radii must enter `themeOverrides` first (through the theme mechanism); scattered hardcoding is not allowed; record `designDirection` in ue-spec and leave a manifest trace of "which external design intelligence was consulted and what was adopted".
3. **Built-in floor rules** (§3) — for the parts neither the app nor the user answered.

Three default knobs (when the user does not say otherwise): **density follows the app** (an enterprise admin console keeps its compact scale; no marketing-page whitespace aesthetics), **restrained motion** (micro-interactions 150–300ms; no scroll choreography or gimmicky layouts), **no gimmicky layout** (no style experiments like Brutalism / glassmorphism — the prototype answers functional questions).

## 2. Verification discipline (semantic results first)

- One question per lookup: first the semantic result (e.g. "where form validation error messages appear"), then the component implementation (the official docs of the matching Naive UI component); if both come up empty, retry once with a narrowed rewrite; if still nothing, say plainly "no authoritative answer found" and offer a fallback labeled as general advice — **never present unverified content as authoritative**.
- Verification entry point: the Naive UI official docs (`https://www.naiveui.com/zh-CN/components/<component>.md`, index `llms.txt`; offline, read the node_modules type declarations).
- Dataset / retrieval design intelligence (e.g. a ui-ux-pro-max-like skill installed in the user's environment) is consulted in only two scenarios: greenfield whole-shell needs a visual direction defined from scratch; the user explicitly asks to explore a new visual. Adopted values always land through the theme mechanism.

## 3. Built-in floor rules (the minimum when neither the app nor the user answered)

**Spacing and rhythm**

- 4/8px spacing system; hierarchy steps 8/16/24/32/48 — one spacing per hierarchy level.
- One content-width policy per page type: tables fill, forms cap around 640–720px; no arbitrary per-screen widths.
- Body line length 65–75 characters (about `max-width: 40em`); line-height 1.5–1.75; body text minimum 14px (desktop admin; 16px on mobile).

**Color and contrast**

- Body contrast ≥ 4.5:1, large text / UI boundaries / meaningful icons ≥ 3:1 — measured against the **composited background** (also test overlays and under scrims, not just the flat page).
- Color is never the only state signal — pair it with an icon / text / weight.
- If the app has a dark theme, prototype pages must hold up in both themes: borders / dividers visible in both, interactive states (hover / pressed / disabled) distinguishable in both, scrim opacity measured on the composited result; if the app has no dark theme, do not invent one.
- Light/dark screenshot reviews follow the stack skill's dual light-dark baseline awareness (the review package reminds the reviewer to toggle the theme once through).

**Icon discipline**

- No emoji as structural icons; icons come from the app's existing set (greenfield xicons `@vicons/*`, brownfield keeps the project's icon library), rendered via `<n-icon :component="Icon" />`.
- Uniform stroke width within one visual layer (1.5px or 2px), uniform size steps (e.g. 16/20/24); no mixing filled and outline styles within one hierarchy.
- Meaningful standalone icons get `aria-label` or a text alternative; decorative icons accompanying visible text get `aria-hidden="true"`.
- Hand-drawn SVG must carry a `viewBox`.

**Motion**

- Micro-interaction durations 150–300ms with one scale across the site (do not reuse a single duration for every transition); exits are faster than entrances.
- Motion only signals state changes (hover / expand / feedback landing); purely decorative animation is forbidden; support `prefers-reduced-motion` (branch on `@vueuse/core`'s `usePreferredReducedMotion`).
- Feedback duration matches the feedback channel: light notices go through `message` (the stack skill fixes the scale at 2000ms); animation never substitutes for feedback.

**Form UX**

- Labels always visible (`n-form-item`'s `label`); never rely on a placeholder as the label.
- Validation timing matches interaction: validate single fields on blur (`trigger: 'blur'`), full validation on submit; errors sit below the field (Naive UI's built-in feedback path), and long complex forms focus the first errored item after a failed submit.
- Required marks explicit (`*` or copy); common fields carry `autocomplete` attributes (name / tel / email).
- Submit button loading → success (`message.success`) / failure (error below the field + retryable) full-chain feedback; double submits are forbidden during validation and submission (existing stack-skill convention).
- Edit scenarios prevent redundant entry: backfill existing values into the form — never retype from zero.

**Content realism (paired with mock volume realism)**

- Thousand separators on numbers, currency and precision on amounts, localized date formats or relative time — mock data follows the same rules (handler rules in reference/fence-mocks.md).
- Truncation has an escape: ellipsis + hover tooltip or expansion on the detail page, with the full text reachable by keyboard / screen reader; only unpredictable-length values (usernames, titles) get nowrap + truncation; fixed-structure text is never truncated.
- Empty-state copy distinguishes "no data" from "no results for this filter" (the latter gets a clear-filters action); placeholder data uses realistic business samples (real copy in the product's language, no lorem / test123).
- Pages are the product's final cut: present only the product content the end user will see; review comments, decision notes, to-be-confirmed markers, and other discussion content must not enter the page DOM (they live in the review layer and the ue-spec / feedback exports) — notes for reviewers go in the review package, not on the page.

**Page composition defaults (when no existing page can be referenced)**

- Lists over cards, cards over freeform layout — an enterprise review audience reads tables fastest.
- One primary action per view (top right of the page or a bottom action bar, following the app's conventions); destructive actions separated + double-confirm.
- Filters above the list; counts / aggregates near the list header; pagination at the bottom — copy the nearest existing page first for all of it.
- Density follows the app (enterprise admin compact scale).

**Accessibility floors**

- Interactive elements keyboard-reachable with visible focus (focus returns when an overlay closes); dialog focus trapped.
- Click targets ≥ 40px (44px for touch); icon buttons must have an accessible name.
- Heading levels do not skip (`h1 → h2`); form fields bound to labels, errors announced by screen readers (prefer the component library's built-in path).
- Popovers / sticky elements never cover keyboard focus.

**Honest prototype UI**

- An always-on PROTOTYPE banner (mode, page id, review link — clicking appends `?uer=1` to the current URL, mock-channel badge) — loud enough that a screenshot cannot pass for the product, without disrupting review.
- Feedback channels and the three loading layers (`n-skeleton` first paint / button `loading` / route loadingBar) all use the component library's ready-made parts — no homemade spinners / toasts.

## 4. Token discipline (three layers, each in its place)

- **Consumption layer**: no bare colors / radii / font sizes in business components — theme values come through `useThemeVars()` (script) or theme CSS variables + `n-el` (style); component visual customization goes through `theme-overrides` and the component's own props (stack-skill red line).
- **New-value layer**: new colors / radii / font sizes from the user's explicit direction enter `themeOverrides` first (common or component-level, with the reason noted) before being referenced — new values join the mechanism, never scattered points.
- **Temporary self-build layer**: rung-3 components consume only the consumption and new-value layers; all style values come from the theme; registered in `gaps.md`.

## 5. Pre-delivery visual and accessibility checklist (the fine print of the Step 4 gate)

- [ ] Icons: no emoji structural icons; one style per layer; meaningful icons contrast ≥ 3:1; standalone icons have accessible names
- [ ] Contrast: body ≥ 4.5:1, boundaries ≥ 3:1, passing in both light and dark themes (not inferred)
- [ ] Focus: keyboard-walk the main flow once; focus visible, returned after overlays close, never covered
- [ ] States: color never the only signal (disabled / selected / error all carry a second signal)
- [ ] Motion: one 150–300ms scale; `prefers-reduced-motion` does not break usability
- [ ] Content: thousand separators / localized dates / truncation has an escape / empty states distinguish the two semantics
- [ ] Tokens: no bare colors / radii (`useThemeVars` / theme CSS variables coverage); new values already in `themeOverrides`
- [ ] Composition: one primary action, destructive separated with confirmation, density follows the app
