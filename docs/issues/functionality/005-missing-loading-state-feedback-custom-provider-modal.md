# Missing loading state feedback in CustomProviderModal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md
- **Location**: src-ui/src/lib/components/config/CustomProviderModal.svelte:94-115
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
While there is an isSubmitting state, there's no visual feedback during the save operation beyond the spinner.

## Impact
Users may not be aware that their configuration is being saved, leading to confusion about whether the save operation completed successfully or if the application is unresponsive.

## Suggested Fix
Add a progress indicator or status message during the save operation.

```typescript
// Example fix for CustomProviderModal.svelte
const handleSubmit = async () => {
  if (!validateForm()) return;

  isSubmitting = true;
  saveProgress = 'Initializing...'; // Add progress state

  try {
    saveProgress = 'Validating configuration...';
    if (!validateForm()) return;

    saveProgress = 'Saving configuration...';
    await saveProvider(provider);

    saveProgress = 'Configuration saved successfully!';
    setTimeout(() => {
      onClose();
    }, 1000); // Brief delay to show success message
  } catch (error) {
    error = error.message || 'Failed to save configuration';
  } finally {
    isSubmitting = false;
    saveProgress = null;
  }
};

// In template:
{#if isSubmitting}
  <div class="save-status">
    <Spinner />
    <span>{saveProgress || 'Saving...'}</span>
  </div>
{/if}
```

## Related Components
- CustomProviderModal.svelte
- Spinner component
- Toast/Notification system

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md