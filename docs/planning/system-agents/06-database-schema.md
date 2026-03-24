# Database Schema

## System Agents Database

**Database File:** `coder_system.db`

---

## 1. ER Diagram

```
┌─────────────────────────┐         ┌─────────────────────────┐
│    system_sessions      │         │   system_schedules      │
├─────────────────────────┤         ├─────────────────────────┤
│ id (PK)                │         │ task_type (PK)          │
│ task_type              │────────▶│ schedule                │
│ status                 │         │ enabled                 │
│ started_at             │         │ last_run                │
│ completed_at           │         │ next_run                │
│ error_message         │         │ failure_count           │
│ workspace_root         │         │ config                  │
│ created_at             │         └─────────────────────────┘
└───────────┬───────────┘
            │
            │ 1:N
            ▼
┌─────────────────────────┐
│   system_task_runs     │
├─────────────────────────┤
│ id (PK)                │
│ session_id (FK)        │────────▶ system_sessions.id
│ tool_name              │
│ tool_input             │
│ tool_output            │
│ executed_at            │
│ success                │
└─────────────────────────┘
```

---

## 2. Tables

### 2.1 system_sessions

Task execution instances.

```sql
CREATE TABLE system_sessions (
    id TEXT PRIMARY KEY,
    task_type TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    workspace_root TEXT,
    created_at TEXT NOT NULL
);
```

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | TEXT | PRIMARY KEY | UUID of the session |
| `task_type` | TEXT | NOT NULL | Task identifier: `sanitize`, `review`, `report` |
| `status` | TEXT | NOT NULL | Current status |
| `started_at` | TEXT | | When task started (ISO 8601) |
| `completed_at` | TEXT | | When task completed (ISO 8601) |
| `error_message` | TEXT | | Error message if failed |
| `workspace_root` | TEXT | | Workspace path for task |
| `created_at` | TEXT | NOT NULL | When session created (ISO 8601) |

**Status Values:**
- `pending` - Task is queued
- `running` - Task is executing
- `completed` - Task finished successfully
- `failed` - Task failed
- `paused` - Task paused due to failures

---

### 2.2 system_task_runs

Individual tool executions within a session.

```sql
CREATE TABLE system_task_runs (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES system_sessions(id),
    tool_name TEXT NOT NULL,
    tool_input TEXT,
    tool_output TEXT,
    executed_at TEXT NOT NULL,
    success INTEGER
);
```

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | TEXT | PRIMARY KEY | UUID of the run |
| `session_id` | TEXT | REFERENCES system_sessions(id) | Parent session |
| `tool_name` | TEXT | NOT NULL | Name of tool executed |
| `tool_input` | TEXT | | JSON of tool arguments |
| `tool_output` | TEXT | | JSON of tool result |
| `executed_at` | TEXT | NOT NULL | When executed (ISO 8601) |
| `success` | INTEGER | | 1 if success, 0 if failed |

---

### 2.3 system_schedules

Task schedules and configuration.

```sql
CREATE TABLE system_schedules (
    task_type TEXT PRIMARY KEY,
    schedule TEXT NOT NULL,
    enabled INTEGER DEFAULT 1,
    last_run TEXT,
    next_run TEXT,
    failure_count INTEGER DEFAULT 0,
    config TEXT
);
```

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `task_type` | TEXT | PRIMARY KEY | Task identifier |
| `schedule` | TEXT | NOT NULL | Cron expression |
| `enabled` | INTEGER | DEFAULT 1 | 1 if enabled, 0 if disabled |
| `last_run` | TEXT | | Last execution time (ISO 8601) |
| `next_run` | TEXT | | Next scheduled execution |
| `failure_count` | INTEGER | DEFAULT 0 | Consecutive failures |
| `config` | TEXT | | JSON for task-specific config |

---

## 3. Indexes

