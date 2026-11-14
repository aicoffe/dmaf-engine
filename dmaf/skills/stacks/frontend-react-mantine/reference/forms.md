# Mantine form conventions (@mantine/form, Mantine 9.7)

> Applies to Mantine 9.7 / zod 4. When project dependencies below the baseline matrix are detected (brownfield mode), this document's
> "forbidden / required" items downgrade to greenfield suggestions; the compatibility mapping in `reference/brownfield.md` governs
> (form mode and resolver follow the project as-is).

Distilled from the official Mantine agent skill (mantinedev/skills, Mantine 9.x), condensed to red lines and standard styles per this stack's conventions.
Read this file before starting any form task; never guess at an API you are unsure about — verification entry points at the end of this file.

## Mode: uncontrolled is the in-stack default

- `useForm` **declares `mode: 'uncontrolled'` explicitly** (the officially recommended mode: values live in refs, typing does not re-render the form; the default is
  `'controlled'` — omit it and you get controlled. Brownfield follows the project's existing mode, see brownfield.md).
  Generics are inferred from `initialValues`; no explicit `<Values>` needed.
- **Every input must carry `key={form.key('path')}`**: without it, `setFieldValue` / `setValues` / `reset` will not update the input.
- Under uncontrolled, `form.values` **does not update while typing**: read snapshots in event handlers with `form.getValues()`; for render-time
  conditional visibility / linkage use `form.useWatchValue('path')` (nested paths like `members.0.role` supported).
- `form.errors` / `form.isDirty()` / `form.submitting` / `form.validating` are React state in both modes,
  usable directly in rendering.

```tsx
const form = useForm({
  mode: 'uncontrolled',
  initialValues: { email: '', name: '' },
  validate: schemaResolver(schema, { sync: true }),
});
<TextInput label="Email" key={form.key('email')} {...form.getInputProps('email')} />
```

## Binding details (getInputProps)

- Default is the input type; Checkbox / Switch use `{ type: 'checkbox' }`; a standalone Radio without a Group uses
  `{ type: 'radio', value }` with a `key` on each.
- Components without an `error` prop (Slider / RangeSlider / Rating / SegmentedControl / Chip.Group) pass
  `{ withError: false }` and render errors themselves (`form.errors.path` + `Text c="red"`).
- **`required` adds the native attribute, triggering browser submit blocking** (bypassing `form.onSubmit`); for the visual required asterisk use `withAsterisk`.

## Submit and errors

- Submit via `<form onSubmit={form.onSubmit(handler, onError?)}>`: the handler is not called when validation fails; while the handler's
  returned Promise is pending, `form.submitting` is true and the submit button gets `loading={form.submitting}` (covering async validation + the whole submission).
- In onError, focus the first invalid field: errors are keyed by dotted path and unordered — maintain your own visual-order array + `form.getInputNode(path)?.focus()`.
- **Server field errors are written back via `form.setErrors({ 'lines.2.quantity': 'Insufficient stock' })`** (keys are dotted paths, list items supported);
  a user edit to that field clears them automatically; errors that belong to no field go into their own state, not into form.errors.

## Validation

- Choose the three forms in order: **zod schema + `schemaResolver(schema, { sync: true })`** (Mantine 9 + zod 4 only;
  type source shared with `z.infer`, the in-stack default; brownfield keeps the project's current resolver); cross-field logic is a function
  `(values) => FormErrors`; simple fields can use the built-in validators directly
  (`isNotEmpty` / `isEmail` / `hasLength` / `matchesField` / `isInRange`…).
- **Async validation**: the rule returns a Promise, and the fourth argument `AbortSignal` guards against stale results (discard when `signal?.aborted`);
  pair with `validateInputOnChange: ['field']` + `validateDebounce: 500`; submission is blocked while `form.validating`.
- Validation timing defaults to submit-only; `validateInputOnChange` / `validateInputOnBlur` can be turned on globally or per field
  (list items use `` `members.${FORM_INDEX}.email` ``).
- **Layout-shift trap of onBlur validation**: the error pushes the submit button away and the click lands on nothing — put the submit button where errors cannot move it
  (a fixed footer), or switch to onChange validation.
- Conditional fields: **hidden fields keep their values and still participate in validation** — rules depend on the visibility-controlling value (the rules' second argument is the full values;
  zod uses `superRefine` + `ctx.addIssue({ path })` to target the specific field).
- `isEmail` / `isUrl` / `matches` reject empty strings: check for empty first on optional fields before calling the validator.

## Nesting and list fields

- Nesting uses dotted paths (`user.address.city`); the validate rule object nests following the value shape.
- List items: each item carries `key: randomId()` (`@mantine/hooks`) as the React key, separate in responsibility from `form.key()`;
  add/remove/reorder via `insertListItem` / `removeListItem` / `reorderListItem` / `replaceListItem`
  (both modes trigger re-renders; row errors travel with rows).
- Validate the array as a whole with `formRootRule` (the error lands in `form.errors.employees`); per-item rules take the third argument with paths like `employees.1.name`,
  enabling cross-item duplicate checks.

## Lifecycle and the dirty baseline

- After server data arrives, `form.initialize(data)`: sets values and initial values together (afterwards not dirty, and `reset()` returns to it);
  **takes effect only once**; later calls are ignored.
- Swapping records in an always-mounted edit modal: `form.setInitialValues(record); form.reset();` (replaces values and clears the previous record's
  errors / touched / dirty state). The key-remount hack for swapping records is forbidden.
- Changing the baseline after a successful save: `form.setInitialValues(values); form.resetDirty(values);` (`setInitialValues` alone does not touch the
  dirty state). Discard-changes button `disabled={!form.isDirty()}`.
- Shape values before submit with `transformValues` (onSubmit receives the transformed type `TransformedValues<typeof form>`;
  validation runs on the raw values).

## Cross-component and external control

- Sharing within the form tree: `createFormContext<Values>()` → `FormProvider` + `useFormContext`, children free of props drilling.
  The form returned by `useFormContext()` always types validation methods as synchronous — **always `await` `validate()` when async rules exist**.
- Control from outside the form tree (toolbars etc.): `createFormActions<Values>('settings-form')` + `useForm({ name: … })`;
  the form must already be mounted; there is no external submit action — submit externally with `<Button type="submit" form="form-id">`.
- Batch-modify input props (e.g. disable everything while submitting) with `enhanceGetInputProps: ({ form }) => ({ disabled: form.submitting })`.

## Other

- Single-field scenarios use `useField` (independent of the form).
- Custom inputs must stay compatible with `getInputProps`: accept `value` / `defaultValue` / `onChange` / `error` / `onFocus` /
  `onBlur`, use `@mantine/hooks` `useUncontrolled` internally to support both modes, and pass the remaining props through to the focusable element
  (`getInputProps` attaches `data-path`, which `getInputNode` uses for addressing).
- Transforming user input mid-typing (uppercasing, stripping characters): make that single input controlled (`useWatchValue` + `setFieldValue`),
  **that input gets no `key`** — `setFieldValue` changes the key, causing a remount that loses focus.

## Verification entry points

Never guess official APIs from memory: prefer the `https://mantine.dev/llms.txt` index → open the corresponding `.md` page
(e.g. `.../llms/use-form.md`); offline, read the `.d.ts` type declarations in `node_modules/@mantine/form`.
