# [ISSUE-004] - Global State Pollution with Window Icons

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/ListItemActions.svelte_2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/ListItemActions.svelte`
- **Line Numbers**: TBD

## Description
The component directly modifies global window object properties for icon management. This creates global state pollution and prevents proper garbage collection of icon resources.

## Impact
Memory Leaks - This causes:
- Global namespace pollution
- Memory leaks from uncleaned icon references
- Difficult to track memory usage
- Potential conflicts with other code

## Suggested Fix
Use encapsulated state management for icons:

```typescript
// Use a reactive store for icon management
const iconStore = writable(new Map());

// Component code
$: {
  // Get icons from store instead of window
  const iconMap = $iconStore;
  const icon = iconMap.get(iconName);

  // Cleanup on unmount
  onDestroy(() => {
    // Remove from store if no longer needed
    iconStore.update(map => {
      if (!Array.from(map.values()).some(ref => ref !== icon)) {
        map.delete(iconName);
        // Clean up icon resource
        icon?.destroy();
      }
      return map;
    });
  });
}

// Alternative: Use WeakMap for better memory management
const iconCache = new WeakMap();

function getIcon(name: string) {
  if (!iconCache.has(name)) {
    const icon = createIcon(name);
    iconCache.set(name, icon);
  }
  return iconCache.get(name);
}
```

## Related Issues
- None