```sql
CREATE INDEX IF NOT EXISTS idx_sessions_task_type 
    ON system_sessions(task_type);

CREATE INDEX IF NOT EXISTS idx_task_runs_session 
    ON system_task_runs(session_id);
```

---

## 4. Relationships

```
system_schedules
    │
    │ 1:N (task_type)
    ▼
system_sessions
    │
    │ 1:N (session_id)
    ▼
system_task_runs
```

---

## 5. Example Data

### 5.1 system_sessions

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "task_type": "sanitize",
  "status": "completed",
  "started_at": "2024-01-15T00:00:00Z",
  "completed_at": "2024-01-15T00:05:00Z",
  "error_message": null,
  "workspace_root": "/home/user/project",
  "created_at": "2024-01-15T00:00:00Z"
}
```

### 5.2 system_task_runs

```json
{
  "id": "660e8400-e29b-41d4-a716-446655440001",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "tool_name": "system_get_app_info",
  "tool_input": "{}",
  "tool_output": "{\"version\": \"0.1.0\", \"status\": \"running\"}",
  "executed_at": "2024-01-15T00:00:01Z",
  "success": 1
}
```

### 5.3 system_schedules

```json
{
  "task_type": "sanitize",
  "schedule": "0 0 * * *",
  "enabled": 1,
  "last_run": "2024-01-15T00:00:00Z",
  "next_run": "2024-01-16T00:00:00Z",
  "failure_count": 0,
  "config": null
}
```

---

## 6. Queries

### 6.1 Get Task History

```sql
SELECT * FROM system_sessions 
WHERE task_type = 'sanitize' 
ORDER BY created_at DESC 
LIMIT 10;
```

### 6.2 Get Running Sessions

```sql
SELECT * FROM system_sessions 
WHERE status = 'running';
```

### 6.3 Get Failed Tasks

```sql
SELECT * FROM system_sessions 
WHERE status = 'failed' 
AND created_at > datetime('now', '-7 days');
```

### 6.4 Get Tool Run Details

```sql
SELECT * FROM system_task_runs 
WHERE session_id = '550e8400-e29b-41d4-a716-446655440000';
```

---

## 7. Migrations

### 7.1 Adding a New Table

```sql
-- Example migration
CREATE TABLE IF NOT EXISTS system_new_table (
    id TEXT PRIMARY KEY,
    -- ... columns
);

-- Add migration record
INSERT INTO schema_migrations (version, applied_at) 
VALUES ('001', datetime('now'));
```

### 7.2 Schema Version Tracking

```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL
);
```

---

## 8. Database Location

| Platform | Path |
|----------|------|
| Linux | `~/.local/share/com.coder.app/coder_system.db` |
| macOS | `~/Library/Application Support/com.coder.app/coder_system.db` |
| Windows | `%APPDATA%\com.coder.app\coder_system.db` |

---

## 9. Backup and Restore

### 9.1 Backup

```bash
cp ~/.local/share/com.coder.app/coder_system.db backup_system.db
```

### 9.2 Restore

```bash
cp backup_system.db ~/.local/share/com.coder.app/coder_system.db
```

---

## 10. Comparison with Existing DBs

| Database | Tables | Purpose |
|----------|--------|---------|
| `coder_config.db` | 5 | Configuration, providers, models |
| `coder_chat.db` | 4 | Conversations, threads, messages |
| `coder_agents.db` | 8 | Agent sessions, tasks, tool calls |
| `coder_context.db` | 2 | NLP context |
| `coder_system.db` | 4 | System agent tasks (NEW) |

---

## 11. Testing with SQLite

### 11.1 Interactive Testing

```bash
sqlite3 ~/.local/share/com.coder.app/coder_system.db

# List tables
.tables

# Describe table
.schema system_sessions

# Query data
SELECT * FROM system_sessions;
```

### 11.2 Schema Validation

```sql
-- Check all tables exist
SELECT name FROM sqlite_master 
WHERE type='table' 
AND name LIKE 'system_%';
```
