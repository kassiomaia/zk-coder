/**
 * Format a date as a relative time string.
 * Provides consistent time formatting across ConversationItem and ThreadItem components.
 *
 * @param date - Date object or ISO string to format
 * @returns Formatted relative time string (e.g., "Just now", "5m ago", "2h ago", "3d ago", "Mar 15")
 *
 * @deprecated Use `toRelativeTime` from `$lib/utils/date` instead.
 */
export { toRelativeTime as formatRelativeTime } from './date';
