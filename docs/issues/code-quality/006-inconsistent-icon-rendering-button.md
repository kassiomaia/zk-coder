# [ISSUE-006] - Inconsistent Icon Rendering in Button Component

## Status
✅ **RESOLVED** (2026-03-04)

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-Button.svelte.md`
- **Component/File**: `src-ui/src/lib/components/Button.svelte`
- **Line Numbers**: TBD

## Description
The Button component had duplicated button markup (one with Tooltip wrapper, one without). This created visual inconsistencies and maintenance challenges.

## Impact
Code Quality - This affects:
- Code duplication
- Maintenance difficulty
- Potential for inconsistency between the two button variants

## Resolution
Refactored the Button component to use Svelte 5 snippets (`{#snippet}`) to define reusable button content. This eliminates the duplicated markup:

**Changes Made:**
- Created `{#snippet renderSpinner()}` for the loading spinner
- Created `{#snippet renderButton()}` for the button element
- Used conditional rendering to wrap with Tooltip only when needed
- Both tooltip and non-tooltip paths now render the same button content via `{@render renderButton()}`

## Files Changed
- `src-ui/src/lib/components/shared/Button.svelte` - Refactored to use snippets

## Related Issues
- None
