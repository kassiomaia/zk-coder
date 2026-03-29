# Database Access & Debugging

**Status**: ACTIVE
**Last reviewed**: 2026-03-29
**Purpose**: SQLite schema, queries, and debugging instructions for the Coder application

---

The application uses **SQLite** via `rusqlite` v0.31 (bundled, no ORM). All SQL is written as raw string literals in the Rust repository layer. Schema and migrations are code-based and run idempotently at every startup.

## Database Files & Locations

The app identifier is `com.coder.app`. All database files live in Tauri's platform-specific app data directory:

| Platform | Base Path |
|----------|-----------|
| **Linux** | `~/.local/share/com.coder.app/` |
| **macOS** | `~/Library/Application Support/com.coder.app/` |
| **Windows** | `C:\Users\<User>\AppData\Roaming\com.coder.app\` |

There are **four** separate SQLite database files:

| File | Purpose | Key Tables |
|------|---------|------------|
| `coder_config.db` | Providers, models, API keys, operation defaults | `providers`, `models`, `api_keys`, `operation_defaults`, `manual_models` |
| `coder_chat.db` | Conversations, threads, and chat messages | `conversations`, `threads`, `chat_messages` |
| `coder_context.db` | NLP conversation context windows | `context_windows`, `context_messages` |
| `coder_agents.db` | Agent sessions, tasks, tool calls, execution traces | `agent_sessions`, `agent_tasks`, `agent_subtasks`, `agent_tool_calls`, `agent_execution_steps`, `agent_runtime_traces`, `agent_execution_snapshots`, `agent_recovery_events`, `agent_files_touched`, `agent_terminal_output`, `agent_status_phrases` |

## Opening a Database

Use the `sqlite3` CLI tool. Always open in **read-only mode** to avoid corrupting data while the app is running:

```bash
# Linux/macOS
sqlite3 -readonly ~/.local/share/com.coder.app/coder_chat.db

# Open with column mode for readable output
sqlite3 -readonly -header -column ~/.local/share/com.coder.app/coder_agents.db
```

Useful sqlite3 dot-commands inside the REPL:

```
.tables                          -- list all tables
.schema <table_name>             -- show CREATE TABLE statement
.headers on                      -- show column headers
.mode column                     -- column-aligned output
.mode json                       -- JSON output (good for metadata columns)
.output /tmp/query_result.txt    -- redirect output to file
```

## Investigating Conversations & Messages (coder_chat.db)

```sql
-- List all conversations with message counts
SELECT c.id, c.title, c.archived, c.created_at, c.updated_at,
       (SELECT COUNT(*) FROM chat_messages WHERE conversation_id = c.id) AS msg_count
FROM conversations c
ORDER BY c.updated_at DESC;

-- List all threads for a conversation
SELECT t.id, t.title, t.message_count, t.metadata
FROM threads t
WHERE t.conversation_id = '<conversation_id>'
ORDER BY t.created_at;

-- View messages in a thread (most recent first)
SELECT m.role, substr(m.content, 1, 200) AS content_preview, m.timestamp, m.status
FROM chat_messages m
WHERE m.thread_id = '<thread_id>'
ORDER BY m.timestamp DESC;

-- Find all archived conversations
SELECT id, title, updated_at FROM conversations WHERE archived = 1;

-- Check thread metadata (resolved state, agent session links)
SELECT t.id, t.title, json_extract(t.metadata, '$.resolved') AS resolved,
       json_extract(t.metadata, '$.agentSessionId') AS agent_session_id
FROM threads t;
```

## Investigating Agent Execution & Traces (coder_agents.db)

```sql
-- List all agent sessions
SELECT id, agent_name, state, workspace_root, model_id, created_at, updated_at
FROM agent_sessions
ORDER BY created_at DESC;

-- View tasks for a session
SELECT id, substr(request, 1, 150) AS request_preview, status, created_at, completed_at
FROM agent_tasks
WHERE session_id = '<session_id>'
ORDER BY created_at;

-- View subtasks for a task
SELECT id, title, mode, status, order_index, started_at, completed_at
FROM agent_subtasks
WHERE task_id = '<task_id>'
ORDER BY order_index;

-- View all tool calls for a session (with duration)
SELECT tool_name, is_error, duration_ms,
       json_extract(arguments, '$') AS args,
       substr(json_extract(result, '$'), 1, 300) AS result_preview,
       timestamp
FROM agent_tool_calls
WHERE session_id = '<session_id>'
ORDER BY timestamp;

