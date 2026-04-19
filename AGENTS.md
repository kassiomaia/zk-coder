# Coder - AI Assistant Desktop Application

@README.md - Overview, quick start, key features, development commands, module documentation pointers
@RULES.md - Coding standards and architectural rules

Desktop application built with Rust (Tauri v2) and Svelte/TypeScript for AI-powered developer automation.

**Core principles**: User-centric design, performance via Rust/Tauri, modular architecture, security, cross-platform (Linux/macOS/Windows).

**Key capabilities**: AI agent integration, code generation, task automation, productivity tools.

## Tech Stack

- **Backend**: Rust
- **Frontend**: Svelte + TypeScript
- **Styling**: Custom CSS modules + Component library
- **Desktop Framework**: Tauri v2

### Coding Rules

All coding standards are in **`RULES.md`** (project root):

- UI/UX golden rule (design philosophy)
- Documentation governance (lifecycle, creation, maintenance, removal)
- Component structure, naming conventions
- CSS & styling rules (prohibited patterns, verification checklist, shared files)
- Rust, TypeScript/Svelte code style
- Architecture rules, quality assurance requirements

**Read `RULES.md` before making any code changes.**

### Mandatory Code Quality Guidance

`AGENTS.md` is the mandatory entry point for development guidance in this repository.

Before making **any** code change, every agent or contributor must read, in order:

1. `AGENTS.md`
2. `RULES.md`
3. `docs/quality/guidelines/README.md`
4. Every touched-area guide:
   - `docs/quality/guidelines/rust-backend.md`
   - `docs/quality/guidelines/typescript.md`
   - `docs/quality/guidelines/svelte.md`
   - `docs/quality/guidelines/configuration.md`
   - `docs/quality/guidelines/scripts.md`
5. The relevant architecture/feature/database docs for the area being changed

If a task spans multiple layers or technologies, **all applicable guides must be followed**.

These guides are mandatory and define the expected level of quality for this codebase:

- clean, maintainable code with stale references removed
- strict separation of concerns
- refactor discipline with behavior protected before restructuring
- test-first or characterization-first changes whenever practical
- no acceptance of duplicate logic, dead code, debug leftovers, hardcoded design values, or other avoidable quality drift

Violations are blocking. Work that diverges from `RULES.md` or the applicable files under `docs/quality/guidelines/` is not ready for implementation, review, or merge.

When working in an area with recurring quality issues, consult `docs/issues/README.md` and the relevant issue folder before finalizing the change.

## Build Commands

```bash
# Rust backend
cargo build              # Build the project
cargo test               # Run tests
cargo clippy             # Run linter

# Frontend (run from src-ui/)
npm install              # Install dependencies
npm run dev              # Development server
npm run build            # Production build
npm run check            # Type checking

# Tauri (run from root)
cargo tauri dev          # Run in development mode
cargo tauri build        # Build for production
```

## Feature Documentation

| Feature | Document | Description |
|---------|----------|-------------|
| Thread Locking | `docs/features/thread-locking.md` | Backend/frontend behavior, user flow, design tokens |
| Agent Streaming Message IDs | `docs/features/agent-streaming-message-ids.md` | Id scheme for `streaming_content` events; backend/frontend contract that prevents messages from being overwritten |
| Agent Clarification | `docs/features/agent-clarification.md` | Pause/resume on `agent_narrate { intent: "question" }` — dual-layer store, `ExecutionCheckpoint`, restart rehydration, timeout fallback |
| Database Access & Debugging | `docs/database-reference.md` | SQLite schema, SQL queries, tracing, tips |

> Note: Files under `docs/archive/` are preserved for historical context and migration traceability. Archived documents may reference legacy paths and do **not** necessarily reflect current architecture or shipped features.

Database is SQLite via `rusqlite` v0.31 (bundled, no ORM). Four DB files in Tauri's app data dir (`~/.local/share/com.coder.app/`): `coder_config.db`, `coder_chat.db`, `coder_context.db`, `coder_agents.db`. Always use `sqlite3 -readonly` while the app is running.

## Dependencies

### Rust
- `tauri` - Desktop application framework for building native Rust modules and API bindings for the frontend
- `tokio` - Async runtime
- `serde` - Serialization/deserialization
- `anyhow` - Error handling
- `rusqlite` - SQLite bindings (bundled, no external SQLite library required)

### Frontend
- `svelte` - UI framework
- `@sveltejs/kit` - SvelteKit framework
- `typescript` - Type safety
- `vite` - Build tool

## Development Process

- **Branching**: Feature branches. Merge to main after review and testing.
- **Planning**: Before any work, create a plan with expected outcome. Review with team if uncertain.
- **Implementation**: Follow `RULES.md` and every applicable guide under `docs/quality/guidelines/`. Write clean, tested code and remove stale references in the touched area.
- **Testing**: Unit tests for new code. Integration tests for critical flows. Manual validation for UX.
- **Documentation**: Update relevant docs and changelog as part of the work.
