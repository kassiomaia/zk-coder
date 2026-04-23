# Slash Prompt Templates

## Summary

Every slash command in the chat input (`/refactor`, `/explain`, …) resolves to
a user-editable prompt template stored in `coder_config.db`. When a message
starts with a recognized slug, the backend rewrites it into the template body
before the orchestrator sees it — the LLM never receives the raw `/slug` text,
only the expanded prompt.

The feature is **prompt expansion only**. It does not invoke tools, control
the UI, or dispatch on intent enums. Commands that would require other
mechanisms (git, file navigation, local UI actions) are out of scope.

## User flow

1. Open chat. Type `/` — the command palette surfaces every enabled template.
2. Arrow-select one, press <kbd>Enter</kbd>. The palette inserts `/slug ` into
   the textarea at the cursor.
3. Type arguments after the slash — e.g. `/refactor tighten this loop`.
4. Send. The backend expands the message to the template body with
   `{args}` replaced by `tighten this loop`, and forwards it to the LLM.

## Placeholder syntax

Exactly one placeholder is supported today:

| Token    | Replaced with                                   |
|----------|-------------------------------------------------|
| `{args}` | Everything the user typed after `/slug` + space |

If `{args}` is absent from the template body, expansion still runs but the
user's arguments are discarded. If the user types the slug alone, `{args}`
expands to an empty string.

Multi-line arguments are preserved verbatim — newlines between `/slug` and
the rest of the message become part of `{args}`.

## Reset semantics

Builtin templates cannot be deleted; the **Reset** action restores the row to
its seeded default content by deleting it and running the startup seed for
that slug. Edits to a builtin live in place until reset.

User templates (`isBuiltin = 0`) can be edited or deleted outright; there is
no reset because there is no default to restore to.

## Builtins

Seeded on first run (and re-inserted on reset):

| Slug       | Purpose                            |
|------------|------------------------------------|
| `generate` | Generate code from a description   |
| `explain`  | Explain selected code              |
| `refactor` | Refactor selected code             |
| `optimize` | Optimize selected code             |
| `test`     | Generate tests                     |
| `document` | Write documentation                |

Content is seeded from `src-tauri/src/infrastructure/sqlite/repositories/config/prompt_templates.rs`
(`BUILTIN_TEMPLATES`).

## Architecture

### Data

Stored in `coder_config.db` (the user-config database), table
`prompt_templates`. Schema: `src-tauri/src/infrastructure/sqlite/migrations/config_schema.rs`.

```
id TEXT PK | slug TEXT UNIQUE | title | description | icon | category |
content | is_builtin | is_enabled | sort_order | created_at | updated_at
```

### Backend

- **Domain type**: `core::config::PromptTemplate` (+ `SavePromptTemplateRequest`).
- **Repository**: `core::config::PromptTemplateRepository`, implemented by
  `SqliteConfigRepository`. Seed is invoked from `SqliteConfigRepository::new`.
- **Tauri commands**: `commands::config::prompt_templates::{list, save, delete, reset}_prompt_template`.
- **Resolver**: `commands::llm::slash_resolver::resolve`. Called from
  `commands::llm::commands::send_chat_message` (and the streaming variant)
  exactly once per user turn, before delegating to `OrchestratedConversationRunner`.

### Frontend

- **API**: `$lib/api/config.ts` (`listPromptTemplates`, `savePromptTemplate`,
  `deletePromptTemplate`, `resetPromptTemplate`).
- **Store**: `$lib/stores/promptTemplatesStore.ts`. Initialized in
  `routes/+page.svelte` alongside `configStore`. Exposes a derived
  `promptTemplateCommands: Command[]` adapter for the chat palette.
- **Chat palette**: `ChatInput.svelte` consumes `promptTemplateCommands` when
  no explicit `commands` prop is passed.
- **Settings UI**: `PromptTemplatesSection.svelte` lives inside
  `SettingsModal.svelte`; CRUD goes through `PromptTemplateEditorModal.svelte`.

## Non-goals

- Rich placeholders (`{selection}`, `{current_file}`, `{clipboard}`, …).
- Tool-backed commands (`/git-status`, `/git-diff`, …).
- UI-action commands (`/new-chat`, `/clear`, `/theme`, `/settings`, `/export`).
- Import/export, sharing, or a template marketplace.
- Per-conversation or per-workspace overrides.

## Verification

**Backend**

```bash
cargo test --lib \
    infrastructure::sqlite::repositories::config::prompt_templates \
    commands::config::prompt_templates \
    commands::llm::slash_resolver
```

**Frontend**

```bash
cd src-ui && npm run check
```

**Manual end-to-end**: see the *Verification* section of the approved plan
(`~/.claude/plans/let-adjust-somethings-1-quirky-pond.md`) or reproduce
quickly by typing `/refactor hello` in chat and watching the backend log for
the `Expanded slash command` debug line.

**Database inspection**

```bash
sqlite3 -readonly ~/.local/share/com.coder.app/coder_config.db \
    'SELECT slug, is_builtin, is_enabled FROM prompt_templates'
```
