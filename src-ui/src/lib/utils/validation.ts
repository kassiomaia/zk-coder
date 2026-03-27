/**
 * Validation Utilities
 * Input validation functions for the application.
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
 * Model IDs typically follow patterns like:
 * - `provider/model-name` (e.g., `anthropic/claude-3-opus`)
 * - `model-name` (e.g., `gpt-4o`)
 * - `provider:model-name` (e.g., `openai:gpt-4`)
 *
 * @param modelId - The value to validate
 * @returns True if the value is a valid model ID
 *
 * @example
 * ```typescript
 * if (isValidModelId(modelId)) {
 *   // modelId is now typed as string
 *   console.log(modelId.toUpperCase());
 * }
 * ```
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
 * This function is useful for sanitizing model IDs before passing them
 * to components or APIs that expect valid model IDs.
 *
 * @param modelId - The model ID to validate
 * @returns The validated model ID (trimmed) or null if invalid
 *
 * @example
 * ```typescript
 * const safeModelId = safeModelId(userInput);
 * if (safeModelId) {
 *   // Safe to use
 *   selectModel(safeModelId);
 * }
 * ```
 */
export function sanitizeModelId(modelId: unknown): string | null {
	if (isValidModelId(modelId)) {
		return modelId.trim();
	}
	return null;
}
