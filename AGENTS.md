# AGENTS.md

Desktop application built with **Rust (Tauri v2)** and **Svelte/TypeScript** for AI-powered developer automation.

## Project Overview

### Core Principles
- User-centric design
- Performance through Rust/Tauri
- Modular architecture
- Security
- Cross-platform support (Linux, macOS, Windows)

### Key Capabilities
- AI agent integration
- Code generation
- Task automation
- Productivity tools

## Tech Stack

### Backend
- Rust
- Tauri v2

### Frontend
- Svelte
- TypeScript

### Styling
- Custom CSS modules
- Component library

# Mandatory Reading Order

`AGENTS.md` is the mandatory entry point for development guidance in this repository.

Before making **any code change**, every agent or contributor must read, in order:

1. `RULES.md`
2. `docs/quality/guidelines/README.md`
3. Every guide relevant to the touched area:
   - `docs/quality/guidelines/rust-backend.md`
   - `docs/quality/guidelines/typescript.md`
   - `docs/quality/guidelines/svelte.md`
   - `docs/quality/guidelines/configuration.md`
   - `docs/quality/guidelines/scripts.md`
4. The relevant architecture, feature, and database documentation for the area being changed

If a task spans multiple layers or technologies, **all applicable guides must be followed**.

When working in an area with recurring quality issues, consult:
- `docs/issues/README.md`
- the relevant issue folder

# Repository Rules Authority

All coding standards are defined in **`RULES.md`** at the project root.

This includes:
- UI/UX design philosophy
- Documentation governance
- Component structure
- Naming conventions
- CSS and styling rules
- Rust code style
- TypeScript/Svelte code style
- Architecture rules
- Quality assurance requirements

**Read `RULES.md` before making any code changes.**

Violations are blocking. Work that diverges from `RULES.md` or the applicable files under `docs/quality/guidelines/` is **not ready** for implementation, review, or merge.

# Quality Expectations

These guides define the expected level of quality for this codebase:

- Clean, maintainable code with stale references removed
- Strict separation of concerns
- Refactor discipline with behavior protected before restructuring
- Test-first or characterization-first changes whenever practical
- No acceptance of:
  - duplicate logic
  - dead code
  - debug leftovers
  - hardcoded design values
  - other avoidable quality drift

If code changes touch an area, stale references to removed features, outdated architecture, or previous implementations must be cleaned up as part of the work.

# Documentation Rules

Documentation is a first-class citizen in this repository.

## Mandatory Documentation Requirements
- Every agent or contributor must read the documentation in the order defined in **Mandatory Reading Order** before making any code change.
- Documentation must be accurate, up to date, and comprehensive.
- Every code change must include the necessary documentation updates.
- Obsolete or inaccurate documentation must be updated or removed as part of the same change.
- Stale references to removed features, old architecture, or previous implementations are not acceptable.

## Feature Documentation
| Feature | Document | Description |
|---------|----------|-------------|
| Thread Locking | `docs/features/thread-locking.md` | Backend/frontend behavior, user flow, design tokens |
| Agent Streaming Message IDs | `docs/features/agent-streaming-message-ids.md` | ID scheme for `streaming_content` events; backend/frontend contract that prevents messages from being overwritten |
| Agent Clarification | `docs/features/agent-clarification.md` | Pause/resume on `agent_narrate { intent: "question" }`; dual-layer store, `ExecutionCheckpoint`, restart rehydration, timeout fallback |
| Database Access & Debugging | `docs/database-reference.md` | SQLite schema, SQL queries, tracing, tips |

> Note: Files under `docs/archive/` are preserved for historical context and migration traceability. Archived documents may reference legacy paths and do **not** necessarily reflect current architecture or shipped features.

# UI/UX Rules

## UI/UX Golden Rule
Every user interaction must be designed with the user's needs and experience in mind.

This means prioritizing:
- clarity
- simplicity
- responsiveness

The underlying functionality must remain robust and efficient. All design decisions should follow this rule to create a seamless and intuitive user experience.

## Mandatory UI/UX Requirements
- All UI/UX changes must use `/frontend-design` skills
- All UI/UX changes must follow the UI/UX Golden Rule
- Changes must adhere to established design patterns
- Changes must ensure consistency across the application
- Changes should be validated through user testing when possible

## UI/UX Prototyping
For any relevant UI/UX creation or change:
- create an HTML prototype first
- ask for feedback before implementing the Svelte component

This is required to ensure the design is sound and meets user needs before implementation effort is spent.

### Prototype Storage
Prototypes must be saved to:

`.prototypes/<feature>/<prototype-name>.html`

This preserves design history and helps other team members understand the design process.

# Build Commands

## Rust Backend
```bash
cargo build
cargo test
cargo clippy
````

## Frontend

Run from `src-ui/`:

```bash
npm install
npm run dev
npm run build
npm run check
```

## Tauri

Run from repository root:

```bash
cargo tauri dev
cargo tauri build
```

# Database

Database is **SQLite** via `rusqlite` v0.31` (bundled, no ORM).

## Database Files

Stored in Tauri app data directory:

`~/.local/share/com.coder.app/`

Files:

* `coder_config.db`
* `coder_chat.db`
* `coder_context.db`
* `coder_agents.db`

## Rule for Live Inspection

Always use:

```bash
sqlite3 -readonly
```

while the app is running.

# Dependencies

## Rust

* `tauri` — desktop application framework for native Rust modules and frontend bindings
* `tokio` — async runtime
* `serde` — serialization/deserialization
* `anyhow` — error handling
* `rusqlite` — SQLite bindings (bundled, no external SQLite library required)

! ADD NEW DEPENDENCIES WHEN YOU SEE IT IN THE PROJECT AND UNDERSTAND HOW RELEVANT IT IS TO THE PROJECT. AVOID ADDING DEPENDENCIES UNNECESSARILY.

## Frontend

* `svelte` — UI framework
* `@sveltejs/kit` — SvelteKit framework
* `typescript` — type safety
* `vite` — build tool

---

# Development Process

## 1. Planning

Before any work, create a plan with the expected outcome. Review with the team if uncertain.

## 2. Implementation

Follow:

* `RULES.md`
* every applicable guide under `docs/quality/guidelines/`

Write clean, tested code and remove stale references in the touched area.

## 3. Testing

* Unit tests for new code
* Integration tests for critical flows
* Manual validation for UX

## 4. Documentation

Update relevant documentation and changelog as part of the work.

## 5. Branching

Use feature branches. Merge to `main` after review and testing.

## 6. Issues

Anytime the user ask to investigate an issue, and the issue is something on the agent flow execution,
something that degradete the agent execution, performance, or usability, you must to investigate it deeply,
you must have in mind that this it the core of the product and needs full attention, collect all the information you can,
document the evidence, and create a detailed issue with all the information you have, and if possible, a reproduction steps.
With that start a discussion with the team to find the best solution, and if you have a solution, create a plan to implement it, and share it with the team before start working on it.

IF The user share any backend log you must be able to check the tracking information in the database, and correlate it with the backend logs, to have a full picture of what is going on, and to be able to find the root cause of the issue.