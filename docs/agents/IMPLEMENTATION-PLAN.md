# Agent Orchestration Implementation Plan

> **Status**: Planning Phase
> **Created**: 2026-03-22
> **Related**: [Product PRD](../product/multi-agent-platform-prd.md) | [Architecture Research](../analysis/agent-orchestration-research.md) | [Fleet Research](../references/fleet-research/SYNTHESIS.md)

---

## Executive Summary

Transform Coder from a chat-based AI assistant into an **agent platform** where AI agents can perform real work (read files, execute commands, modify code) with human-in-the-loop approval for destructive operations.

**Phase 1 Target**: Code Agent with file read/write, command execution, and approval gates.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend (Svelte 5)                      │
│                                                              │
│  Chat UI (existing)     │     Agent Panel (new)              │
│  - Messages             │     - Todo List                    │
│  - Input                │     - File Tree                    │
│  - Thread Panel         │     - Terminal View                │
│                         │     - Approval Dialog              │
│                         │     - Progress Indicator           │
└──────────────────────┬──────────────────────────────────────┘
                       │ Tauri IPC
┌──────────────────────▼──────────────────────────────────────┐
│                    Command Layer                             │
│                                                              │
│  Existing Commands      │     New Agent Commands             │
│  - send_message         │     - create_session               │
│  - load_conversations   │     - send_agent_request          │
│  - config/*             │     - approve_action              │
│                         │     - cancel_session              │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────┐
│                  Agent Runtime (new)                         │
│                                                              │
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │ AgentRegistry   │  │ AgentSession    │                   │
│  │ - Built-in      │  │ - State machine │                   │
│  │ - Configs       │  │ - Context       │                   │
│  └─────────────────┘  └────────┬────────┘                   │
│                                │                             │
│  ┌─────────────────────────────▼──────────────────────────┐ │
│  │                    Execution Loop                       │ │
│  │  LLM Request → Tool Calls → Execute → Append → Loop    │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                │                             │
│  ┌─────────────────────────────▼──────────────────────────┐ │
│  │                  Permission System                      │ │
│  │  Tool → Permission Check → Auto-run or Approval Queue  │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────┐
│                   Existing Layers                            │
│                                                              │
│  LLM Client (llm/)    │    Tools (tools/)    │    Storage   │
│  - OpenAI             │    - fs/*            │    - SQLite  │
│  - Anthropic          │    - search/*        │    - stores  │
│  - Google             │    - conversation/*  │              │
│                       │    - system/*        │              │
└──────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Foundation (Current Focus)

**Goal**: Establish the agent runtime with permission-based execution.

| Milestone | Description | Documents |
|-----------|-------------|-----------|
| 1.1 | Agent types and session abstraction | [Backend Tasks](./backend-tasks.md) §1 |
| 1.2 | Permission system and approval queue | [Backend Tasks](./backend-tasks.md) §2 |
| 1.3 | Agent runtime extraction from LLM commands | [Backend Tasks](./backend-tasks.md) §3 |
| 1.4 | Frontend agent panel components | [Frontend Tasks](./frontend-tasks.md) §1 |
| 1.5 | Approval dialog and action handling | [Frontend Tasks](./frontend-tasks.md) §2 |
| 1.6 | Integration and E2E testing | Both |

### Phase 2: Code Agent Features

**Goal**: Full file operations and command execution.

| Milestone | Description |
|-----------|-------------|
| 2.1 | File write/modify with diff preview |
| 2.2 | Command execution with output streaming |
| 2.3 | Workspace browser UI |
| 2.4 | Diff viewer for proposed changes |
| 2.5 | Rollback capability |

### Phase 3: Advanced Features

**Goal**: Multi-session support and task management.

| Milestone | Description |
|-----------|-------------|
| 3.1 | Parallel session execution |
| 3.2 | Task decomposition UI |
| 3.3 | Session history and replay |
| 3.4 | Agent configuration UI |

---

## Key Design Decisions

### 1. Session vs Agent

**Decision**: Separate concepts.

- **Agent** = Configuration (name, system prompt, allowed tools, model)
- **Session** = Runtime instance (state, context, pending actions)

**Rationale**: Enables multiple concurrent sessions with same agent config. Matches Fleet pattern.

### 2. Permission Model

**Decision**: Tool-level permissions with trust modes.

```rust
pub enum PermissionLevel {
    AutoRun,     // Execute immediately (read operations)
    ApprovalRequired,  // Queue for user approval (write, delete, execute)
    Disabled,    // Tool not available for this agent
}

pub enum TrustMode {
    Trusted,     // Auto-run all tools
    Untrusted,   // Require approval for destructive ops
    Restricted,  // Require approval for all tools
}
```

**Rationale**: Simple but extensible. Can add operation-level permissions later.

### 3. Approval Flow

**Decision**: Synchronous blocking with timeout.

```
Tool Call → Permission Check → Approval Required?
    │                              │
    │ No (AutoRun)                 │ Yes
    ▼                              ▼
Execute                Emit event to frontend
    │                              │
    │                       User sees dialog
    │                              │
    │                       Approve │ Deny
    │                              │     │
    │                              ▼     ▼
    │                        Execute  Block
```

**Rationale**: Simple UX. User sees exactly what agent wants to do before it happens.

### 4. Frontend Panel Integration

**Decision**: Right panel alongside chat, collapsible.

```
┌──────────────┬──────────────────────┬─────────────────┐
│   Sidebar    │      Chat Area       │   Agent Panel   │
│              │                      │                 │
│ Conversations│  Messages            │  ▸ Tasks        │
│ Threads      │  ────────────        │  ▸ Files        │
│              │  Input               │  ▸ Terminal     │
│              │                      │  ▸ Actions      │
└──────────────┴──────────────────────┴─────────────────┘
```

**Rationale**: Maintains existing UX while adding agent visibility. See [UI Components Design](./ui-components-design.md).

### 5. Workspace Association

**Decision**: Each conversation has an associated workspace folder.

**Flow**:
```
Create conversation → Select workspace folder (optional) → Store with conversation
                                                                    ↓
                                              Agent uses workspace for file operations
```

**UI**:
- Workspace selector in conversation settings or at creation
- File tree shows workspace contents when agent is active
- Workspace path shown in conversation header

**Backend**:
- `Conversation.workspace_root: Option<PathBuf>`
- Tools receive workspace path via `ToolExecutionContext`
- File operations restricted to workspace boundary

---

## Task Breakdown

| Section | Description | Document |
|---------|-------------|----------|
| §1 | Agent types and session abstraction | [Backend Tasks](./backend-tasks.md) §1 |
| §2 | Permission system and approval queue | [Backend Tasks](./backend-tasks.md) §2 |
| §3 | Agent runtime extraction | [Backend Tasks](./backend-tasks.md) §3 |
| §4 | Tauri commands for agent API | [Backend Tasks](./backend-tasks.md) §4 |
| §5 | Session storage (SQLite) | [Backend Tasks](./backend-tasks.md) §5 |
| §6 | Event system for frontend | [Backend Tasks](./backend-tasks.md) §6 |
| §1 | Layout integration and panel | [Frontend Tasks](./frontend-tasks.md) §1 |
| §2 | Task list component | [Frontend Tasks](./frontend-tasks.md) §2 |
| §3 | File tree component | [Frontend Tasks](./frontend-tasks.md) §3 |
| §4 | Terminal view component | [Frontend Tasks](./frontend-tasks.md) §4 |
| §5 | Approval dialog component | [Frontend Tasks](./frontend-tasks.md) §5 |
| §6 | TypeScript types | [Frontend Tasks](./frontend-tasks.md) §6 |
| §7 | Agent store (state management) | [Frontend Tasks](./frontend-tasks.md) §7 |
| - | Visual design specifications | [UI Components Design](./ui-components-design.md) |

---

## Dependencies

### New Rust Crates

```toml
# Agent execution
async-trait = "0.1"

# Diff generation (Phase 2)
similar = "2.0"
```

### New Frontend Dependencies

```json
{
  "dependencies": {
    "@xyflow/svelte": "^4.0.0"
  }
}
```

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Agent makes destructive mistakes | All writes require approval; diffs shown before apply |
| Session state inconsistency | Single source of truth in Rust; frontend subscribes |
| UI complexity | Progressive disclosure; agent panel is optional |
| LLM token limits for context | Implement context budget management |

---

## Success Criteria (Phase 1)

- [ ] User can see agent executing tasks in real-time
- [ ] User can approve/reject file writes before they happen
- [ ] User can see diff preview for code changes
- [ ] Agent respects workspace boundaries (files outside workspace are blocked)
- [ ] All actions logged in thread history
- [ ] Chat continues to work if agent system fails
- [ ] LLM automatically decides when to use tools (no manual trigger needed)
- [ ] Each conversation can have its own workspace folder
- [ ] Pending approvals auto-deny after 5 minutes of inactivity

---

## Design Decisions (Resolved)

| # | Question | Decision | Rationale |
|---|----------|----------|-----------|
| 1 | Session persistence across restarts? | **SQLite persistence** | Users can resume incomplete tasks; session history preserved |
| 2 | Multiple sessions per thread? | **Single session per thread** | Simpler state management; matches current UX patterns |
| 3 | Approval timeout behavior? | **Auto-deny after 5 minutes** | Safer default; prevents indefinite blocking; user can re-trigger action |
| 4 | Default trust mode? | **Untrusted** | Require approval for destructive ops; safer for new users; can be elevated per-session |
| 5 | When does agent activate? | **LLM decides** | Model automatically determines if tools are needed based on user prompt; no extra UI |
| 6 | Workspace folder? | **Per-conversation workspace** | Each conversation has an associated workspace folder; user selects when creating conversation |

---

## Implementation Status (Updated 2026-03-23)

### Backend: ✅ Phase 1 Complete

| Milestone | Status | Notes |
|-----------|--------|-------|
| 1.1 Agent types and session abstraction | ✅ Done | `types.rs`, `session.rs`, `task.rs` |
| 1.2 Permission system and approval queue | ✅ Done | `permissions.rs`, `approval.rs` with 5-min timeout |
| 1.3 Agent runtime extraction | ✅ Done | `runtime.rs` with LLM retry, permission checks |
| 1.4 Tauri commands | ✅ Done | Full command set in `commands/agent.rs` |
| 1.5 Session storage (SQLite) | ✅ Done | `store.rs` with persistence |
| 1.6 Event system | ✅ Done | `events.rs` with all event types |
| **Bonus: Rollback system** | ✅ Done | `rollback.rs` for file reverts |
| **Bonus: Subtasks** | ✅ Done | Task decomposition support |
| **Bonus: Conversation workspace** | ✅ Done | `workspace_root` on ConversationSettings |

### Frontend: ✅ Phase 1 Complete

| Milestone | Status | Notes |
|-----------|--------|-------|
| 1.4 Agent panel components | ✅ Done | All core components implemented |
| 1.5 Approval dialog | ✅ Done | With DiffViewer preview |
| Integration | ✅ Done | ChatLayout integrated, store wiring complete |
| Conversation workspace | ✅ Done | WorkspaceSelector in agent panel empty state |
| Execution complete fix | ✅ Done | Event now includes conversationId/threadId |

### Missing / Deviated Items

| Item | Plan | Actual | Action |
|------|------|--------|--------|
| AgentLayout component | Separate wrapper | Integrated in ChatLayout | Valid simplification - works |
| ~~Conversation workspace~~ | ~~Persistent on Conversation~~ | ✅ Implemented | `workspace_root` on ConversationSettings |
| ~~CSS variable~~ | ~~`--agent-panel-width`~~ | ✅ Implemented | In design-system.css with responsive behavior |
| Frontend tests | Required | Not done | Add tests |
| Focus management | Specified | Partial | Complete implementation |

### Recent Fixes (2026-03-23)

1. **AgentSession optional fields**: Made `tasks`, `filesTouched`, `terminalOutput` optional in TypeScript type to match backend response
2. **ExecutionComplete event**: Added `conversationId` and `threadId` to event so assistant message can be added even when session not in store (page reload scenario)
3. **AgentPanel null checks**: Added `?? []` fallbacks for optional session fields
4. **CSS variable cleanup**: Removed redundant fallback in `AgentPanel.svelte` since `--agent-panel-width` is always defined in design system

---

## Next Steps

1. ~~Add conversation-level workspace persistence~~ ✅ Done
2. ~~Add CSS variable for agent panel width~~ ✅ Done
3. Add frontend tests for agent components
4. Complete focus management in approval dialog

---

**Document Status**: Updated - Phase 1 Complete
**Author**: Development Team
**Last Updated**: 2026-03-23
