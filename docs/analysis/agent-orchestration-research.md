# Agent Orchestration Layer - Research & Analysis

## 1. Current Architecture Overview

### What exists today

Coder is a **Tauri desktop chat application** with these layers:

```
Frontend (Svelte 5 + TypeScript)
    ↕ Tauri IPC
Backend (Rust)
    ├── commands/    → Tauri command handlers (chat, config, llm, nlp)
    ├── llm/         → Multi-provider LLM client abstraction (OpenAI, Anthropic, Google)
    ├── tools/       → Tool registry with categories (fs, search, conversation, system)
    ├── nlp/         → Intent classification, context management, code understanding
    ├── agents/      → Minimal stub types (Agent, AgentType, Task, TaskStatus)
    ├── storage/     → SQLite persistence (chat, config, context)
    └── api/         → (module exists, purpose TBD)
```

### Current chat flow

```
User message → send_chat_message_streaming (Tauri command)
  → OrchestratedConversationRunner (default)
    → run_orchestrated_loop()
      1. Load conversation history from ChatStore
      2. Build LLM context (messages + system prompt)
      3. Loop up to ORCHESTRATOR_MAX_STEPS (6):
         a. Send ChatRequest to LLM (with tool definitions if available)
         b. If response has tool_calls → execute them → append results → loop
         c. If no tool_calls → return final response
      4. Save assistant message to ChatStore
  → Emit LlmEvent to frontend (started → chunks → complete)
```

### What "orchestration" means today

The current system is a **tool-calling loop**:
- Single LLM call → tool execution → re-submit → repeat
- No concept of "agents" as distinct entities
- No planning, delegation, or multi-agent coordination
- Tools are flat functions registered by category

---

## 2. Gap Analysis: Current vs. Target

| Capability | Current State | Target State |
|---|---|---|
| LLM chat | Working (streaming + non-streaming) | Same, but orchestrated by agents |
| Tool execution | Bounded loop (6 steps) | Agent-owned tool sets with strategic usage |
| Intent classification | NLP module exists (pattern-based) | Agent-driven task decomposition |
| Agent concept | Stub types only (`agents/types.rs`) | Full agent lifecycle (create, configure, execute) |
| Task planning | `ExecutionPlanner` (parallel intent ordering) | Hierarchical plan → subtask delegation |
| Human-in-the-loop | None | Approval gates for destructive actions |
| Multi-agent | None | Agents can delegate to other agents |
| Persistence | Chat messages + config | Agent state, task history, execution logs |

---

## 3. Key Design Decisions to Explore

### 3.1 What is an "Agent"?

An agent needs to be more than a chat turn. It needs:

- **Identity**: Name, description, system prompt, capabilities
- **Tool access**: Which tools it can use (subset of registry)
- **Model config**: Which LLM model to use (may differ per agent)
- **Memory**: Short-term (conversation context) + long-term (across threads)
- **Behavior**: Approval policies, retry strategies, error handling

**Question**: Should agents be first-class persisted entities (like conversations) or ephemeral per-request?

### 3.2 Orchestration Patterns

Three main patterns to consider:

**A. Single-Agent with Tool Loop** (current approach, enhanced)
```
User → Agent → LLM + Tools → Response
```
- Simplest to implement
- Agent has a system prompt + tool set
- Loop until task is complete
- Good for Phase 1

**B. Orchestrator + Delegation**
```
User → Orchestrator → Plan → [Agent A, Agent B, ...] → Synthesize → Response
```
- Orchestrator decomposes tasks, assigns to specialized agents
- Each agent executes independently
- Results are synthesized back
- More complex, enables multi-agent collaboration

**C. Hierarchical Agent Tree**
```
User → Root Agent → Sub-agents → Sub-sub-agents → ...
```
- Each agent can spawn child agents
- Recursive delegation for complex tasks
- Most flexible but hardest to manage

### 3.3 Where does orchestration live?

**Option A: Rust backend (orchestration service)**
- Pros: Full control, typed, direct tool access, no latency
- Cons: More backend code, harder to iterate

