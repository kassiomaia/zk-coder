# URL validation limitation in CustomProviderModal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md
- **Location**: src-ui/src/lib/components/config/CustomProviderModal.svelte:79-84
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
The current validation only checks if the URL is well-formed but doesn't validate that it's a valid HTTPS URL.

## Impact
Users might enter HTTP URLs or malformed URLs, leading to failed API connections and poor user experience.

## Suggested Fix
Add additional validation to ensure the URL uses HTTPS protocol.

```typescript
// Example fix for CustomProviderModal.svelte
const validateUrl = (url: string): boolean => {
  // Clear previous error
  clearFieldError('base_url');

  // Basic URL format check
  if (!url || typeof url !== 'string') {
    showFieldError('base_url', 'URL is required');
    return false;
  }

  // Trim whitespace
  const trimmedUrl = url.trim();

  // Check for empty string after trim
  if (!trimmedUrl) {
    showFieldError('base_url', 'URL cannot be empty');
    return false;
  }

  // Validate URL structure using regex
  const urlPattern = /^https?:\/\/.+$/;
  if (!urlPattern.test(trimmedUrl)) {
    showFieldError('base_url', 'URL must start with http:// or https://');
    return false;
  }

  // Check for HTTPS requirement
  if (!trimmedUrl.startsWith('https://')) {
    // Allow HTTP for local development
    if (!trimmedUrl.startsWith('http://localhost:') &&
        !trimmedUrl.startsWith('http://127.0.0.1:')) {
      showFieldError('base_url', 'Production URLs must use HTTPS protocol');
      return false;
    }
  }

  // Additional validation: check for valid domain structure
  try {
    const urlObj = new URL(trimmedUrl);

    // Check for valid hostname
    if (!urlObj.hostname || urlObj.hostname === '') {
      showFieldError('base_url', 'Invalid hostname');
      return false;
    }

    // Check for reasonable path
    if (!urlObj.pathname || urlObj.pathname === '/') {
      // Optional: require non-root path for API endpoints
      showFieldError('base_url', 'API endpoint must include a path');
      return false;
    }

  } catch (error) {
    showFieldError('base_url', 'Invalid URL format');
    return false;
  }

  return true;
};

// On input change
const handleUrlChange = (e: Event) => {
  const value = (e.target as HTMLInputElement).value;
  provider.base_url = value;

  // Validate in real-time
  if (value) {
    validateUrl(value);
  } else {
    clearFieldError('base_url');
  }
};

// On blur - final validation
const handleUrlBlur = (e: Event) => {
  validateUrl((e.target as HTMLInputElement).value);
};
```

## Related Components
- CustomProviderModal.svelte
- Form validation system
- URL validation utility

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md