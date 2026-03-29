# Coder — Coding Rules

All rules in this document are mandatory. Violations must be corrected before committing.

---

## UI/UX Golden Rule

Design and develop the user interface as a high-quality, production-grade system that prioritizes clarity, precision, and intentionality over decoration. The interface must feel like a professional tool built for expert users, not a generic dashboard or AI-generated layout.

Strictly follow the established design system and style guide at all times. Treat the style guide as a source of truth for spacing, typography, color usage, component structure, and interaction patterns. Do not introduce new styles, patterns, or visual treatments unless they are clearly justified and consistent with the system.

Continuously evaluate the interface for inconsistencies, visual noise, and low-quality patterns. Identify and correct issues such as redundant containers, excessive borders, misaligned spacing, inconsistent typography, weak hierarchy, and unnecessary visual effects. Proactively refine any part of the UI that feels generic, cluttered, or artificially styled.

Adopt a restrained, pale design system built on neutral backgrounds, subtle surface elevation, and minimal, controlled use of accent colors. Avoid gradients, glows, saturated tones, and decorative styling. Color must only be used to communicate meaning, hierarchy, or state.

Use typography as the primary driver of hierarchy. Ensure a clear typographic scale with strong contrast between titles, sections, body text, and metadata. Maintain consistency in font usage, weight, and spacing, and prioritize readability and structure over stylistic variation.

Eliminate unnecessary visual noise by removing redundant containers, nested card structures, and excessive UI framing. Replace these with spacing, alignment, and layout rhythm. Every element must justify its presence and contribute to clarity.

Design components to be minimal, structured, and functional. System events such as permissions, execution states, and results must be presented as clear, purposeful UI elements rather than decorative cards. Emphasize communication and usability over visual styling.

Ensure all technical content such as code blocks, tables, and structured outputs are presented with precision and clarity, inspired by professional developer tools. Maintain consistency with the overall system while preserving readability and function.

Transform the interaction model into a command-driven system. The input must act as a central control surface for both communication and execution. Ensure it is visually integrated, minimal, and responsive to system state.

Aggregate low-level system operations into meaningful, high-level UI components. Avoid repetitive or verbose messaging. Replace raw operational logs with structured, user-facing progress representations that communicate intent and state clearly.

Apply micro-interactions with discipline. Use motion only to reinforce state changes, continuity, and responsiveness. Avoid decorative or exaggerated animations. All transitions must feel fast, smooth, and intentional.

Maintain strict consistency across spacing, alignment, layout structure, and interaction patterns. Use a defined spacing system and grid, ensuring visual rhythm and coherence across all screens and components.

Enforce strict iconography standards across the interface. Never generate, draw, or approximate icons manually. Always use a dedicated, high-quality icon library to ensure consistency, clarity, and professional quality. Icons must be stylistically consistent, optically balanced, and aligned with the overall design system. Prefer established libraries such as Lucide, Heroicons, Phosphor, or Radix Icons. If the current system lacks appropriate icons, suggest a suitable library rather than creating new ones. Icons must remain simple, recognizable, and visually aligned with text and components, avoiding stylistic mismatches or decorative overload.

Before finalizing any UI decision, evaluate it against the following quality filters:

- Does this reduce or increase visual noise?
- Does this improve clarity and hierarchy?
- Does this align with the existing design system?
- Does this feel like a professional tool or a generic UI?
- Can this be simplified further without losing meaning?

If any element feels like "AI-generated UI," overly styled, redundant, or unclear, refine or remove it.

Actively look for opportunities to improve the interface beyond the immediate request. Suggest refinements that enhance consistency, usability, and visual quality, even if they are not explicitly asked for.

The final result must meet the standard of top-tier products such as Apple system interfaces, Linear, and OpenAI tools, where simplicity, precision, and coherence define the experience, and every detail is intentional.

---

## Documentation Governance

Treat documentation as a living system with a clear lifecycle. Every document must have a purpose, a status, and an owner. Documentation without a current purpose is technical debt — it misleads, wastes time, and erodes trust.

### Lifecycle: Plan → Active → Complete → Archive → Remove

Every document follows one path:

