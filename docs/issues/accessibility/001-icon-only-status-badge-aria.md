# [ISSUE-001] - Icon-Only Status Badge Missing ARIA Labels

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-StatusBadge-review.md`
- **Component/File**: `src-ui/src/lib/components/StatusBadge.svelte`
- **Line Numbers**: TBD

## Description
The StatusBadge component uses only icons without text labels or proper ARIA attributes. This makes it inaccessible to screen reader users who cannot perceive the visual icons and don't know what status is being conveyed.

## Impact
Accessibility - This fails WCAG guidelines by:
- Excluding screen reader users from understanding status information
- Violating WCAG 1.1.1 (Non-text Content)
- Making the application unusable for visually impaired users
- Creating a barrier for users who rely on assistive technologies

## Suggested Fix
Add proper ARIA labels and consider adding visible text:

```typescript
<StatusBadge variant={status}>
  <span class="sr-only">{getStatusText(status)}</span>
  <Icon variant={status} />
</StatusBadge>

// Or use aria-label for icon-only version
<StatusBadge variant={status} aria-label={getStatusText(status)}>
  <Icon variant={status} />
</StatusBadge>

function getStatusText(status: Status): string {
  switch (status) {
    case 'online': return 'Status: Online';
    case 'offline': return 'Status: Offline';
    case 'busy': return 'Status: Busy';
    default: return 'Status: Unknown';
  }
}
```

## Related Issues
- None

## Resolution Status
**RESOLVED** - False Positive

The StatusBadge component already has proper ARIA accessibility implemented:
- `aria-label="Status: {displayLabel}"` is set on the root element
- The component displays text labels by default (not icon-only)
- In compact mode, the aria-label still provides screen reader text
- For resolved status with icon, the icon is accompanied by aria-label context

## Proposed Solution
No changes required. The existing implementation properly handles accessibility:
```svelte
<span class={containerClasses} aria-label="Status: {displayLabel}">
```

Date: 2026-03-04