-- Runtime trace events for a session (most detailed debugging info)
SELECT step_number, trace_type, json_extract(payload_json, '$') AS payload
FROM agent_runtime_traces
WHERE session_id = '<session_id>'
ORDER BY step_number;

-- Filter traces by type (e.g., only errors or specific events)
SELECT step_number, trace_type, payload_json
FROM agent_runtime_traces
WHERE session_id = '<session_id>' AND trace_type = 'error'
ORDER BY step_number;

-- Execution steps with token usage
SELECT step_number, action_type,
       json_extract(action_data, '$') AS action,
       duration_ms,
       input_tokens, output_tokens, total_tokens
FROM agent_execution_steps
WHERE session_id = '<session_id>'
ORDER BY step_number;

-- Current execution snapshot (recovery state)
SELECT phase, step_number, context_usage_percent,
       total_tokens, last_model, last_error
FROM agent_execution_snapshots
WHERE session_id = '<session_id>';

-- Recovery events (crash/restart history)
SELECT reason, recovered_at
FROM agent_recovery_events
WHERE session_id = '<session_id>'
ORDER BY recovered_at DESC;

-- Files touched during a session
SELECT file_path, action, substr(diff, 1, 200) AS diff_preview, timestamp
FROM agent_files_touched
WHERE session_id = '<session_id>'
ORDER BY timestamp;

-- Terminal output for a session
SELECT line_type, substr(content, 1, 300) AS content_preview, timestamp
FROM agent_terminal_output
WHERE session_id = '<session_id>'
ORDER BY timestamp;

-- Status phrases reported during execution
SELECT phrase, timestamp
FROM agent_status_phrases
WHERE session_id = '<session_id>'
ORDER BY timestamp;
```

## Investigating Config & Providers (coder_config.db)

```sql
-- List all providers
SELECT id, name, api_type, base_url, is_configured, models_source
FROM providers;

-- List models for a provider
SELECT m.id, m.name, m.supports_analysis, m.supports_code, m.supports_vision,
       m.last_fetched
FROM models m
WHERE m.provider_id = '<provider_id>';

-- Operation defaults (which model is used for what)
SELECT od.operation, m.name AS model_name, p.name AS provider_name
FROM operation_defaults od
JOIN models m ON m.id = od.model_id
JOIN providers p ON p.id = m.provider_id;

-- Manual models
SELECT mm.name, p.name AS provider_name
FROM manual_models mm
JOIN providers p ON p.id = mm.provider_id;
```

## Investigating NLP Context (coder_context.db)

```sql
-- Context windows for all conversations
SELECT id, conversation_id, thread_id, created_at, updated_at
FROM context_windows;

-- Messages in a context window with token counts
SELECT role, substr(content, 1, 200) AS content_preview, tokens, timestamp
FROM context_messages
WHERE context_id = '<context_id>'
ORDER BY timestamp;

-- Total token usage per context window
SELECT cw.id, cw.conversation_id, SUM(cm.tokens) AS total_tokens
FROM context_windows cw
LEFT JOIN context_messages cm ON cm.context_id = cw.id
GROUP BY cw.id;
```

## Quick Reference: Schema Definitions

Schema definitions live in `src-tauri/src/infrastructure/sqlite/migrations/`:

| File | Database | Contents |
|------|----------|----------|
| `config_schema.rs` | `coder_config.db` | `providers`, `models`, `api_keys`, `operation_defaults`, `manual_models` |
| `chat_schema.rs` | `coder_chat.db` | `conversations`, `threads`, `chat_messages` |
| `nlp_context.rs` | `coder_context.db` | `context_windows`, `context_messages` |
| `agent.rs` | `coder_agents.db` | 11 tables (sessions, tasks, subtasks, tool calls, traces, snapshots, etc.) |

Data migrations are in the same directory:

| File | Migrations |
|------|-----------|
| `config.rs` | API type constraint, models source, URL normalization |
| `chat.rs` | Orphan messages to threads |
| `agent.rs` | Subtask mode, started_at, tool_call subtask_id, session model_id |

## Tips

- **Always use `-readonly`** when opening databases while the app is running to avoid lock conflicts.
- The `metadata` columns in `threads` and `conversations` are JSON — use `json_extract()` to query specific fields.
- The `arguments` and `result` columns in `agent_tool_calls` are JSON.
- The `steps` column in `agent_tasks` is JSON.
- If the app won't start due to a database issue, you can inspect the DB file directly with `sqlite3` when the app is stopped.
- Schema migrations are idempotent (`CREATE TABLE IF NOT EXISTS` + `ALTER TABLE` with error suppression), so they are safe to run on every startup.
