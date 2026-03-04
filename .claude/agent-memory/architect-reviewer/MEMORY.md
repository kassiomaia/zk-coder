# Architect Memory: Coder Project

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

## Key Type Locations

- Chat types: `nlp/chat.rs` (should move to dedicated chat module)
- Config types: `nlp/config.rs` (Provider, Model, ApiType)
- Context types: `nlp/context.rs` (ContextWindow, ContextMessage)
- Intent types: `nlp/intent.rs` (Intent enum, IntentClassification)

## Store Architecture

- `ChatState` - Manages conversations, threads, messages (`coder_chat.db`)
- `ConfigState` - Manages providers, models, settings (`coder_config.db`)
- `NlpState` - Manages intent classification, context (`coder_context.db`)
- `LlmState` - Manages LLM interactions (`coder_config.db` shared)
