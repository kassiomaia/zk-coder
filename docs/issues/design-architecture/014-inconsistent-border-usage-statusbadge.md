# Inconsistent Border Usage in StatusBadge

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-10-30-00-StatusBadge-review.md
- **Location**: src-ui/src/lib/components/chat/shared/StatusBadge.module.css:24-41
- **Priority**: Low
- **Category**: Design & Architecture
- **Status**: Open

## Description
The styling uses borders inconsistently across variants - open has a border, resolved and pending don't. The StatusBadge component has different styling patterns for different variants, with the "open" variant using a border while "resolved" and "pending" variants don't. This inconsistency creates visual imbalance and makes the component feel unfinished or inconsistent with the design system.

## Impact
- Visual inconsistency across badge variants
- Design system inconsistency
- Users may perceive different variants as having different importance levels
- Maintenance burden when adding new variants
- Inconsistent visual hierarchy

## Suggested Fix
Apply consistent border styling across all variants or consider using outline/border consistently.

Option 1: All variants use borders
```css
.status-badge {
  padding: 4px 12px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  display: inline-block;
  border: 1px solid;
}

.status-badge.open {
  background: var(--color-background-open);
  color: var(--color-text-open);
  border-color: var(--color-border-open);
}

.status-badge.resolved {
  background: var(--color-background-resolved);
  color: var(--color-text-resolved);
  border-color: var(--color-border-resolved);
}

.status-badge.pending {
  background: var(--color-background-pending);
  color: var(--color-text-pending);
  border-color: var(--color-border-pending);
}
```

Option 2: No borders, use outlines or backgrounds only
```css
.status-badge {
  padding: 6px 16px;
  border-radius: 16px;
  font-size: 12px;
  font-weight: 500;
  display: inline-block;
  /* Remove borders */
}

.status-badge.open {
  background: var(--color-background-open);
  color: var(--color-text-open);
}

/* ... other variants ... */
```

## Related Components
- src-ui/src/lib/components/chat/shared/StatusBadge.svelte
- src-ui/src/lib/components/chat/shared/StatusBadge.module.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-10-30-00-StatusBadge-review.md