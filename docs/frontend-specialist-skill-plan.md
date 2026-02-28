# Frontend Specialist Skill Plan (Final)

## Overview

This document defines the complete **frontend-specialist** skill for the Coder project, synthesizing:
- Existing `design-system.css` implementation
- AI Code Agent App Design Specification
- Claude Code Web Interface Analysis

**Core Principle**: Design tokens from `design-system.css` are the source of truth. All component specifications reference these tokens.

---

## Skill Structure

```
.claude/skills/frontend-specialist/
├── SKILL.md                              # Main entry point
│
├── styleguide/
│   ├── design-tokens.md                  # Complete token reference
│   ├── color-system.md                   # Color palette and usage
│   ├── typography.md                     # Font scale and usage
│   ├── theming.md                        # Dark/light theme guidelines
│   ├── spacing-layout.md                 # Spacing and layout tokens
│   ├── accessibility.md                  # WCAG compliance guide
│   └── animations.md                     # Motion and transitions
│
├── layout/
│   ├── app-structure.md                  # Application shell architecture
│   ├── sidebar-modes.md                  # Collapsed/expanded sidebar
│   ├── responsive-design.md              # Breakpoints and adaptations
│   └── grid-system.md                    # Grid and flex patterns
│
├── components/
│   ├── README.md                         # How to use component specs
│   ├── sidebar.md                        # Navigation sidebar
│   ├── header.md                         # Top header bar
│   ├── greeting-section.md               # Personalized welcome
│   ├── suggestion-cards.md               # Quick action cards
│   ├── input-box.md                      # Message input component
│   ├── theme-toggle.md                   # Theme switch button
│   ├── chat-message.md                   # Message bubbles
│   ├── code-block.md                     # Syntax-highlighted code
│   ├── file-attachment.md                # File previews
│   └── buttons.md                        # Button variants
│
├── state/
│   ├── app-state.md                      # Application state structure
│   ├── chat-state.md                     # Chat/conversation state
│   └── preferences.md                    # User preferences
│
├── references/
│   ├── README.md                         # How to add HTML references
│   └── (html-reference-files)            # Reference implementations
│
└── examples/
    ├── component-template.svelte         # Standard component template
    ├── component-template.module.css     # Standard styles template
    ├── page-template.svelte              # Page layout template
    └── store-template.ts                 # State store template
```

---

## Core Design Decisions

### 1. Color System

**Decision**: Use indigo/violet palette (matches Claude Code analysis recommendation)

```
Primary:     #6366f1 (indigo-500)
Accent:      #8b5cf6 (violet-500) for system messages
Success:     #10b981 (emerald-500)
Error:       #ef4444 (standard red)
Warning:     #f59e0b (amber-500)
```

### 2. Typography

**Decision**: Inter + JetBrains Mono (optimal for coding environments)

```
Sans-serif:  Inter (primary UI font)
Monospace:   JetBrains Mono (code)
Sizes:       10px, 12px, 14px, 16px, 18px, 20px, 24px, 28px
```

### 3. Sidebar

**Decision**: Collapsible with smooth transitions

```
Collapsed:   70px (icons only)
Expanded:    280px (with labels)
```

### 4. Responsive Breakpoints

```
Mobile:      < 600px
Tablet:      600px - 1024px
Desktop:     > 1024px
```

---

## File Specifications

### SKILL.md (Main Entry Point)

