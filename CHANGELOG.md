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

- **CSS integrity across the UI**: Eliminated undefined CSS variables (`--color-text-quaternary`, `--color-error-200/600/subtle`, `--color-success-200/600`, `--color-warning-200/600/subtle`, `--color-border/-default/-hover/-muted`, `--color-bg-hover`, `--color-surface-disabled`, `--color-surface-primary`, `--color-primary-glow-strong`, `--color-accent-cyan`, `--color-accent-rose`, `--text-tertiary`) across `StatusBadge`, `Input`, `Select`, `ArtifactPanel`, `ArtifactCard`, `AgentConfig`, `AgentPanelHeader`, `AppShell`, `MentionMenu`, `MarkdownPreview`, `CommandPalette`, `ConversationSidebar`, `ConversationItem`, `SystemNoticeMessage`, and `ThreadItem` by remapping each reference to the correct token in `design-system.css`. Added the missing `--space-4-5` and `--space-7-5` spacing tokens that `ThinkingMessage`, `ToolCallMessage`, and `ToolCallItem` expected. Removed two empty rulesets (`.hasUnread`, `.streaming`) in `ThreadItem.module.css` that previously triggered Vite's CSS module loader failure mode. Replaced the invalid jQuery-style selector `.artifact-card:has(.artifact-language:contains('Terminal'))` in `ArtifactCard.svelte` with a Svelte state class (`class:terminal={type === 'terminal'}`) + `.artifact-card.terminal`; the previous selector was rejected by Svelte's CSS parser and was the root cause of the cascading `[vite-plugin-svelte:load] failed to load virtual css module` warnings for `+page.svelte`, `AppShell.svelte`, `AgentPanel.svelte`, `SessionHistory.svelte`, and `AgentConfig.svelte`. To prevent this recurring class of bug, added `src-ui/src/lib/styles/css-integrity.test.ts` which fails the test suite when any `var(--token)` without a fallback references an undefined variable, any CSS ruleset has a whitespace-only body, or any selector uses a jQuery-only pseudo-class (`:contains`, `:parent`, `:eq`, `:gt`, `:lt`), and configured `vite.config.ts` with `server.warmup.clientFiles: ['./src/**/*.svelte']` so Svelte's CSS parser runs against every component at dev-server start — invalid selectors now surface as a first-class error in the dev log instead of hiding behind a cascade of virtual-CSS load warnings. Wired the test and the warmup into the CSS Verification Checklist in `RULES.md`.

- **Frontend dev server startup**: Restored modern SvelteKit dependency versions in `src-ui` so `vite.config.ts` can resolve `@sveltejs/kit/vite` again during `npm run dev`.
- **LLM tools regression**: Tools like workspace file listing were disabled after the refactor due to environment flag requirements. Now tools work out-of-the-box with graceful fallback to legacy runner if needed.
- **Agent session runtime persistence**: Thread-linked agent sessions now rehydrate persisted files touched and tool-call history when a thread is reopened, switched back to, or restored after restarting the app. Tool-call IPC payloads now include stable camelCase fields and persisted timestamps.
- **PPTX blank slides**: `pptx_finalize` now renders slide title and body content (text boxes, bullet lists, numbered lists) into generated `.pptx` files instead of creating empty slides, and regression tests now assert slide XML contains authored text.
- **Agent command spawning**: Added a backend spawn service with policy checks, bounded concurrency, cancellation handling, and runtime trace telemetry so command failures stay inside the tool/runtime boundary instead of crashing the app.
- **Agent messages replaced on the frontend**: Subtask results, the "continuing with next subtask..." divider, and the final summary all shared a single session-wide message id when the backend emitted `streaming_content` without a `messageId`, so each new message overwrote the previous one in the chat store. Introduced a phase-aware id scheme (`stream-{phase}-{session}-{task}-{subtask}-s{step}`) built by a new `streaming_message_id` helper, made `emit_streaming_content` / `emit_streaming_message` require an explicit id at compile time, and added a defensive frontend fallback that generates a unique orphan id per event with a logged warning if a missing id ever slips through. Covered by new unit and integration tests in `message_ids.rs`, `termination.rs`, and `streamingEvents.test.ts`. See `docs/features/agent-streaming-message-ids.md`.
- **Silent provider-removal failures**: `configStore.removeProvider` now captures backend errors into `state.error` before rethrowing, and `+page.svelte` forwards that error into the existing status-bar surface so users see a visible message instead of the Remove button appearing to do nothing. Removed the dead inline fallback in `SettingsModal.svelte` — the callsite always supplies `onremoveprovider`. Covered by new tests in `configStore.test.ts` and an updated handler assertion in `SettingsModal.test.ts`.
- **Stale validation error in `CustomProviderModal`**: A failed submit (e.g. "Invalid URL format") previously remained on screen while the user typed into the field that would fix it. Input change handlers for provider name, provider id, base URL, and API-type radios now clear the inline error on edit, so the banner disappears as soon as the user starts correcting the form. Covered by new tests in `CustomProviderModal.test.ts`.
- **`ThreadIndicator` didn't surface resolved state**: the inline thread-badge rendered on messages ignored `thread.metadata.resolved` entirely, while the sidebar `ThreadItem` already used a check-circle icon for resolved threads — so users had inconsistent visual signals about thread lifecycle. `ThreadIndicator.svelte` now takes an optional `resolved` prop, renders a check-circle icon and muted `.resolved` styling when set, and exposes a distinct `aria-label`. `ChatMessagesList.svelte` wires the flag through via a new `isThreadResolved(id)` helper that reads from the existing `threads` prop. Covered by new tests in `ThreadIndicator.test.ts`.
- **`STREAM_END` final scroll was unreachable**: `handleStreamingEnd` in `scrollController.ts` called `sendEvent('STREAM_END')` before reading `context.autoScrollEnabled`, but the state-change listener synchronously flipped that flag to `false` as soon as the state machine transitioned to `idle`, so the guarded catch-up scroll at stream end never ran. Long `isComplete.content` payloads could leave the viewport a few lines short of the true bottom. `handleStreamingEnd` now snapshots `autoScrollEnabled` before dispatching the event, and the dead mirror-branch for `STREAM_END` inside `handleStateTransition` was removed. Covered by new tests in `scrollController.test.ts`.

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
