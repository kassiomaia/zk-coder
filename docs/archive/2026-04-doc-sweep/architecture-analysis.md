# Architecture Analysis & Restructure Plan

> **ARCHIVED (2026-04-04)**: Historical analysis captured before the current layered module layout. Paths and size data may be outdated.

## Executive Summary

This document analyzes the current architecture of the Coder application and proposes a restructure to improve separation of concerns, reduce coupling, and increase scalability.

**Analysis Date:** 2025-03-25

**Critical Issues Identified:**
- 9 files exceed the 500-line soft limit
- Tight coupling between commands, state, and business logic
- Frontend stores mixing multiple concerns
- No clear abstraction layers for LLM providers

---

## 1. Current State Analysis

### 1.1 Files Exceeding Size Limits

| File | Lines | Issue |
|------|-------|-------|
| `src-tauri/src/llm/mod.rs` | 887 | Mixes types, traits, streaming logic, tests |
| `src-tauri/src/llm/openai/client.rs` | 836 | Client implementation too large |
| `src-ui/src/lib/components/chat/ChatArea.svelte` | 835 | UI component too large |
| `src-tauri/src/commands/llm.rs` | 815 | Commands + business logic mixed |
| `src-tauri/src/llm/anthropic/client.rs` | 729 | Client implementation too large |
| `src-tauri/src/commands/config.rs` | 706 | Config commands too large |
| `src-tauri/src/commands/agent.rs` | 655 | Agent commands too large |
| `src-tauri/src/commands/llm/orchestrator.rs` | 619 | Orchestrator too large |
| `src-tauri/src/api/mod.rs` | 611 | Provider-specific validation mixed |

### 1.2 Coupling Issues

#### Backend (Rust)

1. **Commands Layer Anti-Pattern**
   - `commands/agent.rs` (655 lines): Directly accesses registry, runtime, session_store, approval_queue
   - `commands/chat.rs` (399 lines): Directly accesses store, but also contains thread validation logic
   - `commands/config.rs` (706 lines): Mixes API validation with config management

2. **NLP Module Boundary Issues**
   - `nlp/mod.rs` re-exports `chat` types (ChatMessage, Conversation, Thread)
   - Creates circular dependency: `nlp` → `chat` → `nlp` (via MessageRole)
   - `config` module inside `nlp/` for LLM provider management (misplaced)

3. **LLM Module Structure**
   - `llm/mod.rs` contains 880+ lines mixing:
     - Message types
     - Request/response types
     - Streaming types
     - Event types
     - Client trait
     - Factory function
     - Helper functions (stream_to_events)
     - Tests

#### Frontend (Svelte/TypeScript)

1. **Store Coupling**
   - `chat.ts` (1250+ lines) directly imports and uses `agentStore`
   - Creates circular dependency potential
   - Mixes chat, NLP, and LLM concerns

2. **Component Size**
   - `ChatArea.svelte` (835 lines) handles:
     - Message rendering
     - Scroll management
     - Streaming display
     - Thread panel integration
     - Input handling

### 1.3 Scalability Issues

| Concern | Current State | Impact |
|---------|---------------|---------|
| Adding LLM Provider | Modify `llm/mod.rs`, `api/mod.rs`, `commands/config.rs` | High touch points |
| Adding Agent Tool | Modify runtime, understand entire tool system | Complex |
| Adding Chat Feature | Modify large `chat.ts` store | Risk of regression |
| Testing | Business logic in command handlers | Hard to unit test |

---

## 2. Proposed Architecture

### 2.1 Layered Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Presentation Layer                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   Commands   │  │    Events    │  │   Queries    │         │
│  │   (IPC API)  │  │   (Emitter)  │  │  (Read-only) │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Application Layer                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   Services   │  │  Workflows   │  │  Orchestrator│         │
│  │  (Business   │  │  (Multi-step │  │  (Complex    │         │
│  │   Logic)     │  │  operations) │  │   flows)     │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                          Domain Layer                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   Entities   │  │  Value Objs  │  │  Domain      │         │
│  │  (Core       │  │  (Immutable  │  │  Services    │         │
│  │   Models)    │  │   data)      │  │  (Interfaces)│         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Infrastructure Layer                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │  Repositories│  │   External   │  │    Events     │         │
│  │  (Storage    │  │   Services   │  │   (Bus)      │         │
│  │   Access)    │  │  (LLM, FS)   │  │              │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Module Restructuring

#### Backend Structure