```markdown
---
name: frontend-specialist
description: Expert frontend developer for Coder AI Code Agent. Implements components following the design system, ensures accessibility compliance, and creates consistent, high-quality UI code. Use when creating or modifying any UI components, pages, or layouts.
---

# Frontend Specialist Skill

Expert guide for implementing the Coder AI Code Agent interface.

## Core Principles

1. **Design Tokens First**: Always use CSS variables, never hardcoded values
2. **Theme-Aware**: Use semantic tokens (--color-text-primary) not palette tokens
3. **Accessibility Built-In**: WCAG AA compliance required
4. **Type-Safe**: All components with TypeScript interfaces
5. **Svelte 5 Runes**: Use $state, $derived, $effect patterns

## Quick Reference

### Colors
\`\`\`css
background: var(--color-surface);
color: var(--color-text-primary);
border-color: var(--color-border-primary);
\`\`\`

### Spacing
\`\`\`css
padding: var(--space-4);    /* 16px */
gap: var(--space-3);        /* 12px */
margin: var(--space-6);     /* 24px */
\`\`\`

### Typography
\`\`\`css
font-size: var(--text-sm);           /* 14px */
font-weight: var(--font-medium);     /* 500 */
line-height: var(--leading-normal);  /* 1.5 */
\`\`\`

## Sections

- [Styleguide](./styleguide/): Design tokens and usage
- [Layout](./layout/): Application structure
- [Components](./components/): Component specifications
- [State](./state/): State management patterns
- [References](./references/): HTML reference files
- [Examples](./examples/): Templates and examples

## Component Checklist

Before creating any component:

- [ ] Define TypeScript Props interface
- [ ] Use CSS module with design tokens only
- [ ] Include accessibility attributes (ARIA, keyboard)
- [ ] Test in both themes (dark/light)
- [ ] Test responsive breakpoints
- [ ] Include focus-visible states
```

---

### styleguide/design-tokens.md

