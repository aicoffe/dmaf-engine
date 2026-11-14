# Forms, input components, and data tables

> Applies to Vue 3.5+ / Naive UI 2.37+. When project dependencies below the baseline are detected (brownfield mode), this document's
> "forbidden / required" items downgrade to suggestions; the compatibility mapping in `reference/brownfield.md` governs.

Required reading when a task touches forms, data entry, or table lists.

## Forms (`n-form`)

- Instance: `const formRef = useTemplateRef<FormInst>('formRef')` (3.5+; at 3.4 and below use typed `ref<FormInst | null>(null)`, see brownfield.md); rules are defined centrally in `n-form`'s `rules` (`FormRules`); rules reused across items use `n-form-item`'s `rule-path`
- Synchronous `validator` returns `boolean | Error`; async is written as a `validator` or `asyncValidator` returning a Promise — a rejection means failure (a non-Promise returned from `asyncValidator` is ignored)
- `validate()` rejects with `FormValidationError[]` on failure; rules with `level: 'warning'` do not block submission and come back via the resolve value's `warnings` (`warnings` / `level: 'warning'` may be unsupported in early 2.3x versions — verify against the official docs per the project version; brownfield see brownfield.md)
- Dynamically added/removed list items write `path` as `` `items[${index}].name` ``; rules and partial validation both match by path exactly
- Reset: reassign the model's initial value via a factory function (clearing a controlled value uses `null`), then call `restoreValidation()` — Naive UI has no API to reset values, and `restoreValidation()` only clears validation state
- The submit button is `loading` and disabled during validation and submission (neutral interaction forms section)

## Input components

- Date / time (`n-date-picker`, `n-time-picker`): `v-model:value` is a millisecond timestamp (date ranges are `[number, number]`), not a `Date` or string; when the API wants strings, switch to `v-model:formatted-value` + `value-format` — the format string is date-fns syntax (`yyyy-MM-dd HH:mm:ss`), not dayjs / moment's `YYYY-MM-DD`
- Selector `n-select`: when the `options` field names are not label / value, map them with `label-field` / `value-field` instead of reshaping arrays on the frontend; remote search sets `remote` + `filterable` + `@search` (debounced) + `loading` — under `remote`, built-in filtering and `tag` stop working
- When the value is not among the `options`, `n-select` generates a fallback option from the raw value by default (the UI shows the id instead of the name): in echo scenarios with asynchronously loaded options, merge the option matching the current value into `options` first, or provide the correct label with a `fallback-option` function
- Upload `n-upload`: always `custom-request` calling the shared request wrapper (auth headers and error translation consistent with the request layer), driving `onProgress` / `onFinish` / `onError` in the callback, never the component's built-in direct upload; type / size validation goes in `on-before-upload` (returning `false` or rejecting cancels); manual upload sets `:default-upload="false"` then calls `submit()`; a controlled file list uses `v-model:file-list`

## Data tables (`n-data-table`)

- Annotate column definitions with `DataTableColumns<Row>`: static columns are constants defined in setup; when they depend on permissions / language or controlled sort/filter state, derive them with `computed` instead of mutating the column objects in place
- When row data has no `key` field, `row-key` must be set (e.g. `(row) => row.id`): selection, expansion, and tree data rely on it to identify rows stably
- Server-side pagination / sorting / filtering: set `remote` on the table (otherwise it re-paginates the current page locally) and `sorter: true` / `filter: true` on columns (icons only; sorting and filtering are delegated to the API); `pagination` is controlled (`page` / `pageSize` / `itemCount` come from the URL query and the response)
- Sort/filter state lives in the URL query (see reference/state-routing.md): write the query in the `update:page` / `update:sorter` / `update:filters` callbacks to drive a refetch, then feed it back via `computed` into the columns' `sortOrder` / `filterOptionValue(s)` (a value of `undefined` means uncontrolled); when sorting is cancelled, `update:sorter` receives `null`
- Fixed columns (`fixed`) require a numeric `width` on the column and `scroll-x` on the table
- Interactive cell components go in the column `render` with `h()` (style in reference/components.md "Controlled values and bindings"); for large data volumes use `virtual-scroll` + `max-height`
