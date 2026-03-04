# [ISSUE-007] - Duplicate Validation Logic in ConfigModal

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer-ConfigModal.svelte-2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/config/ConfigModal.svelte`
- **Line Numbers**: TBD

## Description
The ConfigModal component contains duplicate validation logic for API keys. The same validation patterns are repeated across different provider configurations, creating maintenance overhead and potential inconsistencies.

## Impact
Code Quality - This affects:
- Maintenance difficulty
- Code duplication
- Inconsistent validation
- Increased bug potential

## Suggested Fix
Extract validation logic into shared utility functions:

```typescript
// Create validation utilities
const validators = {
  apiKey: (key: string, provider: string) => {
    if (!key || key.trim() === '') {
      return { valid: false, error: 'API key is required' };
    }

    // Provider-specific validation
    switch (provider) {
      case 'openai':
        return key.startsWith('sk-')
          ? { valid: true }
          : { valid: false, error: 'OpenAI API key must start with sk-' };
      case 'claude':
        return key.startsWith('sk-ant-')
          ? { valid: true }
          : { valid: false, error: 'Claude API key must start with sk-ant-' };
      default:
        return { valid: true };
    }
  },

  modelId: (modelId: string) => {
    if (!modelId || modelId.trim() === '') {
      return { valid: false, error: 'Model ID is required' };
    }
    return { valid: true };
  }
};

// Reusable validation hook
function useValidation(value: string, validator: (value: string) => ValidationResult) {
  const error = $derived(() => {
    const result = validator(value);
    return result.valid ? null : result.error;
  });

  const isValid = $derived(() => error === null);

  return { error, isValid };
}

// Usage in component
const { error: apiKeyError, isValid: isApiKeyValid } = useValidation(
  apiKey,
  (key) => validators.apiKey(key, selectedProvider?.id || '')
);
```

## Related Issues
- None