```markdown
# Design Tokens Reference

Complete reference of all CSS custom properties defined in `src-ui/src/lib/styles/design-system.css`.

## Color Palette

### Primary (Indigo)
| Token | Value | Usage |
|-------|-------|-------|
| --color-primary-50 | #eef2ff | Light backgrounds |
| --color-primary-100 | #e0e7ff | Subtle backgrounds |
| --color-primary-200 | #c7d2fe | Borders, dividers |
| --color-primary-300 | #a5b4fc | Disabled accents |
| --color-primary-400 | #818cf8 | Hover states |
| --color-primary-500 | #6366f1 | **Primary accent** |
| --color-primary-600 | #4f46e5 | Active/hover |
| --color-primary-700 | #4338ca | Pressed states |

### Accent Colors
| Token | Value | Usage |
|-------|-------|-------|
| --color-accent-violet | #8b5cf6 | System messages, premium |
| --color-accent-cyan | #06b6d4 | Info elements |
| --color-accent-emerald | #10b981 | Success states |
| --color-accent-amber | #f59e0b | Warning states |
| --color-accent-rose | #f43f5e | Gradients, accents |

### Semantic Colors
| Token | Value | Usage |
|-------|-------|-------|
| --color-success | #10b981 | Success messages |
| --color-warning | #f59e0b | Warning alerts |
| --color-error | #ef4444 | Error states |
| --color-info | #06b6d4 | Information |

### Theme-Aware Tokens (USE THESE!)

| Token | Dark Mode | Light Mode | Usage |
|-------|-----------|------------|-------|
| --color-bg-primary | #111827 | #ffffff | Main background |
| --color-bg-secondary | #1f2937 | #f9fafb | Sidebar, panels |
| --color-bg-tertiary | #374151 | #f3f4f6 | Cards, elevated |
| --color-text-primary | #e2e8f0 | #111827 | Primary text |
| --color-text-secondary | #94a3b8 | #4b5563 | Secondary text |
| --color-text-muted | #475569 | #9ca3af | Disabled, hints |
| --color-surface | #1f2937 | #ffffff | Component surfaces |
| --color-surface-hover | #374151 | #f3f4f6 | Hover states |
| --color-border-primary | #374151 | #e5e7eb | Primary borders |

## Typography

### Font Families
| Token | Value | Usage |
|-------|-------|-------|
| --font-sans | Inter, system-ui | Body text, UI |
| --font-mono | JetBrains Mono, monospace | Code |
| --font-display | Inter, sans-serif | Headings |

### Font Sizes
| Token | Size | Usage |
|-------|------|-------|
| --text-2xs | 10px | Captions, metadata |
| --text-xs | 12px | Small text, labels |
| --text-sm | 14px | Body small, code |
| --text-base | 16px | Body text |
| --text-lg | 18px | Emphasized body |
| --text-xl | 20px | H3 headings |
| --text-2xl | 24px | H2 headings |
| --text-3xl | 28px | H1 headings |
| --text-4xl | 36px | Display headings |

### Font Weights
| Token | Value | Usage |
|-------|-------|-------|
| --font-normal | 400 | Body text |
| --font-medium | 500 | Emphasized text |
| --font-semibold | 600 | Headings, labels |
| --font-bold | 700 | Strong emphasis |

### Line Heights
| Token | Value | Usage |
|-------|-------|-------|
| --leading-tight | 1.25 | Headings |
| --leading-normal | 1.5 | Body text |
| --leading-relaxed | 1.6 | Long-form text |
| --leading-code | 1.4 | Code blocks |

## Spacing

| Token | Size | Usage |
|-------|------|-------|
| --space-0 | 0 | No spacing |
| --space-1 | 4px | Tight gaps |
| --space-2 | 8px | Element gaps |
| --space-3 | 12px | Small padding |
| --space-4 | 16px | Standard padding |
| --space-5 | 20px | Medium padding |
| --space-6 | 24px | Section gaps |
| --space-8 | 32px | Large gaps |
| --space-10 | 40px | Major sections |
| --space-12 | 48px | Page sections |

## Border Radius

| Token | Size | Usage |
|-------|------|-------|
| --radius-none | 0 | No rounding |
| --radius-sm | 2px | Subtle rounding |
| --radius-md | 6px | Buttons, inputs |
| --radius-lg | 8px | Cards, panels |
| --radius-xl | 12px | Modals, large cards |
| --radius-2xl | 16px | Input boxes |
| --radius-full | 9999px | Circular elements |

## Shadows

| Token | Usage |
|-------|-------|
| --shadow-sm | Subtle elevation |
| --shadow-md | Dropdowns, popovers |
| --shadow-lg | Modals, dialogs |
| --shadow-xl | High-emphasis elements |

## Transitions

| Token | Duration | Usage |
|-------|----------|-------|
| --transition-fast | 150ms | Micro-interactions |
| --transition-normal | 200ms | Standard transitions |
| --transition-slow | 300ms | Theme changes, sidebar |

## Layout

| Token | Value | Usage |
|-------|-------|-------|
| --sidebar-collapsed | 70px | Icon-only sidebar |
| --sidebar-expanded | 280px | Full sidebar |
| --sidebar-item-size | 40px | Navigation item size |
| --header-height | 60px | Header bar height |
| --content-max-width | 1200px | Content constraint |

## Responsive Breakpoints

| Token | Value | Usage |
|-------|-------|-------|
| --breakpoint-sm | 600px | Mobile breakpoint |
| --breakpoint-md | 768px | Tablet breakpoint |
| --breakpoint-lg | 1024px | Desktop breakpoint |
| --breakpoint-xl | 1280px | Large desktop |

## Z-Index Scale

| Token | Value | Usage |
|-------|-------|-------|
| --z-base | 0 | Default layer |
| --z-dropdown | 100 | Dropdowns |
| --z-sticky | 200 | Sticky elements |
| --z-modal | 300 | Modals |
| --z-tooltip | 400 | Tooltips |
| --z-toast | 500 | Toast notifications |

## State Opacities

| Token | Value | Usage |
|-------|-------|-------|
| --opacity-muted | 0.7 | Inactive icons |
| --opacity-disabled | 0.5 | Disabled states |
| --opacity-subtle | 0.2 | Subtle backgrounds |
| --opacity-hover | 0.8 | Hover transitions |

## Code Display

| Token | Value | Usage |
|-------|-------|-------|
| --color-code-bg | #1e1e1e | Code background |
| --color-code-text | #d4d4d4 | Code text |
| --color-code-header | #2d2d2d | Code header |
| --color-code-line-number | #6e7681 | Line numbers |
```

---

### components/chat-message.md

