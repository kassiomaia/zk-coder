# Search Feature - Product Requirements Document

## Problem Statement

Users cannot find past conversations or specific information within their chat history. As the number of conversations grows, this becomes increasingly frustrating and reduces the value of the knowledge accumulated in previous chats. Users waste time scrolling through long lists or re-asking questions they've already discussed with the AI.

## Success Metrics

- **Time to Find**: Users can locate a specific conversation in < 5 seconds
- **Search Adoption**: > 60% of active users use search at least once per session by week 4
- **Search Success Rate**: > 85% of searches result in the user finding what they were looking for
- **Keyboard Activation**: > 40% of searches initiated via keyboard shortcut (Cmd/Ctrl+K)

## User Stories

1. **As a developer**, I want to search for a specific code solution I discussed last week so that I can reference it without re-explaining the context.

2. **As a power user**, I want to search across all my conversations at once so that I don't have to remember which conversation contained specific information.

3. **As a new user**, I want to see search results with highlighted matches so that I can quickly identify which conversation is relevant.

4. **As a user with many conversations**, I want to filter search by date range so that I can narrow down results to a specific time period.

5. **As a keyboard-focused user**, I want to open search with a keyboard shortcut so that I don't have to reach for my mouse.

## Functional Requirements

### Core Search

- [ ] **Full-text search** across conversation titles
- [ ] **Full-text search** across all message content (user and assistant messages)
- [ ] **Case-insensitive** search by default
- [ ] **Real-time results** as user types (debounced at 300ms)
- [ ] **Result highlighting** showing matching text snippets

### Search UI

- [ ] **Search input** accessible from sidebar header
- [ ] **Keyboard shortcut** Cmd/Ctrl+K opens search with focus
- [ ] **Search results dropdown** showing top 10 matches
- [ ] **Result item** displays: conversation title, matching snippet, timestamp
- [ ] **Click on result** navigates to that conversation and scrolls to match
- [ ] **Escape key** closes search and clears input

### Advanced Filters (Phase 2)

- [ ] **Date range filter** - before/after specific dates
- [ ] **Thread state filter** - search only open or resolved threads
- [ ] **Provider filter** - search conversations by LLM provider used

## Non-Functional Requirements

### Performance

- **Search latency**: < 200ms for databases up to 10,000 messages
- **Index updates**: New messages searchable within 1 second of creation
- **Memory usage**: Search index should not exceed 50MB for 10,000 messages

### User Experience

- **Zero results state**: Clear messaging with suggestions (check spelling, try broader terms)
- **Loading state**: Show skeleton or spinner for searches taking > 100ms
- **Empty search**: Show recent conversations when search input is empty but focused

### Technical

- **Database**: Use SQLite FTS5 (Full-Text Search) extension
- **Index strategy**: Maintain real-time FTS index on messages table
- **Backwards compatible**: Build index on first run for existing databases

## Dependencies

### Internal

- `src-tauri/src/storage/chat_store.rs` - SQLite storage layer
- `src-tauri/src/commands/chat.rs` - Tauri command handlers
- `src-ui/src/lib/stores/chat.ts` - Frontend state management

### External

- SQLite FTS5 extension (included in standard SQLite builds)

## Technical Approach

### Backend Changes

```rust
// New migration: Create FTS5 virtual table
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    content='messages',
    content_rowid='id',
    tokenize='porter unicode61'
);

// New command: search_messages
#[tauri::command]
pub async fn search_messages(
    query: String,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<SearchResult>, String>;
```

### Frontend Changes

```
New components:
- src-ui/src/lib/components/search/SearchInput.svelte
- src-ui/src/lib/components/search/SearchResults.svelte
- src-ui/src/lib/components/search/SearchResultItem.svelte

Store additions:
- searchQuery: string
- searchResults: SearchResult[]
- isSearching: boolean
```

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| FTS5 performance on large datasets | Low | High | Benchmark with 100k messages; implement pagination |
| Index corruption | Low | Medium | Implement index rebuild command; detect and auto-rebuild |
| Search quality not meeting expectations | Medium | Medium | User testing before release; iterate on ranking |
| Keyboard shortcut conflicts | Low | Low | Allow shortcut customization in future |

## Out of Scope

- Semantic/vector search (future enhancement)
- Search within file attachments
- Global search across multiple workspaces
- Search history

## Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Backend implementation | 3 days | FTS index, search command |
| Frontend UI | 2 days | Search components, integration |
| Keyboard shortcuts | 1 day | Cmd/Ctrl+K, escape handling |
| Testing & polish | 2 days | Edge cases, performance testing |
| **Total** | **8 days** | **Feature complete** |

## Open Questions

1. Should search include archived conversations by default?
2. What is the minimum query length before search executes?
3. Should we implement "search as you type" or require Enter key?

## Acceptance Criteria

- [ ] User can search conversations by title
- [ ] User can search messages by content
- [ ] Results show highlighted matching text
- [ ] Clicking result navigates to conversation
- [ ] Cmd/Ctrl+K opens search
- [ ] Escape closes search
- [ ] Search works offline (local SQLite)
- [ ] Performance meets < 200ms requirement
- [ ] Works with existing databases (migration tested)

---

**Document Status**: Draft
**Author**: Product Team
**Created**: 2026-03-02
**Last Updated**: 2026-03-02
