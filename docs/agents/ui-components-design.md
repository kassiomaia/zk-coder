# UI Components Design

> **Parent**: [Implementation Plan](./IMPLEMENTATION-PLAN.md)
> **Focus**: Visual design specifications for agent UI components

---

## Design Principles

Based on existing `design-system.css`:

1. **Low contrast, subtle hierarchy** — Use `--color-text-secondary` and `--color-text-muted` for less important content
2. **Minimal visual noise** — Avoid borders; use subtle backgrounds instead
3. **Dark theme first** — Design for dark mode, ensure light mode works
4. **Accessible interactions** — Clear focus states, sufficient touch targets
5. **Consistent spacing** — 4px base unit (`--space-1`)

---

## Layout Integration

### Current Layout

```
┌──────────────┬──────────────────────────────────────┐
│   Sidebar    │            Chat Area                 │
│   280px      │           flex-grow                  │
│              │                                      │
│ [Conv list]  │  [Messages]                         │
│ [Threads]    │  [Input area]                        │
│              │                                      │
└──────────────┴──────────────────────────────────────┘
```

### New Layout with Agent Panel

```
┌──────────────┬────────────────────────┬──────────────┐
│   Sidebar    │       Chat Area        │  Agent Panel │
│   280px      │       flex-grow        │    320px     │
│              │                        │              │
│ [Conv list]  │  [Messages]            │  [Status]    │
│ [Threads]    │  [Input area]          │  [Tasks]     │
│              │                        │  [Files]     │
│              │                        │  [Terminal]  │
│              │                        │              │
└──────────────┴────────────────────────┴──────────────┘
```

### Responsive Behavior

| Breakpoint | Sidebar | Chat | Agent Panel |
|------------|---------|------|-------------|
| ≥1280px | 280px | flex | 320px |
| 1024-1279px | 280px | flex | 280px |
| 768-1023px | Collapsed | flex | Hidden (toggle) |
| <768px | Overlay | flex | Overlay |

---

## Component Specifications

### 1. Agent Panel

**Dimensions**:
- Default width: `320px` (`--agent-panel-width`)
- Min width: `240px`
- Max width: `400px`

**Structure**:
```
┌─────────────────────────────┐
│  Header: Agent Name    [×]  │  48px
├─────────────────────────────┤
│  Status: ● Running Step 2/6 │  32px
├─────────────────────────────┤
│  ▼ TASKS           [badge]  │  32px (collapsible)
│    ○ Read config file       │
│    ◐ Modify settings        │
│    ○ Write changes          │
├─────────────────────────────┤
│  ▼ FILES           [badge]  │  32px (collapsible)
│    📁 src/                  │
│      📝 config.ts      [M]  │
│      📝 types.ts            │
├─────────────────────────────┤
│  ▼ TERMINAL                 │  flex-grow
│    $ npm run build          │
│    ✓ Built in 2.3s          │
│                             │
└─────────────────────────────┘
```

**Colors** (dark theme):

| Element | Variable | Value |
|---------|----------|-------|
| Background | `--color-bg-secondary` | `#141417` |
| Header BG | `--color-surface` | `#141417` |
| Section header | `--color-surface-hover` | `#18181b` |
| Border | `--color-border-primary` | `rgba(255,255,255,0.06)` |
| Text primary | `--color-text-primary` | `#e4e4e7` |
| Text secondary | `--color-text-secondary` | `#a1a1aa` |
| Text muted | `--color-text-muted` | `#71717a` |

### 2. Task List

**Item structure**:
```
┌─────────────────────────────────┐
│ [icon] Task title               │
│         Optional description    │
└─────────────────────────────────┘
```

**Status indicators**:

| Status | Icon | Color | Animation |
|--------|------|-------|-----------|
| Pending | `○` Circle | `--color-text-muted` | None |
| In Progress | `◐` Loader2 | `--color-primary-400` | Spin |
| Completed | `●` CheckCircle | `--color-success` | None |
| Failed | `●` XCircle | `--color-error` | None |

**Typography**:
- Title: `--text-sm` (14px), `--color-text-primary`
- Description: `--text-xs` (12px), `--color-text-muted`
- Single line truncate with ellipsis

**Interaction**:
- Hover: `--color-interactive-hover` background
- Selected: `--color-interactive-selected` background
- Click: Navigate to related message or file

### 3. File Tree

