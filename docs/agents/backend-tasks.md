# Backend Implementation Tasks

> **Parent**: [Implementation Plan](./IMPLEMENTATION-PLAN.md)
> **Focus**: Rust backend changes in `src-tauri/src/`

---

## §1. Agent Types and Session Abstraction

### 1.1 Core Type Definitions

**File**: `src-tauri/src/agents/types.rs`

Current state: Stub types only. Need to expand significantly.

```rust
// Agent configuration (persisted)
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<ToolId>,
    pub default_model_id: String,
    pub trust_mode: TrustMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Agent ID (newtype for type safety)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

// Trust mode determines permission behavior
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrustMode {
    #[default]
    Untrusted,   // Require approval for destructive ops
    Trusted,     // Auto-run all tools
    Restricted,  // Require approval for all tools
}
```

### 1.2 Session Types

**File**: `src-tauri/src/agents/session.rs` (new)

```rust
// Session ID
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

// Running session (ephemeral, stateful)
pub struct AgentSession {
    pub id: SessionId,
    pub agent_id: AgentId,
    pub conversation_id: ConversationId,
    pub thread_id: ThreadId,
    pub state: SessionState,
    pub parent_session_id: Option<SessionId>,
    pub created_at: DateTime<Utc>,
    pub workspace_root: Option<PathBuf>,
    pub pending_actions: Vec<PendingAction>,
}

// Session lifecycle state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionState {
    Launching,
    Running {
        current_step: u32,
        max_steps: u32,
    },
    WaitForRequest {
        last_action_at: DateTime<Utc>,
    },
    UserInputRequired {
        pending_actions: Vec<PendingAction>,
    },
    Finished {
        completed_at: DateTime<Utc>,
    },
    Cancelled {
        reason: String,
    },
    Error {
        message: String,
        recoverable: bool,
    },
}

// Pending action awaiting user approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: String,
    pub tool_name: String,
    pub action_type: ActionType,
    pub description: String,
    pub preview: Option<ActionPreview>,
    pub arguments: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionType {
    ReadFile { path: PathBuf },
    WriteFile { path: PathBuf, size_bytes: u64 },
    DeleteFile { path: PathBuf },
    ExecuteCommand { command: String },
    WebRequest { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPreview {
    pub content_type: String,  // "diff", "text", "json"
    pub content: String,
    pub language: Option<String>,
}
```

### 1.3 Task Definition

**File**: `src-tauri/src/agents/task.rs` (new)

```rust
// Agent request (what user sends)
pub struct AgentRequest {
    pub content: String,
    pub attachments: Vec<Attachment>,
    pub context: Option<RequestContext>,
}

// Task execution tracking
pub struct AgentTask {
    pub id: String,
    pub session_id: SessionId,
    pub request: AgentRequest,
    pub status: TaskStatus,
    pub steps: Vec<ExecutionStep>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_number: u32,
    pub action: StepAction,
    pub result: Option<StepResult>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepAction {
    LlmRequest { model: String },
    ToolCall { tool: String, arguments: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub tokens_used: Option<u32>,
    pub duration_ms: u64,
}
```

### 1.4 Module Structure

**File**: `src-tauri/src/agents/mod.rs`

```rust
pub mod types;
pub mod session;
pub mod task;
pub mod registry;
pub mod runtime;
pub mod permissions;
pub mod store;

pub use types::*;
pub use session::*;
pub use task::*;
```

---

## §2. Permission System

### 2.1 Permission Definitions

**File**: `src-tauri/src/agents/permissions.rs` (new)

