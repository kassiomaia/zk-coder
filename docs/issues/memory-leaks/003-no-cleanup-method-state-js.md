# [ISSUE-003] - No Cleanup Method in State Management

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-state.js-review.md`
- **Component/File**: `src-ui/src/lib/state/state.js`
- **Line Numbers**: TBD

## Description
The state management system subscribes to various events and data sources but doesn't provide proper cleanup methods. This prevents unsubscribing from events and leads to memory leaks when components are destroyed.

## Impact
Memory Leaks - This causes:
- Unsubscribed event listeners remaining in memory
- Memory accumulation over time
- Potential memory exhaustion
- Performance degradation in long-running applications

## Suggested Fix
Implement cleanup methods for state management:

```javascript
class StateManager {
  constructor() {
    this.subscriptions = new Map();
    this.listeners = new Map();
  }

  // Add subscription with cleanup tracking
  subscribe(key, callback) {
    if (!this.subscriptions.has(key)) {
      this.subscriptions.set(key, new Set());
    }

    this.subscriptions.get(key).add(callback);

    // Return cleanup function
    return () => {
      this.subscriptions.get(key)?.delete(callback);
      if (this.subscriptions.get(key)?.size === 0) {
        this.subscriptions.delete(key);
      }
    };
  }

  // Add event listener with cleanup
  addEventListener(event, callback) {
    window.addEventListener(event, callback);

    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }

    this.listeners.get(event).add(callback);
  }

  // Cleanup all subscriptions
  cleanup() {
    // Remove all event listeners
    for (const [event, callbacks] of this.listeners) {
      for (const callback of callbacks) {
        window.removeEventListener(event, callback);
      }
    }

    // Clear all data
    this.subscriptions.clear();
    this.listeners.clear();
  }
}
```

## Related Issues
- None