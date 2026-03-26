# Architect Memory: Coder Project

## Architecture Overview

Coder is a Rust + Tauri + Svelte desktop application implementing a **layered architecture** with clear separation between domain, application, infrastructure, and presentation layers.

## Layer Structure

### Core Layer (`src-tauri/src/core/`)
**Purpose**: Domain entities, value objects, repository interfaces, service interfaces
- **Zero dependencies** on infrastructure or external services
- Pure data types with serde serialization
- Repository trait definitions (no implementations)
- Service trait definitions
- `chat/` - ChatMessage, Conversation, Thread, MessageRole, etc.
- `agent/` - AgentSession, AgentTask, PendingAction
- `llm/` - LlmService interface

### Application Layer (`src-tauri/src/application/`)
**Purpose**: Service implementations coordinating business logic
- `agent/service.rs` - AgentServiceImpl (1652 LOC - **EXCEEDS SOFT LIMIT**)
- `chat/service.rs` - ChatServiceImpl (1054 LOC - **EXCEEDS SOFT LIMIT**)
- `llm/service.rs` - LlmServiceImpl (850 LOC)
- Implements core service traits
- Enforces business rules (e.g., ThreadResolved validation)
- Bridges between core entities and storage

### Infrastructure Layer
- **Storage** (`src-tauri/src/storage/`):
  - `chat_store/` - SqliteChatStore with conversations, threads, messages
  - `config_store/` - Provider/model configuration
  - `agents/store/` - Session persistence
- **Commands** (`src-tauri/src/commands/`):
  - Thin adapter layer over services
  - Tauri command handlers with state management
- **LLM** (`src-tauri/src/llm/`):
  - Provider-agnostic client trait
  - OpenAI, Anthropic implementations
  - Streaming with Tauri events
- **Agents** (`src-tauri/src/agents/`):
  - Runtime for tool execution with permissions
  - Approval queue, rollback store
  - Session management

### Presentation Layer (`src-ui/`)
- Svelte components with CSS modules
- Svelte stores for state management
- API layer with typed wrappers for Tauri IPC

## Key Patterns (Preserve These)

### 1. Repository Pattern
Storage operations abstracted behind traits in `core/`. Allows:
- Easy testing with mock implementations
- Multiple storage backends
- Clear separation of persistence logic

### 2. Service Layer Pattern
Application services (`application/*`) implement core service traits:
- Coordinate between repositories
- Enforce business rules
- Handle transaction boundaries

### 3. Event-Driven Agent Updates
Agent state flows through Tauri events:
- `agent:event` carries state changes
- Frontend store processes events reactively
- No polling required

### 4. Modular Store Structure
Frontend stores organized into subdirectories:
- `stores/chat/` - Main chat store with llm-handler, derived-stores
- `stores/agent/` - Modular agent store (actions, events, streaming, callbacks)
- Re-exports from index files for clean imports

### 5. Design System
Centralized design tokens in `design-system.css`:
- Color palettes (primary, neutral, semantic, provider, syntax)
- Typography scale with Inter/JetBrains Mono
- 4px spacing base unit
- All components reference variables

## Architectural Debt & Issues

### High Priority

#### 1. File Size Violations (Rust Backend)
**Files exceeding 500 LOC soft limit**:
- `agents/runtime/execution.rs` - 1801 LOC
- `application/agent/service.rs` - 1652 LOC
- `commands/config.rs` - 1395 LOC
- `commands/llm/orchestrator.rs` - 1211 LOC
- `agents/runtime/permissions.rs` - 1128 LOC
- `commands/chat.rs` - 1114 LOC
- `application/chat/service.rs` - 1054 LOC

**Recommendation**: Extract submodules following recent patterns:
```
service.rs (main) →
  - session_operations.rs
  - execution_operations.rs
  - approval_operations.rs
  - conversion.rs (agent types <-> core types)
```

#### 2. NLP Module Type Confusion
`nlp/chat.rs` and `nlp/config.rs` contain non-NLP types:
- `MessageRole` duplicate (exists in core/chat/entities.rs)
- `Provider`, `Model` (config types, not NLP)

**Recommendation**: Create dedicated modules:
- Move `chat.rs` -> `core/chat/messaging.rs` or `infrastructure/messaging.rs`
- Move `config.rs` types -> `core/config/` or `infrastructure/config/`

