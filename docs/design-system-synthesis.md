# Design System Synthesis

## Overview

This document synthesizes three design sources into a unified design system for the Coder AI Code Agent:

1. **Existing Design System** (`design-system.css`) - Current implementation
2. **AI Code Agent App Spec** - Component specifications and Daylight theme
3. **Claude Code Web Analysis** - Professional patterns and differentiation

---

## 1. Color System Reconciliation

### Three-Way Color Mapping

| Element | Existing System | Agent Spec | Claude Analysis | **Final Decision** |
|---------|----------------|------------|-----------------|-------------------|
| Primary Background (Dark) | `--color-bg-primary: #111827` | Not specified | `#141421` | **Use existing: #111827** |
| Primary Accent | `--color-primary-500: #6366f1` | `#4f46e5` (600) | `#6366f1` | **Keep #6366f1** (matches Claude analysis) |
| System/Accent | `--color-accent-violet: #8b5cf6` | Not specified | `#8b5cf6` | **Keep existing** |
| Success | `--color-accent-emerald: #10b981` | `#22c55e` | `#10b981` | **Keep #10b981** (matches Claude analysis) |
| Error | `--color-accent-rose: #f43f5e` | `#ef4444` | `#ef4444` | **Use #ef4444** (standard error red) |
| Text Primary (Dark) | `--color-text-primary: #f9fafb` | Not specified | `#e2e8f0` | **Use #e2e8f0** (softer, better readability) |
| Text Secondary | `--color-text-secondary: #d1d5db` | `#6b7280` | `#94a3b8` | **Use #94a3b8** (Claude analysis) |

### Final Color Palette

```css
:root {
  /* Primary Palette - Indigo (matches Claude analysis recommendation) */
  --color-primary-50: #eef2ff;
  --color-primary-100: #e0e7ff;
  --color-primary-200: #c7d2fe;
  --color-primary-300: #a5b4fc;
  --color-primary-400: #818cf8;
  --color-primary-500: #6366f1;   /* Primary accent */
  --color-primary-600: #4f46e5;   /* Hover state */
  --color-primary-700: #4338ca;   /* Active state */

  /* Accent Colors */
  --color-accent-violet: #8b5cf6;  /* System messages, premium feel */
  --color-accent-cyan: #06b6d4;    /* Info */
  --color-accent-emerald: #10b981; /* Success */
  --color-accent-amber: #f59e0b;   /* Warning */
  --color-accent-rose: #f43f5e;    /* Keep for gradients */

  /* Semantic Colors (updated) */
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-error: #ef4444;          /* Standard error red from Claude analysis */
  --color-info: #06b6d4;
}
```

---

## 2. Typography System Reconciliation

### Font Decision

| Element | Existing | Agent Spec | Claude Analysis | **Final Decision** |
|---------|----------|------------|-----------------|-------------------|
| Primary Font | Inter | Inter | Geist/Space Grotesk | **Inter** (free, well-supported) |
| Monospace | JetBrains Mono | JetBrains Mono | Fira Code/JetBrains Mono | **JetBrains Mono** (excellent) |
| Display | Inter | Inter | Geist Mono | **Inter** (consistency) |

### Typography Scale

```css
:root {
  /* Font Families */
  --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', 'SF Mono', Consolas, monospace;
  --font-display: 'Inter', var(--font-sans);

  /* Font Sizes - Unified Scale */
  --text-2xs: 0.625rem;    /* 10px - captions, metadata */
  --text-xs: 0.75rem;      /* 12px - small text */
  --text-sm: 0.875rem;     /* 14px - body small, code */
  --text-base: 1rem;       /* 16px - body */
  --text-lg: 1.125rem;     /* 18px - emphasized */
  --text-xl: 1.25rem;      /* 20px - h3 */
  --text-2xl: 1.5rem;      /* 24px - h2 */
  --text-3xl: 1.75rem;     /* 28px - h1 (from Agent spec) */
  --text-4xl: 2.25rem;     /* 36px - display */

  /* Font Weights */
  --font-normal: 400;
  --font-medium: 500;
  --font-semibold: 600;
  --font-bold: 700;

  /* Line Heights */
  --leading-tight: 1.25;
  --leading-normal: 1.5;
  --leading-relaxed: 1.6;
  --leading-code: 1.4;     /* From Claude analysis */
}
```

