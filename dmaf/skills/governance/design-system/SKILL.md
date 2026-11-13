---
name: design-system
description: Design system maintenance. Design token inventory management, component library maintenance, visual consistency. Use when the user asks for "design tokens", "design system", "adding colors/fonts", or "component library maintenance". Tokens are brand-related configuration; a new value must enter the inventory before use.
---

# Design System Maintenance

Every visual value comes from tokens; a new value enters the inventory first — using it before it is registered is forbidden.

## Workflow

### 1. Token inventory management
| Category | Contents |
| :--- | :--- |
| Color | primary-400/500/600, success/warning/error-500, neutral-50~900 |
| Typography | font-family-base/mono; heading-1/2/3; body-lg/base/sm; caption (font size / line height / font weight) |
| Spacing | space-1/2/3/4/6/8/12 (4/8/12/16/24/32/48px) |
| Radius | radius-sm/md/lg/full (4/8/12/9999px) |
| Shadow | shadow-sm/md/lg |
| State semantics | primary-600 Hover, primary-400 Disabled, success/warning/error-500 semantic feedback; the neutral scale neutral-50~900 carries backgrounds/text/borders |

- Tokens live at `design/tokens/`, in JSON format, under version control
- Adding/changing a token: enter the inventory first → sync the project's tokens → notify consumers

### 2. Component library maintenance (source of truth for promotion and deprecation criteria; consumers such as frontend-development cite this section)
- Layering: the UI library's ready-made > shared component library > new domain components
- Shared component promotion: used by ≥ 2 domains + API review + Stories/tests complete
- Stories for all component states: default/hover/focus/disabled/error
- Deprecation: @deprecated + migration notes, kept for one release cycle
- Visual regression: screenshot comparison of core components (baseline management)

### 3. Brand consistency guard
- Copy tone: success positive and concise, failure neutral and constructive, warnings clear and actionable, never blaming the user
- Wording: avoid technical jargon, avoid negative phrasing, avoid ambiguity, one concept one term
- primary is used only for primary actions and emphasis; semantic colors use success/warning/error
- Insert spaces between Chinese and digits/Latin text; use full-width punctuation

### 4. Responsive breakpoints
- xs < 640 / sm 640–767 / md 768–1023 / lg 1024–1279 / xl ≥ 1280 (default values)
- Mobile first; verify the layout at every breakpoint

## Boundaries (not in this skill)

- Token consumption in component implementation goes through `frontend-development`; this skill governs token inventory governance and the component library maintenance standards
- Brand tone and copy details are in §3

## Reference files

- `rules/guides/component-design-guide.md` — component library maintenance
- `rules/standards/ux-standard.md` — higher-level constraints