#### 3. Dual State Management in Chat Commands
`commands/chat.rs::ChatState` contains both:
- `service: Arc<dyn ChatService>` (NEW - layered)
- `store: Arc<Mutex<SqliteChatStore>>` (OLD - direct access)

**Impact**: Migration incomplete, creates two code paths

**Recommendation**: Complete migration to service-only access, remove `store` field

### Medium Priority

#### 4. Agent Service Conversion Boilerplate
`application/agent/service.rs` has many conversion functions:
- `to_core_agent()`, `to_core_session()`, `to_core_pending_action()`
- `subtask_to_core_task()`, task status conversions
- SessionId, AgentId type conversions

**Recommendation**: Consider:
- Shared newtype pattern with `Into` implementations
- Or consolidate into a dedicated `conversion.rs` module

#### 5. Test File Sizes
Test files also exceeding limits:
- `application/agent/service_tests.rs` - 1116 LOC
- `application/chat/service_tests.rs` - 1054 LOC

**Recommendation**: Extract test utilities into `common.rs` or `fixtures.rs`

### Low Priority

#### 6. Component Size Issues (Frontend)
- `AgentPanel.svelte` - 707 LOC (over 500 limit)
- `ChatInput.svelte` - 515 LOC (over 500 limit)

**Recommendation**: Extract subcomponents following modular pattern

## Recent Architectural Evolution (Positive!)

The git history shows **intentional refactoring** toward better modularity:

1. **Layered Architecture** (ad8d84a) - Introduced service layer
2. **Module Extraction** - Breaking large files into subdirectories:
   - `llm/anthropic/mod.rs` -> `client.rs`, `stream.rs`, `types.rs`
   - `llm/openai/mod.rs` -> `client.rs`, `types.rs`
   - `agents/runtime/mod.rs` -> `client/`, `context/`, `execution/`, `permissions/`
   - `storage/chat_store/mod.rs` -> `conversations/`, `threads/`, `messages/`
   - `storage/config_store/mod.rs` -> `providers/`, `models/`, `api_keys/`
   - `agents/store/mod.rs` -> `sessions/`, `tasks/`, `runtime_data/`

This **pattern should continue** for the remaining large files.

## Database Architecture

### Schema Design (Excellent)
- **chat_store**: conversations -> threads -> messages (hierarchical)
- **config_store**: providers, models, operation defaults, api_keys
- **agents/store**: sessions, tasks, subtasks, files_touched, terminal_output, tool_calls, execution_steps

### Foreign Key Relationships (Good)
- Threads cascade delete to messages
- Subtasks cascade delete with tasks
- Runtime data (files, terminal, status_phrases) cascade with sessions

### Migration Strategy (Good)
- Version-agnostic migrations in each store
- `ALTER TABLE` with ignore-error pattern for idempotency

## NLP Module Structure

**Location**: `/home/kassiomaia/devel/zk/coder/src-tauri/src/nlp/`

**Key Components**:
- `intent.rs` - 26 developer intent types (GenerateCode, FindBugs, etc.)
- `patterns.rs` - Regex-based fast-path classification (0.95 confidence)
- `classifier.rs` - Hybrid pattern + AI intent classification
- `context_manager.rs` - Conversation context with 50K token compression
- `code_analyzer.rs` - Tree-sitter based analysis (Rust, TS, JS, Python)
- `execution_planner.rs` - Multi-intent execution ordering

**Integration Points**:
- Tauri commands in `commands/nlp.rs` expose 8 NLP operations
- Frontend API at `src-ui/src/lib/api/nlp.ts`
- Frontend types at `src-ui/src/lib/types/nlp.ts`
- Currently only intent classification is actively used in ChatInput.svelte

**Architectural Concerns**:
- `nlp/chat.rs` and `nlp/config.rs` contain non-NLP types that should be relocated
- Dual MessageRole definitions exist in `context.rs` and `chat.rs`
- AI client uses hardcoded "dev-api-key" in development mode

## Store Architecture

- `ChatState` - Manages conversations, threads, messages (`coder_chat.db`)
- `ConfigState` - Manages providers, models, settings (`coder_config.db`)
- `NlpState` - Manages intent classification, context (`coder_context.db`)
- `LlmState` - Manages LLM interactions (`coder_config.db` shared)
