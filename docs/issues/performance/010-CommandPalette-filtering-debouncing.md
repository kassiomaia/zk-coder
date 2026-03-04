# Command filtering without debouncing in state.js

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-state.js-review.md
- **Location**: docs/prototypes/agent-coding-input/js/state.js:172-181
- **Priority**: Low
- **Category**: Performance
- **Status**: Open

## Description
Command palette filtering happens on every keystroke without debouncing, which could impact performance with large datasets.

## Impact
Immediate filtering on every keystroke can cause:
- Performance issues with large command lists
- Unnecessary re-renders
- UI lag during typing
- Increased CPU usage
- Poor user experience on slower devices

## Suggested Fix
Implement debouncing for the filter operations.

```javascript
// Current implementation (problematic)
const filterCommands = (query) => {
  setFilteredCommands(commands.filter(cmd =>
    cmd.name.toLowerCase().includes(query.toLowerCase())
  ));
};

// Optimized with debouncing
let debounceTimeout;
const filterCommandsDebounced = (query) => {
  clearTimeout(debounceTimeout);
  debounceTimeout = setTimeout(() => {
    setFilteredCommands(commands.filter(cmd =>
      cmd.name.toLowerCase().includes(query.toLowerCase())
    ));
  }, 100); // 100ms delay
};

// Alternative: Use a proper debounce function
const debounce = (fn, delay) => {
  let timeout;
  return (...args) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => fn(...args), delay);
  };
};

const filterCommandsDebounced = debounce((query) => {
  setFilteredCommands(commands.filter(cmd =>
    cmd.name.toLowerCase().includes(query.toLowerCase())
  ));
}, 100);
```

## Related Components
- CommandPalette.svelte
- state.js (prototype)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-state.js-review.md