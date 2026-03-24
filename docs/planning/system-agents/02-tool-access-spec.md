# Tool Access Specification

## G1: Which Tools Can System Tasks Use?

### Overview

System tasks execute in the background without user supervision. To ensure security and prevent unintended modifications, system tasks will have **restricted tool access**.

---

## 1. Tool Categories

| Category | Tools | System Task Access |
|----------|-------|-------------------|
| **File System Read** | `fs_read_file`, `fs_list_files` | ✅ Allowed |
| **System Info** | `system_get_app_info`, `system_execute` (read-only) | ✅ Allowed |
| **Search** | `search_grep`, `google_scrape` | ✅ Allowed |
| **Conversation** | `conversation_search_messages` | ✅ Allowed |
| **File System Write** | `fs_write_file` | ❌ Blocked |
| **File Delete** | `Any delete operations` | ❌ Blocked |
| **Agent Control** | `agent_*` | ❌ Blocked |

---

## 2. Implementation

### 2.1 Tool Allowlist

```rust
// In executor.rs - tool allowlist
const SYSTEM_TASK_ALLOWED_TOOLS: &[&str] = &[
    "fs_read_file",
    "fs_list_files", 
    "system_get_app_info",
    "system_execute",      // Only safe commands (see below)
    "search_grep",
    "google_scrape",
    "conversation_search_messages",
];
```

### 2.2 Executor Implementation

```rust
impl SystemExecutor {
    pub fn new(tool_registry: ToolRegistry) -> Self {
        Self { tool_registry }
    }
    
    pub fn can_execute_tool(&self, tool_name: &str) -> bool {
        SYSTEM_TASK_ALLOWED_TOOLS.contains(&tool_name)
    }
    
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        workspace_root: Option<&str>,
    ) -> Result<ToolResult, ExecutorError> {
        // Check allowlist
        if !self.can_execute_tool(tool_name) {
            return Err(ExecutorError::ToolNotAllowed(format!(
                "Tool '{}' is not allowed for system tasks",
                tool_name
            )));
        }
        
        // ... execute tool
    }
}
```

### 2.3 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    #[error("Tool not allowed for system tasks: {0}")]
    ToolNotAllowed(String),
    
    #[error("Execution error: {0}")]
    Execution(String),
    
    #[error("Timeout")]
    Timeout,
}
```

---

## 3. Safe System Execute Commands

The `system_execute` tool runs shell commands. For system tasks, only safe read-only commands are allowed:

### 3.1 Allowed Commands

```rust
const SAFE_SYSTEM_COMMANDS: &[&str] = &[
    // Disk/Storage
    "df",           // disk free space
    "du",           // disk usage (read-only mode)
    
    // Memory
    "free",         // memory info
    
    // System
    "uptime",       // system uptime
    "uname",        // system info
    
    // Processes (read-only)
    "ps",           // list processes
    
    // File operations (read-only)
    "find",         // find files
    "ls",           // list directory
    "ls -la",       // list with details
    "ls -R",        // recursive list
    
    // File content (read-only)
    "cat",          // read file contents
    "head",         // read first lines
    "tail",         // read last lines
    "wc",           // word count
    "file",         // file type
    
    // Search
    "grep",         // search in files
    "rg",           // ripgrep
];

