# Redundant SVG icon in EmptyThreadState

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-EmptyThreadState.svelte.md
- **Location**: EmptyThreadState.svelte:81-86,96-101
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
The message icon SVG is repeated in two different contexts ('none' and 'thread') with identical paths, violating the DRY principle.

## Impact
Duplicate code makes maintenance difficult, increases the risk of inconsistencies, and violates the DRY (Don't Repeat Yourself) principle.

## Suggested Fix
Extract the message icon into a separate component or constant to avoid duplication.

```javascript
// Create a MessageIcon component (MessageIcon.svelte)
export function MessageIcon({ size = 24, variant = 'outline' }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none">
      <!-- SVG path -->
    </svg>
  );
}

// In EmptyThreadState.svelte
import { MessageIcon } './MessageIcon.svelte';

<!-- Use the component instead of duplicate SVG -->
<MessageIcon variant={variant} size={24} />
```

## Related Components
- src-ui/src/lib/components/chat/EmptyThreadState.svelte
- src-ui/src/lib/components/icons/MessageIcon.svelte (new file)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-EmptyThreadState.svelte.md