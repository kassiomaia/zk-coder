/**
 * Format a date as a relative time string.
 * Provides consistent time formatting across ConversationItem and ThreadItem components.
 *
 * @param date - Date object or ISO string to format
 * @returns Formatted relative time string (e.g., "Just now", "5m ago", "2h ago", "3d ago", "Mar 15")
 */
export function formatRelativeTime(date: Date | string): string {
	// Handle both Date objects and ISO strings from backend
	const d = date instanceof Date ? date : new Date(date);

	// Check for invalid date
	if (isNaN(d.getTime())) {
		return '';
	}

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
