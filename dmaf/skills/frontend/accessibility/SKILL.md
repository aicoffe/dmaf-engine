---
name: accessibility
description: 'Accessibility implementation. WCAG 2.1 AA baseline: keyboard access, contrast ratio, screen-reader compatibility, form accessibility. Use when the user asks for "accessibility", "a11y", "WCAG", "keyboard navigation", or "screen reader".'
---

# Accessibility Implementation

Accessibility is a baseline requirement, not an option. Target WCAG 2.1 AA.

## Workflow

### 1. Perceivable
- Contrast ratio: body text ≥ 4.5:1; large text (≥ 18pt) and graphics ≥ 3:1 (verify with the token palette)
- Informative images have alt text; decorative images have empty alt
- State must not be conveyed by color alone (success/error must pair an icon or text)
- Pages declare the lang attribute

### 2. Operable
- All interactive elements are keyboard-operable, with no keyboard traps
- Focus styles must never be removed; Tab order follows visual order
- Clickable areas ≥ 44×44px
- Repeated navigation offers a "skip to main content" link

### 3. Understandable
- Every input is associated with a label; error messages are associated via aria-describedby
- Validation errors give correction suggestions; the same function uses the same icon and wording site-wide

### 4. Robust
- Semantic HTML first: use button/nav/table instead of simulating them with div
- Dynamic content announces via aria-live; modals manage focus (move in on open, return on close)

### 5. Verification
- CI: axe / Lighthouse automated checks in the pipeline; blocking-level violations must not merge
- Manual: keyboard walkthrough of core pages + screen reader (NVDA/VoiceOver) smoke test, at least once per iteration

## Boundaries (not in this skill)

- The routine aria baseline within component implementation is executed with `frontend-development`; this skill owns the dedicated WCAG AA audit and verification
- Palette adjustments for insufficient contrast go through `design-system` (this skill verifies with tokens; it never edits the token inventory)

## Hard rules

- Accessibility never yields to visual aesthetics (fix insufficient contrast by adjusting the palette; never lower the standard)
- Automated checks passing ≠ compliant; the manual walkthrough must not be skipped

## Reference files

- `rules/standards/ux-standard.md` / `rules/standards/frontend-standard.md` — higher-level constraints
