# Code Quality Guidelines

**Status**: ACTIVE
**Last reviewed**: 2026-04-04
**Purpose**: Mandatory entry point for code-quality expectations and area-specific implementation guides.

## Why this exists

`RULES.md` defines the global product, documentation, styling, architecture, and QA standards for the repository.

This directory turns those standards into **implementation-time rules** for the code we write every day. These guides are mandatory for all changes in the touched area. They are intentionally aligned with the current repository structure:

- Rust backend with layered modules in `src-tauri/src/`
- Svelte 5 + TypeScript frontend in `src-ui/src/lib/`
- Shared design tokens in `src-ui/src/lib/styles/design-system.css`
- Repository automation scripts in the root and `scripts/`

## Mandatory reading order before code changes

1. Read `AGENTS.md`
2. Read `RULES.md`
3. Read this file
4. Read every touched-area guide:
   - `docs/quality/guidelines/rust-backend.md`
   - `docs/quality/guidelines/typescript.md`
   - `docs/quality/guidelines/svelte.md`
   - `docs/quality/guidelines/configuration.md`
   - `docs/quality/guidelines/scripts.md`
5. Read the relevant feature or architecture document when the change touches behavior, UX, persistence, or system boundaries
6. Review `docs/issues/README.md` and any relevant issue folder when working in an area with recurring quality problems

If a change spans multiple areas, **all corresponding guides apply**.

## Non-negotiable repository baseline

### 1. Keep the codebase clean

Every change must leave the touched area cleaner than it was.

Do not add or preserve:

- dead code, commented-out logic, abandoned flags, or placeholder branches
- unused imports, exports, props, state, variables, queries, types, or helpers
- duplicate validation, mapping, formatting, or branching logic
- hardcoded design tokens, magic strings, or magic numbers when a shared source exists
- production `console` debugging, ad-hoc logging, or secret-bearing logs
- inline icon markup, HTML strings, or repeated SVG blocks when a shared icon/component already exists
- stale comments, stale docs, or references to behavior that no longer exists

"Unimportant references" are repository debt. Remove them instead of carrying them forward.

### 2. Protect separation of concerns

Keep code in the layer that owns the responsibility:

- backend commands adapt transport, application services orchestrate use cases, core owns domain contracts, infrastructure owns SQLite/provider/tool details
- frontend components render UI and local interaction, stores coordinate shared client state, API wrappers isolate Tauri IPC, utilities provide reusable pure logic
- configuration files describe tooling/runtime behavior and must not become feature-logic backdoors
- scripts automate workflows and must not hide application logic that belongs in Rust or TypeScript

### 3. Prefer test-first change safety

For non-trivial behavior changes:

- write or update a failing test first when practical
- for refactors, add a characterization test before moving logic
- for bug fixes, reproduce the bug in a test before fixing it
- if test-first is genuinely impractical, document the constraint in the PR/working notes and add tests in the same change

### 4. Refactor as a disciplined process

Refactoring is not free-form editing. Follow this order:

1. identify the boundary and the duplicated or overgrown responsibility
2. protect behavior with tests
3. extract one concern at a time
4. keep public interfaces stable unless the change is intentional and documented
5. delete the superseded path in the same change
6. rerun validation before considering the work complete

### 5. Changes must be reviewable

Optimize for maintainers reading the change later:

- small focused diffs
- clear names
- explicit ownership of side effects
- no mixed concerns in one file unless the boundary truly demands it
- no “temporary” hacks without a tracked follow-up

## Guides in this directory

| Guide | Applies to | Focus |
|---|---|---|
| `rust-backend.md` | `src-tauri/` Rust code | layered backend design, domain boundaries, async, errors, tests, refactors |
| `typescript.md` | `src-ui/` TypeScript modules | client-side domain logic, stores, API wrappers, utilities, tests |
| `svelte.md` | `.svelte` components | UI composition, runes usage, accessibility, styling discipline, component tests |
| `configuration.md` | manifests and tool configuration | dependency hygiene, build/test/lint config, security-sensitive settings |
| `scripts.md` | root scripts and `scripts/` | safe automation, shell discipline, logging, validation |

## Definition of done for implementation work

A change is not done until all of the following are true:

- the touched-area guides were followed
- duplication was reduced, not increased
- stale references introduced by the change were removed
- the relevant tests were added or updated
- the relevant validation commands were run
- related documentation was updated when behavior, architecture, or workflow changed

Any divergence from these guides is a blocking issue.
