# [ISSUE-003] - Hardcoded Animation Delay in Inline Thread List

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-InlineThreadList.svelte.md`
- **Component/File**: `src-ui/src/lib/components/chat/sidebar/InlineThreadList.svelte`
- **Line Numbers**: 76

## Description
The InlineThreadList component had hardcoded animation delays that were not configurable through design system tokens. This made animations difficult to adjust for different contexts or accessibility needs.

## Impact
Code Quality - This affects:
- Difficult maintenance
- Inconsistent animation timing
- Hard to adapt for accessibility
- Reduced code reusability

## Resolution Status
**RESOLVED** - 2026-03-04

## Proposed Solution
1. Added new animation stagger tokens to design system (`design-system.css`):
   - `--stagger-fast: 30ms`
   - `--stagger-normal: 50ms`
   - `--stagger-slow: 80ms`

2. Updated `InlineThreadList.svelte` to use design system tokens:
   ```svelte
   style="animation-delay: calc(var(--stagger-normal) * {index})"
   ```

3. Updated `ConversationSidebar.svelte` to use design system tokens:
   ```svelte
   style="animation-delay: calc({(groupOffset + index)} * var(--stagger-normal))"
   ```

4. Updated component documentation to reference design system tokens.

## Files Changed
- `src-ui/src/lib/styles/design-system.css` - Added stagger tokens
- `src-ui/src/lib/components/chat/sidebar/InlineThreadList.svelte` - Updated animation delay
- `src-ui/src/lib/components/chat/sidebar/ConversationSidebar.svelte` - Updated animation delay

## Related Issues
- Issue 001 - Duplicate CSS in ConversationItem Component
- Issue 002 - Conflicting Icon Styles in Search Bar