# Popover ID generation security

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-Popover-Review.md
- **Location**: src-ui/src/lib/components/shared/Popover.svelte:39
- **Priority**: Medium
- **Category**: Security
- **Status**: Open

## Description
Uses Math.random() for ID generation which is not guaranteed to be unique. Could cause issues if multiple popovers are created simultaneously.

## Impact
- Potential ID collisions between multiple popovers
- Could lead to DOM manipulation issues
- Accessibility problems with duplicate IDs
- Inconsistent behavior in rapid popover creation
- Security risk if IDs are used for sensitive operations

## Suggested Fix
Use a more robust ID generation method like crypto.randomUUID() or a counter.

```typescript
// Using crypto.randomUUID() (recommended)
function generatePopoverId(): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return `popover-${crypto.randomUUID()}`;
  }

  // Fallback to counter-based approach
  return generatePopoverIdWithCounter();
}

// Counter-based approach with uniqueness guarantee
let popoverCounter = 0;
const activePopovers = new Set<string>();

function generatePopoverIdWithCounter(): string {
  const id = `popover-${Date.now()}-${++popoverCounter}`;

  // Ensure uniqueness (though highly unlikely with this approach)
  if (activePopovers.has(id)) {
    return generatePopoverIdWithCounter(); // Recursive call
  }

  activePopovers.add(id);
  return id;
}

// Better approach: Use a factory pattern with proper cleanup
class PopoverIdGenerator {
  private static counter = 0;
  private static activePopovers = new Map<string, number>();

  static generate(): string {
    if (typeof crypto !== 'undefined' && crypto.randomUUID) {
      const id = `popover-${crypto.randomUUID()}`;
      this.activePopovers.set(id, Date.now());
      return id;
    }

    // Fallback method
    const timestamp = Date.now();
    const counter = ++this.counter;
    const id = `popover-${timestamp}-${counter}`;

    this.activePopovers.set(id, timestamp);
    return id;
  }

  static release(id: string): void {
    this.activePopovers.delete(id);
  }

  static isUnique(id: string): boolean {
    return !this.activePopovers.has(id);
  }
}

// Usage in Popover.svelte
let popoverId: string;

onMount(() => {
  popoverId = PopoverIdGenerator.generate();
});

onDestroy(() => {
  PopoverIdGenerator.release(popoverId);
});

// Alternative: Using a library like nanoid
import { nanoid } from 'nanoid';

function generateNanoidPopover(): string {
  return `popover-${nanoid(8)}`;
}

// Even simpler: Use Symbol for component-scoped IDs
const popoverSymbol = Symbol('popover');

function generateSymbolPopover(): symbol {
  return Symbol('popover');
}
```

## Related Components
- Popover.svelte
- Tooltip component (similar pattern)
- Modal components
- Accessibility components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-Popover-Review.md
- crypto.randomUUID MDN: https://developer.mozilla.org/en-US/docs/Web/API/Crypto/randomUUID
- nanoid Documentation: https://github.com/ai/nanoid
- Unique ID generation best practices: https://stackoverflow.com/a/2117523