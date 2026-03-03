# Sidebar Hierarchical Navigation Refactoring Plan

## Context

The current sidebar architecture uses a **two-panel approach** where:
- **ConversationSidebar** displays conversations grouped by date (Today, Yesterday, This Week, Older)
- **ThreadView** displays threads grouped by status (Open, Resolved) in a separate panel

The user wants to consolidate these into a **single hierarchical navigation model** where threads appear indented under their parent conversation, eliminating the need for a secondary "THREADS" header while preserving all existing behavior.

### Problem Statement
1. Two separate headers create visual duplication and cognitive overhead
2. Users must context-switch between two panels to navigate conversations and threads
3. Duplicated action controls (New Conversation vs New Thread) add complexity
4. The hierarchical relationship between conversations and threads is not visually clear

### Goal
Refactor the sidebar to show threads inline under their selected conversation with:
- Single unified header that adapts contextually
- Thread indentation with controlled reveal animation
- Preserved routing logic, data bindings, and selection behavior
- No disruptive layout shifts or scrollbar instability

---

## Implementation Strategy

### Phase 1: Unified Header Component

**Objective**: Create a contextual header that adapts based on selection state.

**Changes to `ConversationSidebar.svelte`**:
1. Modify header to show contextual actions:
   - No conversation selected → "New Conversation" button
   - Conversation selected → "New Thread" button
2. Deprecate the separate ThreadView header by:
   - Removing "THREADS" title and status text
   - Migrating NewThreadButton into ConversationSidebar header
3. Update status text to reflect combined state:
   - "Select a chat to begin" → No selection
   - "Active conversation" → Conversation selected (no thread)
   - "Active thread" → Thread selected

### Phase 2: Inline Thread Rendering

**Objective**: Render threads inline under selected conversation with indentation.

**Key Decision**: Render threads in `ConversationSidebar` AFTER the selected `ConversationItem`, NOT inside it. This:
- Maintains separation of concerns
- Simplifies state management (centralized in parent)
- Allows cleaner animation orchestration
- Follows existing pattern from `ThreadView.svelte`

**New component: `InlineThreadList.svelte`**:
1. Renders ThreadItem components with indentation
2. Groups threads by status (Open/Resolved) with subtle section labels
3. Uses Svelte 5 snippets for customization
4. Uses `variant="inline"` prop on ThreadItem for visual weight

**Changes to `ThreadItem.svelte`**:
1. Add `variant?: 'default' | 'inline'` prop
2. Apply reduced sizing/muted colors when `variant="inline"`
3. Preserve all existing behavior (selection, hover, actions, inline rename)

### Phase 3: Animation System

**Objective**: Implement controlled reveal animation for thread expansion using design system tokens.

**State Management** (Svelte 5 runes):
```typescript
// ConversationSidebar.svelte
// Auto-expand threads when conversation is selected (no manual toggle)
let showThreadsForId = $derived(selectedId);

// Thread grouping (matches existing ThreadView pattern)
let threadsForSelected = $derived(threadsForSelected ?? []);
let openThreads = $derived(threadsForSelected.filter(t => !t.metadata.resolved));
let resolvedThreads = $derived(threadsForSelected.filter(t => t.metadata.resolved));
```

**CSS Changes** (using design system tokens):
```css
/* Thread container animation */
.threadContainer {
  max-height: 0;
  opacity: 0;
  overflow: hidden;
  overflow-anchor: none; /* Prevent scroll jump during expansion */
  transition:
    max-height var(--transition-normal),  /* 250ms */
    opacity var(--transition-fast);       /* 150ms */
}

.threadContainer.expanded {
  max-height: 500px; /* Sufficient for ~6 threads */
  opacity: 1;
}

/* Thread item stagger - follow existing animation-delay pattern */
.threadItem {
  animation: slideIn var(--transition-normal) var(--ease-out) backwards;
}
```

**Stagger pattern** (follows existing codebase convention):
```svelte
{#each openThreads as thread, index (thread.id)}
  <ThreadItem
    {thread}
    variant="inline"
    style="animation-delay: {index * 50}ms"
    ...
  />
{/each}
```

**Animation constraints**:
- Duration: 200-250ms (matches design system `--transition-normal`)
- Easing: Use design system tokens (`--transition-normal`, `--ease-out`)
- Scroll stability: Use `overflow-anchor: none` on thread container

### Phase 4: ChatLayout Integration

**Objective**: Update layout to pass threads data to ConversationSidebar for inline rendering.

**Changes to `ChatLayout.svelte`**:
1. Add `threadsForSelected` prop to ConversationSidebar
2. Remove separate ThreadView panel from desktop layout
3. Handle thread selection through ConversationSidebar
4. Keep ThreadView as mobile-only overlay (responsive behavior)
5. Update responsive behavior for unified model

**Data flow change**:
```svelte
<!-- ChatLayout.svelte -->
<ConversationSidebar
  {conversations}
  selectedId={selectedConversation?.id}
  threadsForSelected={threads}  <!-- NEW: Pass threads for inline rendering -->
  selectedThreadId={selectedThread?.id}
  onSelectThread={handleSelectThread}
  onCreateThread={onCreateThread}
  onResolveThread={onResolveThread}
  ...
/>
```

