# Naive UI theming and design tokens

> Applies to Naive UI 2.37+. When project dependencies below the baseline are detected (brownfield mode), this document's "forbidden / required"
> items downgrade to suggestions; the compatibility mapping in `reference/brownfield.md` governs (verify the style-conflict solution per the project version).

Required reading when a task touches theming / design system / style adjustments.

## The single customization entry

- Application root `n-config-provider`: `theme` holds only the theme object (`null` for light, `darkTheme` for dark); customization variables go through `theme-overrides` (type `GlobalThemeOverrides`); overriding component internal style classes (`:deep(.n-*)`) or global CSS hacks are forbidden
- Global tokens go into `common` (e.g. `primaryColor`, `borderRadius`, `fontFamily`; key names per the `GlobalThemeOverrides` type hints, not Ant Design's `colorPrimary`); component-level overrides (`Button: {…}` in overrides or a component's own `theme-overrides` prop) are used only when `common` cannot satisfy the need, with the reason noted; component level takes precedence over global
- Business components read theme variables via `useThemeVars()` (called in setup of a descendant of `n-config-provider`); bare colors / bare radii in business components are forbidden

## Dark theme

- Light-dark switching swaps only the `theme` reference (`null` ↔ `darkTheme`) and its accompanying `theme-overrides`; business code is unaware of it; the toggle goes into preferences
- Place `<n-global-style />` inside `n-config-provider`: on theme switch it syncs the body's background color and fonts (the provider cannot reach the body outside itself)
- The feedback channel's discrete singleton binds the same theme source via `configProviderProps`, otherwise message / dialog do not follow light-dark switches (see "Feedback channel implementation" in reference/components.md)
- Build separate screenshot baselines for the light and dark themes

## Design token mapping

- Design token naming and visual value sources follow the neutral design-system skill; this document maintains only the "design token → `GlobalThemeOverrides` key" mapping table, and mapping changes go through design-system review
- When own elements need theme values, wrap them with `n-el` (`tag` designates the element) and reference the theme CSS variables directly in styles (e.g. `var(--primary-color)`, follows light-dark); read values in script with `useThemeVars()`; everything else references `shared/` style constants

## Style conflicts and overlays

- When a global reset is introduced (e.g. Tailwind preflight), follow the official "potential style conflicts" guidance and append `<meta name="naive-ui-style" />` to the end of `head` before `mount()`: Naive UI styles are inserted before that meta, i.e. after the reset, and are no longer overridden by it; when Naive UI's built-in global preflight is not wanted, set `preflight-style-disabled` on `n-config-provider` (verify the meta approach and `preflight-style-disabled` per the project's Naive UI version; brownfield is not forced to adopt it and keeps the project's existing global style approach — see brownfield.md)
- Style boundaries of teleported overlays are in reference/components.md "Overlays and modals"

## Version anchors

- The Naive UI version is subject to the root package.json; upgrading theme-related versions must pass all screenshot baselines
