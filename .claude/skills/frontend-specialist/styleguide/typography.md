# Typography

Guidelines for using typography in the Coder application.

## Font Families

### Primary: Inter

Used for all UI text including body, headings, and labels.

```css
font-family: var(--font-sans);
/* 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif */
```

**Characteristics:**
- Excellent legibility at small sizes
- Clear distinction between similar characters (l, I, 1)
- Variable font for performance
- Modern, professional appearance

### Monospace: JetBrains Mono

Used for code, command outputs, and technical content.

```css
font-family: var(--font-mono);
/* 'JetBrains Mono', 'Fira Code', 'SF Mono', Consolas, monospace */
```

**Characteristics:**
- Designed specifically for developers
- Clear distinction between O/0, l/1/I
- Excellent for code readability
- Supports ligatures (optional)

---

## Type Scale

### Heading Sizes

| Element | Token | Size | Weight | Usage |
|---------|-------|------|--------|-------|
| H1 | `--text-3xl` | 28px | 700 (`--font-bold`) | Page titles |
| H2 | `--text-2xl` | 24px | 600 (`--font-semibold`) | Section titles |
| H3 | `--text-xl` | 20px | 600 (`--font-semibold`) | Subsection titles |
| H4 | `--text-lg` | 18px | 600 (`--font-semibold`) | Card titles |

```css
h1 {
  font-size: var(--text-3xl);
  font-weight: var(--font-bold);
  line-height: var(--leading-tight);
}

h2 {
  font-size: var(--text-2xl);
  font-weight: var(--font-semibold);
  line-height: var(--leading-tight);
}

h3 {
  font-size: var(--text-xl);
  font-weight: var(--font-semibold);
  line-height: var(--leading-tight);
}
```

### Body Sizes

| Token | Size | Weight | Usage |
|-------|------|--------|-------|
| `--text-base` | 16px | 400 | Primary content |
| `--text-sm` | 14px | 400 | Standard content, chat messages |
| `--text-xs` | 12px | 400 | Secondary content, labels |
| `--text-2xs` | 10px | 400 | Captions, metadata, timestamps |

```css
.body-large {
  font-size: var(--text-base);
  line-height: var(--leading-normal);
}

.body {
  font-size: var(--text-sm);
  line-height: var(--leading-normal);
}

.body-small {
  font-size: var(--text-xs);
  line-height: var(--leading-normal);
}

.caption {
  font-size: var(--text-2xs);
  line-height: var(--leading-normal);
}
```

### Code Size

```css
.code {
  font-family: var(--font-mono);
  font-size: var(--text-xs);      /* 12px in blocks */
  line-height: var(--leading-code); /* 1.4 for code */
}

.code-inline {
  font-family: var(--font-mono);
  font-size: var(--text-sm);      /* 14px inline */
}
```

---

## Font Weights

| Token | Value | Usage |
|-------|-------|-------|
| `--font-normal` | 400 | Body text, descriptions |
| `--font-medium` | 500 | Emphasized text, buttons |
| `--font-semibold` | 600 | Headings, labels, nav items |
| `--font-bold` | 700 | Strong emphasis, page titles |

```css
/* Usage examples */
.description { font-weight: var(--font-normal); }
.button-label { font-weight: var(--font-medium); }
.section-title { font-weight: var(--font-semibold); }
.page-title { font-weight: var(--font-bold); }
```

---

## Line Heights

| Token | Value | Usage |
|-------|-------|-------|
| `--leading-tight` | 1.25 | Headings, UI labels |
| `--leading-normal` | 1.5 | Body text |
| `--leading-relaxed` | 1.625 | Long-form content |
| `--leading-code` | 1.4 | Code blocks |

```css
.heading { line-height: var(--leading-tight); }
.body { line-height: var(--leading-normal); }
.article { line-height: var(--leading-relaxed); }
pre code { line-height: var(--leading-code); }
```

---

## Common Patterns

### Page Title

```css
.page-title {
  font-family: var(--font-display);
  font-size: var(--text-3xl);
  font-weight: var(--font-bold);
  line-height: var(--leading-tight);
  letter-spacing: var(--tracking-tight);
  color: var(--color-text-primary);
}
```

### Card Title

```css
.card-title {
  font-size: var(--text-lg);
  font-weight: var(--font-semibold);
  line-height: var(--leading-tight);
  color: var(--color-text-primary);
}
```

### Chat Message

```css
.chat-message {
  font-size: var(--text-sm);
  font-weight: var(--font-normal);
  line-height: var(--leading-normal);
  color: var(--color-text-primary);
}
```

### Timestamp

```css
.timestamp {
  font-size: var(--text-2xs);
  font-weight: var(--font-normal);
  color: var(--color-text-muted);
}
```

### Navigation Item

```css
.nav-item {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--color-text-secondary);
}

.nav-item.active {
  font-weight: var(--font-semibold);
  color: var(--color-text-primary);
}
```

### Code Block

```css
.code-block {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  font-weight: var(--font-normal);
  line-height: var(--leading-code);
}

.code-block-header {
  font-family: var(--font-sans);
  font-size: var(--text-2xs);
  font-weight: var(--font-medium);
}
```

---

## Text Colors

### Hierarchy

```css
/* Primary text - headings, important content */
.primary-text { color: var(--color-text-primary); }

/* Secondary text - descriptions, subheadings */
.secondary-text { color: var(--color-text-secondary); }

/* Tertiary text - labels, hints */
.tertiary-text { color: var(--color-text-tertiary); }

/* Muted text - disabled, timestamps */
.muted-text { color: var(--color-text-muted); }
```

### Special Colors

```css
/* Links and interactive */
.link { color: var(--color-primary-500); }

/* System/premium text */
.system-text { color: var(--color-accent-violet); }

/* Status colors */
.success-text { color: var(--color-success); }
.warning-text { color: var(--color-warning); }
.error-text { color: var(--color-error); }
```

---

## Text Utilities

### Truncation

```css
.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
```

### Selection

```css
::selection {
  background: rgba(99, 102, 241, 0.3);
  color: var(--color-text-primary);
}
```

### Code Inline

```css
code {
  font-family: var(--font-mono);
  font-size: 0.9em;
  padding: 0.125em 0.375em;
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
}
```