**Item structure**:
```
Directory (collapsed):
┌─────────────────────────────────┐
│ ▶ 📁 src/                       │
└─────────────────────────────────┘

Directory (expanded):
┌─────────────────────────────────┐
│ ▼ 📂 src/                       │
│     📄 config.ts          [M]   │
│     📄 types.ts                 │
│     📁 utils/                   │
└─────────────────────────────────┘
```

**Indentation**:
- Root level: `--space-3` (12px)
- Each depth: `+ 12px`

**File modification indicators**:

| Indicator | Meaning | Color |
|-----------|---------|-------|
| M | Modified | `--color-warning` |
| A | Added | `--color-success` |
| D | Deleted | `--color-error` |

**Icons** (lucide-svelte):
- Directory collapsed: `Folder`
- Directory expanded: `FolderOpen`
- Code file: `FileCode`
- Text file: `FileText`
- Generic file: `File`

### 4. Terminal View

**Layout**:
```
┌─────────────────────────────────┐
│ 12:34:56  $ npm run build       │  command
│ 12:34:57  Building...           │  stdout
│ 12:34:59  ✓ Compiled in 2.3s    │  stdout
│ 12:35:00  Error: Out of memory  │  stderr
│           at line 42...          │  stderr
└─────────────────────────────────┘
```

**Colors**:

| Line Type | Text Color | Background |
|-----------|------------|------------|
| command | `--color-primary-400` | transparent |
| stdout | `--color-text-primary` | transparent |
| stderr | `--color-error` | transparent |
| info | `--color-text-muted` | transparent |

**Typography**:
- Font: `--font-mono` (JetBrains Mono)
- Size: `--text-xs` (12px)
- Line height: `--leading-relaxed` (1.625)

**Timestamp**:
- Format: `HH:mm:ss` (24h)
- Color: `--color-text-muted` at 60% opacity

### 5. Approval Dialog

**Structure**:
```
┌─────────────────────────────────────┐
│  ⚠️  Allow Write File?              │
│      Tool: fs_write_file            │
├─────────────────────────────────────┤
│                                     │
│  Write 1.2KB to src/config.ts       │
│                                     │
│  ┌─────────────────────────────┐    │
│  │ Preview                     │    │
│  ├─────────────────────────────┤    │
│  │ - const old = 'value';      │    │
│  │ + const new = 'updated';    │    │
│  └─────────────────────────────┘    │
│                                     │
├─────────────────────────────────────┤
│              [Deny]  [Allow]         │
└─────────────────────────────────────┘
```

**Dimensions**:
- Min width: `400px`
- Max width: `600px`

**Action type icons**:

| Action | Icon | Background Color |
|--------|------|------------------|
| Write File | `FileEdit` | `--color-warning-light` |
| Delete File | `Trash2` | `--color-error-light` |
| Execute Command | `Terminal` | `--color-info-light` |
| Web Request | `Globe` | `--color-surface-hover` |

