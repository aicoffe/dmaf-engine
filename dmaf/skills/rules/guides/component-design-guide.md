---
name: component-design-guide
level: L2
domain: component-design
description: Component design implementation-level conventions covering component APIs, controlled/uncontrolled components, composition patterns, and Story and testing. Load when designing or reviewing components.
owner: Frontend Team
based_on: [frontend-standard, ux-standard]
enforcement: mandatory
version: v1.0.0
review_cycle: every six months
---

# Component Design Work Specification

> Styling references design system tokens only.

## 1. Priority Order

```
1. Off-the-shelf UI library components (Ant Design 5+ / Element Plus)
2. The shared component library (shared/, accumulated by this organization)
3. New domain components (may be promoted to shared after review)
```

- ❌ Reinventing the wheel is forbidden: search existing components before writing a new one

## 2. Component API Design

| Rule | Description |
| :--- | :--- |
| **Typed Props** | Exported explicitly as a TypeScript interface; required/optional is explicit |
| **Callback naming** | Event callbacks are `onXxx` (`onChange`, `onConfirm`); inventing a different naming style is forbidden |
| **Default values** | Optional Props have explicit default values, documented |
| **Controlled/uncontrolled** | Form components must support controlled mode (`value` + `onChange`); also provide the default uncontrolled usage |
| **Pass-through** | Style class names and native attributes may pass through (`className`, `style`, `data-*`) |
| **Single responsibility** | One component, one responsibility; compose with children/slots, not giant config Props (more than 15 Props means a redesign is required) |

## 3. Composition Patterns

- List / card / form components use compound components: `<Table><Table.Column /></Table>`
- Slots use `children` and named slot props; never pass JSX to render as strings

## 4. Styling and Accessibility

- Styling references tokens only; ❌ hard-coded colors/spacing are forbidden
- Interactive components carry aria attributes and keyboard support; focus styles must not be removed

## 5. Story and Testing

- Every shared component must have a Story (Storybook): covering all states — default/hover/focus/disabled/error
- Interaction tests: core interactions (click, input, submit) have component tests (Testing Library, behavioral assertions)
- Visual regression: snapshot tests for core components are optional; changes require confirmation

## 6. Promotion and Deprecation

- Promoting a domain component to shared: used by ≥ 2 domains + API review + Story/tests in place
- Deprecating a component: mark `@deprecated` + a migration note; keep it for one version cycle
