# Svelte Quality Guidelines

**Status**: ACTIVE
**Last reviewed**: 2026-04-04
**Purpose**: Mandatory implementation rules for Svelte components and UI composition in `src-ui/`.

## Scope

This guide applies to:

- `src-ui/src/lib/components/**/*.svelte`
- route-level Svelte files when they contain product UI
- component tests and component-scoped styling decisions

Use this guide together with `RULES.md` and the TypeScript guide.

## Repository-specific expectations

The current UI is built with:

- Svelte 5 runes
- components organized by feature area under `src-ui/src/lib/components/`
- reusable primitives in `shared/`
- design tokens in `src-ui/src/lib/styles/design-system.css`
- icon components under `$lib/icons` and library-backed icon usage

New Svelte code must extend this structure instead of inventing a parallel one.

## Component responsibility rules

A component should have one clear job:

- render a focused UI surface
- manage local interaction state that truly belongs to the component
- emit explicit callbacks/events upward
- consume stores or helpers without re-implementing them

Do not let a component become all of the following at once:

- view renderer
- business-rule container
- validation layer
- transport client
- design-token definition site

When logic stops being local to the UI surface, move it into a store, API module, or helper.

## Runes and state rules

- use `$state`, `$derived`, and `$effect` intentionally and minimally
- prefer `$derived` for computed values instead of synchronizing duplicate writable state
- keep `$effect` focused on real side effects; do not use it as a catch-all control-flow tool
- if an effect manages listeners, timers, or subscriptions, cleanup must be obvious and reliable
- avoid hidden coupling between sibling components through ad-hoc globals

## Props and composition rules

- define a typed `Props` interface for non-trivial components
- keep prop names explicit and stable
- prefer composition through reusable child components and snippets over copy-pasted markup blocks
- use shared components when the same interaction or structure already exists in `shared/`
- if the same icon, status block, validation presentation, or action row appears repeatedly, extract it

## Styling rules for Svelte components

`RULES.md` remains the source of truth for styling. In day-to-day implementation, also enforce the following:

- use the existing design tokens instead of hardcoded colors, spacing, radii, shadows, or durations
- do not introduce visual noise through unnecessary wrappers, borders, gradients, glows, or nested card patterns
- do not embed raw SVG blocks repeatedly when an icon component or icon library already exists
- do not use HTML strings to render icons or UI fragments
- keep styling aligned with the design system and shared CSS files already in the repo

Quality issues already seen in this codebase must not be reintroduced:

- hardcoded colors and animation durations
- inconsistent icon rendering patterns
- duplicated SVG markup
- conflicting hover/focus styling
- large mixed-responsibility components that hide design drift

## Accessibility and interaction rules

- interactive elements must use semantic HTML first
- icon-only controls need accessible names
- dialogs, overlays, and menus must preserve keyboard and focus behavior
- loading, validation, and status changes must be understandable without relying only on color or motion
- motion must remain purposeful and compatible with reduced-motion expectations already established in the repo

## Clean-code rules for Svelte

### Keep markup understandable

- split large conditional trees when they hide intent
- name snippets/helpers by role, not by visual coincidence
- keep class composition readable
- remove dead branches, stale props, unused snippets, and commented-out markup immediately

### Keep components small enough to reason about

- if a component grows toward the repository soft limit or gains multiple responsibilities, split it
- extract repeated subtrees before they diverge
- preserve folder organization by feature area

### Remove unimportant references

Do not keep:

- obsolete props kept “just in case”
- local state duplicated from stores without need
- inline one-off SVG markup that should be shared
- stale TODOs, placeholder handlers, or debug outputs
- legacy CSS hooks that no longer affect rendered output

## Testing rules

### Tests-first expectation

For UI bugs, component refactors, and shared interaction changes:

- add or update a failing test first when practical
- for complex refactors, protect the visible behavior before restructuring the component
- prefer regression coverage for accessibility, rendering branches, and interaction states

### What to test

Add or update tests for:

- state-dependent rendering
- keyboard and pointer interactions
- accessibility-critical attributes and labels
- loading, empty, error, and success states
- extracted shared components that replace duplicated UI logic

## Refactor process for Svelte

1. identify the view concern that is overloaded or duplicated
2. protect the behavior with component tests where practical
3. move business logic out of the component first
4. extract repeated UI into child components/snippets
5. remove the superseded markup and styles immediately
6. verify styling, accessibility, and tests before finishing

## Svelte review checklist

Before considering a component change complete, verify:

- the component still has one clear responsibility
- local state and effects are minimal and necessary
- no business logic was duplicated from stores/helpers
- design tokens were used instead of hardcoded values
- no repeated inline SVG/HTML-string icon pattern was introduced
- accessibility semantics still hold
- stale props, state, markup, and styles were removed
- component tests cover the changed behavior where appropriate

Any exception must be explicit, temporary, and justified.