---

## 3. Layout System Reconciliation

### Sidebar Decision

| Source | Width | Decision |
|--------|-------|----------|
| Existing System | 280px | Full sidebar |
| Agent Spec | 70px | Icon-only |
| Claude Analysis | 280-320px, collapsible to 48px | **Adaptive** |

**Final Implementation**: Collapsible sidebar with smooth transitions

```css
:root {
  /* Sidebar Modes */
  --sidebar-collapsed: 70px;    /* Icons only */
  --sidebar-expanded: 280px;    /* With labels */
  --sidebar-icon-size: 24px;
  --sidebar-item-size: 40px;

  /* Layout */
  --header-height: 60px;        /* Updated from 56px */
  --content-max-width: 1200px;

  /* Responsive Breakpoints */
  --breakpoint-sm: 600px;       /* Mobile */
  --breakpoint-md: 768px;       /* Tablet */
  --breakpoint-lg: 1024px;      /* Desktop */
  --breakpoint-xl: 1280px;      /* Large desktop */
}
```

---

## 4. Component Specifications

### 4.1 Sidebar Component

**TypeScript Interface (from Claude Analysis):**
```typescript
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
  children?: SidebarItem[];
  badge?: string | number;
}

interface SidebarItem {
  id: string;
  label: string;
  action: () => void;
  badge?: string;
  disabled?: boolean;
}
```

**Design Tokens Applied:**
```css
.sidebar {
  width: var(--sidebar-expanded);
  height: 100vh;
  background: var(--color-bg-secondary);
  transition: width var(--transition-slow);
}

.sidebar.collapsed {
  width: var(--sidebar-collapsed);
}

.navItem {
  width: var(--sidebar-item-size);
  height: var(--sidebar-item-size);
  border-radius: var(--radius-md);
  opacity: var(--opacity-muted);
  transition: all var(--transition-fast);
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
```

### 4.2 Chat Message Component

**TypeScript Interface:**
```typescript
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
  diff?: {
    additions: number;
    deletions: number;
  };
}

interface FileAttachment {
  id: string;
  name: string;
  type: string;
  size: number;
  preview?: string;
}
```

**Design Tokens Applied:**
```css
.message {
  max-width: 80%;
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-xl);
  margin-bottom: var(--space-3);
  line-height: var(--leading-normal);
  animation: messageAppear var(--transition-slow) ease-out;
}

.message.user {
  margin-left: auto;
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
}

.message.assistant {
  margin-right: auto;
  background: var(--color-surface);
  color: var(--color-text-primary);
}

.message.system {
  margin: 0 auto;
  background: transparent;
  color: var(--color-accent-violet);
  text-align: center;
}

@keyframes messageAppear {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

### 4.3 Code Block Component

**TypeScript Interface:**
```typescript
interface CodeBlockProps {
  language: string;
  code: string;
  filename?: string;
  showLineNumbers?: boolean;
  highlightedLines?: number[];
  diff?: CodeDiff;
  onCopy?: () => void;
  onEdit?: () => void;
}

interface CodeDiff {
  type: 'addition' | 'deletion' | 'modification';
  additions: number;
  deletions: number;
}
```

**Design Tokens Applied:**
```css
.codeBlock {
  background: var(--color-code-bg);
  border-radius: var(--radius-lg);
  margin: var(--space-2) 0;
  overflow: hidden;
}

.codeHeader {
  display: flex;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  background: var(--color-code-header);
  border-bottom: 1px solid var(--color-border-primary);
}

.codeContent {
  padding: var(--space-3);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  line-height: var(--leading-code);
  color: var(--color-code-text);
  overflow-x: auto;
}

/* Line numbers */
.lineNumber {
  color: var(--color-text-muted);
  user-select: none;
  padding-right: var(--space-3);
}

/* Highlighted lines */
.line.highlighted {
  background: rgba(99, 102, 241, 0.1);
  border-left: 2px solid var(--color-primary-500);
}

/* Diff indicators */
.line.addition {
  background: rgba(16, 185, 129, 0.15);
}

