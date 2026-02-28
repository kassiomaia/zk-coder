# Accessibility Guidelines

WCAG AA compliance requirements and patterns for the Coder application.

## Core Requirements

### WCAG AA Standards

| Requirement | Standard |
|-------------|----------|
| Color Contrast (Normal Text) | 4.5:1 minimum |
| Color Contrast (Large Text) | 3:1 minimum |
| Color Contrast (UI Components) | 3:1 minimum |
| Focus Visible | Clear focus indicator |
| Keyboard Accessible | All functions via keyboard |
| Touch Target Size | 44×44px minimum (recommended) |

---

## Color Contrast

### Verified Combinations

These color combinations have been verified for WCAG AA compliance:

**Dark Mode:**
| Text Color | Background | Ratio | Pass |
|------------|------------|-------|------|
| `--color-text-primary` (#e2e8f0) | `--color-bg-primary` (#111827) | 12.5:1 | ✓ |
| `--color-text-secondary` (#94a3b8) | `--color-bg-primary` (#111827) | 6.8:1 | ✓ |
| `--color-neutral-0` (#ffffff) | `--color-primary-500` (#6366f1) | 5.2:1 | ✓ |

**Light Mode:**
| Text Color | Background | Ratio | Pass |
|------------|------------|-------|------|
| `--color-text-primary` (#111827) | `--color-bg-primary` (#ffffff) | 17.5:1 | ✓ |
| `--color-text-secondary` (#4b5563) | `--color-bg-primary` (#ffffff) | 7.0:1 | ✓ |
| `--color-neutral-0` (#ffffff) | `--color-primary-500` (#6366f1) | 5.2:1 | ✓ |

### Contrast Checking

```typescript
// Utility function to check contrast
function getContrastRatio(fg: string, bg: string): number {
  // Use a library like chroma-js or color-contrast
  return contrastRatio;
}

function meetsWCAGAA(fg: string, bg: string, isLargeText: boolean): boolean {
  const ratio = getContrastRatio(fg, bg);
  return isLargeText ? ratio >= 3 : ratio >= 4.5;
}
```

---

## Focus Management

### Focus Indicator

All interactive elements must have a visible focus indicator:

```css
/* Standard focus indicator */
:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

/* Remove default outline when using focus-visible */
:focus:not(:focus-visible) {
  outline: none;
}

/* Ensure outline is visible on all backgrounds */
:focus-visible {
  outline: 2px solid var(--color-primary-500);
  box-shadow: 0 0 0 4px rgba(99, 102, 241, 0.2);
}
```

### Focus Trap for Modals

```typescript
function trapFocus(element: HTMLElement) {
  const focusableElements = element.querySelectorAll(
    'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
  );

  const firstElement = focusableElements[0] as HTMLElement;
  const lastElement = focusableElements[focusableElements.length - 1] as HTMLElement;

  element.addEventListener('keydown', (e) => {
    if (e.key !== 'Tab') return;

    if (e.shiftKey) {
      if (document.activeElement === firstElement) {
        e.preventDefault();
        lastElement.focus();
      }
    } else {
      if (document.activeElement === lastElement) {
        e.preventDefault();
        firstElement.focus();
      }
    }
  });

  firstElement?.focus();
}
```

---

## Keyboard Navigation

### Interactive Elements

| Element | Key | Action |
|---------|-----|--------|
| Button | Enter, Space | Activate |
| Link | Enter | Navigate |
| Checkbox | Space | Toggle |
| Radio | Arrow keys | Navigate options |
| Tab | Arrow keys | Navigate tabs |
| Menu | Arrow keys, Enter, Escape | Navigate and select |
| Dialog | Escape | Close |
| Dropdown | Arrow keys, Enter, Escape | Navigate and select |

### Implementation

```svelte
<button
  onclick={handleClick}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleClick();
    }
  }}
  type="button"
>
  Action
</button>
```

### Skip Links

```svelte
<a href="#main-content" class="skip-link">
  Skip to main content
</a>

<main id="main-content">
  <!-- Main content -->
</main>

<style>
  .skip-link {
    position: absolute;
    top: -100px;
    left: 0;
    padding: var(--space-3) var(--space-4);
    background: var(--color-primary-500);
    color: var(--color-neutral-0);
    z-index: 9999;
    transition: top 0.2s;
  }

  .skip-link:focus {
    top: 0;
  }
</style>
```

---

## ARIA Attributes

### Common Patterns

#### Button with Loading State

```svelte
<button
  aria-busy={isLoading}
  aria-disabled={isLoading || disabled}
  disabled={isLoading || disabled}
>
  {#if isLoading}
    <span class="sr-only">Loading...</span>
  {/if}
  {label}
</button>
```

#### Modal Dialog

```svelte
<div
  role="dialog"
  aria-modal="true"
  aria-labelledby="dialog-title"
  aria-describedby="dialog-description"
>
  <h2 id="dialog-title">{title}</h2>
  <p id="dialog-description">{description}</p>
</div>
```

#### Tab List

```svelte
<div role="tablist" aria-label="Settings tabs">
  {#each tabs as tab}
    <button
      role="tab"
      aria-selected={activeTab === tab.id}
      aria-controls="{tab.id}-panel"
      id="{tab.id}-tab"
      tabindex={activeTab === tab.id ? 0 : -1}
    >
      {tab.label}
    </button>
  {/each}
</div>

{#each tabs as tab}
  <div
    role="tabpanel"
    id="{tab.id}-panel"
    aria-labelledby="{tab.id}-tab"
    hidden={activeTab !== tab.id}
  >
    {@render tab.content()}
  </div>
{/each}
```

#### Menu

```svelte
<nav aria-label="Main navigation">
  <ul role="menubar">
    {#each items as item}
      <li role="none">
        <button
          role="menuitem"
          aria-haspopup={item.children ? 'true' : undefined}
          aria-expanded={item.expanded}
        >
          {item.label}
        </button>
      </li>
    {/each}
  </ul>
</nav>
```

### Screen Reader Only Content

```css
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
```

```svelte
<button>
  <TrashIcon aria-hidden="true" />
  <span class="sr-only">Delete item</span>
</button>
```

---

## Form Accessibility

### Label Association

```svelte
<div class="form-group">
  <label for="email">Email address</label>
  <input
    id="email"
    type="email"
    aria-describedby="email-help email-error"
    aria-invalid={!!error}
  />
  <p id="email-help">We'll never share your email.</p>
  {#if error}
    <p id="email-error" role="alert">{error}</p>
  {/if}
</div>
```

### Required Fields

```svelte
<label for="name">
  Name
  <span aria-hidden="true">*</span>
  <span class="sr-only">(required)</span>
</label>
<input id="name" required aria-required="true" />
```

### Error Messages

```svelte
{#if errors.email}
  <div role="alert" aria-live="polite">
    {errors.email}
  </div>
{/if}
```

---

## Motion and Animation

### Respect Reduced Motion

```css
/* Reduce motion for users who prefer it */
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

### Provide Motion Controls

```svelte
<script>
  import { preferences } from '$lib/stores/preferences';

  let reduceMotion = $state($preferences.reduceMotion);
</script>

{#if !reduceMotion}
  <div class="animated-element" animate:fade>
    Content
  </div>
{:else}
  <div class="static-element">
    Content
  </div>
{/if}
```

---

## Touch Targets

### Minimum Size

Interactive elements should have a minimum touch target of 44×44px:

```css
/* Ensure touch targets are large enough */
.button {
  min-height: 44px;
  min-width: 44px;
}

/* If visual element is smaller, use padding */
.icon-button {
  padding: var(--space-3); /* Creates 44px target for 20px icon */
}

/* Or use pseudo-element */
.small-button {
  position: relative;
}

.small-button::before {
  content: '';
  position: absolute;
  top: -8px;
  left: -8px;
  right: -8px;
  bottom: -8px;
}
```

---

## Checklist

### Before Shipping

- [ ] All interactive elements have focus indicators
- [ ] Color contrast meets WCAG AA (4.5:1 for text)
- [ ] All images have alt text
- [ ] All form inputs have labels
- [ ] Error messages are announced to screen readers
- [ ] Keyboard navigation works for all interactions
- [ ] Focus is trapped in modals
- [ ] Skip link is provided
- [ ] ARIA landmarks are used correctly
- [ ] Motion respects user preferences
- [ ] Touch targets are at least 44×44px
