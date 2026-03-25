# Architecture Documentation

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
- `agent/` - Agent domain entities (Session, Task, Action)
- `llm/` - LLM domain entities (Provider, Model, Message)

### Application Layer (`src-tauri/src/application/`)

Service implementations that coordinate business logic.

**Modules:**
- `llm/` - LLM service implementation
- (Additional services to be added)

### Commands Layer (`src-tauri/src/commands/`)

Tauri commands expose backend functionality to the frontend via IPC.

**Modules:**
- `agent.rs` - Agent-related commands
- `chat.rs` - Chat-related commands
- `config.rs` - Configuration commands
- `file.rs` - File operations
- `llm.rs` - LLM interaction commands
- `nlp.rs` - NLP commands

### Infrastructure Modules

| Module | Purpose |
|--------|---------|
| `agents/` | Agent orchestration runtime, planning, approvals |
| `llm/` | LLM provider integrations (OpenAI, Anthropic) |
| `nlp/` | Natural language processing utilities |
| `storage/` | SQLite database access (config, context, chat) |
| `tools/` | Agent tool implementations (file operations, etc.) |
| `utils/` | Shared utilities |

## Frontend Structure

### Component Organization

```
src-ui/src/lib/components/
├── chat/           # Chat interface components
│   ├── ChatArea.svelte
│   ├── ChatInput.svelte
│   ├── ChatMessage.svelte
│   └── ...
├── config/         # Configuration UI components
│   ├── ProvidersModal.svelte
│   ├── ModelSelector.svelte
│   └── ...
├── layout/         # Layout components
│   ├── Sidebar.svelte
│   ├── Header.svelte
│   └── ...
└── shared/         # Reusable components
    ├── Button.svelte
    ├── Modal.svelte
    └── ...
```

### State Management

Svelte 5 runes are used for reactive state management:

- `$state` - Local component state
- `$derived` - Computed values
- `$effect` - Side effects

Global stores are in `src/lib/stores/` for cross-component state sharing.

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
commands::chat::send_message (Backend)
    │
    ├─▶ ChatStore.save_message()
    ├─▶ AgentService.process()
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
commands::config::save_provider (Backend)
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

Key files:
- `agents/runtime/` - Agent execution runtime
- `agents/planner.rs` - Task planning
- `agents/approval.rs` - Action approval flow
- `agents/rollback.rs` - File rollback system

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
- `llm/openai/` - OpenAI API client
- `llm/anthropic/` - Anthropic API client

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