```
src-tauri/src/
├── core/                    # New: Core domain layer
│   ├── chat/               # Extracted from nlp/chat
│   │   ├── entities/       # Message, Thread, Conversation
│   │   ├── repositories/   # Storage traits
│   │   └── services/       # ChatService interface
│   ├── agent/              # Agent domain (extract from agents/)
│   │   ├── entities/       # Session, Task, Approval
│   │   ├── repositories/   # SessionStore traits
│   │   └── services/       # AgentService interface
│   └── llm/                # LLM domain (extract from llm/)
│       ├── entities/       # Message, Request, Response
│       ├── repositories/   # Provider registry traits
│       └── services/       # LlmService interface
│
├── infrastructure/          # New: Infrastructure implementations
│   ├── persistence/        # Database implementations
│   │   ├── sqlite/         # ChatStore, SessionStore, etc.
│   │   └── memory/         # In-memory for testing
│   ├── llm/                # LLM provider implementations
│   │   ├── openai/         # OpenAI client (refactored)
│   │   ├── anthropic/      # Anthropic client (refactored)
│   │   ├── google/         # Google client (new module)
│   │   └── factory.rs      # Provider factory
│   └── events/             # Event bus implementation
│
├── application/             # New: Application services
│   ├── chat/               # ChatService implementation
│   │   ├── commands/       # CreateThread, SendMessage, etc.
│   │   ├── queries/        # GetMessages, GetThreads, etc.
│   │   └── workflows/      # complex multi-step operations
│   ├── agent/              # AgentService implementation
│   │   ├── commands/       # CreateSession, SendRequest, etc.
│   │   ├── queries/        # GetSession, GetHistory, etc.
│   │   └── workflows/      # Orchestration workflows
│   └── llm/                # LlmService implementation
│       ├── streaming.rs    # StreamToEvents (extracted)
│       └── validation.rs   # API key validation (extracted)
│
├── adapters/               # New: IPC adapters (thin layer)
│   ├── commands/           # Refactored to delegate to services
│   │   ├── chat.rs         # ~100 lines: delegate to ChatService
│   │   ├── agent.rs        # ~100 lines: delegate to AgentService
│   │   ├── config.rs       # ~100 lines: delegate to ConfigService
│   │   └── llm.rs          # ~100 lines: delegate to LlmService
│   └── events/             # Event forwarding to frontend
│
├── agents/                 # Keep but refactor internals
│   ├── runtime/            # Split into smaller modules
│   ├── permissions/        # Keep
│   └── tools/              # Keep
│
├── tools/                  # Keep
├── utils/                  # Keep
├── api/                    # Deprecate: merge into infrastructure/llm
├── commands/               # Replace with adapters/commands
├── llm/                    # Replace with core/llm + infrastructure/llm
├── nlp/                    # Simplify: extract chat types
├── storage/                # Replace with infrastructure/persistence
└── lib.rs                  # Update with new structure
```

#### Frontend Structure

```
src-ui/src/lib/
├── core/                   # New: Core domain types
│   ├── chat/              # Chat entities (shared with backend)
│   ├── agent/             # Agent entities (shared with backend)
│   └── llm/               # LLM entities (shared with backend)
│
├── stores/                # Refactored stores
│   ├── chat/
│   │   ├── state.ts       # State types only
│   │   ├── actions.ts     # Actions (~200 lines)
│   │   ├── selectors.ts   # Derived state (extracted)
│   │   ├── llm/           # LLM-specific logic
│   │   │   ├── handler.ts # Streaming handler
│   │   │   └── state.ts   # LLM state
│   │   └── index.ts       # Store factory
│   └── agent/
│       ├── state.ts       # State types only
│       ├── actions/       # Split into smaller files
│       ├── events/        # Event handlers (split)
│       └── index.ts       # Store factory
│
├── components/
│   └── chat/
│       ├── ChatArea/      # New: Directory for complex component
│       │   ├── ChatArea.svelte       # Main component (~200 lines)
│       │   ├── MessageList.svelte    # Extracted
│       │   ├── StreamingContent.svelte # Extracted
│       │   ├── ThreadPanel.svelte    # Already separate
│       │   └── types.ts             # Local types
│       ├── input/
│       │   └── ChatInput.svelte      # Keep, maybe split
│       └── messages/
│           ├── ChatMessage.svelte    # Keep
│           └── ...
│
├── api/                    # API clients (refactored)
│   ├── chat/              # Keep
│   ├── agent/             # Keep
│   └── llm/               # Keep
│
└── services/              # New: Business logic layer
    ├── chat/              # ChatService (extracted from store)
    ├── agent/             # AgentService (extracted from store)
    └── llm/               # LlmService (extracted from store)
```

---

## 3. Restructuring Steps

### Phase 1: Foundation (High Impact, Low Risk)

1. **Create Core Domain Layer**
   - Create `core/` directory structure
   - Move entities from `nlp/chat/` to `core/chat/`
   - Define repository traits
   - Define service interfaces