**Option B: LLM-driven (model decides orchestration)**
- Pros: Most flexible, adapts to novel tasks
- Cons: Unpredictable, token-heavy, hard to debug

**Option C: Hybrid (Rust framework + LLM decisions)**
- Pros: Best of both — structured loop controlled by Rust, decisions by LLM
- Cons: More complex, but this is what the PRD describes

### 3.4 Tool Ownership

Currently tools are in a flat registry. With agents:

- Should each agent have a **tool allow-list**?
- Should tools be **namespaced** per agent type?
- Can agents share tools or must they be exclusive?
- How do we handle tool result schemas across agents?

---

## 4. Existing Code We Can Reuse

| Module | What it provides | Reuse potential |
|---|---|---|
| `llm/` | Multi-provider LLM client, streaming, tool calls | **Direct reuse** — agent execution engine |
| `tools/` | Tool registry, categories, execution context | **Direct reuse** — agent tool sets |
| `tools/context.rs` | `ToolExecutionContext` with conversation_id, thread_id, workspace | **Extend** — add agent_id, task_id |
| `commands/llm.rs` | `ConversationRunner` trait, orchestrated loop | **Refactor** — extract into agent runtime |
| `nlp/execution_planner.rs` | Execution plan with parallel levels | **Reuse** — task decomposition |
| `nlp/intent.rs` | Intent taxonomy (GenerateCode, FindBugs, etc.) | **Reuse** — agent routing |
| `agents/types.rs` | Agent, AgentType, Task, TaskStatus stubs | **Expand** — foundation for agent model |
| `storage/` | SQLite persistence | **Extend** — agent + task tables |
| `nlp/chat.rs` | ChatMessage, Conversation, Thread types | **Direct reuse** — message history |

### Code to refactor/extract

1. **`run_orchestrated_loop`** (~130 lines in `commands/llm.rs:1275-1407`)
   - Currently tightly coupled to Tauri commands and chat state
   - Should be extracted into a standalone `AgentRuntime` in `agents/`
   - The loop pattern (LLM call → tool execution → repeat) is exactly what agents need

2. **`ToolExecutor` trait** (`commands/llm.rs:300-308`)
   - Already abstracts tool execution for the orchestrator
   - Should move to `tools/` as a first-class abstraction
   - The two implementations (`NoopToolExecutor`, `RegistryToolExecutor`) belong in `tools/`

3. **`ConversationRunner` trait** (`commands/llm.rs:92-106`)
   - Good abstraction but chat-specific naming
   - Could evolve into `AgentRunner` or stay as the top-level entry point

---

## 5. Proposed Architecture (High-Level)

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend (Svelte)                        │
│  Chat UI │ Agent Selector │ Approval Gates │ Progress View   │
└──────────────────────┬──────────────────────────────────────┘
                       │ Tauri IPC
┌──────────────────────▼──────────────────────────────────────┐
│                   Command Layer                               │
│  send_message │ manage_agents │ approve_action │ get_status  │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────┐
│                 Agent Orchestration Layer                     │
│                                                              │
│  ┌─────────────┐    ┌──────────────────┐                    │
│  │ AgentConfig │    │  AgentRuntime     │                    │
│  │ (persisted) │───▶│  (execution loop) │                    │
│  └─────────────┘    └────────┬─────────┘                    │
│                              │                               │
│  ┌──────────┐  ┌────────────▼───────────┐  ┌────────────┐  │
│  │  Planner  │  │  Tool Execution Engine │  │  Memory   │  │
│  │ (optional)│  │  (from tools/)        │  │ (context) │  │
│  └──────────┘  └────────────────────────┘  └────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────────┐│
│  │              LLM Client (llm/)                           ││
│  │    OpenAI │ Anthropic │ Google │ Streaming support       ││
│  └──────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## 6. Open Questions for Discussion

1. **Agent persistence**: Should agents be user-configurable (create/edit/delete) or system-provided? The PRD mentions "Agent Selector" UI suggesting user choice.