```rust
// Permission level for tools
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    #[default]
    AutoRun,           // Execute immediately
    ApprovalRequired,  // Queue for user approval
    Disabled,          // Tool not available
}

// Tool permission configuration
pub struct ToolPermission {
    pub tool_id: ToolId,
    pub level: PermissionLevel,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

// Permission resolver
pub struct PermissionResolver {
    trust_mode: TrustMode,
    tool_permissions: HashMap<ToolId, PermissionLevel>,
    category_permissions: HashMap<String, PermissionLevel>,
}

impl PermissionResolver {
    pub fn new(trust_mode: TrustMode) -> Self {
        Self {
            trust_mode,
            tool_permissions: Self::default_tool_permissions(),
            category_permissions: Self::default_category_permissions(),
        }
    }
    
    pub fn check_permission(&self, tool: &ToolId, action: &ActionType) -> PermissionLevel {
        // Trust mode overrides
        match self.trust_mode {
            TrustMode::Trusted => PermissionLevel::AutoRun,
            TrustMode::Restricted => PermissionLevel::ApprovalRequired,
            TrustMode::Untrusted => {
                // Check tool-specific permission
                if let Some(level) = self.tool_permissions.get(tool) {
                    return *level;
                }
                
                // Check category permission
                if let Some(level) = self.category_permissions.get(tool.category) {
                    return *level;
                }
                
                // Default based on action type
                self.default_for_action(action)
            }
        }
    }
    
    fn default_for_action(&self, action: &ActionType) -> PermissionLevel {
        match action {
            ActionType::ReadFile { .. } => PermissionLevel::AutoRun,
            ActionType::WriteFile { .. } => PermissionLevel::ApprovalRequired,
            ActionType::DeleteFile { .. } => PermissionLevel::ApprovalRequired,
            ActionType::ExecuteCommand { .. } => PermissionLevel::ApprovalRequired,
            ActionType::WebRequest { .. } => PermissionLevel::AutoRun,
        }
    }
    
    fn default_tool_permissions() -> HashMap<ToolId, PermissionLevel> {
        HashMap::new()  // Start empty, add overrides
    }
    
    fn default_category_permissions() -> HashMap<String, PermissionLevel> {
        HashMap::new()  // Start empty, add overrides
    }
}
```

### 2.2 Approval Queue

**File**: `src-tauri/src/agents/approval.rs` (new)

```rust
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, broadcast};

// Approval queue manages pending actions
// 
// Design decisions:
// - Sessions persist in SQLite across app restarts
// - Single session per thread (no concurrent sessions)
// - Auto-deny after APPROVAL_TIMEOUT_MINUTES of inactivity
// - Default trust mode is Untrusted

const APPROVAL_TIMEOUT_MINUTES: u64 = 5;

pub struct ApprovalQueue {
    pending: Arc<RwLock<HashMap<String, PendingAction>>>,
    tx: broadcast::Sender<ApprovalEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApprovalEvent {
    ActionQueued { action: PendingAction },
    ActionApproved { action_id: String },
    ActionDenied { action_id: String },
    ActionTimeout { action_id: String },
}

impl ApprovalQueue {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            tx,
        }
    }
    
    pub async fn queue(&self, action: PendingAction) {
        let mut pending = self.pending.write().await;
        pending.insert(action.id.clone(), action.clone());
        let _ = self.tx.send(ApprovalEvent::ActionQueued { action });
    }
    
    pub async fn approve(&self, action_id: &str) -> Option<PendingAction> {
        let mut pending = self.pending.write().await;
        if let Some(action) = pending.remove(action_id) {
            let _ = self.tx.send(ApprovalEvent::ActionApproved { 
                action_id: action_id.to_string() 
            });
            return Some(action);
        }
        None
    }
    
    pub async fn deny(&self, action_id: &str) {
        let mut pending = self.pending.write().await;
        pending.remove(action_id);
        let _ = self.tx.send(ApprovalEvent::ActionDenied { 
            action_id: action_id.to_string() 
        });
    }
    
    /// Check for timed-out actions and auto-deny them.
    /// Should be called periodically by a background task.
    pub async fn check_timeouts(&self) -> Vec<String> {
        let mut pending = self.pending.write().await;
        let now = Utc::now();
        let timeout = chrono::Duration::minutes(APPROVAL_TIMEOUT_MINUTES as i64);
        
        let timed_out: Vec<String> = pending
            .iter()
            .filter(|(_, action)| now.signed_duration_since(action.created_at) > timeout)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in &timed_out {
            pending.remove(id);
            let _ = self.tx.send(ApprovalEvent::ActionTimeout { 
                action_id: id.clone() 
            });
        }
        
        timed_out
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<ApprovalEvent> {
        self.tx.subscribe()
    }
    
    pub async fn get_pending(&self) -> Vec<PendingAction> {
        let pending = self.pending.read().await;
        pending.values().cloned().collect()
    }
}
```

---

## §3. Agent Runtime

### 3.1 Extract from LLM Commands

**Refactor**: `src-tauri/src/commands/llm.rs`

Extract `run_orchestrated_loop` into a reusable runtime.

**File**: `src-tauri/src/agents/runtime.rs` (new)

