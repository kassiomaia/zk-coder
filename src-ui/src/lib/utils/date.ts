/**
 * Date Utilities
 * Consistent date parsing and formatting across the application.
 */

/**
 * Parses a value that may be a Date object or a date string.
 * Returns a valid Date object, or null for invalid dates.
 *
 * @param date - Date object or ISO string
 * @returns Valid Date or null if parsing fails
 *
 * @example
 * ```typescript
 * parseDate(new Date()) // Date
 * parseDate('2024-01-15T10:30:00Z') // Date
 * parseDate('invalid') // null
 * ```
 */
export function parseDate(date: Date | string | null | undefined): Date | null {
	if (!date) return null;
	if (date instanceof Date) {
		return isNaN(date.getTime()) ? null : date;
	}
	const parsed = new Date(date);
	return isNaN(parsed.getTime()) ? null : parsed;
}

/**
 * Ensures a value is a Date object, with optional fallback.
 *
 * @param date - Date object or string
 * @param fallback - Fallback date if parsing fails (defaults to now)
 * @returns Date object
 */
export function toDate(date: Date | string, fallback: Date = new Date()): Date {
	const parsed = parseDate(date);
	return parsed ?? fallback;
}

/**
 * Checks if a date is valid.
 */
export function isValidDate(date: Date | string | null | undefined): boolean {
	return parseDate(date) !== null;
}

/**
 * Returns current timestamp as Date.
 */
export function now(): Date {
	return new Date();
}

/**
 * Formats a date as ISO string (for API compatibility).
 */
export function toIsoString(date: Date | string | null | undefined): string | null {
	const parsed = parseDate(date);
	return parsed?.toISOString() ?? null;
}

/**
 * Calculates relative time strings.
 * Returns intervals like "Just now", "5m ago", "2h ago", "3d ago", "Mar 15"
 */
export function toRelativeTime(date: Date | string): string {
	const d = parseDate(date);
	if (!d) return '';

	const now = new Date();
	const diff = now.getTime() - d.getTime();
	const minutes = Math.floor(diff / 60000);
	const hours = Math.floor(diff / 3600000);
	const days = Math.floor(diff / 86400000);

	if (minutes < 1) return 'Just now';
	if (minutes < 60) return `${minutes}m ago`;
	if (hours < 24) return `${hours}h ago`;
	if (days < 7) return `${days}d ago`;
	return d.toLocaleDateString([], { month: 'short', day: 'numeric' });
}