### Phase 5: Visual Weight Normalization

**Objective**: Distinguish parent from child through visual weight via `variant` prop.

**ThreadItem `variant="inline"` adjustments**:
```css
/* ThreadItem.module.css */
.item[data-variant='inline'] {
  margin-left: var(--space-6); /* Indentation */
}

.item[data-variant='inline'] .button {
  height: 40px; /* Reduced from 44px */
  padding: var(--space-2) var(--space-3);
}

.item[data-variant='inline'] .icon svg {
  width: 14px;
  height: 14px;
}

.item[data-variant='inline'] .title {
  font-size: var(--text-sm); /* 14px vs 16px for conversation */
  color: var(--color-text-secondary); /* Muted */
}
```

**Typography scale**:
- Conversation item: `--text-base` (16px), `--font-medium`
- Thread item (inline): `--text-sm` (14px), `--font-normal`
- Section labels (Resolved): `--text-xs` (12px), `--font-medium`

---

## Files to Modify

### Primary Changes
| File | Changes |
|------|---------|
| `src-ui/src/lib/components/chat/sidebar/ConversationSidebar.svelte` | Unified header, inline thread rendering with `$derived`, pass thread handlers |
| `src-ui/src/lib/components/chat/sidebar/ConversationSidebar.module.css` | Thread container animation, overflow-anchor |
| `src-ui/src/lib/components/chat/sidebar/InlineThreadList.svelte` | NEW: Renders threads inline with grouping |
| `src-ui/src/lib/components/chat/thread/ThreadItem.svelte` | Add `variant="inline"` prop |
| `src-ui/src/lib/components/chat/thread/ThreadItem.module.css` | Inline variant styles |
| `src-ui/src/lib/components/layout/ChatLayout.svelte` | Pass `threadsForSelected` to sidebar, deprecate ThreadView panel |

### Secondary Changes
| File | Changes |
|------|---------|
| `src-ui/src/lib/components/chat/thread/ThreadView.svelte` | Mobile-only or deprecate |
| `src-ui/src/lib/components/layout/ChatLayout.module.css` | Layout adjustments for unified sidebar |

---

## Preservation Requirements

### Must Preserve
1. **ConversationItem behavior**: onclick, ondoubleclick, selection, hover, actions, inline rename
2. **ThreadItem behavior**: onclick, resolve/reopen, rename, delete, archive
3. **Routing logic**: URL-based navigation, deep linking
4. **Selection state**: Single selection model, visual feedback
5. **Data bindings**: Reactive updates to conversations and threads
6. **Accessibility**: ARIA roles, keyboard navigation, screen reader support

### Must Not Introduce
1. New header components that replicate hierarchy signals
2. Layout shift during thread expansion
3. Scrollbar instability during reveal animation
4. Disruptive reflow or jarring transitions
5. Breaking changes to props interfaces

---

## Accessibility Additions

1. **ARIA expansion state** on conversation item:
```svelte
aria-expanded={conv.id === selectedId && hasThreads}
aria-controls={hasThreads ? `threads-${conv.id}` : undefined}
```

2. **Keyboard navigation**: Threads navigable with arrow keys when expanded

3. **Focus management**: Focus remains stable during expansion/collapse

---

## Implementation Order

1. **ThreadItem.svelte** - Add `variant="inline"` prop with visual adjustments
2. **ThreadItem.module.css** - Add inline variant styles
3. **InlineThreadList.svelte** - Create new component for inline thread rendering
4. **ConversationSidebar.svelte** - Integrate InlineThreadList, unified header, pass thread handlers
5. **ConversationSidebar.module.css** - Thread container animation styles
6. **ChatLayout.svelte** - Pass `threadsForSelected` to sidebar, deprecate ThreadView panel
7. **ChatLayout.module.css** - Layout adjustments
8. **ThreadView.svelte** - Mobile-only fallback or deprecate

---

## Verification

### Manual Testing
1. Select conversation → threads appear inline with animation (auto-expand)
2. Select different conversation → previous threads collapse, new threads expand
3. Select thread → thread highlights, messages load in main area
4. Create new thread from contextual header → thread appears inline
5. Resolve thread → thread moves to "Resolved" section inline
6. Rename thread inline → preserves functionality
7. Mobile view → ThreadView overlay still works (if kept)

### Visual Verification
1. No layout shift during thread expansion
2. Scrollbar position stable during reveal (overflow-anchor working)
3. Animation timing consistent (200-250ms, using design tokens)
4. Typography hierarchy clear (conversation > thread)
5. Visual weight shifts appropriately (inline variant)
6. Stagger animation on thread items (50ms delay per item)

### Accessibility Testing
1. Keyboard navigation through threads with arrow keys
2. Screen reader announces thread expansion via aria-expanded
3. Focus management during state changes
4. Reduced motion preference respected

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Layout shift during expansion | Use `max-height` with generous value, `overflow-anchor: none` |
| Scroll position jumping | CSS overflow-anchor on content container |
| Breaking existing behavior | Keep ConversationItem/ThreadItem interfaces intact, add new props |
| Mobile experience degradation | Keep ThreadView as mobile overlay option |
| Animation jank | Use CSS transitions with design system tokens |
| Thread data not available | Pass `threadsForSelected` prop from ChatLayout |