```markdown
# Chat Message Component

Message bubbles for user, assistant, and system messages.

## TypeScript Interface

\`\`\`typescript
interface ChatMessageProps {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  codeBlocks?: CodeBlock[];
  files?: FileAttachment[];
  timestamp: Date;
  status?: 'sending' | 'sent' | 'error';
  onEdit?: () => void;
  onDelete?: () => void;
  onRetry?: () => void;
}

interface CodeBlock {
  id: string;
  language: string;
  code: string;
  filename?: string;
  highlightedLines?: number[];
}

interface FileAttachment {
  id: string;
  name: string;
  type: string;
  size: number;
  preview?: string;
}
\`\`\`

## Design Tokens

\`\`\`css
/* Dimensions */
max-width: 80%;
padding: var(--space-3) var(--space-4);
border-radius: var(--radius-xl);
margin-bottom: var(--space-3);

/* Typography */
font-size: var(--text-sm);
line-height: var(--leading-normal);

/* Animation */
animation: messageSlideIn var(--transition-slow) ease-out;
\`\`\`

## Variants

| Variant | Alignment | Background | Text Color |
|---------|-----------|------------|------------|
| User | Right | --color-primary-500 | --color-neutral-0 |
| Assistant | Left | --color-surface | --color-text-primary |
| System | Center | transparent | --color-accent-violet |

## States

| State | Visual Treatment |
|-------|-----------------|
| Default | Normal appearance |
| Sending | Pulsing opacity |
| Error | Red border, retry button |
| Editing | Inline input field |

## Implementation

\`\`\`svelte
<script lang="ts">
  import styles from './ChatMessage.module.css';

  interface Props {
    id: string;
    role: 'user' | 'assistant' | 'system';
    content: string;
    timestamp: Date;
    status?: 'sending' | 'sent' | 'error';
    onEdit?: () => void;
    onDelete?: () => void;
    onRetry?: () => void;
  }

  let {
    id,
    role,
    content,
    timestamp,
    status = 'sent',
    onEdit,
    onDelete,
    onRetry
  }: Props = $props();

  let messageClasses = $derived(
    [styles.message, styles[role], status === 'error' && styles.error]
      .filter(Boolean)
      .join(' ')
  );
</script>

<article class={messageClasses} role="article" aria-label="{role} message">
  {#if role === 'assistant'}
    <span class={styles.sender}>Assistant</span>
  {/if}

  <div class={styles.content}>
    {@html renderedContent}
  </div>

  <footer class={styles.footer}>
    <time datetime={timestamp.toISOString()}>
      {formatTime(timestamp)}
    </time>

    {#if status === 'error'}
      <button onclick={onRetry} class={styles.retryButton}>
        Retry
      </button>
    {/if}

    {#if onEdit}
      <button onclick={onEdit} class={styles.editButton} aria-label="Edit">
        <EditIcon />
      </button>
    {/if}
  </footer>
</article>
\`\`\`

\`\`\`css
/* ChatMessage.module.css */
.message {
  max-width: 80%;
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-xl);
  margin-bottom: var(--space-3);
  animation: messageSlideIn var(--transition-slow) ease-out;
}

/* Variants */
.user {
  margin-left: auto;
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
}

.assistant {
  margin-right: auto;
  background: var(--color-surface);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-primary);
}

.system {
  margin: var(--space-4) auto;
  background: transparent;
  color: var(--color-accent-violet);
  text-align: center;
  font-size: var(--text-xs);
}

/* States */
.error {
  border: 1px solid var(--color-error);
}

/* Elements */
.sender {
  display: block;
  font-size: var(--text-xs);
  font-weight: var(--font-semibold);
  color: var(--color-text-secondary);
  margin-bottom: var(--space-1);
}

.content {
  font-size: var(--text-sm);
  line-height: var(--leading-normal);
}

.footer {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-top: var(--space-2);
  font-size: var(--text-2xs);
  color: var(--color-text-muted);
}

/* Animation */
@keyframes messageSlideIn {
  from {
    opacity: 0;
    transform: translateY(12px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
\`\`\`
```

---

### components/sidebar.md

