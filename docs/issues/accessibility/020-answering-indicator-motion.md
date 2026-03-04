# Motion sensitivity in AnsweringIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/AnsweringIndicator.svelte:119
- **Priority**: Low
- **Category**: Accessibility
- **Status**: Open

## Description
The text scrambling animation might be disorienting for users with motion sensitivities. While there's a reduced motion media query, the animation might still be too intense or unpredictable for users with vestibular disorders or photosensitive epilepsy.

## Impact
Users with motion sensitivities may experience discomfort, dizziness, or other adverse reactions when the scrambling animation is triggered, potentially causing them to avoid using the application during AI processing times.

## Suggested Fix
Implement stronger reduced motion support that completely disables animations and shows stable states only.

```html
<!-- Fix example with enhanced reduced motion support -->
<div
  class="answering-indicator"
  aria-live="polite"
  aria-label="AI is typing"
>
  <div class="indicator-text" class:animate={!reducedMotion}>
    {#if reducedMotion}
      <!-- Static state for users with motion sensitivities -->
      <span class="static-text">AI is thinking...</span>
    {:else}
      <!-- Animated version for other users -->
      <span class="scrambling-text">{scramblingText}</span>
    {/if}
  </div>
</div>
```

```css
/* Enhanced motion handling */
:root {
  --animation-speed: 0.1s;
}

@media (prefers-reduced-motion: reduce) {
  .answering-indicator .indicator-text.animate {
    animation: none;
    opacity: 1;
  }

  .answering-indicator .static-text {
    animation: none;
    opacity: 1;
    text-decoration: none;
  }
}

@keyframes scramble {
  /* Simplified animation for better accessibility */
  0% { opacity: 0.8; }
  50% { opacity: 1; }
  100% { opacity: 0.8; }
}
```

```javascript
// Enhanced motion detection
const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
let reducedMotion = prefersReducedMotion.matches;

// Listen for changes in motion preference
prefersReducedMotion.addEventListener('change', (e) => {
  reducedMotion = e.matches;
});
```

## Related Components
- AnsweringIndicator.svelte
- ChatArea.svelte
- TypingIndicator.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
- prefers-reduced-motion: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion