# [ISSUE-003] - Missing Dialog Attributes in ConfirmDialog

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-confirm-dialog-review.md`
- **Component/File**: `src-ui/src/lib/components/ConfirmDialog.svelte`
- **Line Numbers**: TBD

## Description
The ConfirmDialog component lacks proper ARIA dialog attributes. It doesn't inform screen readers about its role as a dialog, set focus appropriately, or provide proper announcement of dialog content to assistive technology users.

## Impact
Accessibility - This fails WCAG guidelines by:
- Violating WCAG 4.1.2 (Name, Role, Value)
- Not properly announcing dialog content to screen readers
- Not trapping focus within the dialog
- Creating confusion for assistive technology users

## Suggested Fix
Implement proper dialog ARIA attributes:

```typescript
<div
  role="dialog"
  aria-modal="true"
  aria-labelledby="dialog-title"
  aria-describedby="dialog-description"
  class="dialog-overlay"
>
  <div class="dialog-content">
    <h2 id="dialog-title">{title}</h2>
    <p id="dialog-description">{message}</p>
    <div class="dialog-actions">
      <button on:click={onCancel}>Cancel</button>
      <button on:click={onConfirm}>Confirm</button>
    </div>
  </div>
</div>

// Also implement focus trapping
$: {
  if (open) {
    // Focus first element and trap focus
    setTimeout(() => {
      const firstButton = document.querySelector('.dialog-actions button');
      firstButton?.focus();
    }, 100);
  }
}
```

## Related Issues
- None

## Resolution Status
**RESOLVED** - 2026-03-04

## Proposed Solution
The Modal component (which ConfirmDialog uses) already had proper ARIA dialog attributes:
- `role="dialog"`
- `aria-modal="true"`
- `aria-labelledby` pointing to the title

The fix adds `aria-describedby` support for better screen reader announcement of dialog content:

**Modal.svelte changes:**
1. Added `describedby?: string` prop
2. Added `aria-describedby={describedby}` to the dialog element

**ConfirmDialog.svelte changes:**
1. Added `id="confirm-dialog-message"` to the message paragraph
2. Passed `describedby="confirm-dialog-message"` to Modal

This follows WCAG 2.1 guidelines for dialog patterns.