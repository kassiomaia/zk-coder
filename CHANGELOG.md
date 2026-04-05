# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial project structure with Tauri v2 + Svelte + TypeScript
- Rust backend with modular architecture (commands, agents, utils)
- SvelteKit frontend with static adapter for Tauri integration
- Design system with CSS custom properties for theming
- Dark mode support by default
- Global styles with Inter and JetBrains Mono fonts
- ESLint and Prettier configuration for frontend code quality
- Clippy configuration for Rust code quality
- Basic Tauri commands (`health_check`, `get_app_info`)
- AI agent type definitions
- A mandatory code-quality guideline set under `docs/quality/guidelines/` for Rust backend, TypeScript, Svelte, configuration, and scripts, with `AGENTS.md` enforcing the required reading order.

### Changed

- Unified NLP LLM integration with the shared provider-agnostic `llm` layer via `NlpLlmGateway` and config-driven model/provider/API-key resolution.
- Migrated NLP components (`IntentClassifier`, `MultiIntentDetector`, `ContextManager`) from legacy Claude-specific usage to shared gateway abstractions with deterministic fallback behavior on LLM failures.
- Rewired NLP app state/bootstrap initialization to use config/context databases and removed hardcoded dev API-key initialization paths from runtime wiring.
- Removed legacy Claude module files (`src-tauri/src/ai/claude.rs`, `src-tauri/src/ai/mod.rs`) and `pub mod ai` export from backend crate root.
- **Enabled LLM tools by default**: The orchestrated conversation runner with tool support is now the default, restoring workspace file listing and other tool functionality without requiring environment variable configuration.

### Fixed

- **Frontend dev server startup**: Restored modern SvelteKit dependency versions in `src-ui` so `vite.config.ts` can resolve `@sveltejs/kit/vite` again during `npm run dev`.
- **LLM tools regression**: Tools like workspace file listing were disabled after the refactor due to environment flag requirements. Now tools work out-of-the-box with graceful fallback to legacy runner if needed.
- **Agent session runtime persistence**: Thread-linked agent sessions now rehydrate persisted files touched and tool-call history when a thread is reopened, switched back to, or restored after restarting the app. Tool-call IPC payloads now include stable camelCase fields and persisted timestamps.
- **PPTX blank slides**: `pptx_finalize` now renders slide title and body content (text boxes, bullet lists, numbered lists) into generated `.pptx` files instead of creating empty slides, and regression tests now assert slide XML contains authored text.

### Technical Details

- **Backend**: Rust with Tauri v2, tokio, serde, anyhow, thiserror
- **Frontend**: Svelte 5, SvelteKit, TypeScript, Vite
- **Styling**: Custom CSS modules with design tokens
- **Architecture**: Modular design with clear separation of concerns

---

## [0.1.0] - 2026-02-28

### Added

- Project initialization
- Basic application skeleton