```markdown
# Sidebar Component

Vertical navigation with collapsible modes.

## TypeScript Interface

\`\`\`typescript
interface SidebarProps {
  collapsed: boolean;
  onToggle: () => void;
  activeSection: string;
  sections: SidebarSection[];
}

interface SidebarSection {
  id: string;
  icon: ComponentType;
  label: string;
  badge?: string | number;
  children?: SidebarItem[];
}

interface SidebarItem {
  id: string;
  label: string;
  action: () => void;
  badge?: string;
  disabled?: boolean;
}
\`\`\`

## Design Tokens

\`\`\`css
/* Width modes */
width: var(--sidebar-collapsed);  /* 70px */
width: var(--sidebar-expanded);   /* 280px */

/* Item sizing */
--sidebar-item-size: 40px;
--sidebar-icon-size: 24px;

/* Spacing */
padding: var(--space-5) 0;
margin: var(--space-6) auto;

/* States */
opacity: var(--opacity-muted);  /* 0.7 normal */
opacity: 1;                     /* hover/active */
\`\`\`

## Layout Modes

### Collapsed (70px)
- Icons only
- Centered horizontally
- Tooltips on hover

### Expanded (280px)
- Icons with labels
- Left-aligned content
- Nested items visible

## States

| State | Opacity | Background |
|-------|---------|------------|
| Normal | 0.7 | transparent |
| Hover | 1.0 | --color-surface-hover |
| Active | 1.0 | rgba(--color-primary-500, 0.2) |
| Disabled | 0.5 | transparent |

## Implementation

\`\`\`svelte
<script lang="ts">
  import styles from './Sidebar.module.css';

  interface Props {
    collapsed: boolean;
    onToggle: () => void;
    activeSection: string;
    sections: SidebarSection[];
  }

  let { collapsed, onToggle, activeSection, sections }: Props = $props();

  let sidebarClasses = $derived(
    [styles.sidebar, collapsed && styles.collapsed]
      .filter(Boolean)
      .join(' ')
  );
</script>

<aside class={sidebarClasses} role="navigation" aria-label="Main navigation">
  <div class={styles.logo}>
    <LogoIcon size={32} />
  </div>

  <nav class={styles.nav}>
    {#each sections as section}
      <button
        class={[styles.navItem, activeSection === section.id && styles.active]
          .filter(Boolean)
          .join(' ')}
        onclick={() => section.action?.()}
        aria-current={activeSection === section.id ? 'page' : undefined}
        aria-label={section.label}
      >
        <svelte:component this={section.icon} size={24} />
        {#if !collapsed}
          <span class={styles.label}>{section.label}</span>
        {/if}
        {#if section.badge}
          <span class={styles.badge}>{section.badge}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <button
    class={styles.toggleButton}
    onclick={onToggle}
    aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
  >
    <ChevronIcon direction={collapsed ? 'right' : 'left'} />
  </button>
</aside>
\`\`\`

\`\`\`css
/* Sidebar.module.css */
.sidebar {
  width: var(--sidebar-expanded);
  height: 100vh;
  padding: var(--space-5) 0;
  background: var(--color-bg-secondary);
  display: flex;
  flex-direction: column;
  transition: width var(--transition-slow);
}

.sidebar.collapsed {
  width: var(--sidebar-collapsed);
}

.logo {
  width: 40px;
  height: 40px;
  margin: 0 auto var(--space-8);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-lg);
  background: var(--color-primary-500);
}

.nav {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: 0 var(--space-2);
}

.navItem {
  width: 100%;
  height: var(--sidebar-item-size);
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: 0 var(--space-3);
  border-radius: var(--radius-md);
  background: transparent;
  border: none;
  cursor: pointer;
  opacity: var(--opacity-muted);
  transition: all var(--transition-fast);
}

.collapsed .navItem {
  justify-content: center;
  padding: 0;
}

.navItem:hover {
  opacity: 1;
  background: var(--color-surface-hover);
}

.navItem.active {
  opacity: 1;
  background: rgba(99, 102, 241, var(--opacity-subtle));
  color: var(--color-primary-500);
}

.label {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--color-text-primary);
  white-space: nowrap;
}

.badge {
  margin-left: auto;
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-2xs);
  font-weight: var(--font-medium);
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
  border-radius: var(--radius-full);
}

.toggleButton {
  margin: var(--space-4) auto;
  padding: var(--space-2);
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  cursor: pointer;
  opacity: var(--opacity-muted);
  transition: all var(--transition-fast);
}

.toggleButton:hover {
  opacity: 1;
  background: var(--color-surface-hover);
}
\`\`\`
```