```rust
use std::sync::Arc;
use futures::Stream;
use crate::llm::{LlmClient, ChatRequest, Message, LlmEvent};
use crate::tools::ToolRegistry;

pub struct AgentRuntime {
    client: Arc<dyn LlmClient>,
    tool_registry: Arc<ToolRegistry>,
    permission_resolver: PermissionResolver,
    approval_queue: Arc<ApprovalQueue>,
    max_steps: u32,
}

impl AgentRuntime {
    pub fn new(
        client: Arc<dyn LlmClient>,
        tool_registry: Arc<ToolRegistry>,
        trust_mode: TrustMode,
    ) -> Self {
        Self {
            client,
            tool_registry,
            permission_resolver: PermissionResolver::new(trust_mode),
            approval_queue: Arc::new(ApprovalQueue::new()),
            max_steps: 6,
        }
    }
    
    pub async fn execute(
        &self,
        session: &AgentSession,
        request: AgentRequest,
    ) -> Result<AgentExecutionResult, AgentError> {
        let mut context = self.build_context(session, &request).await?;
        let mut steps = Vec::new();
        
        for step in 1..=self.max_steps {
            // Build request with tools
            let chat_request = self.build_chat_request(&context, session);
            
            // Send to LLM
            let response = self.client.chat(chat_request).await?;
            
            // Check for tool calls
            if response.tool_calls.is_empty() {
                return Ok(AgentExecutionResult {
                    content: response.content,
                    steps,
                    usage: response.usage,
                });
            }
            
            // Process tool calls
            let tool_results = self.process_tool_calls(
                &response.tool_calls,
                session,
            ).await?;
            
            // Append to context
            context.push(Message::assistant_with_tool_calls(
                response.content,
                response.tool_calls,
            ));
            self.append_tool_results(&mut context, &tool_results);
            
            steps.push(ExecutionStep {
                step_number: step,
                action: StepAction::ToolCall { /* ... */ },
                result: Some(/* ... */),
                timestamp: Utc::now(),
            });
        }
        
        Err(AgentError::MaxStepsExceeded)
    }
    
    async fn process_tool_calls(
        &self,
        tool_calls: &[ToolCall],
        session: &AgentSession,
    ) -> Result<Vec<ToolResult>, AgentError> {
        let mut results = Vec::new();
        
        for call in tool_calls {
            let action_type = self.infer_action_type(&call.name, &call.arguments);
            let permission = self.permission_resolver.check_permission(
                &ToolId::from_name(&call.name),
                &action_type,
            );
            
            match permission {
                PermissionLevel::AutoRun => {
                    let result = self.tool_registry.execute(call.clone()).await?;
                    results.push(result);
                }
                PermissionLevel::ApprovalRequired => {
                    let pending = PendingAction::from_tool_call(call, &action_type);
                    self.approval_queue.queue(pending.clone()).await;
                    
                    // Wait for approval (with timeout)
                    let result = self.wait_for_approval(&pending.id).await?;
                    results.push(result);
                }
                PermissionLevel::Disabled => {
                    results.push(ToolResult {
                        call_id: call.id.clone(),
                        output: serde_json::json!({ "error": "Tool disabled" }),
                        is_error: true,
                    });
                }
            }
        }
        
        Ok(results)
    }
    
    async fn wait_for_approval(&self, action_id: &str) -> Result<ToolResult, AgentError> {
        let mut rx = self.approval_queue.subscribe();
        
        loop {
            match rx.recv().await {
                Ok(ApprovalEvent::ActionApproved { action_id: approved_id }) 
                    if approved_id == action_id => 
                {
                    // Execute the approved action
                    let pending = self.approval_queue.get_pending().await;
                    // ... execute and return result
                }
                Ok(ApprovalEvent::ActionDenied { action_id: denied_id }) 
                    if denied_id == action_id => 
                {
                    return Ok(ToolResult {
                        call_id: action_id.to_string(),
                        output: serde_json::json!({ "error": "Denied by user" }),
                        is_error: true,
                    });
                }
                _ => continue,
            }
        }
    }
}

pub struct AgentExecutionResult {
    pub content: String,
    pub steps: Vec<ExecutionStep>,
    pub usage: Option<UsageStats>,
}
```

### 3.2 Agent Registry

**File**: `src-tauri/src/agents/registry.rs` (new)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<AgentId, Agent>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        let mut agents = HashMap::new();
        
        // Register built-in agents
        agents.insert(
            AgentId("code-agent".to_string()),
            Agent::code_agent(),
        );
        
        Self {
            agents: Arc::new(RwLock::new(agents)),
        }
    }
    
    pub async fn get(&self, id: &AgentId) -> Option<Agent> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }
    
    pub async fn list(&self) -> Vec<Agent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }
}

