# Mantine 9 theming and design tokens

> Applies to Mantine 9.7. The `light` variant at Mantine 8 and below still has opacity semantics; do not apply this document's solid-color mapping
> (see `reference/brownfield.md`).

## The single customization entry

- Theming goes only through `createTheme` passed to `MantineProvider`; **overriding component internal class names or global CSS hacks are forbidden**
- Write global tokens into the theme's `colors` / `primaryColor` / `primaryShade` / `radius` / `defaultRadius` etc.
- Component-level tokens (`theme.components`) are used only when globals cannot satisfy the need, with the reason noted
- **v9 defaults `defaultRadius: 'md'`**: greenfield projects keep it as-is, no need to set it back to `sm`

## CSS variables and dark mode

- Mantine 9 component styling is based on CSS variables (`--mantine-*`); custom styles consume these variables first rather than hardcoding
- The `light` variant in v9 uses **solid-color CSS variables** (no longer opacity-based); new projects map design tokens per v9's solid-color semantics
  (Mantine 8 and below remain opacity-based; brownfield must not apply this)
- Light-dark switching: `useMantineColorScheme` + a color scheme manager; mount the color-scheme script guarding against white flash (FOUC) at the application entry (the standard snippet from the Mantine docs)
- **Build separate screenshot baselines for the light and dark themes**

## Design token mapping

- Design token naming and visual value sources follow the neutral design-system skill
- This document maintains only the "design token → `createTheme` key / CSS variable" mapping table
- **Bare colors / bare radii in business components are forbidden** — always reference the theme, CSS variables, or `shared/` style constants
- **The style color prop is uniformly `c`**: color values go through theme color scales (`c="blue.6"`), no hardcoded hex
- Color / size resolution inside wrapped components uses `getThemeColor` / `variantColorResolver` / `getSize` / `getRadius`,
  the same red line as this section (conventions in "Wrapping Mantine-style components" in reference/components.md)

## Version anchors

- Pinned to **Mantine 9.7.x**, subject to the root `package.json`
- **v9 requires React 19.2+**; this skill's version matrix already satisfies it, no extra check needed
- Upgrading theme-related versions must pass **all screenshot baselines** (light and dark separately)
