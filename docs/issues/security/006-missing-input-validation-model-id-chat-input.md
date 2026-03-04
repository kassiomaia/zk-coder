# Missing input validation for model ID in ChatInput

## Metadata
- **Source Review**: docs/reviews/claude-opus-4-6/2026-03-04-14-30-00-code-review-chat-input.md
- **Location**: ChatInput.svelte:432-434
- **Priority**: Low
- **Category**: Security
- **Status**: Open

## Description
Missing validation for model ID prop.

## Impact
- Undefined or invalid model IDs could cause runtime errors
- Potential for null/undefined to be passed to API calls
- Could lead to unpredictable behavior in message processing
- May cause crashes when model ID is used in API requests

## Suggested Fix
Add type checking and validation for modelId prop to prevent runtime errors.

```typescript
// Type definition for model ID
type ModelId = 'claude-3-opus-20240229' | 'claude-3-sonnet-20240229' | 'claude-3-haiku-20240307' | string;

// Validation function
function isValidModelId(modelId: unknown): modelId is ModelId {
  if (typeof modelId !== 'string') return false;
  if (!modelId.trim()) return false;

  // Basic validation - allow alphanumeric, hyphens, and underscores
  if (!/^[a-zA-Z0-9_-]+$/.test(modelId)) return false;

  // Check minimum length
  if (modelId.length < 3) return false;

  return true;
}

// Svelte prop declaration
let modelId: ModelId = 'claude-3-opus-20240229';

$: {
  // Validate modelId when it changes
  if (!isValidModelId(modelId)) {
    console.error('Invalid model ID:', modelId);
    // Fallback to default or throw error
    modelId = 'claude-3-opus-20240229';
  }
}

// Usage in send message function
async function sendMessage() {
  if (!isValidModelId(modelId)) {
    throw new Error('Invalid model ID');
  }

  // Proceed with API call using validated modelId
  // ...
}
```

## Related Components
- ChatInput.svelte
- API client system
- Model configuration

## References
- Original review: docs/reviews/claude-opus-4-6/2026-03-04-14-30-00-code-review-chat-input.md
- TypeScript Validation: https://www.typescriptlang.org/docs/handbook/2/narrowing.html#type-guards
- Input Validation Best Practices: https://owasp.org/www-community/controls/Input_Validation

---

## Proposed Solution

### Overview
Implement a model ID validation utility and integrate it into the ChatInput component to prevent runtime errors from invalid model IDs.

### Implementation Details

#### 1. Create Validation Utility (`src-ui/src/lib/utils/validation.ts`)

Create a new validation utility file with a type guard function for model IDs:

```typescript
/**
 * Validation utilities for input validation across the application.
 */

/** Valid model ID pattern: allows alphanumeric, hyphens, underscores, colons, dots, and forward slashes */
const MODEL_ID_PATTERN = /^[a-zA-Z0-9_\-:.\/]+$/;

/** Minimum length for a valid model ID */
const MIN_MODEL_ID_LENGTH = 3;

/** Maximum length for a valid model ID */
const MAX_MODEL_ID_LENGTH = 256;

/**
 * Type guard to validate if a value is a valid model ID.
 *
 * A valid model ID must:
 * - Be a string type
 * - Not be empty after trimming
 * - Be between 3 and 256 characters
 * - Contain only alphanumeric characters, hyphens, underscores, colons, dots, and forward slashes
 *
 * @param modelId - The value to validate
 * @returns True if the value is a valid model ID
 */
export function isValidModelId(modelId: unknown): modelId is string {
	// Must be a string
	if (typeof modelId !== 'string') {
		return false;
	}

	// Must not be empty after trimming
	const trimmedId = modelId.trim();
	if (trimmedId.length === 0) {
		return false;
	}

	// Must meet minimum length requirement
	if (trimmedId.length < MIN_MODEL_ID_LENGTH) {
		return false;
	}

	// Must not exceed maximum length
	if (trimmedId.length > MAX_MODEL_ID_LENGTH) {
		return false;
	}

	// Must match valid pattern
	if (!MODEL_ID_PATTERN.test(trimmedId)) {
		return false;
	}

	return true;
}

/**
 * Validates a model ID and returns a safe version.
 * If the model ID is invalid, returns null.
 *
 * @param modelId - The model ID to validate
 * @returns The validated model ID or null if invalid
 */
export function sanitizeModelId(modelId: unknown): string | null {
	if (isValidModelId(modelId)) {
		return modelId.trim();
	}
	return null;
}
```

#### 2. Update ChatInput.svelte

Add validation import and reactive validation:

```typescript
// Add import
import { isValidModelId, sanitizeModelId } from '$lib/utils/validation';

// In Props interface, the modelId is already typed as `string | null`
// Add validation state
let validatedModelId = $derived(sanitizeModelId(modelId));

// Add validation effect for logging
$effect(() => {
	if (modelId !== null && !isValidModelId(modelId)) {
		console.warn('[ChatInput] Invalid modelId prop received:', modelId);
	}
});

// Update the template to use validatedModelId
{#if validatedModelId !== null && onModelChange}
	<ModelSelector selectedModelId={validatedModelId} {onModelChange} {onOpenSettings} />
{/if}
```

### Benefits
- **Type Safety**: Type guard ensures proper type narrowing
- **Runtime Validation**: Catches invalid values at runtime
- **Debugging Support**: Warning logs help identify source of invalid data
- **Graceful Degradation**: Invalid model IDs result in component being hidden rather than crashing
- **Reusable**: Utility can be used across the application

### Files Changed
1. `src-ui/src/lib/utils/validation.ts` - New file with validation utilities
2. `src-ui/src/lib/components/chat/input/ChatInput.svelte` - Add validation

---

## Resolution Status

**Status**: DONE

**Assignee**: Issue Resolver Agent

**Started**: 2026-03-04

**Completed**: 2026-03-04

### Changes Made
1. Created `/home/kassiomaia/devel/zk/coder/src-ui/src/lib/utils/validation.ts` with:
   - `isValidModelId()` type guard function
   - `sanitizeModelId()` helper function
   - Comprehensive validation rules for model IDs

2. Updated `/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/chat/input/ChatInput.svelte`:
   - Added import for validation utilities
   - Added `validatedModelId` derived value using `sanitizeModelId()`
   - Added `$effect` to log warnings for invalid model IDs
   - Updated template to use `validatedModelId` instead of raw `modelId`

### Validation Rules
A valid model ID must:
- Be a string type
- Not be empty after trimming
- Be between 3 and 256 characters
- Contain only alphanumeric characters, hyphens, underscores, colons, dots, and forward slashes

### Testing
- TypeScript type checking passes: 0 errors
- ESLint passes (only pre-existing unrelated warning)

### Lessons Learned
- Input validation should be applied at component boundaries where external data enters
- Using type guards provides both runtime safety and compile-time type narrowing
- Derived values in Svelte 5 runes mode provide a clean way to transform and validate props