# Repository Guidelines

## Project Structure & Module Organization
The workspace is split between `src-tauri` (Rust backend) and `src-ui` (Svelte + TypeScript frontend). Keep shared data contracts in `src-tauri/src/agents` and backend utilities under `src-tauri/src/utils`. UI routes live in `src-ui/src/routes`, reusable components and helpers under `src-ui/src/lib`, while static assets stay in `src-ui/static` or `src-ui/src/lib/assets`.

## Build, Test, and Development Commands
Run `cd src-ui && npm install` once to install UI deps. Start the full desktop experience with `cargo tauri dev`, which compiles the Rust side and launches Vite in dev mode. For frontend-only work, `cd src-ui && npm run dev` spins up a browser preview. Ship-ready artifacts use `cargo build --manifest-path src-tauri/Cargo.toml` for Rust and `cd src-ui && npm run build` for static assets. Format and lint via `cargo fmt`, `cargo clippy`, and `cd src-ui && npm run lint`.

## Coding Style & Naming Conventions
Rust modules follow 4-space indentation, `snake_case` file names, exhaustive error handling, and the Clippy pedantic/nursery profile (see `Cargo.toml`). Avoid `unwrap`/`expect`; favor `anyhow::Result`. Frontend code uses 2-space indentation, TypeScript strict mode, and Svelte single-file components named in `PascalCase`. Keep utility files co-located with tests (e.g., `formatTimestamp.ts` + `.test.ts`) and rely on Prettier + ESLint for formatting. CSS belongs in the design system styles within `src-ui/src/lib/styles` unless a component-specific `<style>` block is essential.

## Testing Guidelines
Run backend tests with `cargo test`. Frontend unit tests use Vitest + Happy DOM via `cd src-ui && npm run test` (or `npm run test:watch`). Mirror the production file structure when naming specs—`foo.ts` pairs with `foo.test.ts`. Aim to cover every new helper and branch that touches async flows or command wiring, and update existing snapshots whenever UI copy or structure changes.

## Commit & Pull Request Guidelines
Follow the conventional `type(scope): summary` format used in history (`feat(skills): add svelte...`). Keep commits atomic, reference issue IDs in the scope or footer, and include relevant docs or config updates. Every PR should describe the change, list test evidence (`cargo test`, `npm run test`, `cargo tauri dev` smoke run), and call out UI changes with screenshots. Ensure CHANGELOG.md entries are added when user-facing behavior shifts.

## Security & Configuration Tips
Review `src-tauri/tauri.conf.json` and `src-tauri/capabilities/default.json` before exposing new Tauri commands; least-privilege capability files keep the desktop shell safe. Never hardcode secrets—use environment variables surfaced through Tauri’s config layering. Run `cargo audit` periodically if you add backend crates, and bundle only vetted static assets into `src-ui/static`.
