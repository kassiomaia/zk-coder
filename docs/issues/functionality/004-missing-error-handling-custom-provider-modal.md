# Missing error handling in CustomProviderModal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md
- **Location**: src-ui/src/lib/components/config/CustomProviderModal.svelte:112
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
Error messages are direct and technical, which might confuse users who aren't familiar with API configurations.

## Impact
Users may not understand what went wrong when API configuration fails, leading to frustration and difficulty troubleshooting configuration issues.

## Suggested Fix
Provide more descriptive error messages that explain what went wrong and suggest potential solutions.

```typescript
// Example fix for CustomProviderModal.svelte
const handleSubmit = async () => {
  if (!validateForm()) return;

  try {
    isSubmitting = true;
    error = null;

    // API call to save provider
    await saveProvider(provider);

    // Success handling
    onClose();
  } catch (err) {
    // Map technical errors to user-friendly messages
    error = getFriendlyErrorMessage(err);

    // Example mapping function:
    function getFriendlyErrorMessage(error: any) {
      if (error.message?.includes('invalid_url')) {
        return 'Please enter a valid URL. Make sure it starts with https:// and includes the full endpoint.';
      }
      if (error.message?.includes('authentication_failed')) {
        return 'Authentication failed. Please check your API key and try again.';
      }
      if (error.message?.includes('network_error')) {
        return 'Unable to connect to the server. Please check your internet connection.';
      }
      return 'Failed to save configuration. Please check your inputs and try again.';
    }
  } finally {
    isSubmitting = false;
  }
};
```

## Related Components
- CustomProviderModal.svelte
- Error notification system
- Form validation system

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md