**Preview section**:
- Max height: `200px`
- Overflow: scroll
- Background: `--color-neutral-950` (#09090b)
- Border: `--color-border-primary`

**Diff highlighting**:
- Added lines (`+`): `--color-success-subtle` background
- Removed lines (`-`): `--color-error-light` background

---

## Interaction Patterns

### Collapsible Sections

```
Initial state: All expanded except terminal

Click header → Toggle expand/collapse
  - Chevron rotates 90deg
  - Content slides up/down (200ms ease)
```

**Animation**:
```css
.section-content {
  overflow: hidden;
  transition: max-height 200ms ease-out;
}
.section-content.collapsed {
  max-height: 0;
}
```

### Approval Flow

```
1. Agent emits approval_required event
2. Dialog appears immediately (no delay)
3. User clicks Allow or Deny
4. Button shows loading state
5. Dialog closes on success
6. Agent continues execution
```

**Timeout behavior**:
- 5 minute inactivity timeout (backend auto-denies)
- Visual countdown after 4 minutes
- Toast notification when action times out
- User can re-trigger the action from the task list

### File Tree Expansion

```
Click directory → Expand inline
  - Load children if not loaded
  - Animate height change
  - Persist expand state for session
```

### Terminal Auto-scroll

```
New output arrives → Scroll to bottom
  - If user scrolled up → Show "new output" badge
  - Click badge → Scroll to bottom
```

---

## Motion Tokens

Use existing design system tokens:

| Token | Value | Use |
|-------|-------|-----|
| `--transition-fast` | 150ms | Hover states |
| `--transition-normal` | 250ms | Expand/collapse |
| `--transition-slow` | 350ms | Panel open/close |
| `--ease-out` | cubic-bezier(0, 0, 0.2, 1) | Most animations |
| `--ease-in-out` | cubic-bezier(0.4, 0, 0.2, 1) | Bidirectional |

**Spinner rotation**:
```css
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.loading-icon {
  animation: spin 1s linear infinite;
}
```

**Status pulse**:
```css
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
.status-running {
  animation: pulse 1.5s ease-in-out infinite;
}
```

---

## Accessibility

### Keyboard Navigation

| Key | Action |
|-----|--------|
| `Tab` | Move focus between items |
| `Enter` | Expand/collapse, select |
| `Escape` | Close dialog, collapse panel |
| `Arrow Up/Down` | Navigate list items |

### ARIA Labels

```html
<nav aria-label="Agent panel">
  <button aria-expanded="true" aria-controls="tasks-section">
    Tasks
  </button>
  <ul role="list" aria-label="Task list">
    <li role="listitem" aria-label="Read config file - completed">
      ...
    </li>
  </ul>
</nav>
```

### Focus Management

- Dialog opens → Focus on Allow button
- Dialog closes → Return focus to trigger
- Panel opens → Focus on panel header
- Panel closes → Return focus to chat input

### Color Contrast

All text must meet WCAG AA:
- Normal text: 4.5:1 minimum
- Large text: 3:1 minimum
- Use `--color-text-primary` on `--color-bg-primary`

---

## CSS Module Structure

```
src-ui/src/lib/components/agent/
├── AgentPanel.module.css
├── TaskList.module.css
├── FileTree.module.css
├── TerminalView.module.css
└── ApprovalDialog.module.css
```

**Example**: `AgentPanel.module.css`

```css
.panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--color-bg-secondary);
  border-left: 1px solid var(--color-border-primary);
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--color-border-primary);
}

.title {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--color-text-primary);
}

.closeButton {
  padding: var(--space-1);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  transition: background var(--transition-fast), color var(--transition-fast);
}

.closeButton:hover {
  background: var(--color-interactive-hover);
  color: var(--color-text-primary);
}

.status {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-4);
  background: var(--color-surface);
  font-size: var(--text-xs);
  color: var(--color-text-secondary);
}

.status.waiting {
  background: var(--color-warning-light);
  color: var(--color-warning-dark);
}

.statusDot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-text-muted);
}

.statusDot.running {
  background: var(--color-primary-400);
  animation: pulse 1.5s ease-in-out infinite;
}

.section {
  border-bottom: 1px solid var(--color-border-subtle);
}

.sectionHeader {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-2) var(--space-4);
  font-size: var(--text-xs);
  font-weight: var(--font-medium);
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: var(--tracking-wide);
  transition: background var(--transition-fast);
}

.sectionHeader:hover {
  background: var(--color-interactive-hover);
}

.badge {
  margin-left: auto;
  padding: 0 var(--space-1-5);
  background: var(--color-neutral-700);
  border-radius: var(--radius-full);
  font-size: var(--text-2xs);
  color: var(--color-text-muted);
}
```

---

## Light Mode Adaptations

All components should work in light mode using existing theme variables:

| Element | Dark | Light |
|---------|------|-------|
| Panel background | `--color-bg-secondary` | Same (auto-switches) |
| Text primary | `--color-text-primary` | Same |
| Border | `rgba(255,255,255,0.06)` | `--color-border-primary` |
| Interactive hover | `rgba(255,255,255,0.03)` | `rgba(0,0,0,0.03)` |

**Test checklist**:
- [ ] All text readable in light mode
- [ ] Status indicators visible
- [ ] Terminal output readable
- [ ] Diff highlights work
- [ ] Focus states visible

---

## Icons Reference

Using `lucide-svelte` (already in dependencies):

| Icon | Import | Use |
|------|--------|-----|
| Folder | `Folder` | Collapsed directory |
| FolderOpen | `FolderOpen` | Expanded directory |
| File | `File` | Generic file |
| FileCode | `FileCode` | Code file |
| FileText | `FileText` | Text file |
| Circle | `Circle` | Pending task |
| CheckCircle | `CheckCircle` | Completed task |
| XCircle | `XCircle` | Failed task |
| Loader2 | `Loader2` | In progress (spinning) |
| ChevronDown | `ChevronDown` | Expanded indicator |
| ChevronRight | `ChevronRight` | Collapsed indicator |
| Terminal | `Terminal` | Terminal section |
| AlertTriangle | `AlertTriangle` | Warning |
| FileEdit | `FileEdit` | Write action |
| Trash2 | `Trash2` | Delete action |
| Globe | `Globe` | Web request |
