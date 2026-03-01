# Backend (NLP) Implementation Progress

**Last Updated**: 2026-02-28
**Status**: Phase 1 In Progress - Foundation Setup

---

## Overview

This document tracks the backend NLP implementation for the Coder AI desktop application. The implementation follows the plan defined in `docs/nlp-implementation-plan.md` with detailed code in `docs/nlp-backend-implementation.md`.

## Synchronization with Frontend

The frontend team is implementing the Chat Interface components in parallel. See `docs/progress.md` for frontend status.

**Integration Points**:
- Frontend Phase 7 (Backend Integration) depends on our Tauri IPC commands
- IntentIndicator component needs `classify_intent` command
- ContextIndicator component needs context management commands
- ClarificationPrompt component needs `detect_ambiguities` command

---

## Current State

### Phase Progress

### Phase 1: Foundation
**Status**: 🔄 IN PROGRESS
**Task ID**: #B1

| Subtask | Status | Notes |
|---------|--------|-------|
| Create progress document | ✅ Done | This document |
| Update Cargo.toml with dependencies | ⏳ Pending | regex, chrono, rusqlite, reqwest, tree-sitter |
| Create nlp/mod.rs | ⏳ Pending | Module exports |
| Create nlp/intent.rs | ⏳ Pending | Intent types & taxonomy |
| Create nlp/patterns.rs | ⏳ Pending | Regex pattern matching |
| Create nlp/context.rs | ⏳ Pending | Context data models |
| Create storage/mod.rs | ⏳ Pending | Storage module exports |
| Create storage/context_store.rs | ⏳ Pending | SQLite persistence |
| Create ai/mod.rs | ⏳ Pending | AI module exports |
| Create ai/claude.rs | ⏳ Pending | Claude API client |

---

### Phase 2: Intent Recognition
**Status**: NOT STARTED
**Task ID**: #B2
**Blocked By**: Phase 1

| Subtask | Status | Notes |
|---------|--------|-------|
| nlp/classifier.rs | Pending | Intent classification engine |
| Pattern matching integration | Pending | Fast-path classification |
| Claude API integration | Pending | Slow-path classification |
| Intent params extraction | Pending | Parameter extraction |

---

### Phase 3: Context Preservation
**Status**: NOT STARTED
**Task ID**: #B3
**Blocked By**: Phase 1

| Subtask | Status | Notes |
|---------|--------|-------|
| nlp/context_manager.rs | Pending | Context lifecycle management |
| Context compression | Pending | AI summarization |
| Token estimation | Pending | Token counting |
| SQLite integration | Pending | Persistence |

---

### Phase 4: Multi-Intent Detection
**Status**: NOT STARTED
**Task ID**: #B4
**Blocked By**: Phase 2

| Subtask | Status | Notes |
|---------|--------|-------|
| nlp/multi_intent.rs | Pending | Multi-intent types |
| nlp/multi_detector.rs | Pending | Detection engine |
| nlp/execution_planner.rs | Pending | Execution planning |

---

### Phase 5: Ambiguity Resolution
**Status**: NOT STARTED
**Task ID**: #B5
**Blocked By**: Phase 2, Phase 3

| Subtask | Status | Notes |
|---------|--------|-------|
| nlp/ambiguity.rs | Pending | Ambiguity types |
| nlp/ambiguity_resolver.rs | Pending | Resolution engine |
| Context-aware resolution | Pending | Use conversation context |

---

### Phase 6: Code Understanding
**Status**: NOT STARTED
**Task ID**: #B6
**Blocked By**: Phase 1

| Subtask | Status | Notes |
|---------|--------|-------|
| nlp/code_understanding.rs | Pending | Code types |
| nlp/code_analyzer.rs | Pending | tree-sitter integration |
| nlp/nl_to_code.rs | Pending | NL → code mapping |

---

### Phase 7: Tauri IPC Commands
**Status**: NOT STARTED
**Task ID**: #B7
**Blocked By**: Phase 2-6

| Subtask | Status | Notes |
|---------|--------|-------|
| commands/nlp.rs | Pending | IPC command definitions |
| Register commands | Pending | Tauri registry |
| AppState setup | Pending | Shared state |
| Frontend integration test | Pending | End-to-end test |

---

## File Structure (Target)

```
src-tauri/src/
├── nlp/
│   ├── mod.rs                    # Module exports
│   ├── intent.rs                 # Intent types & taxonomy
│   ├── classifier.rs             # Intent classification engine
│   ├── patterns.rs               # Regex pattern matching
│   ├── context.rs                # Context data models
│   ├── context_manager.rs        # Context lifecycle management
│   ├── multi_intent.rs           # Multi-intent types
│   ├── multi_detector.rs         # Multi-intent detection
│   ├── execution_planner.rs      # Intent execution planning
│   ├── ambiguity.rs              # Ambiguity types
│   ├── ambiguity_resolver.rs     # Ambiguity resolution
│   ├── code_understanding.rs     # Code analysis types
│   ├── code_analyzer.rs          # tree-sitter code analysis
│   └── nl_to_code.rs             # Natural language → code mapping
├── storage/
│   ├── mod.rs
│   └── context_store.rs          # SQLite persistence
├── ai/
│   ├── mod.rs
│   └── claude.rs                 # Claude API client
└── commands/
    ├── mod.rs                    # Existing + nlp exports
    └── nlp.rs                    # NLP IPC commands
```

---

## Dependencies

### Required Cargo Additions

```toml
# NLP
regex = "1"

# Date/time
chrono = { version = "0.4", features = ["serde"] }

# Database
rusqlite = { version = "0.31", features = ["bundled"] }

# HTTP client
reqwest = { version = "0.12", features = ["json"] }

# Code parsing (Phase 6)
tree-sitter = "0.22"
tree-sitter-rust = "0.21"
tree-sitter-typescript = "0.21"
tree-sitter-python = "0.21"
```

---

## Tauri IPC Commands to Implement

| Command | Phase | Frontend Consumer |
|---------|-------|-------------------|
| `classify_intent` | B7 | IntentIndicator.svelte |
| `get_conversation_context` | B7 | ContextIndicator.svelte |
| `add_context_message` | B7 | ChatArea.svelte |
| `detect_multi_intent` | B7 | MultiIntentIndicator.svelte |
| `detect_ambiguities` | B7 | ClarificationPrompt.svelte |
| `analyze_code` | B7 | CodeContextIndicator.svelte |

---

## Quality Checklist

Each module must meet these criteria:

- [ ] Uses `thiserror` for error types
- [ ] Has documentation comments on public items
- [ ] Follows Rust naming conventions
- [ ] Uses `tokio` for async operations
- [ ] Proper error propagation with `Result`
- [ ] No `unwrap()` or `expect()` in production code
- [ ] Unit tests for core functionality
- [ ] File under 500 lines (soft limit)

---

## Notes

- Frontend team has completed Phase 5 (Conversation History)
- Frontend Phase 6 (Responsive Layout) is pending
- Our Phase 7 must be ready before Frontend Phase 7 begins
- Coordinate with frontend team on TypeScript type definitions in `src-ui/src/lib/types/nlp.ts`

---

## Related Documents

- [NLP Implementation Plan](./nlp-implementation-plan.md)
- [NLP Backend Implementation (Detailed)](./nlp-backend-implementation.md)
- [Chat Interface Implementation Plan](./chat-interface-implementation-plan.md)
- [Frontend Progress](./progress.md)
