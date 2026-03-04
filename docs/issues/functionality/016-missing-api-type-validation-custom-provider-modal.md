# Missing API type validation in CustomProviderModal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md
- **Location**: src-ui/src/lib/components/config/CustomProviderModal.svelte:183
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
The component allows users to select from three API types but doesn't validate that the selected API type is compatible with the entered base URL.

## Impact
Users may create configurations that won't work with the selected API type, leading to failed API calls and poor user experience.

## Suggested Fix
Add validation logic that checks API type compatibility with the base URL.

```typescript
// Example fix for CustomProviderModal.svelte
const validateApiTypeCompatibility = () => {
  const url = provider.base_url.toLowerCase();
  const apiType = provider.api_type;
  let isValid = true;
  let errorMessage = '';

  switch (apiType) {
    case 'openai':
      // OpenAI typically uses api.openai.com or custom domains
      if (!url.includes('openai.com') && !url.match(/^https?:\/\/[a-zA-Z0-9.-]+$/)) {
        isValid = false;
        errorMessage = 'OpenAI-compatible endpoints should be openai.com or a valid custom domain';
      }
      break;

    case 'anthropic':
      // Anthropic uses anthropic.com
      if (!url.includes('anthropic.com')) {
        isValid = false;
        errorMessage = 'Anthropic API must use an anthropic.com endpoint';
      }
      break;

    case 'custom':
      // For custom APIs, we accept any HTTPS URL
      if (!url.startsWith('https://')) {
        isValid = false;
        errorMessage = 'Custom API endpoints must use HTTPS protocol';
      }
      break;
  }

  if (!isValid) {
    showFieldError('api_type', errorMessage);
    return false;
  }

  return true;
};

// In form submit handler
const handleSubmit = async () => {
  // Clear previous errors
  clearFieldErrors();

  // Validate API type compatibility
  if (!validateApiTypeCompatibility()) {
    return;
  }

  // Continue with submit...
};
```

## Related Components
- CustomProviderModal.svelte
- Form validation system
- API service layer

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md