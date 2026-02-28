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
