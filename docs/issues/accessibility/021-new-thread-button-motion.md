# Motion sensitivity in NewThreadButton

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-NewThreadButton.svelte.md
- **Location**: NewThreadButton.module.css:33
- **Priority**: Medium
- **Category**: Accessibility
- **Status**: Open

## Description
The scale transform on active state could trigger motion sensitivity issues for some users, despite the reduced motion media query. The abrupt scale change can be disorienting for users with vestibular disorders or other motion-related sensitivities.

## Impact
Users with motion sensitivities may experience discomfort when hovering or clicking the NewThreadButton, potentially causing them to avoid using this important feature of the application.

## Suggested Fix
Consider using opacity changes or background color changes instead of transform for active states.

```css
/* Fix example - Replace scale with safer transitions */
.new-thread-button {
  /* ... existing styles ... */
  transition: all 0.2s ease;

  /* Replace scale with opacity for active state */
  &:hover {
    opacity: 0.9;
    background-color: var(--color-primary-light);
  }

  &:active {
    opacity: 0.8;
    background-color: var(--color-primary-dark);
    transform: translateY(1px); /* Optional: minimal movement */
  }
}

/* Enhanced reduced motion support */
@media (prefers-reduced-motion: reduce) {
  .new-thread-button {
    transition: none;

    &:hover,
    &:active {
      transform: none;
      background-color: var(--color-primary);
      opacity: 1;
    }
  }
}
```

```html
<!-- Alternative with focus states -->
<button
  class="new-thread-button"
  aria-label="Create new thread"
  aria-pressed={isActive}
>
  <svg class="icon" aria-hidden="true">...</svg>
  <span class="label">New Thread</span>
</button>
```

```css
/* Alternative approach using focus states only */
.new-thread-button {
  /* ... existing styles ... */

  &:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  /* Remove hover/active animations for better accessibility */
  &:hover {
    background-color: var(--color-primary-light);
  }

  &:active {
    background-color: var(--color-primary);
  }
}

@media (prefers-reduced-motion: reduce) {
  .new-thread-button {
    transition: background-color 0s;

    &:hover,
    &:active {
      background-color: var(--color-primary);
    }
  }
}
```

## Related Components
- NewThreadButton.svelte
- NewThreadButton.module.css
- Sidebar.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-NewThreadButton.svelte.md
- prefers-reduced-motion: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion

## Resolution Status
**RESOLVED** - False Positive - 2026-03-04

The NewThreadButton component already has proper reduced motion support:

```css
@media (prefers-reduced-motion: reduce) {
  .button {
    transition: none;
  }

  .button:active:not(:disabled) {
    transform: none;
  }
}
```

The scale transform is minimal (0.98) and is disabled for users who prefer reduced motion. This follows WCAG 2.1 guidelines for motion sensitivity.