2. **Extract LLM Streaming Logic**
   - Move `stream_to_events` from `llm/mod.rs` to `application/llm/streaming.rs`
   - Create separate module for chunk batching logic
   - Add unit tests for streaming behavior

3. **Split ChatArea Component**
   - Create `ChatMessageList.svelte` (message rendering)
   - Create `StreamingDisplay.svelte` (streaming UI)
   - Create `ThreadPanel.svelte` (already exists)
   - Reduce `ChatArea.svelte` to ~200 lines

### Phase 2: Application Services (Medium Impact, Medium Risk)

4. **Create ChatService**
   - Create `application/chat/` directory
   - Implement `ChatService` with business logic
   - Move thread validation logic here
   - Add comprehensive tests

5. **Create LlmService**
   - Create `application/llm/` directory
   - Move validation from `api/mod.rs`
   - Implement `LlmService` interface
   - Add provider-agnostic API

6. **Refactor Commands Layer**
   - Convert `commands/` to thin adapters
   - Each command delegates to service
   - Reduce command files to ~100 lines each

### Phase 3: Frontend Decoupling (Medium Impact, Medium Risk)

7. **Extract Business Logic from Stores**
   - Create `services/` directory
   - Move non-state logic from `chat.ts`
   - Move non-state logic from `agentStore.ts`

8. **Split Store Files**
   - `chat.ts` → `chat/index.ts` + modules
   - `agentStore.ts` → `agent/index.ts` + modules
   - Create clear boundaries between stores

9. **Create Service Layer**
   - `ChatService`: Chat operations
   - `AgentService`: Agent operations
   - `LlmService`: LLM operations
   - Stores manage state only

### Phase 4: Infrastructure (Low Impact, Low Risk)

10. **Refactor LLM Module**
    - Split `llm/mod.rs` into smaller files
    - Move provider-specific logic to `infrastructure/llm/`
    - Create provider factory pattern

11. **Merge API Module**
    - Move `api/` content to `infrastructure/llm/`
    - Deprecate `api/` module
    - Update imports

12. **Consolidate Storage**
    - Move `storage/` to `infrastructure/persistence/`
    - Unify repository patterns
    - Add in-memory implementations for testing

### Phase 5: NLP Cleanup (Low Impact, Low Risk)

13. **Extract Chat Types from NLP**
    - Move `nlp/chat/` to `core/chat/`
    - Update imports across codebase
    - Remove circular dependencies

14. **Simplify NLP Module**
    - Keep only NLP-specific functionality
    - Move config to own module
    - Clean up exports

---

## 4. Design Principles

### 4.1 Dependency Inversion

**Before:**
```rust
// commands/agent.rs directly depends on concrete implementations
pub struct AgentState {
    pub registry: Arc<AgentRegistry>,
    pub session_store: Arc<SessionStore>,
    pub approval_queue: Arc<ApprovalQueue>,
    pub runtime: Arc<AgentRuntime>,
}
```

**After:**
```rust
// adapters/commands/agent.rs depends on service interface
pub struct AgentState {
    pub agent_service: Arc<dyn AgentService>,
}

// application/agent/service.rs implements interface
struct AgentServiceImpl {
    registry: Arc<AgentRegistry>,
    session_store: Arc<SessionStore>,
    // ... dependencies hidden
}
```

### 4.2 Single Responsibility

**Before:**
```rust
// commands/llm.rs does validation, streaming, and IPC
pub async fn send_chat_message_streaming(...) -> Result<(), String> {
    // 1. Validate provider
    // 2. Create client
    // 3. Handle streaming
    // 4. Emit events
}
```

**After:**
```rust
// adapters/commands/llm.rs - IPC only
pub async fn send_chat_message_streaming(
    request: ChatRequest,
    state: State<'_, LlmState>
) -> Result<(), String> {
    state.llm_service.stream_chat(request).await?
}

// application/llm/service.rs - business logic
impl LlmService for LlmServiceImpl {
    async fn stream_chat(&self, request: ChatRequest) -> Result<Stream> {
        // Validation, client creation, streaming orchestration
    }
}
```

### 4.3 Clear Boundaries

**Module Dependencies:**
```
adapters/commands → application/services → core/domain
                                     → infrastructure/implementations
```

**No circular dependencies allowed.**

---

## 5. Testing Strategy

### 5.1 Unit Tests

- **Core Layer**: Pure functions, no I/O
- **Application Layer**: Mock infrastructure
- **Adapters Layer**: Mock services

### 5.2 Integration Tests

- Test service implementations with real infrastructure
- Test workflows end-to-end