2. **Per-agent model selection**: Should each agent use the conversation's model or its own configured model? (e.g., a fast model for planning, a capable model for code generation)

3. **Multi-agent coordination**: For Phase 1, is single-agent sufficient? The PRD focuses on Code Agent first.

4. **Approval flow**: The PRD mentions human-in-the-loop for destructive operations. Should this be:
   - Synchronous (block until user approves)? 
   - Asynchronous (queue action, continue, user approves later)?

5. **Task lifecycle**: Should tasks be persisted separately from messages, or embedded within thread history?

6. **Relationship to existing NLP modules**: The NLP module has intent classification, execution planning, code understanding. How much of this should be absorbed into the agent system vs. kept separate?

7. **Event model**: Current streaming uses `llm:event` Tauri events. Do we need new event types for agent lifecycle (agent:started, agent:tool_used, agent:action_pending_approval, etc.)?

---

## 7. Suggested Phased Approach

### Phase 1a: Agent Runtime Foundation
- Extract `run_orchestrated_loop` into `agents/runtime.rs`
- Define `Agent` trait and `AgentConfig` struct
- Move `ToolExecutor` to `tools/`
- Agent-based tool allow-lists

### Phase 1b: Built-in Code Agent
- Implement a `CodeAgent` with fs + search tools
- System prompt for code assistance
- Persist agent configuration in SQLite

### Phase 1c: Approval Gates
- Define "safe" vs "approval-required" tool categories
- Tauri events for pending approvals
- Frontend approval UI

### Phase 1d: Task Decomposition
- Optional planning step before execution
- Sub-task tracking within the runtime loop
- Progress reporting to frontend

---

## 8. Reference Implementations to Study

These are well-known agent frameworks we can learn patterns from:

- **LangGraph**: State machine-based agent orchestration with conditional edges
- **OpenAI Assistants API**: Tools + runs + thread-based conversation
- **Claude Computer Use**: Tool-calling loop with screenshot/human feedback
- **AutoGen**: Multi-agent conversation framework
- **CrewAI**: Role-based agent teams with task delegation

---

## 9. Reference: JetBrains Fleet Agent Architecture (Knowledge Base)

> **Note**: This section is for background knowledge only. We are designing our own solution, not adopting Fleet's architecture directly.

### 9.1 Session-Based Architecture (Not Agent Instances)

Fleet uses **sessions** as the primary abstraction, not agent instances:

```
User Request → ChatAgentSessionEntity → AgentSessionDescriptor → Execution
```

**Key insight**: An "agent" is a session with state, not a static entity. Sessions have:
- `id` - Session identifier
- `state` - Lifecycle state (Launching, Running, WaitForRequest, Finished, etc.)
- `agentId` - Which agent configuration to use
- `parentSessionId` - For hierarchical task relationships
- `taskType` - General vs CodeReview

**Adoption for Coder**: We should introduce `AgentSession` as a first-class concept, separate from `Agent` (configuration) and `Task` (execution unit).

### 9.2 AgentFrontend as LLM Abstraction

Fleet uses an `AgentFrontend` interface that different LLM providers implement:

```kotlin
interface AgentFrontend {
    fun getAgentId(): ReportableAgentId
    fun getPrompt(args: SerializedArguments): String
    fun buildArguments(context: AgentContext): AgentArguments
    fun handleCompletion(result: AgentResult)
}
```

**Adoption for Coder**: Our `llm/` module already has `LlmClient` trait. We could either:
- Extend `LlmClient` to include agent-specific methods (`buildSystemPrompt`, `formatToolResults`)
- Create a new `AgentFrontend` trait that wraps `LlmClient`

### 9.3 Session Lifecycle States

Fleet's session states are more granular than our current approach:

