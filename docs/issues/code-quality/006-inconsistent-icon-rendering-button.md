# [ISSUE-006] - Inconsistent Icon Rendering in Button Component

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-Button.svelte.md`
- **Component/File**: `src-ui/src/lib/components/Button.svelte`
- **Line Numbers**: TBD

## Description
The Button component has inconsistent icon rendering logic. Different button variants handle icons differently, creating visual inconsistencies and maintenance challenges.

## Impact
Code Quality - This affects:
- Visual consistency
- Maintenance difficulty
- Developer confusion
- User experience inconsistencies

## Suggested Fix
Standardize icon rendering:

```typescript
// Define consistent icon props
interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  icon?: string;
  iconPosition?: 'left' | 'right';
  loading?: boolean;
  disabled?: boolean;
}

// Use consistent icon rendering
{#if icon && !loading}
  <Icon
    name={icon}
    class:icon-left={iconPosition === 'left'}
    class:icon-right={iconPosition === 'right'}
    class:icon-only={!$slots.default}
  />
{/if}

{#if loading}
  <Icon name="spinner" class="animate-spin" />
{/if}
```

## Related Issues
- None