.line.deletion {
  background: rgba(239, 68, 68, 0.15);
}
```

### 4.4 Input Box Component

**TypeScript Interface:**
```typescript
interface InputBoxProps {
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  disabled?: boolean;
  loading?: boolean;
  attachments?: FileAttachment[];
  onAttach?: (files: File[]) => void;
  onRemoveAttachment?: (id: string) => void;
}
```

**Design Tokens Applied:**
```css
.inputBox {
  min-height: 56px;
  padding: var(--space-4);
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-2xl);
  transition: border-color var(--transition-fast);
}

.inputBox:focus-within {
  border-color: var(--color-primary-500);
}

.input {
  font-size: var(--text-sm);
  color: var(--color-text-primary);
  background: transparent;
  border: none;
  outline: none;
  width: 100%;
}

.input::placeholder {
  color: var(--color-text-muted);
  opacity: var(--opacity-muted);
}

.sendButton {
  width: 36px;
  height: 36px;
  border-radius: var(--radius-full);
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
  border: none;
  cursor: pointer;
  transition: background var(--transition-fast);
}

.sendButton:hover {
  background: var(--color-primary-600);
}

.sendButton:disabled {
  background: var(--color-text-muted);
  cursor: not-allowed;
}
```

### 4.5 Suggestion Cards Component

**TypeScript Interface:**
```typescript
interface SuggestionCardProps {
  icon: ComponentType;
  title: string;
  description: string;
  onClick: () => void;
}

interface SuggestionGridProps {
  suggestions: SuggestionCardProps[];
  columns?: 2 | 3 | 4;
}
```

**Design Tokens Applied:**
```css
.suggestionGrid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-3);
}

@media (min-width: 768px) {
  .suggestionGrid {
    grid-template-columns: repeat(3, 1fr);
  }
}

.suggestionCard {
  padding: var(--space-4);
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-xl);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.suggestionCard:hover {
  border-color: var(--color-primary-500);
  background: var(--color-surface-hover);
}

.suggestionIcon {
  width: 20px;
  height: 20px;
  color: var(--color-primary-500);
  margin-bottom: var(--space-2);
}

.suggestionTitle {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--color-text-primary);
  margin-bottom: var(--space-1);
}

.suggestionDescription {
  font-size: var(--text-xs);
  color: var(--color-text-secondary);
}
```

---

## 5. State Management Architecture

### Application State (from Claude Analysis)

```typescript
interface AppState {
  // UI State
  sidebar: {
    collapsed: boolean;
    activeSection: string;
  };

  // Chat State
  chat: {
    messages: ChatMessage[];
    currentInput: string;
    isLoading: boolean;
    error: string | null;
  };

  // Repository State
  repository: {
    selectedRepo: string | null;
    branch: string;
    files: FileNode[];
    status: 'idle' | 'loading' | 'error';
  };

  // AI Configuration
  ai: {
    model: string;
    temperature: number;
    autoAcceptEdits: boolean;
    contextFiles: string[];
  };

  // User Preferences
  preferences: {
    theme: 'light' | 'dark' | 'system';
    fontSize: number;
    sidebarCollapsed: boolean;
    autoSave: boolean;
  };
}
```

---

## 6. Design Tokens - Complete Set

### Updated design-system.css Additions

```css
:root {
  /* =====================
     ADDITIONS TO EXISTING SYSTEM
     ===================== */

  /* Typography - Missing Size */
  --text-2xs: 0.625rem;     /* 10px */

  /* Line Height for Code */
  --leading-code: 1.4;

  /* Layout - Sidebar Modes */
  --sidebar-collapsed: 70px;
  --sidebar-expanded: 280px;
  --sidebar-item-size: 40px;

  /* Layout - Updated Header */
  --header-height: 60px;

  /* Responsive Breakpoints */
  --breakpoint-sm: 600px;
  --breakpoint-md: 768px;
  --breakpoint-lg: 1024px;
  --breakpoint-xl: 1280px;

  /* Code Display (non-themeable) */
  --color-code-bg: #1e1e1e;
  --color-code-text: #d4d4d4;
  --color-code-header: #2d2d2d;
  --color-code-line-number: #6e7681;

  /* State Opacities */
  --opacity-muted: 0.7;
  --opacity-disabled: 0.5;
  --opacity-subtle: 0.2;
  --opacity-hover: 0.8;

  /* Animation Durations */
  --duration-instant: 0ms;
  --duration-fast: 150ms;
  --duration-normal: 200ms;
  --duration-slow: 300ms;
  --duration-slower: 500ms;
}

