# Missing virtual scrolling in CommandPalette

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_10-30-45_CommandPalette-Review.md
- **Location**: CommandPalette.svelte:89-208
- **Priority**: Low
- **Category**: Performance
- **Status**: Open

## Description
For large command lists, the current implementation could cause performance issues due to rendering all commands at once.

## Impact
Rendering all commands simultaneously can cause:
- Poor initial load performance
- Increased memory usage
- Slow scrolling through large lists
- UI lag when opening the palette
- Potential browser freezing on very large datasets

## Suggested Fix
Consider implementing virtual scrolling for better performance with large datasets.

```javascript
// Virtual scrolling implementation
import { VirtualList } from 'svelte-virtual-scroll';

const ITEM_HEIGHT = 40; // Approximate height of each command item
const VISIBLE_ITEMS = 10; // Number of visible items

function renderVirtualCommands(commands) {
  return new VirtualList({
    items: commands,
    height: VISIBLE_ITEMS * ITEM_HEIGHT,
    itemHeight: ITEM_HEIGHT,
    renderItem: (command, index) => {
      return `<CommandItem
        data-index="${index}"
        data-command="${command.name}"
        class:index === selectedIndex ? 'selected' : ''
      >${command.name}</CommandItem>`;
    }
  });
}

// Alternative: Implement lazy loading
let visibleCommands = $derived(commands.slice(0, INITIAL_LOAD_SIZE));
let isLoading = $state(false);
let selectedIndex = $state(0);

const handleScroll = (e) => {
  const scrollPosition = e.target.scrollTop;
  const scrollHeight = e.target.scrollHeight;
  const clientHeight = e.target.clientHeight;

  // Load more when near bottom
  if (scrollPosition + clientHeight >= scrollHeight - 100) {
    loadMoreCommands();
  }
};

function loadMoreCommands() {
  if (isLoading) return;

  isLoading = true;
  setTimeout(() => {
    const currentSize = visibleCommands.length;
    visibleCommands = commands.slice(0, currentSize + LOAD_INCREMENT);
    isLoading = false;
  }, 100);
}
```

## Related Components
- CommandPalette.svelte
- CommandItem components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_10-30-45_CommandPalette-Review.md