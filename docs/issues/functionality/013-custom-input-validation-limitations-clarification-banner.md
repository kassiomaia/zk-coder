# Custom input validation limitations in ClarificationBanner

## Metadata
- **Source Review**: docs/reviews/code-reviewer/clarification-banner-review-2026-03-04.md
- **Location**: ClarificationBanner.svelte:49
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
The custom input validation is minimal - only checking if the input has non-whitespace characters.

## Impact
Insufficient validation may allow invalid or meaningless inputs to be submitted, leading to poor AI responses or unexpected behavior.

## Suggested Fix
Add more comprehensive validation for custom input, especially considering the ambiguity type.

```typescript
// Example fix for ClarificationBanner.svelte
const validateCustomInput = (input: string): boolean => {
  // Trim input
  const trimmed = input.trim();

  // Basic validation: non-empty
  if (!trimmed) {
    validationError = 'Input cannot be empty';
    return false;
  }

  // Specific validations based on ambiguity type
  switch (ambiguityType) {
    case 'coding':
      // For coding inputs, require at least some technical content
      if (trimmed.length < 10) {
        validationError = 'Please provide more detail about your coding request';
        return false;
      }
      // Check for potential code patterns
      if (!/[a-zA-Z0-9_{}[\];]/.test(trimmed)) {
        validationError = 'Please include relevant coding terms or code snippets';
        return false;
      }
      break;

    case 'file':
      // For file references, check for file-like patterns
      if (!/\.[a-zA-Z0-9]+$/.test(trimmed)) {
        validationError = 'Please provide a valid file reference (e.g., "src/main.js")';
        return false;
      }
      break;

    case 'general':
      // For general inputs, check length
      if (trimmed.length > 500) {
        validationError = 'Input must be less than 500 characters';
        return false;
      }
      break;

    default:
      // Generic length check
      if (trimmed.length > 1000) {
        validationError = 'Input is too long';
        return false;
      }
  }

  validationError = null;
  return true;
};

// On custom input submit
const handleCustomSubmit = () => {
  const customInput = customResolution.trim();

  if (!validateCustomInput(customInput)) {
    return;
  }

  // Proceed with resolution
  onResolve({
    type: 'custom',
    value: customInput
  });
};
```

## Related Components
- ClarificationBanner.svelte
- Validation system
- AI interaction components

## References
- Original review: docs/reviews/code-reviewer/clarification-banner-review-2026-03-04.md