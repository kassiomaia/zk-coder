/**
 * ID Generator Utility
 *
 * Provides secure, unique ID generation for components.
 * Uses crypto.randomUUID() when available, with a counter-based fallback.
 */

/**
 * Counter for fallback ID generation
 */
let idCounter = 0;

/**
 * Generates a unique identifier using crypto.randomUUID() when available,
 * with a counter-based fallback for environments without crypto support.
 *
 * @param prefix - Optional prefix for the generated ID (default: 'id')
 * @returns A unique string identifier
 *
 * @example
 * ```typescript
 * const id = generateUniqueId('popover'); // 'popover-a1b2c3d4-e5f6-7890-abcd-ef1234567890'
 * const simpleId = generateUniqueId(); // 'id-1698765432100-1'
 * ```
 */
export function generateUniqueId(prefix: string = 'id'): string {
	// Primary: Use crypto.randomUUID() for cryptographically secure unique IDs
	if (typeof crypto !== 'undefined' && crypto.randomUUID) {
		return `${prefix}-${crypto.randomUUID()}`;
	}

	// Fallback: Counter-based approach with timestamp for uniqueness
	// This ensures uniqueness even in rapid creation scenarios
	const timestamp = Date.now();
	const counter = ++idCounter;
	return `${prefix}-${timestamp}-${counter}`;
}

/**
 * Generates a unique ID specifically for popover components.
 * Convenience wrapper around generateUniqueId.
 *
 * @returns A unique popover identifier
 */
export function generatePopoverId(): string {
	return generateUniqueId('popover');
}
