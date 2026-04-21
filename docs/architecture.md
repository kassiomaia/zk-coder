# Architecture Documentation

**Status**: ACTIVE
**Last reviewed**: 2026-04-04
**Purpose**: Current architecture reference for backend/frontend module boundaries and data flow

## Overview

Coder is built using a **layered architecture** with clear separation between the core domain, application services, and infrastructure layers.

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (Svelte)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Components  │  │    Stores    │  │    Utils     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                    Tauri IPC Commands
                            │
┌─────────────────────────────────────────────────────────────┐
│                   Backend (Rust)                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Commands Layer (Tauri commands/API)                 │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Application Layer (Services)                        │  │
│  │  - LlmService                                        │  │
│  │  - AgentService                                      │  │
│  │  - NlpService                                        │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Core Layer (Domain)                                 │  │
│  │  - Entities (Chat, Agent, LLM)                       │  │
│  │  - Value Objects                                     │  │
│  │  - Repository Interfaces                             │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Infrastructure Layer                                │  │
│  │  - Storage (SQLite)                                  │  │
│  │  - LLM Providers (OpenAI, Anthropic)                 │  │
│  │  - Agent Runtime                                     │  │
│  │  - Tools                                             │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Backend Structure

### Core Layer (`src-tauri/src/core/`)

The core layer contains foundational domain entities and interfaces with no dependencies on infrastructure.

**Modules:**
- `chat/` - Chat domain entities (Conversation, Thread, Message)
- `agent/` - Agent domain entities (Session, Task, Action) and strategy contracts (`agent/context.rs` defines `ContextCompactionStrategy` + `ContextPackOutcome` / `CompactionDetails` / `ContextState` value objects)
- `config/` - Provider/model configuration domain contracts
- `llm/` - LLM domain entities (Provider, Model, Message)
- `nlp/` - NLP domain contracts and context entities

### Application Layer (`src-tauri/src/application/`)

Service implementations that coordinate business logic.

**Modules:**
- `agent/` - Agent planning, execution orchestration, and conversions. The
  execution pipeline is split into focused modules under
  `application/agent/execution/`: `bootstrap`, `context_phase`, `tool_phase`,
  `response_phase`, `step_gate`, `park`, and `termination`. Both
  `context_phase` and `termination` are directory modules — their entry
  files retain the public phase entry point (`resolve_context` /
  `cancel_execution` + siblings) while dedicated submodules own narrower
  concerns. `context_phase/resolution.rs` owns memory-refresh triggers,
  retrieval-pass execution, and compaction recording;
  `context_phase/projection.rs` owns pure builders that turn durable state
  into context-policy value objects (`ExecutionBrief`, `MemoryReference`).
  `termination/streaming.rs` owns `StreamingContent` chunking/emission;
  `termination/subtasks.rs` owns the subtask state transitions used when a
  run finishes.
- `chat/` - Chat orchestration and service logic
- `llm/` - Provider-agnostic LLM service implementation
- `nlp/` - NLP orchestration and service logic

### Commands Layer (`src-tauri/src/commands/`)

Tauri commands expose backend functionality to the frontend via IPC.

**Modules:**
- `agent/` - Session lifecycle, approvals, rollback, subtasks, debug/history
- `chat/` - Conversations, threads, and messages commands
- `config/` - Provider, API key, model, and defaults commands
- `documents/` - Session artifact/document lifecycle commands
- `file/` - Read-only file viewing and diff window commands
- `llm/` - Chat request, streaming, and cancellation commands
- `nlp/` - Intent analysis and context commands

### Infrastructure Modules

| Module | Purpose |
|--------|---------|
| `infrastructure/agent/` | Agent runtime, approval/event handling, registry, telemetry |
| `infrastructure/api/` | External API adapters and transport-level integrations |
| `infrastructure/llm/` | Provider implementations (OpenAI, Anthropic), streaming, mocks |
| `infrastructure/nlp/` | NLP infra services and adapters |
| `infrastructure/sqlite/` | SQLite migrations and repository implementations |
| `infrastructure/tools/` | Tool execution, file operations, and document artifacts |

