# Tech-Stack Declaration File Format

> The format source of truth for generating or verifying a project's tech-stack declaration.
> Conventional file name `tech-stack.md`; this document governs the format and field
> semantics — self-describing, with no derivation logic.

```markdown
# Tech-Stack Declaration

## backend_stack
rust

## frontend_stack
vue3-naiveui

## Binding
- backend_stack: stacks/backend-rust/SKILL.md
- frontend_stack: stacks/frontend-vue3-naiveui/SKILL.md

## Verification commands (binding mode defers to the binding skill; required in generalized mode and its sole authority)
- Backend: cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
- Frontend: vue-tsc --build && eslint . && vitest run

## Selection rationale
(mode and rationale; generalized mode notes "confirmed by the user")
```

Field semantics:

- `backend_stack` / `frontend_stack` are **orthogonal and independent**; the value "none" is allowed;
- Out-of-registry stack values use the `<language>-<framework>` short name, and the "Binding" section says "None (beyond the currently supported set)";
- **The "Verification commands" section is required in generalized mode** — the biggest risk of generalization is not inaccurate idioms but nobody pinning down "which test/build/lint commands to run"; this can be derived deterministically from build files and is written in when the architecture-analysis agent generates the declaration;
- When the stacks/ skill is added later, changing only the "Binding" section from "None" to the skill path upgrades to bound mode — the process does not need to be rerun; it takes effect automatically in the next session.
