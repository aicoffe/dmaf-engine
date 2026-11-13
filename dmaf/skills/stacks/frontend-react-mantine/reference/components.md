# Mantine 9 component conventions

> Applies to React 19 / Mantine 9.7 / TanStack Query v5 / zod 4. When project dependencies below the baseline matrix are detected
> (brownfield mode), this document's "forbidden / required" items downgrade to greenfield suggestions; the compatibility mapping in
> `reference/brownfield.md` governs.

## Selection priority

1. **Ready-made Mantine 9.7 components**
2. `shared/` wrapped components
3. New in-domain components (promotable to `shared` after review; wrapping conventions in "Wrapping Mantine-style components" below)

- For complex tables, evaluate `mantine-datatable` first (a controlled wrapper built on the Mantine Table)
- **Introducing a second UI library is forbidden**; when Mantine lacks a capability, first check `createTheme` / CSS variable customization, then consider building your own
- Combobox decision table (check before building any custom dropdown; stop at the first row that satisfies, top to bottom):

| Need | What to use |
|---|---|
| Standard single / multi select, autocomplete, tag input | `Select` / `MultiSelect` / `Autocomplete` / `TagsInput` (all support `renderOption`) |
| Any element (e.g. a button) with a dropdown attached, no input box | `ComboboxPopover` (data-driven: `data` / `value` / `onChange` / `searchable` / `multiple`) |
| Hierarchical tree options | `TreeSelect` (`mode` single / multi / checkbox parent-child cascade) |
| Hundreds of options or more | `Combobox` + `useVirtualizedCombobox` virtual scrolling |
| Custom dropdown content (tabs, create item, header/footer) | Build with the `Combobox` primitives |

## Provider mounting (set up once at the application entry)

- Application root: `MantineProvider` (theme) → `Notifications` → `ModalsProvider`
- Notifications via `notifications.show()`; confirmation dialogs via `modals.openConfirmModal`
- **Business code must not raise native `alert` / `confirm` / `prompt`**
- `notifications.show` must be called after `Notifications` is mounted; in lazily loaded routes, avoid firing notifications before the providers are mounted

## Component usage conventions

- **Feedback semantics split**: light notices `notifications`, blocking confirmations `openConfirmModal`, inline errors `Alert` — pick by semantics, no mixing
- **Destructive operations** must be double-confirmed via `openConfirmModal`, with `confirmProps` flagging it red (`color="red"`)
- **Layered loading states**: page-level `Loader` / `Skeleton`, button-level `loading` prop; global and page-level states are not interchangeable
- **Empty and error states** land per the neutral interaction three-state spec (`EmptyState` wrapped in `shared/`)
- **The style color prop is uniformly `c`** (Mantine 9 only; v7/v8 `color` still works — do not bulk-rewrite legacy projects, see
  brownfield.md): `<Text c="red">` ✅; `<Text color="red">` ❌ (removed in v9)
- Custom styles consume `--mantine-*` CSS variables first; bare colors / bare radii are forbidden
- The Combobox family (`Select` / `MultiSelect` / `Autocomplete` / `TagsInput`) supports `floatingHeight="viewport"`; use it directly when dropdown height is constrained

## Combobox primitives idioms

- Pair the store open/close callbacks: `useCombobox({ onDropdownOpen: () => combobox.selectFirstOption(), onDropdownClose: () => combobox.resetSelectedOption() })`
- Button-triggered: wrap `InputBase component="button"` in `<Combobox.Target targetType="button">`; input-triggered uses the default
  `<Combobox.Target>`; multi-select chips + a separate input use `<Combobox.DropdownTarget>` + `<Combobox.EventsTarget>`
- **selected ≠ active**: selected is the keyboard-highlighted item (`data-combobox-selected`); active is the item holding the current value
  (the `active` prop, unstyled by default — render your own check icon) — use active to reflect the current value; do not mix the two
- The dropdown is target-width by default (`width="target"`), rendered in a portal, and is not clipped inside Modal / Drawer; inside a Modal, Escape
  closes in two stages (dropdown first, then Modal) with no extra code
- Other interactive elements inside the dropdown (buttons in `Combobox.Header`, retry links) get
  `onMouseDown={(e) => e.preventDefault()}`, preventing input blur from closing the dropdown by mistake
- **Free-text search boxes must not call `selectFirstOption()`**: the user must be able to submit text with no matches; highlighting the first item hijacks Enter
- Async search: `Combobox.Search` + `use-debounced-value` debounce + loading / error dropdown states
- `Combobox.ClearButton` is a mouse shortcut with `tabIndex={-1}`; in button-triggered scenarios, clear via `rightSection` holding
  `<CloseButton aria-label="Clear" />` (keyboard accessible)

## Wrapping Mantine-style components (standard for shared/ and in-domain components)

- When a custom component is truly needed, follow the factory pattern, isomorphic with Mantine: `factory()` is the default; use `polymorphicFactory` sparingly
  (high TS overhead, slower completions; the lightweight alternative is `renderRoot`); use `genericFactory` only when props change with generics (e.g. `multiple`)
- Skeleton: `useProps` resolves defaults → `useStyles` gets `getStyles` → `Box {...getStyles('root')} {...others}`;
  **`ref` is a regular prop**, reaching the root naturally via `...others` (React 19) — do not handle it yourself
- variant / size passed to `Box` automatically produce `data-variant` / `data-size` attributes; branch in the CSS module with
  `&[data-variant='outline']`; define size tokens as CSS variable groups and select with `getSize(size, 'token')`
- Color resolution uses `getThemeColor(color, theme)` and `theme.variantColorResolver(...)` (returns the full bg / hover /
  color / border set, following the theme and light-dark); **bare colors are forbidden** (same red line as theme-tokens.md)
- Dynamic styles go through CSS variables: `createVarsResolver` produces `--x-*` variables, with fallbacks in CSS
  (`var(--x-radius, var(--mantine-radius-default))`); a resolver returning `undefined` leaves the variable unset and the fallback applies
- Theme registration `MyComponent.extend({ defaultProps, classNames, styles, vars })`; `withProps` is a runtime preset that
  **does not change types** (required stays required — declare props meant for presets as optional); subcomponent registration inside `theme.components` carries no dot
  (`MyCardSection: MyCard.Section.extend(...)`)
- Compound components (the `Card.Section` shape): subcomponents via `staticComponents` + `compound: true`; subcomponents are excluded from theme
  classNames / styles / vars and cannot use `vars` (single-item variables passed via props); **collecting subcomponents with `Children.map` is forbidden**
  (fragments / conditional rendering break it); use `createSafeContext` to let subcomponents self-register

## Forms (useForm)

For the full form conventions (uncontrolled mode, binding details, validation, list fields, the dirty baseline, server errors, cross-component usage),
see `reference/forms.md`, required reading before starting form tasks. Red-line digest:

- `useForm` with explicit `mode: 'uncontrolled'`; every input `key={form.key('path')}`
- The in-stack validation default is zod + `schemaResolver(schema, { sync: true })` (Mantine 9 + zod 4 only; type source shared with `z.infer`);
  in greenfield `zodResolver` no longer exists — do not reference it (brownfield keeps the project's current resolver,
  see brownfield.md)
- Submit button `loading={form.submitting}`, no submit when validation fails; server field errors written back per dotted path with `form.setErrors`
