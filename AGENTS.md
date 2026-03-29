# Coder - AI Assistant Desktop Application

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
| Database Access & Debugging | `docs/database-reference.md` | SQLite schema, SQL queries, tracing, tips |

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
- **Implementation**: Follow `RULES.md` guidelines. Write clean, tested code.
- **Testing**: Unit tests for new code. Integration tests for critical flows. Manual validation for UX.
- **Documentation**: Update relevant docs and changelog as part of the work.