// Commands that are BLOCKED
const BLOCKED_SYSTEM_COMMANDS: &[&str] = &[
    "rm",           // delete
    "rmdir",        // delete directory
    "del",          // Windows delete
    "format",       // format drive
    "dd",           // disk destroy
    "mkfs",         // make filesystem
    "shutdown",     // shutdown
    "reboot",       // reboot
    "init",         // init
    "halt",         // halt
    "poweroff",     // poweroff
    ">",            // output redirect (can overwrite)
    ">>",           // output redirect append
    "|",            // pipe (can chain to destructive)
    "curl",         // network (potential security)
    "wget",         // network (potential security)
];
```

### 3.2 Command Validation

```rust
impl SystemExecutor {
    fn validate_system_command(command: &str) -> Result<(), ExecutorError> {
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        let base_cmd = cmd_parts.first().unwrap_or(&"");
        
        // Check if command is in allowlist
        if !SAFE_SYSTEM_COMMANDS.contains(base_cmd) {
            return Err(ExecutorError::ToolNotAllowed(format!(
                "System command '{}' is not allowed for system tasks",
                base_cmd
            )));
        }
        
        // Additional checks for dangerous patterns
        let full_command = command.to_lowercase();
        
        // Check for blocked patterns
        for blocked in BLOCKED_SYSTEM_COMMANDS {
            if full_command.contains(blocked) {
                return Err(ExecutorError::ToolNotAllowed(format!(
                    "Command contains blocked pattern: {}",
                    blocked
                )));
            }
        }
        
        Ok(())
    }
}
```

---

## 4. Task-Specific Tool Requirements

### 4.1 Sanitize Task

Requires:
- `system_get_app_info` - Get app version, status
- `system_execute` - Run df, du, ps, etc.
- `fs_list_files` - List log directories
- `search_grep` - Search for error patterns

### 4.2 Review Task

Requires:
- `conversation_search_messages` - Query agent conversations
- `fs_read_file` - Read session files if needed
- `search_grep` - Search for error patterns

### 4.3 Report Task

Requires:
- `conversation_search_messages` - Query conversations
- Read access to system_sessions (via internal store)

---

## 5. Security Considerations

### 5.1 Defense in Depth

1. **Allowlist Approach** - Only explicitly allowed tools can run
2. **Command Validation** - Shell commands validated before execution
3. **No File Write** - Write tools completely blocked
4. **Audit Logging** - Every tool execution logged
5. **Workspace Isolation** - Tasks can only access configured workspace

### 5.2 Audit Trail

All tool executions are logged with:

```rust
pub fn log_tool_run(
    &self,
    session_id: &str,
    tool_name: &str,
    input: &str,
    output: &str,
    success: bool
) -> Result<(), StoreError> {
    // Logs to system_task_runs table
}
```

---

## 6. Future Extensibility

### 6.1 Adding New Allowed Tools

To add a new tool to the allowlist:

1. Add tool name to `SYSTEM_TASK_ALLOWED_TOOLS` constant
2. Add tool to this specification document
3. Update tests
4. Document in changelog

### 6.2 Task-Specific Tool Sets

Future enhancement: Allow different tool sets per task:

```rust
struct TaskToolConfig {
    sanitize: vec!["system_get_app_info", "system_execute", "fs_list_files"],
    review: vec!["conversation_search_messages", "fs_read_file"],
    report: vec!["conversation_search_messages"],
}
```

---

## 7. Testing

### 7.1 Unit Tests

```rust
#[test]
fn test_tool_allowed_list() {
    assert!(SYSTEM_TASK_ALLOWED_TOOLS.contains(&"fs_read_file"));
    assert!(!SYSTEM_TASK_ALLOWED_TOOLS.contains(&"fs_write_file"));
}

#[test]
fn test_safe_commands() {
    assert!(SAFE_SYSTEM_COMMANDS.contains(&"df"));
    assert!(SAFE_SYSTEM_COMMANDS.contains(&"ls"));
}
```

### 7.2 Integration Tests

- Test that blocked tools return error
- Test that blocked commands return error
- Test that allowed tools execute successfully

---

## 8. Summary Table

| Tool/Command | Category | Allowed | Reason |
|--------------|----------|---------|--------|
| `fs_read_file` | File Read | ✅ Yes | Read-only |
| `fs_list_files` | File Read | ✅ Yes | Read-only |
| `fs_write_file` | File Write | ❌ No | Security risk |
| `system_get_app_info` | System | ✅ Yes | Read-only |
| `system_execute` | System | ⚠️ Conditional | Only safe commands |
| `search_grep` | Search | ✅ Yes | Read-only |
| `google_scrape` | Search | ⚠️ Conditional | Limited scope |
| `conversation_search_messages` | Conversation | ✅ Yes | Read-only |
| `agent_*` | Agent Control | ❌ No | Not needed |