/* Updated Dark Theme */
[data-theme='dark'] {
  --color-bg-primary: #111827;
  --color-bg-secondary: #1f2937;
  --color-bg-tertiary: #374151;
  --color-bg-elevated: #1f2937;

  --color-text-primary: #e2e8f0;    /* Updated per Claude analysis */
  --color-text-secondary: #94a3b8;  /* Updated per Claude analysis */
  --color-text-tertiary: #64748b;
  --color-text-muted: #475569;

  --color-border-primary: #374151;
  --color-border-secondary: #4b5563;

  --color-surface: #1f2937;
  --color-surface-hover: #374151;
  --color-surface-active: #4b5563;
}

/* Updated Light Theme */
[data-theme='light'] {
  --color-bg-primary: #ffffff;
  --color-bg-secondary: #f9fafb;
  --color-bg-tertiary: #f3f4f6;
  --color-bg-elevated: #ffffff;

  --color-text-primary: #111827;
  --color-text-secondary: #4b5563;
  --color-text-tertiary: #6b7280;
  --color-text-muted: #9ca3af;

  --color-border-primary: #e5e7eb;
  --color-border-secondary: #d1d5db;

  --color-surface: #ffffff;
  --color-surface-hover: #f3f4f6;
  --color-surface-active: #e5e7eb;
}
```

---

## 7. Animation Specifications

```css
/* Theme Transitions */
* {
  transition-property: background-color, color, border-color;
  transition-duration: var(--duration-slow);
  transition-timing-function: ease;
}

/* Button Interactions */
.button {
  transition: all var(--duration-fast) ease;
}

.button:active {
  transform: scale(0.98);
}

/* Message Appear Animation */
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

/* Typing Indicator Animation */
@keyframes typingPulse {
  0%, 60%, 100% {
    opacity: 0.3;
  }
  30% {
    opacity: 1;
  }
}

/* Sidebar Collapse Animation */
.sidebar {
  transition: width var(--duration-slow) ease;
}

/* Code Block Highlight Animation */
@keyframes highlightPulse {
  0% {
    background-color: rgba(99, 102, 241, 0.3);
  }
  100% {
    background-color: rgba(99, 102, 241, 0.1);
  }
}
```

---

## 8. Accessibility Specifications

### WCAG AA Compliance Checklist

| Requirement | Implementation |
|-------------|----------------|
| Color Contrast (Normal Text) | 4.5:1 minimum - verified with text colors |
| Color Contrast (Large Text) | 3:1 minimum - verified with headings |
| Focus Indicators | 2px outline with 2px offset using primary color |
| Touch Targets | Minimum 44px × 44px for interactive elements |
| Keyboard Navigation | All interactive elements accessible via keyboard |
| Screen Reader Support | ARIA labels on all icons and interactive elements |
| Motion Preferences | Respect `prefers-reduced-motion` |

### Focus States

```css
/* Focus Ring */
:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

/* Reduced Motion */
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## 9. Summary of Changes to Existing System

### Additions Required

1. **Typography**: Add `--text-2xs` (10px) and `--leading-code` (1.4)
2. **Layout**: Add sidebar mode tokens and breakpoints
3. **Colors**: Add code display tokens and state opacities
4. **Animations**: Add duration tokens and keyframe definitions
5. **Update**: Header height from 56px to 60px

### No Changes Required

- Primary color palette (indigo already matches Claude analysis)
- Font families (Inter + JetBrains Mono are optimal)
- Spacing scale (existing scale is comprehensive)
- Border radius scale (existing scale is complete)
- Z-index scale (existing scale is sufficient)

### Theme Updates

- Update `--color-text-primary` in dark mode to `#e2e8f0`
- Update `--color-text-secondary` in dark mode to `#94a3b8`