---

### state/app-state.md

```markdown
# Application State Structure

Core state management architecture for the Coder application.

## State Interface

\`\`\`typescript
interface AppState {
  // UI State
  ui: {
    sidebar: {
      collapsed: boolean;
      activeSection: string;
    };
    theme: 'light' | 'dark' | 'system';
    modals: {
      activeModal: string | null;
      modalData: Record<string, unknown>;
    };
  };

  // Chat State
  chat: {
    conversations: Conversation[];
    activeConversationId: string | null;
    messages: Message[];
    currentInput: string;
    isLoading: boolean;
    error: string | null;
  };

  // Repository State
  repository: {
    selectedRepo: string | null;
    branch: string;
    files: FileNode[];
    status: 'idle' | 'loading' | 'error' | 'ready';
    lastSync: Date | null;
  };

  // AI Configuration
  ai: {
    model: string;
    temperature: number;
    maxTokens: number;
    autoAcceptEdits: boolean;
    contextFiles: string[];
  };

  // User Preferences
  preferences: {
    fontSize: number;
    lineNumbers: boolean;
    wordWrap: boolean;
    autoSave: boolean;
    sidebarCollapsed: boolean;
  };
}
\`\`\`

## Svelte Store Implementation

\`\`\`typescript
// $lib/stores/app.ts
import { writable, derived } from 'svelte/store';
import { persisted } from '$lib/stores/persisted';

// Persisted preferences
export const preferences = persisted('coder-preferences', {
  theme: 'dark' as 'light' | 'dark' | 'system',
  fontSize: 14,
  sidebarCollapsed: false,
  autoSave: true,
});

// UI State
export const sidebar = writable({
  collapsed: false,
  activeSection: 'chat',
});

// Chat State
export const chat = writable({
  messages: [] as Message[],
  currentInput: '',
  isLoading: false,
  error: null as string | null,
});

// Derived stores
export const theme = derived(
  preferences,
  ($prefs) => {
    if ($prefs.theme === 'system') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light';
    }
    return $prefs.theme;
  }
);
\`\`\`

## Usage in Components

\`\`\`svelte
<script lang="ts">
  import { sidebar, chat, preferences } from '$lib/stores/app';

  // Read state
  let sidebarCollapsed = $state($sidebar.collapsed);

  // Update state
  function toggleSidebar() {
    sidebar.update(s => ({ ...s, collapsed: !s.collapsed }));
  }
</script>
\`\`\`
```

---

### examples/component-template.svelte

```svelte
<!--
  ComponentName.svelte
  Brief description of the component's purpose.

  @example
  <ComponentName variant="primary" onclick={handleClick}>
    Click me
  </ComponentName>
-->
<script lang="ts">
  // ============================================
  // 1. IMPORTS
  // ============================================
  import styles from './ComponentName.module.css';

  // ============================================
  // 2. TYPES
  // ============================================
  interface Props {
    /** Visual style variant */
    variant?: 'primary' | 'secondary' | 'ghost';
    /** Size of the component */
    size?: 'sm' | 'md' | 'lg';
    /** Whether the component is disabled */
    disabled?: boolean;
    /** Click handler */
    onclick?: () => void;
    /** Child content */
    children?: import('svelte').Snippet;
  }

  // ============================================
  // 3. PROPS
  // ============================================
  let {
    variant = 'primary',
    size = 'md',
    disabled = false,
    onclick,
    children
  }: Props = $props();

  // ============================================
  // 4. STATE
  // ============================================
  let isHovered = $state(false);
  let isFocused = $state(false);

  // ============================================
  // 5. DERIVED VALUES
  // ============================================
  let classes = $derived(
    [
      styles.component,
      styles[variant],
      styles[size],
      disabled && styles.disabled,
      isHovered && styles.hovered,
      isFocused && styles.focused
    ].filter(Boolean).join(' ')
  );

  // ============================================
  // 6. EFFECTS
  // ============================================
  $effect(() => {
    // Side effects when props change
    if (disabled) {
      isHovered = false;
      isFocused = false;
    }
  });

  // ============================================
  // 7. EVENT HANDLERS
  // ============================================
  function handleClick(e: MouseEvent) {
    if (!disabled && onclick) {
      onclick();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleClick(e as unknown as MouseEvent);
    }
  }
</script>

<!-- ============================================
     8. TEMPLATE
     ============================================ -->
<button
  class={classes}
  {disabled}
  onclick={handleClick}
  onkeydown={handleKeydown}
  onmouseenter={() => isHovered = true}
  onmouseleave={() => isHovered = false}
  onfocus={() => isFocused = true}
  onblur={() => isFocused = false}
  type="button"
  aria-disabled={disabled}
>
  {#if children}
    {@render children()}
  {/if}
</button>
```