impl Agent {
    pub fn code_agent() -> Self {
        Self {
            id: AgentId("code-agent".to_string()),
            name: "Code Agent".to_string(),
            description: "Assists with code reading, writing, and execution".to_string(),
            system_prompt: include_str!("../prompts/code-agent.md").to_string(),
            allowed_tools: vec![
                ToolId::new("fs", "read_file"),
                ToolId::new("fs", "write_file"),
                ToolId::new("fs", "list_files"),
                ToolId::new("search", "grep"),
                ToolId::new("system", "execute_command"),
            ],
            default_model_id: "code".to_string(),
            trust_mode: TrustMode::Untrusted,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
```

---

## §4. Conversation Workspace Integration

### 4.1 Update Conversation Types

**File**: `src-tauri/src/nlp/chat.rs` (update)

Add workspace field to `Conversation`:

```rust
/// Represents a full conversation with its threads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    /// Unique identifier for the conversation
    pub id: String,
    /// Title of the conversation
    pub title: String,
    /// When the conversation was created
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated
    pub updated_at: DateTime<Utc>,
    /// Settings for this conversation
    pub settings: ConversationSettings,
    /// Workspace root path for agent operations (optional)
    pub workspace_root: Option<PathBuf>,
}

/// Settings for a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSettings {
    /// AI model to use
    #[serde(default = "default_model")]
    pub model: String,
    /// Temperature for response generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Maximum tokens for responses
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Custom system prompt
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Workspace path for agent file operations
    #[serde(default)]
    pub workspace_path: Option<PathBuf>,
}
```

### 4.2 Update Chat Store

**File**: `src-tauri/src/storage/chat_store.rs` (update)

Add workspace column to conversations table:

```sql
ALTER TABLE conversations ADD COLUMN workspace_root TEXT;
```

Update `create_conversation` and `update_conversation_settings` to handle workspace.

### 4.3 Update Tool Execution Context

**File**: `src-tauri/src/tools/context.rs` (already exists)

The `ToolExecutionContext` already has `workspace_root`. Ensure it's populated from the conversation when creating sessions:

```rust
impl ToolExecutionContext {
    pub fn from_conversation(conv: &Conversation) -> Self {
        Self {
            conversation_id: conv.id.clone(),
            thread_id: None,
            workspace_root: conv.workspace_root.clone(),
        }
    }
}
```

### 4.4 Add Tauri Command for Workspace Selection

**File**: `src-tauri/src/commands/chat.rs` (update)

```rust
/// Update conversation workspace
#[tauri::command]
pub async fn set_conversation_workspace(
    conversation_id: String,
    workspace_path: String,
    chat_state: State<'_, ChatState>,
) -> Result<(), String> {
    let store = chat_state.store.lock().await;
    store.set_workspace(&conversation_id, &PathBuf::from(workspace_path))
        .map_err(|e| e.to_string())
}

/// Get conversation workspace
#[tauri::command]
pub async fn get_conversation_workspace(
    conversation_id: String,
    chat_state: State<'_, ChatState>,
) -> Result<Option<String>, String> {
    let store = chat_state.store.lock().await;
    let conv = store.load_conversation(&conversation_id)
        .map_err(|e| e.to_string())?
        .ok_or("Conversation not found")?;
    Ok(conv.workspace_root.map(|p| p.to_string_lossy().to_string()))
}
```

---

## §5. Tauri Commands

### 4.1 New Commands

**File**: `src-tauri/src/commands/agent.rs` (new)

```rust
use tauri::{AppHandle, State};
use crate::agents::*;
use crate::AgentState;

/// Create a new agent session
#[tauri::command]
pub async fn create_agent_session(
    agent_id: String,
    conversation_id: String,
    thread_id: String,
    workspace_root: Option<String>,
    state: State<'_, AgentState>,
) -> Result<AgentSession, String> {
    let agent = state.registry.get(&AgentId(agent_id)).await
        .ok_or("Agent not found")?;
    
    let session = AgentSession::new(
        agent.id.clone(),
        conversation_id,
        thread_id,
        workspace_root.map(PathBuf::from),
    );
    
    state.session_store.create(&session).await
        .map_err(|e| e.to_string())?;
    
    Ok(session)
}

/// Send a request to an agent session
#[tauri::command]
pub async fn send_agent_request(
    session_id: String,
    content: String,
    state: State<'_, AgentState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    let session = state.session_store.get(&SessionId(session_id.clone()))
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Session not found")?;
    
    let agent = state.registry.get(&session.agent_id).await
        .ok_or("Agent not found")?;
    
    let request = AgentRequest {
        content,
        attachments: vec![],
        context: None,
    };
    
    // Spawn execution task
    let runtime = state.runtime.clone();
    let session_clone = session.clone();
    let app_handle_clone = app_handle.clone();
    
    tokio::spawn(async move {
        let result = runtime.execute(&session_clone, request).await;
        
        match result {
            Ok(execution_result) => {
                // Emit completion event
                let _ = app_handle_clone.emit("agent:complete", &execution_result);
            }
            Err(e) => {
                let _ = app_handle_clone.emit("agent:error", &e.to_string());
            }
        }
    });
    
    Ok(session_id)
}

/// Approve a pending action
#[tauri::command]
pub async fn approve_action(
    session_id: String,
    action_id: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.approval_queue.approve(&action_id).await
        .ok_or("Action not found")?;
    Ok(())
}

/// Deny a pending action
#[tauri::command]
pub async fn deny_action(
    session_id: String,
    action_id: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.approval_queue.deny(&action_id).await;
    Ok(())
}

/// Get pending actions for a session
#[tauri::command]
pub async fn get_pending_actions(
    session_id: String,
    state: State<'_, AgentState>,
) -> Result<Vec<PendingAction>, String> {
    Ok(state.approval_queue.get_pending().await)
}

/// Get session state
#[tauri::command]
pub async fn get_session_state(
    session_id: String,
    state: State<'_, AgentState>,
) -> Result<SessionState, String> {
    let session = state.session_store.get(&SessionId(session_id))
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Session not found")?;
    
    Ok(session.state)
}

/// Cancel a session
#[tauri::command]
pub async fn cancel_session(
    session_id: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.session_store.update_state(
        &SessionId(session_id),
        SessionState::Cancelled { reason: "User cancelled".to_string() }
    ).await.map_err(|e| e.to_string())
}
```

### 4.2 State Management

**File**: `src-tauri/src/commands/mod.rs` (update)

```rust
pub mod agent;

pub use agent::AgentState;

// AgentState for Tauri state management
pub struct AgentState {
    pub registry: Arc<AgentRegistry>,
    pub session_store: Arc<SessionStore>,
    pub approval_queue: Arc<ApprovalQueue>,
    pub runtime: Arc<AgentRuntime>,
}
```

### 4.3 Register Commands

**File**: `src-tauri/src/lib.rs` (update)

Add to `invoke_handler`:

```rust
commands::agent::create_agent_session,
commands::agent::send_agent_request,
commands::agent::approve_action,
commands::agent::deny_action,
commands::agent::get_pending_actions,
commands::agent::get_session_state,
commands::agent::cancel_session,
```

---

## §5. Storage

### 5.1 Session Store

**File**: `src-tauri/src/agents/store.rs` (new)

```rust
use rusqlite::{Connection, params};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SessionStore {
    db: Arc<Mutex<Connection>>,
}

impl SessionStore {
    pub fn new(db_path: &str) -> Result<Self, StoreError> {
        let conn = Connection::open(db_path)?;
        Self::create_tables(&conn)?;
        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }
    
