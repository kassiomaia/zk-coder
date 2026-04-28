# UI Theme

**Status**: ACTIVE
**Last reviewed**: 2026-04-28
**Purpose**: Document the user-selectable UI theme — backend persistence, frontend store, and Settings UI.

## What it does

Lets the user pick between **Light**, **Dark**, and **System** themes from
Settings → Appearance. The choice is persisted to `coder_config.db` and
restored on the next launch. `System` follows the OS preferred-color-scheme and
updates live when the OS toggles.

## End-to-end flow

```
┌─────────────────┐          ┌──────────────────┐          ┌──────────────────┐
│ AppearanceSect. │ user     │   themeStore     │  invoke  │   Tauri command  │
│  (Svelte UI)    │─────────▶│   (Svelte)       │─────────▶│  set_ui_theme    │
│                 │ click    │                  │          │                  │
└─────────────────┘          └──────────────────┘          └────────┬─────────┘
                                      │                              │
                                      │ document.documentElement     │
                                      │ .setAttribute('data-theme',  │
                                      │   resolved)                  │
                                      ▼                              ▼
                             ┌──────────────────┐          ┌──────────────────┐
                             │ CSS variables    │          │  ui_preferences  │
                             │ from design-     │          │   (SQLite row)   │
                             │ system.css       │          │                  │
                             └──────────────────┘          └──────────────────┘
```

## Backend

### Schema

`config_schema.rs` defines a single-row global table:

```sql
CREATE TABLE IF NOT EXISTS ui_preferences (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    theme       TEXT NOT NULL CHECK (theme IN ('light', 'dark', 'system')),
    updated_at  TEXT NOT NULL
);
```

The `id = 1` constraint enforces "exactly one row" without a sentinel marker
column. The `theme` CHECK is the canonical allow-list — bypassing the
command-layer validator still produces a SQLite error.

### Trait

`core::config::repositories::UiThemeRepository`:

```rust
pub trait UiThemeRepository: Send + Sync {
    fn get_ui_theme(&self) -> RepositoryResult<Option<String>>;
    fn set_ui_theme(&self, theme: &str) -> RepositoryResult<()>;
}
```

`None` means "no preference recorded" — the frontend then falls back to the
OS preferred-color-scheme. The trait is part of the `ConfigRepository`
aggregate, so any consumer that already has a `ConfigRepository` gets it for
free.

### SQLite implementation

`infrastructure/sqlite/repositories/config/ui_theme.rs` is a thin
`get` / `set` pair using `INSERT OR REPLACE`-style upsert keyed on `id = 1`.
Four unit tests cover: empty initial state, round-trip, overwrite, and
CHECK-constraint rejection of an invalid value.

### Tauri commands

`commands/config/ui_theme.rs` exposes:

| Command          | Args                | Returns                |
| ---------------- | ------------------- | ---------------------- |
| `get_ui_theme`   | —                   | `Option<String>`       |
| `set_ui_theme`   | `theme: String`     | `()`                   |

`set_ui_theme` validates `theme` against the allow-list before hitting the
database so the backend never round-trips an invalid value, even though the
schema would also reject it.

Both commands are registered in the `tauri::generate_handler!` block of
`src-tauri/src/lib.rs`.

## Frontend

### Store

`src-ui/src/lib/stores/themeStore.ts` is the single source of truth. It owns
three concerns:

1. **Preference** (`'light' | 'dark' | 'system'`) — the user's choice.
2. **System theme** (`'light' | 'dark'`) — from `matchMedia('(prefers-color-scheme: dark)')`.
3. **Resolved theme** (`'light' | 'dark'`) — `derived([preference, system], …)`,
   the value actually applied to `data-theme`.

Public surface:

```ts
export const theme: Readable<ResolvedTheme>;          // for reactive consumers
export const themePreference: { subscribe };          // raw user preference
export function getResolvedTheme(): ResolvedTheme;    // imperative read
export async function initTheme(): Promise<() => void>;
export const themeStore: { subscribe, setPreference };
```

`initTheme()` is called once from `+layout.svelte`. It:

1. Reads the persisted preference via `getUiTheme()`.
2. If the backend has no value, migrates the legacy `localStorage.theme` key
   (written by the previous theme implementation) and calls `setUiTheme`.
3. Applies `data-theme` synchronously so the first paint is correct.
4. Subscribes to `theme` so future changes flow back to the DOM.
5. Subscribes to the OS color-scheme media query.
6. Returns a teardown function that unwires both subscriptions.

### Pre-hydration boot script

`src-ui/src/app.html` runs a tiny inline script before SvelteKit hydrates. It
reads `localStorage.theme` (synchronously, no await) and applies `data-theme`
to `<html>` so dark-theme users do not see a light flash while the async
backend lookup runs. The store reconciles the localStorage value with the
backend on mount.

### Settings UI

`src-ui/src/lib/components/config/AppearanceSection.svelte` renders three
radio cards (Light / Dark / System). Each card shows:

- A miniature app preview composed from real design tokens (header, sidebar,
  content rows). The previews use scoped per-option color tokens
  (`.previewLight`, `.previewDark`, `.previewSystem`), so they stay truthful
  to the rendered theme without snapshot images.
- An icon (Sun / Moon / Monitor) and a label.
- A check mark that fades in on the selected option.

Keyboard navigation: ←/→/↑/↓ cycle through options and immediately apply each.

The section is mounted as the first entry in `SettingsModal.svelte`.

## Adding a new theme

1. Add a new `[data-theme='<name>']` block to `design-system.css` defining the
   theme-dependent tokens. Cross-reference the existing two blocks — every
   token redefined in one must be redefined in the other.
2. Extend the `theme` CHECK constraint in `config_schema.rs` and the
   `ALLOWED_THEMES` array in `commands/config/ui_theme.rs`.
3. Extend `ThemePreference` and `ResolvedTheme` in `themeStore.ts`.
4. Add a new option to the `options` array in `AppearanceSection.svelte`,
   plus a matching `.preview<Name>` block in `AppearanceSection.module.css`.

## Related files

| Concern             | Path                                                                        |
| ------------------- | --------------------------------------------------------------------------- |
| Design tokens       | `src-ui/src/lib/styles/design-system.css`                                   |
| Schema              | `src-tauri/src/infrastructure/sqlite/migrations/config_schema.rs`           |
| Repo trait          | `src-tauri/src/core/config/repositories.rs` (`UiThemeRepository`)           |
| Repo impl           | `src-tauri/src/infrastructure/sqlite/repositories/config/ui_theme.rs`       |
| Tauri commands      | `src-tauri/src/commands/config/ui_theme.rs`                                 |
| Command registration| `src-tauri/src/lib.rs`                                                      |
| API client          | `src-ui/src/lib/api/config.ts` (`getUiTheme` / `setUiTheme`)                |
| Store               | `src-ui/src/lib/stores/themeStore.ts`                                       |
| Pre-hydration       | `src-ui/src/app.html`                                                       |
| Layout integration  | `src-ui/src/routes/+layout.svelte`                                          |
| Settings UI         | `src-ui/src/lib/components/config/AppearanceSection.{svelte,module.css}`    |
| Modal mount         | `src-ui/src/lib/components/config/SettingsModal.svelte`                     |
| Icons               | `src-ui/src/lib/icons/{Sun,Moon,Monitor}Icon.svelte`                        |