---

### examples/component-template.module.css

```css
/**
 * ComponentName.module.css
 * Styles for ComponentName component.
 *
 * DESIGN TOKENS USED:
 * - Colors: var(--color-*)
 * - Spacing: var(--space-*)
 * - Typography: var(--text-*, --font-*)
 * - Radius: var(--radius-*)
 * - Transitions: var(--transition-*)
 */

/* ============================================
   BASE STYLES
   ============================================ */
.component {
  /* Layout */
  display: inline-flex;
  align-items: center;
  justify-content: center;

  /* Typography */
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  line-height: var(--leading-tight);

  /* Appearance */
  background: var(--color-surface);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);

  /* Interaction */
  cursor: pointer;
  transition: all var(--transition-fast);
}

/* ============================================
   VARIANTS
   ============================================ */
.primary {
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
  border-color: var(--color-primary-500);
}

.primary:hover:not(.disabled) {
  background: var(--color-primary-600);
  border-color: var(--color-primary-600);
}

.secondary {
  background: var(--color-surface);
  color: var(--color-text-primary);
  border-color: var(--color-border-primary);
}

.secondary:hover:not(.disabled) {
  background: var(--color-surface-hover);
}

.ghost {
  background: transparent;
  color: var(--color-text-primary);
  border-color: transparent;
}

.ghost:hover:not(.disabled) {
  background: var(--color-surface-hover);
}

/* ============================================
   SIZES
   ============================================ */
.sm {
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-xs);
  border-radius: var(--radius-sm);
}

.md {
  padding: var(--space-2) var(--space-4);
  font-size: var(--text-sm);
  border-radius: var(--radius-md);
}

.lg {
  padding: var(--space-3) var(--space-6);
  font-size: var(--text-base);
  border-radius: var(--radius-lg);
}

/* ============================================
   STATES
   ============================================ */
.disabled {
  opacity: var(--opacity-disabled);
  cursor: not-allowed;
  pointer-events: none;
}

.hovered {
  /* Hover-specific overrides */
}

.focused {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

/* ============================================
   ACCESSIBILITY
   ============================================ */
.component:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

/* Reduced motion support */
@media (prefers-reduced-motion: reduce) {
  .component {
    transition: none;
  }
}
```

---

## Implementation Checklist

### Phase 1: Setup
- [ ] Create `.claude/skills/frontend-specialist/` directory structure
- [ ] Create main `SKILL.md` file
- [ ] Create all styleguide markdown files

### Phase 2: Component Specs
- [ ] Create all component specification files
- [ ] Create state management documentation
- [ ] Create example templates

### Phase 3: References
- [ ] Create references/README.md with instructions
- [ ] Add placeholder for HTML references

### Phase 4: Integration
- [ ] Update design-system.css with new tokens
- [ ] Test skill with sample component creation
- [ ] Document any refinements needed

---

## Success Criteria

1. ✅ All specs reference design tokens (no hardcoded values)
2. ✅ TypeScript interfaces for all components
3. ✅ Accessibility requirements in every spec
4. ✅ Both themes documented
5. ✅ Responsive patterns included
6. ✅ Example templates provided
7. ✅ State management patterns documented
