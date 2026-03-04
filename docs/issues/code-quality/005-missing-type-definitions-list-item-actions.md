# [ISSUE-005] - Missing Type Definitions for ListItem Actions

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/ListItemActions.svelte_2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/ListItemActions.svelte`
- **Line Numbers**: TBD

## Description
The ListItemActions component uses action callbacks without proper TypeScript type definitions. This reduces type safety and makes it difficult to understand what actions are available.

## Impact
Code Quality - This affects:
- Type safety
- Developer experience
- Code maintainability
- Runtime errors

## Suggested Fix
Add proper type definitions:

```typescript
interface ListItemAction {
  id: string;
  icon: string;
  label: string;
  variant?: 'primary' | 'secondary' | 'danger';
  disabled?: boolean;
  action: () => void;
}

interface ListItemActionsProps {
  actions: ListItemAction[];
  size?: 'sm' | 'md' | 'lg';
  density?: 'compact' | 'normal' | 'spacious';
}

// Use strict typing
function handleAction(actionId: string) {
  const action = props.actions.find(a => a.id === actionId);
  if (action && !action.disabled) {
    action.action();
  }
}
```

## Related Issues
- None