1. **Plan**: Created during planning. Clearly labeled `[DRAFT]`, `[PLAN]`, or `[WIP]`.
2. **Active**: Currently in use as a guide, reference, or spec. Clearly labeled with status and last-reviewed date.
3. **Complete**: The work described is done. Update the header to `[COMPLETE — <date>]` and either:
   - Move to `docs/archive/` immediately, or
   - If it serves as lasting reference (architecture, design decisions), update status and move to the appropriate reference location.
4. **Archive**: Completed plans and progress trackers that are no longer actively consulted live in `docs/archive/`. They are not deleted but are clearly separated from active documentation.
5. **Remove**: Archived documents that have not been referenced in 30+ days and contain no lasting design decisions should be deleted. Do not hoard.

### Rules for Creating Documents

- Before creating a new document, check if an existing one covers the same topic. Extend or update existing docs before creating new ones.
- Every document must have a header with: **title**, **status** (`DRAFT` / `ACTIVE` / `COMPLETE` / `ARCHIVED`), **last-reviewed date**, and **purpose** (one line).
- Implementation plans must state what feature they cover. When the feature is complete, the plan must be marked `COMPLETE` within the same working session.
- Progress trackers are ephemeral. When all phases are complete, mark the tracker `COMPLETE` and archive it. Do not leave completed progress trackers in active directories.

### Rules for Maintaining Documents

- When you finish implementing a feature described in a plan, mark that plan `COMPLETE` immediately. Do not leave "Ready for Implementation" labels on completed work.
- When a document references file paths, verify the paths still exist. If they have moved or been renamed, update the reference or mark the document as stale.
- When you encounter a document that is clearly outdated (wrong project name, stale paths, superseded by newer docs), fix it or archive it — do not leave it to confuse the next reader.
- The `CHANGELOG.md` must be updated whenever a significant feature is completed or a bug is fixed. Do not batch changelog updates — do them as part of the work.

### Rules for Removing Documents

- Proactively identify and remove documents that are no longer serving a purpose. A completed implementation plan that has been fully merged and is no longer referenced is a candidate for archival or removal.
- Before removing a document, ask: **Does this contain a design decision, architectural choice, or rationale that future developers will need?** If yes, extract the decision into the appropriate reference doc (e.g., `docs/architecture.md`) before removing the plan.
- Remove empty directories that serve no purpose.
- Duplicate or overlapping documents must be consolidated. If two docs describe the same feature, merge them into one and remove the other.
- Progress trackers for completed work are always safe to archive. They have served their purpose.

### Quality Filters for Documentation

Before finalizing any documentation decision, evaluate:

- Does this document still serve its intended purpose?
- Is the status label accurate and current?
- Is this duplicating content that exists elsewhere?
- Would a new team member find this document helpful or confusing?
- Can this be simplified, consolidated, or removed without losing information?

If a document feels stale, misleading, or redundant, act on it immediately rather than leaving it for later.

### During Every Development Session

- When you start work on a feature, check if there is an existing plan. If the plan is complete and the feature needs changes, create a new focused plan — do not append to the old one.
- When you finish a feature, audit the related documentation: mark plans as complete, archive progress trackers, update the changelog, and remove any docs that are now obsolete.
- When you encounter stale docs during unrelated work, flag them. At minimum, add a `> ⚠️ This document may be stale — last reviewed <date>` notice at the top. Ideally, archive or remove them.

---

## Component Structure

- One component per file in `src-ui/src/lib/components/`
- Use CSS modules for component-scoped styles (e.g., `Button.module.css`)
- Keep components small and focused on single responsibility

## Naming Conventions

- Components: PascalCase (e.g., `ChatMessage.svelte`)
- CSS modules: Match component name (e.g., `ChatMessage.module.css`)
- Utility functions: camelCase (e.g., `formatTimestamp.ts`)

---

## CSS & Styling

### Approach