Agent meta-tool dispatch (`agent_plan_subtasks`, `agent_advance_to_next_subtask`, `agent_mark_subtask_status`, `agent_narrate`) runs through a static registry at `infrastructure/agent/runtime/tool_handlers/registry.rs`. Every handler module exposes a zero-sized unit struct + `'static` instance implementing the `ToolHandler` trait (`tool_handlers/dispatcher.rs`); the registry is a `OnceLock<HashMap<&'static str, &'static dyn ToolHandler>>` initialised once from an inline array. Adding a new meta-tool is one new handler module plus one entry in that array — no `match` arm to edit in `handle_agent_tool_call`.

Agent SQLite persistence is split by domain (`infrastructure/sqlite/repositories/agent/`). Each core trait is implemented by its own struct over a shared `Arc<Mutex<Connection>>`, so consumers depend only on the interface they need:

| Struct | Traits | File |
|--------|--------|------|
| `SqliteSessionRepository` | `SessionRepository` | `mod.rs` |
| `SqliteTaskRepository` | `TaskRepository` | `task_repo.rs` |
| `SqliteExecutionRepository` | `ExecutionRepository` + `RuntimeTraceRepository` | `execution_repo.rs` |
| `SqliteClarificationRepository` | `ClarificationRepository` + `ExecutionCheckpointRepository` | `clarification_repo.rs` |
| `SqliteMemoryRepository` | `MemoryRepository` | `memory_repo.rs` |
| `SqliteAgentTelemetryRepository` | `AgentTelemetryRepository` | `telemetry.rs` |

Wiring in `commands/agent/state.rs` builds every repo from the shared connection and hands `AgentRuntime` / `AgentServiceImpl` pre-typed `Arc<dyn …>` objects — the concrete SQLite types never cross into the application layer. Tests that need more than one repo use `infrastructure/sqlite/repositories/agent/test_support::TestAgentRepos`.

## Frontend Structure

### Component Organization

```
src-ui/src/lib/components/
├── agent/          # Agent panel, approvals, task/tool progress, workspace selectors
├── chat/           # Conversation/thread/message rendering and chat input surfaces
├── config/         # Provider/model setup and configuration flows
├── layout/         # App shell layout primitives
├── markdown/       # Markdown rendering and rich-content display components
├── shell/          # Higher-level shell components wiring app sections
└── shared/         # Reusable primitives used across feature modules
```

Components never import `@tauri-apps/api/core` directly — every IPC call goes through a typed wrapper in `src-ui/src/lib/api/`, which uses the shared `invokeCommand` / `createInvoker` helpers from `api/core.ts`. That keeps command names, argument shapes, and error classes out of the component layer and puts them behind per-domain modules (`chat`, `config`, `nlp`, `llm`, `file`, `agent`, `artifact`, plus feature-specific ones for web-search, workspace, etc.). Each domain has its own `*ApiError` subclass so callers can `instanceof`-check without conflating unrelated failures.

### State Management

Svelte 5 runes are used for reactive state management:

- `$state` - Local component state
- `$derived` - Computed values
- `$effect` - Side effects

Global stores are in `src-ui/src/lib/stores/` for cross-component state sharing.

The chat store (`src-ui/src/lib/stores/chat.ts`) owns a single `writable<ChatState>` but delegates per-domain mutations to sibling factory modules under `src-ui/src/lib/stores/chat/`. Each factory receives `update` / `getState` (and, for NLP, the shared debounced classifier), returns a bag of methods, and the host store spreads those bags into the final store shape:

| Module | Responsibility |
|---|---|
| `chat/nlp-methods.ts` | Intent classification, ambiguity detection, and resolution |
| `chat/conversation-methods.ts` | Conversation CRUD (create / rename / delete / archive / settings / workspace) |
| `chat/thread-methods.ts` | Simple thread CRUD (resolve / archive / YOLO / delete / rename) |
| `chat/message-methods.ts` | Message CRUD (add / delete / status / feedback / update / remove transient) |
| `chat/utility-methods.ts` | Thin helpers (`clearError`, `markThreadRead`, `setThreadWorkspace`) |
| `chat/llm-handler.ts` | Streaming LLM request lifecycle (validate / init / stream / cancel) |
| `chat/utils.ts` | Pure helpers — factories (`createNewConversation`, …), `deriveThreadTitle`, `sortThreadsByRecent`, `touchThreadAndConversation` |
| `chat/derived-stores.ts` | All `derived(...)` exports, built from the host store to avoid circular deps |

