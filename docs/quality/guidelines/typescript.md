# TypeScript Quality Guidelines

**Status**: ACTIVE
**Last reviewed**: 2026-04-04
**Purpose**: Mandatory implementation rules for TypeScript modules in `src-ui/` outside component markup/styling concerns.

## Scope

This guide applies to TypeScript in:

- `src-ui/src/lib/api/`
- `src-ui/src/lib/stores/`
- `src-ui/src/lib/utils/`
- `src-ui/src/lib/types/`
- supporting `.ts` test files and frontend helpers

Use `docs/quality/guidelines/svelte.md` for component-specific rules.

## Repository-specific module boundaries

The frontend already has a clear split. Preserve it.

### `$lib/api/*`

Owns:

- Tauri IPC wrappers
- backend-to-client response mapping
- transport-safe error wrapping

Rules:

- keep these modules thin and deterministic
- perform data conversion here instead of scattering it across stores/components
- do not mix DOM logic, UI state, or view formatting into API wrappers

### `$lib/stores/*`

Owns:

- shared client-side state
- async orchestration needed by multiple components
- derived state for the UI

Rules:

- stores coordinate; they should not become transport wrappers, DOM controllers, or untyped state bags
- centralize reusable validation, normalization, and synchronization logic here or in dedicated helpers
- keep store updates explicit and predictable

### `$lib/utils/*`

Owns:

- reusable pure helpers
- shared formatting/parsing rules
- logging helpers and cross-cutting client utilities

Rules:

- prefer pure functions
- avoid hidden global dependencies
- do not move feature ownership into `utils` just to “clean up” a component or store

### `$lib/types/*`

Owns:

- shared frontend contracts
- narrow unions and interface definitions that stabilize client code

Rules:

- do not recreate the same shape in multiple files
- prefer one exported canonical type over parallel near-identical interfaces

## Clean-code rules for TypeScript

### Use the type system fully

- keep `strict` mode assumptions intact
- avoid `any`, broad `unknown` passthroughs, and unchecked casts
- prefer discriminated unions, literal unions, and narrow interfaces for stateful flows
- make nullable/optional cases explicit in the type instead of relying on comments

### Keep side effects at the edges

Pure logic should stay separate from:

- Tauri `invoke` calls
- timers
- browser APIs
- store mutation
- logging

If a module mixes decision logic and side effects, extract the decision logic into a pure helper that can be tested directly.

### Eliminate duplication and drift

Do not duplicate:

- validation rules across components/stores
- backend response mapping logic across multiple API helpers
- provider/model capability derivation logic in several places
- error message shaping throughout the UI stack

When the same logic appears more than once, extract the canonical implementation and delete the copies.

### Remove unimportant references

Do not keep:

- unused types, exports, imports, helpers, or derived values
- stale compatibility fields after migrations
- debug `console` calls in production code paths
- old comments that describe replaced behavior
- magic constants or string literals that should be named and centralized

## Logging and error handling rules

- do not use raw `console.log` / `console.warn` / `console.error` in application modules when the logging utility already provides safer behavior
- if diagnostic logging is needed, use the shared logger and protect secrets
- wrap transport failures with meaningful typed/contextual errors at the boundary
- never swallow errors silently unless the fallback is intentional and documented in code

## State management rules

- state must have a clear owner
- derived state should remain derived; do not duplicate it into writable state unless there is a real synchronization need
- keep async store flows linear and easy to reason about
- prefer small helper functions over deeply nested inline callbacks
- if a store exceeds one coherent responsibility, split it

## Testing rules

### Tests-first expectation

For new logic, bug fixes, and refactors:

- add or update a failing test first when practical
- add characterization tests before reorganizing shared state or transformation logic
- write regression tests for bugs reproduced from the UI or backend interactions

### What to test

Add or update tests for:

- state transitions
- data mapping from backend responses
- validation helpers
- fallback behavior and error shaping
- logger-sensitive behavior when secrets or unsafe payloads are possible

### Testing style

- prefer direct unit tests for pure helpers and stores
- keep tests deterministic; avoid hidden time/network dependencies
- assert behavior, not internal implementation details

## Refactor process for TypeScript

1. identify duplication, unstable typing, or mixed ownership
2. protect the existing behavior with tests
3. extract shared types/helpers first
4. move side effects toward the edges
5. simplify call sites
6. delete the obsolete path immediately
7. rerun `npm run check`, tests, and linting as appropriate

## TypeScript review checklist

Before considering a TypeScript change complete, verify:

- boundaries between API/store/utils/types stayed clean
- there is no new `any`-driven or cast-heavy flow
- validation/mapping logic is not duplicated
- production debug logging was not introduced
- stale references and unused exports were removed
- tests cover the changed behavior
- the change remains understandable without reading multiple hidden side paths

Any exception must be explicitly justified and temporary by design.
