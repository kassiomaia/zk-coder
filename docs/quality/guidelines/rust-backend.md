# Rust Backend Quality Guidelines

**Status**: ACTIVE
**Last reviewed**: 2026-04-04
**Purpose**: Mandatory implementation rules for Rust backend code in `src-tauri/`.

## Scope

This guide applies to:

- `src-tauri/src/core/`
- `src-tauri/src/application/`
- `src-tauri/src/commands/`
- `src-tauri/src/infrastructure/`
- Rust tests, migrations, and backend support modules

Use it with `RULES.md`, `docs/architecture.md`, and feature-specific documentation.

## Repository-specific architectural rules

The current backend is intentionally layered. Preserve that shape.

### Core layer (`src-tauri/src/core/`)

Owns:

- entities
- value objects
- domain errors
- service and repository traits
- domain invariants

Must not own:

- SQLite details
- Tauri state/IPC concerns
- HTTP/provider client code
- filesystem/tool execution details

### Application layer (`src-tauri/src/application/`)

Owns:

- use-case orchestration
- coordination between repositories, runtimes, and services
- conversions between layers
- transaction-like workflow decisions

Must not become:

- a second transport layer
- a SQL layer
- a dumping ground for utility functions with no use-case ownership

### Commands layer (`src-tauri/src/commands/`)

Owns:

- Tauri command signatures
- transport-level validation and parsing
- mapping backend errors to frontend-safe responses

Rules:

- keep commands thin
- convert to `String` errors only at the IPC boundary
- do not embed business logic in commands
- do not reach around services directly into infrastructure

### Infrastructure layer (`src-tauri/src/infrastructure/`)

Owns:

- SQLite repositories and migrations
- provider clients
- runtime/tool adapters
- external-system concerns

Rules:

- keep persistence and transport details here
- do not leak infrastructure-only types into core contracts unless intentionally abstracted
- isolate third-party API quirks behind stable internal interfaces

## Clean-code rules for Rust

### Make illegal states hard to represent

Prefer:

- enums over free-form strings
- newtypes/value objects when identifiers or constrained values matter
- typed request/response structs instead of long parameter lists
- helper conversion functions at boundaries instead of stringly-typed branching throughout the flow

If strings enter through IPC or persistence, convert them to typed domain values as early as possible.

### Keep functions single-purpose

A function should usually do one of these well:

- validate input
- convert data
- orchestrate a workflow
- execute persistence
- map errors

If a function is doing several of them, split it.

### Contain duplication aggressively

Do not repeat:

- SQL row-to-domain mapping
- status parsing/formatting logic
- repository error conversions
- runtime registration / cleanup sequences
- request-building boilerplate

Extract shared helpers when the same pattern appears twice with meaningful similarity.

### Remove unimportant references

Do not keep:

- unused modules, helper functions, test fixtures, or trait methods
- temporary compatibility code after the migration is complete
- commented-out queries or disabled branches
- legacy names once all callers are migrated

### Respect file-size pressure

`RULES.md` defines the hard expectations. In practice:

- treat files above ~500 lines as already asking for extraction
- do not make large infrastructure files larger when the new code can be moved to helpers/submodules
- when touching an oversized file, prefer extracting the repeated concern before adding more behavior

## Error handling rules

- prefer domain-specific error types inside core and infrastructure boundaries
- use `thiserror`/typed variants for known failure modes
- use `anyhow` only where flexible aggregation is justified
- include enough context to diagnose the failure without leaking secrets
- never `unwrap`, `expect`, `panic!`, `todo!`, or `unimplemented!` in production paths
- when a boundary must degrade gracefully, make that fallback explicit and test it

## Async and concurrency rules

- async is for I/O and coordination, not to hide design problems
- keep lock scope minimal and document why a lock is needed when the reason is not obvious
- never hold a lock across unrelated expensive work
- prefer passing immutable data into async tasks over sharing mutable state
- make cancellation and cleanup paths explicit for background/runtime flows

## Logging and security rules

- use the logging infrastructure already present in the backend; do not add `println!`-style debugging
- never log secrets, API keys, raw authorization headers, or sensitive payloads
- prefer structured context in errors/logs over noisy message strings
- database access must remain aligned with the repository’s SQLite guidance in `AGENTS.md` and `docs/database-reference.md`

## Testing rules

### Tests-first expectation

For new backend behavior, bug fixes, and refactors:

- add a failing test first when practical
- for refactors, preserve behavior with characterization tests before extraction
- prefer testing domain logic and repository behavior directly before relying on broad end-to-end coverage

### What to test

Add or update tests for:

- domain invariants and parsing logic
- repository edge cases and not-found behavior
- metadata/error/fallback paths
- concurrency-sensitive state transitions
- regression scenarios from real bugs

### Test placement

- keep focused unit tests near the module when it improves readability
- use integration tests for cross-layer behavior that cannot be trusted from unit coverage alone
- name tests by behavior, not implementation detail

## Refactor process for Rust

1. identify the overgrown responsibility or duplicated flow
2. write or update tests around the current behavior
3. extract the smallest useful unit (type, helper, submodule, mapper, repository helper)
4. migrate call sites incrementally
5. remove the superseded code immediately
6. run backend validation before finalizing

## Backend review checklist

Before considering a Rust change complete, verify:

- layer boundaries are still clean
- new strings introduced at boundaries are converted to typed values early
- duplication was reduced
- no dead code or stale references remain
- errors are typed and contextual
- tests cover the changed behavior
- `cargo test` and `cargo clippy` were considered/run for the affected work

Any exception must be rare, documented, and justified.