| Fleet State | Our Equivalent | Gap |
|---|---|---|
| Launching | None | Missing - agent initialization phase |
| Running | In `run_orchestrated_loop` | Implicit, not tracked |
| WaitForRequest | None | Missing - idle between turns |
| UserInputRequired | None | **Critical gap** - approval gates |
| Finished | Loop exits | Implicit |
| Cancelled | `cancel_chat_request` | Exists |
| Error | On error | Implicit |

**Adoption for Coder**: We need explicit session state tracking with these states, especially `UserInputRequired` for approval gates.

### 9.4 Hierarchical Sessions (Not Subagents)

Fleet uses `parentSessionId` for task hierarchy, NOT subagent delegation:

```mermaid
graph TD
    A[Root Session: "Refactor auth module"] --> B[Child Session: "Update login.ts"]
    A --> C[Child Session: "Update logout.ts"]
    A --> D[Child Session: "Add tests"]
```

This is **task decomposition**, not agent delegation. Each child session is a related task, not a separate agent.

**Adoption for Coder**: Our `ExecutionPlanner` already creates parallel execution levels. We should:
- Add `parentSessionId` to sessions
- Track task hierarchy separately from agent instances
- Use hierarchy for progress reporting, not multi-agent coordination

### 9.5 Launch Types (Execution Isolation)

Fleet supports multiple execution environments:

| Launch Type | Description | Relevance |
|---|---|---|
| Local | Same process | Our current approach |
| Docker | Containerized | Future - sandboxed execution |
| JCP | JetBrains Cloud | Not applicable |
| Worktree | Isolated git worktree | Interesting for safe file operations |

**Adoption for Coder**: Start with `Local`. Design for `Docker` later (security sandbox). `Worktree` is interesting — could create a git worktree for risky operations, giving natural rollback.

### 9.6 AgentLauncherApi as RPC Layer

Fleet separates session management from execution:

```kotlin
interface AgentLauncherApi {
    fun createSession(sessionId, sessionConfig): SessionHandle
    fun stopSession(sessionId)
    fun sendRequest(sessionId, request): Unit  // async via stateFlow
    fun stateFlow(sessionId): Flow<AgentSessionState>
}
```

**Adoption for Coder**: We could create a similar `AgentLauncher` trait in Rust:

```rust
trait AgentLauncher: Send + Sync {
    async fn create_session(&self, config: SessionConfig) -> Result<SessionId, AgentError>;
    async fn stop_session(&self, session_id: &SessionId) -> Result<(), AgentError>;
    async fn send_request(&self, session_id: &SessionId, request: AgentRequest) -> Result<(), AgentError>;
    fn state_stream(&self, session_id: &SessionId) -> impl Stream<Item = SessionState> + Send;
}
```

### 9.7 What Fleet Does NOT Have

Importantly, Fleet does **not** implement:
- Multi-agent collaboration (agents talking to each other)
- Agent marketplace/plugins
- User-configurable agents

This validates our Phase 1 approach: single agent with tools, not multi-agent orchestration.

---

## 10. Revised Architecture Proposal

Based on Fleet patterns, here's a refined architecture:

### Core Types

```rust
// agents/types.rs - revised

/// Agent configuration (persisted, user-configurable)
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<ToolId>,
    pub model_id: String,
    pub approval_policy: ApprovalPolicy,
}

/// Running session (ephemeral, stateful)
pub struct AgentSession {
    pub id: SessionId,
    pub agent_id: AgentId,
    pub conversation_id: ConversationId,
    pub thread_id: Option<ThreadId>,
    pub state: SessionState,
    pub parent_session_id: Option<SessionId>,
    pub created_at: DateTime<Utc>,
    pub workspace_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum SessionState {
    Launching,
    Running,
    WaitForRequest,
    UserInputRequired { pending_actions: Vec<PendingAction> },
    Finished,
    Cancelled,
    Error { message: String },
}

/// A request sent to an agent session
pub struct AgentRequest {
    pub content: String,
    pub context: Vec<ContextEntry>,
}

/// Pending action awaiting user approval
pub struct PendingAction {
    pub id: String,
    pub action_type: ActionType,
    pub description: String,
    pub preview: Option<String>,  // e.g., diff preview
}

#[derive(Debug, Clone)]
pub enum ActionType {
    WriteFile { path: PathBuf, content: String },
    ExecuteCommand { command: String },
    DeleteFile { path: PathBuf },
}
```