Cross-domain orchestration (`initialize`, `selectConversation`, `selectThread`, `createThread`, `sendMessage`, `sendMessageWithLlm`, `cancelLlmStream`, `reset`) stays in `chat.ts` because each method reaches across more than one domain (e.g. thread selection activates an agent session and surfaces a paused-session recovery message). That keeps the split honest — domain modules do not need to know about each other.

### Runtime Configuration

Shared runtime constants (stream timeouts, buffering caps, retention windows, debounce intervals) live in `src-ui/src/lib/config/runtime.ts`. Stores and utilities import named constants from this module instead of inlining magic numbers.

## Data Flow

### Chat Message Flow

```
User Input (Frontend)
    │
    ▼
ChatInput Component
    │
    ▼
invoke('send_message', {...})
    │
    ▼
commands::chat::messages::send_message (Backend)
    │
    ├─▶ Chat repositories persist conversation/thread/message state
    ├─▶ Agent service/runtime executes follow-up actions when requested
    │       │
    │       ├─▶ LLMProvider.chat()
    │       │       │
    │       │       └─▶ HTTP Request to API
    │       │
    │       └─▶ AgentRuntime.execute_tools()
    │
    └─▶ Return response to frontend
            │
            ▼
    Update ChatStore (Frontend)
            │
            ▼
    Re-render ChatArea
```

### Configuration Flow

```
Settings UI (Frontend)
    │
    ▼
invoke('save_provider', {...})
    │
    ▼
commands::config::providers::save_provider (Backend)
    │
    ├─▶ Validate API key
    ├─▶ ConfigStore.save_provider()
    │
    └─▶ Return success
            │
            ▼
    Update ConfigStore (Frontend)
```

## Databases

The application uses SQLite for data persistence:

| Database | Purpose | Location |
|----------|---------|----------|
| `coder_config.db` | Configuration (providers, models, API keys) | App data directory |
| `coder_context.db` | NLP conversation context | App data directory |
| `coder_chat.db` | Conversations, threads, messages | App data directory |
| `coder_agents.db` | Agent sessions and state | App data directory |

## Agent System

The agent system orchestrates multi-step AI tasks:

1. **Planning** - Break down complex requests into subtasks
2. **Execution** - Run tools based on plan
3. **Approval** - User can approve/deny actions
4. **Rollback** - Revert changes if needed

Key files/directories:
- `src-tauri/src/application/agent/` - Agent planning/execution service orchestration
- `src-tauri/src/infrastructure/agent/runtime/` - Runtime loop and execution coordination
- `src-tauri/src/commands/agent/approvals.rs` - User approval/denial command handlers
- `src-tauri/src/commands/agent/rollback.rs` - Session/file rollback command handlers

## LLM Integration

The LLM layer is provider-agnostic:

```
┌─────────────────────────────────────┐
│       LlmService (Interface)        │
└─────────────────────────────────────┘
            │
    ┌───────┴───────┐
    ▼               ▼
┌────────┐      ┌────────┐
│ OpenAI │      │Anthropic│  ...
└────────┘      └────────┘
```

Provider implementations:
- `src-tauri/src/infrastructure/llm/openai/` - OpenAI API client
- `src-tauri/src/infrastructure/llm/anthropic/` - Anthropic API client

## Additional References

- `docs/tools-and-agent-integration.md` - Detailed tool registry, tool catalog, parameters, and agent interaction flow

## Key Design Principles

1. **Separation of Concerns** - Clear boundaries between layers
2. **Dependency Injection** - Services depend on interfaces, not implementations
3. **Error Handling** - Comprehensive error types with `thiserror` and `anyhow`
4. **Async/Await** - Non-blocking I/O throughout
5. **Type Safety** - Leverage Rust's type system for correctness

## File Size Limits

To maintain code quality:
- **Soft limit**: 500 lines per file
- **Hard limit**: 800 lines per file
- Refactor when exceeding soft limit
