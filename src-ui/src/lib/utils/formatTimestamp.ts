/**
 * Format a timestamp to a human-readable string.
 * @param timestamp - The timestamp to format (Date or number in milliseconds)
 * @returns Formatted string
 */
export function formatTimestamp(timestamp: Date | number): string {
	const date = typeof timestamp === 'number' ? new Date(timestamp) : timestamp;
	return date.toLocaleString();
}