### 5.3 Test Utilities

```rust
// infrastructure/testing/mod.rs
pub struct TestFixture {
    pub chat_service: Arc<dyn ChatService>,
    pub agent_service: Arc<dyn AgentService>,
    // In-memory implementations
}
```

---

## 6. Migration Strategy

### 6.1 Incremental Migration

1. **New structure alongside old**
2. **Feature flags for new code**
3. **Gradual migration of callers**
4. **Remove old code when complete**

### 6.2 Backward Compatibility

- Keep old IPC commands during transition
- Add deprecation warnings
- Document migration path

---

## 7. Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Files > 500 lines | 9 | 0 |
| Command handler avg lines | ~300 | <100 |
| Store file lines | 1250+ | <400 |
| Time to add LLM provider | 3+ files | 1 file |
| Unit test coverage | ~30% | >70% |
| Circular dependencies | 2+ | 0 |

---

## 8. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking changes | Medium | High | Incremental migration, feature flags |
| Performance regression | Low | Medium | Benchmark before/after |
| Increased complexity | Medium | Medium | Clear patterns, documentation |
| Developer resistance | Low | Low | Training, clear benefits |

---

## 9. Next Steps

1. **Review this document** with the team
2. **Prioritize phases** based on current pain points
3. **Create spike tasks** for high-risk areas
4. **Begin Phase 1** with core domain layer
5. **Establish metrics** to track progress

---

## Appendix A: File Size Details

```
# Exceeding 500 lines (needs immediate attention)
src-tauri/src/llm/mod.rs                        887
src-tauri/src/llm/openai/client.rs              836
src-ui/src/lib/components/chat/ChatArea.svelte   835
src-tauri/src/commands/llm.rs                    815
src-tauri/src/llm/anthropic/client.rs           729
src-tauri/src/commands/config.rs                 706
src-tauri/src/commands/agent.rs                  655
src-tauri/src/commands/llm/orchestrator.rs       619
src-tauri/src/api/mod.rs                         611

# Approaching limit (monitor)
src-tauri/src/nlp/config.rs                      490
src-tauri/src/agents/task.rs                     555
src-tauri/src/agents/runtime/execution.rs        557
src-tauri/src/storage/config_store/mod.rs        569
src-tauri/src/storage/chat_store/mod.rs          388
src-ui/src/lib/stores/chat.ts                    1250+
src-ui/src/lib/stores/agent/store.ts             310
```

## Appendix B: Dependency Graph

```
Current (simplified):
┌─────────────────────────────────────────────────────────────┐
│                         Commands                            │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │ agent   │───▶│  chat   │◀──│   llm   │◀──│  config  │  │
│  └────┬────┘    └─────────┘    └─────────┘    └─────────┘  │
│       │                                                       │
│       ▼                                                       │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐                  │
│  │ agents  │    │   nlp   │    │  llm    │                  │
│  └────┬────┘    └────┬────┘    └────┬────┘                  │
│       │              │              │                        │
│       └──────────────┴──────────────┘                        │
│                      ▼                                       │
│              ┌───────────┐                                   │
│              │  storage  │                                   │
│              └───────────┘                                   │
└─────────────────────────────────────────────────────────────┘

Proposed:
┌─────────────────────────────────────────────────────────────┐
│                       Adapters                              │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │ command │───▶│ command │───▶│ command │───▶│ command │  │
│  │ agent   │    │  chat   │    │   llm   │    │  config │  │
│  └────┬────┘    └────┬────┘    └────┬────┘    └────┬────┘  │
└───────┼──────────────┼──────────────┼──────────────┼───────┘
        │              │              │              │
        ▼              ▼              ▼              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Application                              │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │   Agent │    │   Chat  │    │    LLM  │    │  Config │  │
│  │ Service │    │ Service │    │ Service │    │ Service │  │
│  └────┬────┘    └────┬────┘    └────┬────┘    └────┬────┘  │
└───────┼──────────────┼──────────────┼──────────────┼───────┘
        │              │              │              │
        ▼              ▼              ▼              ▼
┌─────────────────────────────────────────────────────────────┐
│                        Core                                  │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │  Agent  │    │   Chat  │    │    LLM  │    │  Config │  │
│  │Domain   │    │ Domain  │    │ Domain  │    │ Domain  │  │
│  └────┬────┘    └────┬────┘    └────┬────┘    └────┬────┘  │
└───────┼──────────────┼──────────────┼──────────────┼───────┘
        │              │              │              │
        ▼              ▼              ▼              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Infrastructure                             │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │ Runtime │    │  Store  │    │ Clients │    │  Cache  │  │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘  │
└─────────────────────────────────────────────────────────────┘
```