    fn create_tables(conn: &Connection) -> Result<(), StoreError> {
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                thread_id TEXT NOT NULL,
                state TEXT NOT NULL,
                workspace_root TEXT,
                parent_session_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS agent_tasks (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                request TEXT NOT NULL,
                status TEXT NOT NULL,
                steps TEXT NOT NULL,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id)
            );
            
            CREATE INDEX IF NOT EXISTS idx_sessions_conversation 
                ON agent_sessions(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_thread 
                ON agent_sessions(thread_id);
        ")?;
        Ok(())
    }
    
    pub async fn create(&self, session: &AgentSession) -> Result<(), StoreError> {
        let db = self.db.lock().await;
        db.execute(
            "INSERT INTO agent_sessions (id, agent_id, conversation_id, thread_id, state, workspace_root, parent_session_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session.id.0,
                session.agent_id.0,
                session.conversation_id,
                session.thread_id,
                serde_json::to_string(&session.state)?,
                session.workspace_root.as_ref().map(|p| p.to_string_lossy().to_string()),
                session.parent_session_id.as_ref().map(|id| id.0.clone()),
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }
    
    pub async fn get(&self, id: &SessionId) -> Result<Option<AgentSession>, StoreError> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, agent_id, conversation_id, thread_id, state, workspace_root, parent_session_id, created_at, updated_at
             FROM agent_sessions WHERE id = ?1"
        )?;
        
