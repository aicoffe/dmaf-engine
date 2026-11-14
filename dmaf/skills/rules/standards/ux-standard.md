---
name: ux-standard
level: L1
domain: ux
description: Load when user interface, interaction design, brand consistency, or accessibility is involved.
owner: Design Team
based_on: [organization-principles]
enforcement: mandatory
version: v1.0.0
review_cycle: Once every six months
---

# UX Domain Standard

> This standard governs brand and experience standards; design Token values are carried by the design system, and interaction pattern details and the accessibility baseline are carried by the corresponding domain skills and shared clause tables.

---

## 1. Brand & Tone

### 1.1 Brand Personality

- **Core keywords**: professional, trustworthy, concise
- **Brand personality**: steady yet warm, professional but not rigid
- **Forbidden**: avoid over-marketing and exaggerated wording

### 1.2 Copy Tone

| Scenario | Tone | Example |
| :--- | :--- | :--- |
| **Success** | Positive, concise | "Operation successful" |
| **Failure** | Neutral, constructive | "It did not go through; please check and retry" (avoid "Rejected") |
| **Warning** | Clear, actionable | "This action cannot be undone. Continue?" |
| **Guidance** | Friendly, concise | "No data yet. Go create your first one." |
| **Error** | Never blame the user | "Invalid phone number format" (avoid "You entered it wrong") |

### 1.3 Wording Conventions

- **Avoid technical jargon**: say "sign in" rather than "authenticate"; say "save" rather than "persist"
- **Avoid negative sentences**: say "you can do this" rather than "do not do that"
- **Avoid ambiguity**: say "delete" rather than "process"
- **Stay consistent**: use the same word for the same concept

---

## 2. Design System

### 2.1 Design Tokens

- **Token location**: `design/tokens/`
- **Token format**: JSON
- **Usage**: all colors, fonts, spacing, and radii must reference Tokens; hardcoding is forbidden

### 2.2 Token Inventory

Concrete Token values are carried by the design system. At this standard's level only principles are governed:

- All colors, fonts, spacing, and radii must reference Tokens; hardcoding is forbidden (e.g., writing `#3B82F6` in code is forbidden)
- Using undefined values is forbidden; new values must enter the Token inventory first
- The Token inventory is brand-related configuration and may be replaced wholesale when used as a personal/team starting point

---

## 3. Interaction and State Principles

- **Three states are mandatory**: loading, empty, and error states are part of the feature; any data display scenario must handle them (P-009)
- **Immediate feedback**: interaction feedback within 200ms; beyond that a loading indicator is required
- **Destructive operations require confirmation**: state the consequence clearly + danger-colored primary button + default focus on "Cancel"
- **Never blame the user**: error copy is neutral and constructive, exposing no technical details
- ❌ Blank pages, full-screen spinners, and popup-interrupted success notices are forbidden

---

## 4. Component Reuse

### 4.1 Component Libraries

- **UI library**: follow the tech-stack skill bound by the project's stack declaration; libraries outside the inventory go through their introduction process and are recorded as an ADR
- **Forbidden**: reinventing the wheel — reuse existing components first

### 4.2 Component Conventions

- **Naming**: PascalCase (e.g., `StatusPanel`)
- **Props**: must have TypeScript types
- **Styles**: must use design Tokens
- **Accessibility**: must have aria attributes
- **Tests**: core components must have unit tests

### 4.3 Component Documentation

- **Tool**: Storybook
- **Requirements**:
  - Every component must have a Story
  - Show all states (default, hover, focus, disabled, error)
  - Provide usage examples

---

## 5. Responsive Design

### 5.1 Breakpoints

- Mobile first: the layout must be verified at every breakpoint

### 5.2 Layout Strategy

- **Mobile first**: design for mobile first, then adapt to desktop
- **Fluid layout**: use flex / grid; avoid fixed widths
- **Minimum clickable area**: 44x44px (mobile)

### 5.3 Responsive Testing

- Must be verified at the three widths 375px, 768px, and 1440px
- Must be verified under orientation switching (portrait/landscape)

---

## 6. Conflict Adjudication with Other Domains

### 6.1 With Security

- **Adjudication basis**: P-001 (security over experience)
- **Adjudication process**: Security Owner + Designer adjudicate jointly
- **Accommodation**: look for solutions that accommodate both first, such as extending the timeout + secondary authentication for sensitive operations

### 6.2 With Brand

- **Adjudication basis**: P-009 (user experience is part of the feature)
- **Adjudication process**: Brand Owner + Designer adjudicate jointly
- **Accommodation**: e.g., when brand color contrast falls short, adjust the color scale rather than giving up accessibility

### 6.3 With Performance

- **Adjudication basis**: P-009 (user experience is part of the feature)
- **Adjudication process**: Designer + Architect adjudicate jointly
- **Accommodation**: e.g., when animations hurt performance, degrade to simplified animation
