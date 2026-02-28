# Animations & Transitions

Guidelines for implementing motion in the Coder application.

## Transition Tokens

| Token | Duration | Usage |
|-------|----------|-------|
| `--transition-fast` | 150ms ease | Micro-interactions, hover states |
| `--transition-normal` | 200ms ease | Standard transitions |
| `--transition-slow` | 300ms ease | Theme changes, sidebar collapse |

---

## Common Patterns

### Button Interactions

```css
.button {
  transition:
    background-color var(--transition-fast),
    border-color var(--transition-fast),
    transform var(--transition-fast);
}

.button:hover {
  background: var(--color-surface-hover);
}

.button:active {
  transform: scale(0.98);
}

.button:focus-visible {
  transition: box-shadow var(--transition-fast);
}
```

### Theme Transitions

```css
/* Smooth theme switching */
.card {
  transition:
    background-color var(--transition-slow),
    color var(--transition-slow),
    border-color var(--transition-slow);
}
```

### Sidebar Collapse

```css
.sidebar {
  width: var(--sidebar-expanded);
  transition: width var(--transition-slow);
}

.sidebar.collapsed {
  width: var(--sidebar-collapsed);
}

.sidebar-content {
  transition: opacity var(--transition-fast);
}

.sidebar.collapsed .sidebar-content {
  opacity: 0;
}
```

---

## Keyframe Animations

### Message Appearance

```css
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

.message {
  animation: messageSlideIn var(--transition-slow) ease-out;
}
```

### Fade In/Out

```css
@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes fadeOut {
  from { opacity: 1; }
  to { opacity: 0; }
}

.modal {
  animation: fadeIn var(--transition-normal);
}

.modal.closing {
  animation: fadeOut var(--transition-normal);
}
```

### Scale In/Out

```css
@keyframes scaleIn {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

.dropdown {
  animation: scaleIn var(--transition-fast);
  transform-origin: top;
}
```

### Typing Indicator

```css
@keyframes typingPulse {
  0%, 60%, 100% {
    opacity: 0.3;
  }
  30% {
    opacity: 1;
  }
}

.typing-dot {
  animation: typingPulse 1.4s infinite;
}

.typing-dot:nth-child(2) {
  animation-delay: 0.2s;
}

.typing-dot:nth-child(3) {
  animation-delay: 0.4s;
}
```

### Spin

```css
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.spinner {
  animation: spin 1s linear infinite;
}
```

### Pulse

```css
@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.loading-indicator {
  animation: pulse 2s ease-in-out infinite;
}
```

### Shimmer

```css
@keyframes shimmer {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}

.skeleton {
  background: linear-gradient(
    90deg,
    var(--color-surface) 25%,
    var(--color-surface-hover) 50%,
    var(--color-surface) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite;
}
```

---

## Transition Utilities

### Svelte Transitions

```svelte
<script>
  import { fade, fly, slide, scale } from 'svelte/transition';
  import { quintOut } from 'svelte/easing';
</script>

{#if show}
  <!-- Fade -->
  <div transition:fade={{ duration: 200 }}>

  <!-- Fly (slide + fade) -->
  <div transition:fly={{ y: 20, duration: 200 }}>

  <!-- Slide -->
  <div transition:slide={{ duration: 300, easing: quintOut }}>

  <!-- Scale -->
  <div transition:scale={{ duration: 200 }}>
{/if}
```

### Crossfade for Lists

```svelte
<script>
  import { crossfade } from 'svelte/transition';

  const [send, receive] = crossfade({
    duration: 300,
    fallback: fade
  });
</script>

{#each items as item (item.id)}
  <div use:send={{ key: item.id }}>
    {item.name}
  </div>
{/each}
```

---

## Animation Performance

### Use Transform and Opacity

```css
/* GOOD - GPU accelerated */
.performant-animation {
  transition: transform 0.3s, opacity 0.3s;
}

/* AVOID - Triggers layout */
.poor-animation {
  transition: width 0.3s, height 0.3s, left 0.3s, top 0.3s;
}
```

### Will Change Hint

```css
/* Hint browser to optimize */
.will-animate {
  will-change: transform, opacity;
}

/* Remove when animation complete */
.will-animate.done {
  will-change: auto;
}
```

### Avoid Layout Thrashing

```javascript
// BAD - Forces layout recalculation
element.style.width = '100px';
const height = element.offsetHeight; // Forces layout
element.style.height = height + 'px';

// GOOD - Batch reads and writes
const height = element.offsetHeight; // Read
element.style.width = '100px';       // Write
element.style.height = height + 'px'; // Write
```

---

## Accessibility

### Respect Reduced Motion

```css
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

### Provide Alternatives

```svelte
<script>
  import { prefersReducedMotion } from '$lib/stores/preferences';

  let reduceMotion = $state($prefersReducedMotion);
</script>

{#if reduceMotion}
  <div class="static">{content}</div>
{:else}
  <div class="animated" animate:fly>{content}</div>
{/if}
```

---

## Common Animation Patterns

### Modal Open/Close

```css
.modal-backdrop {
  animation: fadeIn var(--transition-normal);
}

.modal-content {
  animation: scaleIn var(--transition-normal);
}

.modal-backdrop.closing {
  animation: fadeOut var(--transition-normal);
}

.modal-content.closing {
  animation: scaleOut var(--transition-normal);
}

@keyframes scaleOut {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(0.95);
  }
}
```

### Notification Toast

```css
.toast {
  animation:
    slideInRight var(--transition-normal),
    fadeOut var(--transition-normal) 4.7s forwards;
  animation-delay: 0s, 4.7s;
}

@keyframes slideInRight {
  from {
    opacity: 0;
    transform: translateX(100%);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}
```

### List Item Stagger

```css
.list-item {
  animation: fadeInUp var(--transition-normal) ease-out;
  animation-fill-mode: both;
}

.list-item:nth-child(1) { animation-delay: 0ms; }
.list-item:nth-child(2) { animation-delay: 50ms; }
.list-item:nth-child(3) { animation-delay: 100ms; }
.list-item:nth-child(4) { animation-delay: 150ms; }
.list-item:nth-child(5) { animation-delay: 200ms; }

@keyframes fadeInUp {
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

### Code Block Highlight

```css
@keyframes highlightPulse {
  0% {
    background-color: rgba(99, 102, 241, 0.3);
  }
  100% {
    background-color: rgba(99, 102, 241, 0.1);
  }
}

.code-line.highlighted {
  animation: highlightPulse 1s ease-out;
}
```