        // ... query and parse
    }
    
    pub async fn update_state(&self, id: &SessionId, state: SessionState) -> Result<(), StoreError> {
        let db = self.db.lock().await;
        db.execute(
            "UPDATE agent_sessions SET state = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                serde_json::to_string(&state)?,
                Utc::now().to_rfc3339(),
                id.0,
            ],
        )?;
        Ok(())
    }
}
```

---

## §6. Event System

### 6.1 Agent Events

**File**: `src-tauri/src/agents/events.rs` (new)

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    SessionCreated { session_id: String, agent_id: String },
    SessionStateChanged { session_id: String, state: SessionState },
    
    ExecutionStarted { session_id: String, request_id: String },
    ExecutionStep { session_id: String, step: ExecutionStep },
    ExecutionComplete { session_id: String, result: AgentExecutionResult },
    ExecutionError { session_id: String, error: String },
    
    ApprovalRequired { session_id: String, action: PendingAction },
    ApprovalApproved { session_id: String, action_id: String },
    ApprovalDenied { session_id: String, action_id: String },
    
    ToolCall { session_id: String, tool: String, arguments: serde_json::Value },
    ToolResult { session_id: String, tool: String, result: serde_json::Value },
}
```

### 6.2 Event Emission

Update `AgentRuntime::execute` to emit events:

```rust
// In execute loop
let _ = app_handle.emit("agent:step", &AgentEvent::ExecutionStep {
    session_id: session.id.0.clone(),
    step: step.clone(),
});

// When approval needed
let _ = app_handle.emit("agent:approval", &AgentEvent::ApprovalRequired {
    session_id: session.id.0.clone(),
    action: pending.clone(),
});
```

---

## Task Checklist

### §1 Agent Types
- [x] Expand `agents/types.rs` with full Agent struct
- [x] Add `TrustMode` enum
- [x] Create `agents/session.rs` with session types
- [x] Create `agents/task.rs` with task types
- [x] Update `agents/mod.rs` exports

### §2 Permission System
- [x] Create `agents/permissions.rs` with permission types
- [x] Implement `PermissionResolver`
- [x] Create `agents/approval.rs` with `ApprovalQueue`
- [x] Add approval timeout checking (5 min)
- [x] Add approval event types

### §3 Agent Runtime
- [x] Extract runtime to `agents/runtime.rs`
- [x] Add permission checking to tool execution
- [x] Add approval wait logic with timeout
- [x] Create `agents/registry.rs` with built-in agents

### §4 Conversation Workspace
- [x] Add `workspace_root` to `ConversationSettings` struct (no migration needed - JSON field)
- [x] Add `update_conversation_workspace` to `ChatStore`
- [x] Add `set_conversation_workspace` command
- [x] Update `ExecutionComplete` event to include `conversation_id` and `thread_id`
- [x] Ensure tools receive workspace from session (passed at session creation)

### §5 Tauri Commands
- [x] Create `commands/agent.rs`
- [x] Add `AgentState` to state management
- [x] Register all new commands in `lib.rs`

### §6 Storage
- [x] Create `agents/store.rs` with session persistence
- [x] Add migration for new tables
- [x] Add database path configuration

### §7 Events
- [x] Create `agents/events.rs` with event types
- [x] Wire event emission into runtime
- [x] Document event contract for frontend

### §8 Testing
- [x] Unit tests for permission resolver
- [x] Unit tests for approval queue (including timeout)
- [x] Unit tests for session state transitions
- [ ] Integration tests for workspace isolation

### §9 Bonus Features (Implemented)
- [x] Create `agents/rollback.rs` for file reverts
- [x] Add subtask support in store and commands

---

## Testing Strategy

### Unit Tests
- `PermissionResolver::check_permission` for all modes
- `ApprovalQueue` queue/approve/deny flow
- `AgentSession` state transitions

### Integration Tests
- Full execution loop with mock LLM
- Approval flow with timeout
- Session persistence roundtrip

### Manual Testing
- Create session, send request, see steps
- Trigger approval, approve/deny
- Cancel mid-execution