### AgentLauncher Trait

```rust
// agents/launcher.rs

pub trait AgentLauncher: Send + Sync {
    async fn create_session(
        &self,
        agent: Agent,
        conversation_id: ConversationId,
        workspace_root: Option<PathBuf>,
    ) -> Result<SessionId, AgentError>;
    
    async fn send_request(
        &self,
        session_id: &SessionId,
        request: AgentRequest,
    ) -> Result<(), AgentError>;
    
    async fn approve_action(
        &self,
        session_id: &SessionId,
        action_id: &str,
        approved: bool,
    ) -> Result<(), AgentError>;
    
    async fn cancel_session(&self, session_id: &SessionId) -> Result<(), AgentError>;
    
    async fn get_session_state(&self, session_id: &SessionId) -> Result<SessionState, AgentError>;
}
```

### Session Store (Persistence)

```rust
// agents/session_store.rs

pub struct AgentSessionStore {
    db: Connection,
}

impl AgentSessionStore {
    pub async fn create_session(&self, session: &AgentSession) -> Result<(), StoreError>;
    pub async fn load_session(&self, id: &SessionId) -> Result<Option<AgentSession>, StoreError>;
    pub async fn update_session_state(&self, id: &SessionId, state: &SessionState) -> Result<(), StoreError>;
    pub async fn load_active_sessions(&self, conversation_id: &ConversationId) -> Result<Vec<AgentSession>, StoreError>;
}
```

---

## 11. Updated Open Questions

Based on Fleet analysis, some questions are answered:

| Question | Fleet's Answer | Our Decision |
|---|---|---|
| Agent persistence | Agents are configs, sessions are ephemeral | Adopt same pattern |
| Multi-agent? | No - hierarchical sessions instead | Phase 1: single agent |
| Approval flow | `UserInputRequired` state with pending actions | Adopt same pattern |
| Session vs Agent | Agent = config, Session = running instance | Adopt same pattern |
| Subagents? | No - `parentSessionId` for task hierarchy | Adopt same pattern |

**Remaining questions**:

1. **Launch types**: Start with Local only, or design for Docker from day one?
2. **Session-to-thread mapping**: One session per thread, or can a session span multiple threads?
3. **Approval UI**: Inline in chat, or separate modal?
4. **Action rollback**: Should approved actions be undoable? Fleet uses git worktrees.

---

## 10. Reference: Context Management Patterns (Knowledge Base)

> **Note**: Background knowledge from JetBrains Fleet research. We design our own solution.

### Key Concepts

**Context hierarchy levels**:
1. **Global context** — Shared across all parallel agent sessions (workspace-level)
2. **Pinned context** — Per-session but persistent, guaranteed inclusion in token budget
3. **Local context** — Ephemeral, session-specific, token-budgeted

**Token budget management**:
- Track token counts per context entry
- Priority-based inclusion (pinned first, then fit what's possible)
- Async content fetching with error handling

**Parallel task isolation**:
- Each session gets its own context "view"
- Global baseline is shared
- Local context is completely isolated
- No context leakage between parallel sessions

**Context lifecycle**:
- Creation → Usage → Analysis (relevance) → Cleanup → Persistence

### Relevance to Coder

Our current context system (`nlp/context.rs`, `storage/context_store.rs`) is simpler:
- `ConversationContext` with `ContextWindow` and `ContextMessage`
- No hierarchy (global/pinned/local)
- No token budget tracking at the entry level

**Questions for our design**:
1. Do we need context hierarchy, or is flat context sufficient for Phase 1?
2. Should we track token counts per context entry?
3. How does context relate to agent sessions — is context session-scoped?

---

**Status**: Research complete with Fleet reference — ready for design decisions.
**Last Updated**: 2026-03-22
