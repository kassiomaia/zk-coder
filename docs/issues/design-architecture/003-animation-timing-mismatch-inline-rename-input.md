# [ISSUE-003] - Animation Timing Mismatch in InlineRenameInput

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-InlineRenameInput/InlineRenameInput.svelte-review.md`
- **Component/File**: `src-ui/src/lib/components/InlineRenameInput.svelte`
- **Line Numbers**: TBD

## Description
The InlineRenameInput component has animation timing mismatches between the input field and its surrounding elements. This creates visual jank and poor user experience during rename operations.

## Impact
Design Architecture - This affects:
- Visual polish
- User experience
- Animation quality
- Component cohesion

## Suggested Fix
Synchronize animation timing:

```typescript
// Define timing constants
const ANIMATION_TIMING = {
  duration: 200,
  easing: 'ease-in-out',
  stagger: 50
};

// Use reactive timing
$: inputWidth = $derived(() => {
  return text.length * 8 + 40; // Approximate character width
});

// In template
<div class="rename-container">
  <div
    class="rename-label"
    transition:opacity={{ duration: ANIMATION_TIMING.duration }}
  >
    {label}
  </div>
  <input
    bind:value={text}
    bind:width={inputWidth}
    transition:all={{
      duration: ANIMATION_TIMING.duration,
      easing: ANIMATION_TIMING.easing
    }}
  />
</div>

// CSS approach
.rename-container {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  transition: all var(--animation-duration-base) var(--easing-base);
}

.rename-label {
  opacity: 0.8;
  transition: opacity var(--animation-duration-base) var(--easing-base);
}

.rename-input {
  flex: 1;
  transition: all var(--animation-duration-base) var(--easing-base);
}
```

## Related Issues
- None