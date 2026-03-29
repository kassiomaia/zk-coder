# Coder - AI Assistant Desktop Application

A modern desktop application built with **Rust** (Tauri v2) and **Svelte + TypeScript** for AI-powered developer automation.

## MANDATORY: Load AGENTS.md

**All development rules live in `AGENTS.md`.** You **MUST** read `AGENTS.md` in full before making any changes to this project. It contains:

- Project overview, design thinking, and goals
- Tech stack and architecture details
- Thread locking behavior
- Database access and debugging instructions (SQLite via rusqlite)
- Development process (branching, feature planning, bug fixes)
- Build commands
- Documentation standards

**Coding rules** (UI/UX golden rule, CSS/styling, Rust/TypeScript style, architecture, QA, documentation governance) are in **`RULES.md`** (project root). Read `RULES.md` before making any code changes.

**Feature specs** are in `docs/`:
- Thread Locking: `docs/features/thread-locking.md`
- Database reference: `docs/database-reference.md`

**Do not proceed with any code change without reading `AGENTS.md` first.**

## Quick Reference: Build Commands

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
