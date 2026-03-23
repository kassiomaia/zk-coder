# Fleet Research - Pattern Synthesis

> **Purpose**: Knowledge base for designing Coder's agent orchestration. Not a blueprint — reference patterns to inform our own design.

---

## Architecture Layers

From the UML diagrams, Fleet has clear separation:

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend Layer                        │
│  UI → Agent Chat Request API → Agent Compatibility      │
└────────────────────────┬────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────┐
│                    Agent Layer                           │
│  Agent Launcher API → Agent Session → Agent Frontend    │
│                         ↓                                │
│               Direct Request Executor                    │
└────────────────────────┬────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────┐
│                   Protocol Layer                         │
│         ACP │ MCP │ Claude Protocol │ Router            │
└────────────────────────┬────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────┐
│                 External Services                        │
│    LLM Provider │ Terminal │ File System │ Tools        │
└─────────────────────────────────────────────────────────┘
                        │
┌────────────────────────▼────────────────────────────────┐
│              Context & Permissions                       │
│         Context Manager │ Permission System             │
└─────────────────────────────────────────────────────────┘
```

---

## Key Patterns Identified

### 1. Session as Primary Abstraction

**Pattern**: Agent configurations are static; sessions are runtime instances.

```
Agent (config) → Session (runtime state) → Request (user interaction)
```

**Components**:
- `AgentConfigScope` — Configuration for agent behavior
- `ChatAgentSessionEntity` — Links chat to session
- `AgentSessionDescriptor` — Full session metadata with lifecycle state

**States**: Launching → Running → WaitForRequest → UserInputRequired → Finished/Cancelled/Error

**Insight**: We should separate configuration from runtime state.

---

### 2. Multi-Protocol Support

**Pattern**: Different LLM providers use different protocols, routed dynamically.

| Protocol | Purpose |
|---|---|
| ACP | Agent Control Protocol — terminal, filesystem, control commands |
| MCP | Model Context Protocol — external tools |
| Claude Protocol | Anthropic-specific — tool calling, streaming |

**Routing**: `ProtocolRouter` selects protocol based on agent type.

**Insight**: Our `llm/` module is protocol-agnostic, which is good. We may need protocol-specific adapters for advanced features.

---

### 3. Permission-Based Execution Flow

**Pattern**: Trust mode determines auto-run vs. approval flow.

```
Tool Request → AccessKey → AccessType → Trust Mode Check
                                         ↓
                    ┌────────────────────┴────────────────────┐
                    ↓                                         ↓
              Trusted Mode                            Untrusted Mode
                    ↓                                         ↓
              Auto-run                                   Manual Approval
                    ↓                                         ↓
              Execute                               Permission Dialog
                                                            ↓
                                                      User Decision
                                                            ↓
                                              Allow → Execute / Deny → Block
```

**Key components**:
- `AgentPermissionHolder` — Permission state container
- `AccessKey` — Permission identifier per tool/operation
- `AccessType` — Hierarchy traversal for inherited permissions

**Insight**: We need a permission model before implementing file writes. "Trusted" vs "Untrusted" is a simple starting point.

---

### 4. Context Hierarchy for Parallel Execution

**Pattern**: Three-level context hierarchy enables parallel tasks with shared baseline.

```
                    Global Context (shared)
                           │
          ┌────────────────┼────────────────┐
          ↓                ↓                ↓
     Chat 1 Context   Chat 2 Context   Chat 3 Context
          │                │                │
          ↓                ↓                ↓
     Local Entries    Local Entries    Local Entries
     (isolated)       (isolated)       (isolated)
```

**Token Budget Flow**:
1. Pinned entries → guaranteed inclusion
2. Calculate pinned token count
3. Remaining budget → fit local entries
4. Global entries → shared baseline

**Insight**: For Phase 1, we may not need this complexity. But if we support parallel agent sessions, this pattern scales well.

---

### 5. Request-Response Event Stream

**Pattern**: Requests flow down, events flow up through layers.

```
Request Flow (down):
  UI → ChatAPI → Session → Launcher → AgentFrontend → LLM

Event Flow (up):
  LLM → AgentFrontend → Launcher → ChatAPI → UI
```

**Event types**:
- `AgentRequest` — Request to agent
- `ClientResponse` — Response from agent
- `ToolCall` / `ToolResult` — Tool invocations
- `Notification` — Status updates

**Insight**: Our current `LlmEvent` (Started, Chunk, Complete, Error) is similar but simpler. We may need more event types for agent lifecycle.

---

## What Fleet Does NOT Have

| Feature | Fleet | Implication |
|---|---|---|
| Subagents | No | Uses hierarchical sessions instead |
| Agent marketplace | No | Built-in agents only |
| User-configurable agents | No | System-provided configurations |
| Multi-agent collaboration | No | Independent sessions only |
| Cross-session memory | No | Context is per-session |

**Implication**: We don't need to design for multi-agent orchestration in Phase 1.

---

## Patterns to Evaluate for Coder

| Pattern | Complexity | Phase 1 Need | Notes |
|---|---|---|---|
| Session abstraction | Medium | **Yes** | Separates config from state |
| Permission system | Medium | **Yes** | Required for file writes |
| Context hierarchy | High | Maybe | Start flat, add hierarchy later |
| Multi-protocol | Low | No | Our LLM abstraction is sufficient |
| Event streaming | Low | **Yes** | Already have `LlmEvent` |
| Hierarchical sessions | Medium | Maybe | For task decomposition |

---

## Questions for Coder Design

1. **Session persistence**: Should sessions survive app restart? Fleet uses in-memory with DB for context.

2. **Permission granularity**: Tool-level (can use terminal), operation-level (can run this command), or both?

3. **Context scope**: Per-conversation, per-thread, or per-session?

4. **Event model**: Extend `LlmEvent` or create new `AgentEvent` type?

---

**Files in this research**:
- `01-agent-request-sequence.puml` — Request processing flow
- `02-context-management-flow.puml` — Context hierarchy and token management
- `03-permission-evaluation-flow.puml` — Permission and trust model
- `04-protocol-communication-architecture.puml` — Multi-protocol routing
- `05-agent-orchestration-architecture.puml` — Full system overview