- Custom CSS modules for all component styles
- Global styles in `src-ui/src/lib/styles/global.css`
- CSS variables for theming (colors, spacing, typography)
- Desktop First Design: Prioritize desktop-specific design patterns and interactions, while ensuring responsiveness for smaller screens.
- All design tokens (colors, spacing, typography, fonts) are defined in `src-ui/src/lib/styles/design-system.css` and must be used consistently. Never hard-code values that should be tokens.
- The application must have a unique, non-generic visual style. Do not use uncustomized component libraries or generic font stacks (Arial, Helvetica, Times New Roman).
- If unsure about the design system or styling decisions, consult the design team before proceeding. Do not generate "AI Slop" — when in doubt, ask.

### Prohibited CSS Patterns

1. **Empty CSS rulesets**: Never create empty CSS rules. They cause Vite virtual CSS module loader to fail.
   ```css
   /* BAD - Causes build errors */
   .sidebar-left {}

   /* GOOD - Add a comment or remove the rule entirely */
   .sidebar-left {
       /* Styles inherited from parent */
   }
   ```

2. **Invalid CSS properties**: Only use valid CSS properties. Data attributes cannot be set via CSS.
   ```css
   /* BAD */
   :root { data-theme: 'light'; }

   /* GOOD - Use JavaScript to set data attributes */
   document.documentElement.setAttribute('data-theme', 'light');
   ```

3. **Undefined CSS variables**: Always verify variables exist in `design-system.css` before using them.
   ```css
   /* BAD - Variable doesn't exist */
   transition: background var(--card-duration-hover) var(--card-ease-hover);

   /* GOOD - Use defined variables from design-system.css */
   transition: background var(--transition-fast) var(--ease-out);
   ```

### CSS Verification Checklist

Before committing any CSS changes:

1. **Check variable existence**: Search `design-system.css` to verify the variable is defined
2. **Run type check**: `npm run check` from `src-ui/`
3. **Run dev server**: `npm run dev` to verify no CSS module loading errors
4. **Check for empty rulesets**: `grep -r "\. {}\|\.className {}" src-ui/src/lib/components`

### Shared CSS Files

| File | Purpose | When to Import |
|------|---------|----------------|
| `animations.css` | 30+ @keyframes (spin, fadeIn, slideIn, pulse) | Component needs animations |
| `scrollbar.css` | Unified scrollbar classes (.standard-scrollbar, .thin-scrollbar) | Component has scrollable content |
| `reduced-motion.css` | WCAG 2.1.2.3 compliant reduced motion | Component has animations |
| `design-system.css` | ALL design tokens (colors, spacing, typography) | Already imported globally — do NOT re-import |
| `global.css` | Global resets and base styles | Already imported in +layout.svelte |

Import pattern in component `<style>` blocks:
```css
@import '../../styles/animations.css';
@import '../../styles/scrollbar.css';
/* NEVER import design-system.css or global.css - already loaded globally */
```

### Common CSS Mistakes

| Mistake | Symptom | Solution |
|---------|---------|----------|
| Empty ruleset `{}` | `vite-plugin-svelte:load` failed | Add comment or remove rule |
| Undefined variable | Style not applied, console warning | Verify in design-system.css |
| Invalid CSS syntax | Build fails, parsing errors | Use valid CSS properties only |
| Duplicate @import | Circular dependency warnings | Don't import global files already loaded |

---

## Rust Code Style

- Follow Rust standard naming conventions (snake_case for functions/variables, PascalCase for types)
- Use `clippy` for linting
- Write unit tests for core functionality

## TypeScript / Svelte Code Style

- Use TypeScript strict mode
- Follow ESLint and Prettier configurations
- Prefer composition over inheritance

---

## Architecture

- Modular design with clear separation of concerns
- Async/await for I/O operations
- Leverage Rust's type system for safety

---

## Quality Assurance

- Write unit tests for Rust backend and TypeScript frontend
- Use integration tests for critical user flows
- Regularly run linters and formatters to maintain code quality
- Small files with focused functionality:
  - **Soft limit**: 500 lines of code per file
  - **Hard limit**: 800 lines of code per file
  - If a file exceeds 500 lines, it must be refactored into smaller files with focused functionality.
- Use Git for version control with clear commit messages. Follow a branching strategy (e.g., Git Flow).
- Maintain a changelog to document significant changes, features, and bug fixes.
