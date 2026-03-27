import {afterEach, beforeEach, describe, expect, it, vi} from 'vitest';
import {formatRelativeTime} from './formatRelativeTime';

describe('formatRelativeTime', () => {
	let realNow: number;

	beforeEach(() => {
		// Store the current timestamp for calculations
		realNow = Date.now();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('with Date objects', () => {
		it('returns "Just now" for times less than a minute ago', () => {
			const now = new Date(realNow - 30000); // 30 seconds ago
			expect(formatRelativeTime(now)).toBe('Just now');
		});

		it('returns "Just now" for current time', () => {
			const now = new Date(realNow);
			expect(formatRelativeTime(now)).toBe('Just now');
		});

		it('returns "Xm ago" for times less than an hour ago', () => {
			const fiveMinutesAgo = new Date(realNow - 5 * 60000);
			expect(formatRelativeTime(fiveMinutesAgo)).toBe('5m ago');

			const thirtyMinutesAgo = new Date(realNow - 30 * 60000);
			expect(formatRelativeTime(thirtyMinutesAgo)).toBe('30m ago');

			const fiftyNineMinutesAgo = new Date(realNow - 59 * 60000);
			expect(formatRelativeTime(fiftyNineMinutesAgo)).toBe('59m ago');
		});

		it('returns "Xh ago" for times less than a day ago', () => {
			const oneHourAgo = new Date(realNow - 60 * 60000);
			expect(formatRelativeTime(oneHourAgo)).toBe('1h ago');

			const fiveHoursAgo = new Date(realNow - 5 * 3600000);
			expect(formatRelativeTime(fiveHoursAgo)).toBe('5h ago');

			const twentyThreeHoursAgo = new Date(realNow - 23 * 3600000);
			expect(formatRelativeTime(twentyThreeHoursAgo)).toBe('23h ago');
		});

		it('returns "Xd ago" for times less than a week ago', () => {
			const oneDayAgo = new Date(realNow - 24 * 3600000);
			expect(formatRelativeTime(oneDayAgo)).toBe('1d ago');

			const threeDaysAgo = new Date(realNow - 3 * 86400000);
			expect(formatRelativeTime(threeDaysAgo)).toBe('3d ago');

			const sixDaysAgo = new Date(realNow - 6 * 86400000);
			expect(formatRelativeTime(sixDaysAgo)).toBe('6d ago');
		});

		it('returns formatted date for times a week or more ago', () => {
			const oneWeekAgo = new Date(realNow - 7 * 86400000);
			const result = formatRelativeTime(oneWeekAgo);
			// Should be in format like "Mar 8" or similar based on locale
			// Just check it's not a relative time
			expect(result).not.toContain('ago');
			expect(result).not.toBe('Just now');
		});

		it('returns formatted date for times much earlier', () => {
			const lastMonth = new Date(realNow - 30 * 86400000);
			const result = formatRelativeTime(lastMonth);
			// Should be a date, not relative time
			expect(result).not.toContain('ago');
			expect(result).not.toBe('Just now');
		});
	});

	describe('with ISO strings', () => {
		it('handles ISO 8601 strings', () => {
			const fiveMinutesAgo = new Date(realNow - 5 * 60000);
			const isoString = fiveMinutesAgo.toISOString();
			expect(formatRelativeTime(isoString)).toBe('5m ago');
		});

		it('handles ISO strings with different timezones', () => {
			const twoHoursAgo = new Date(realNow - 2 * 3600000);
			const isoString = twoHoursAgo.toISOString();
			expect(formatRelativeTime(isoString)).toBe('2h ago');
		});

		it('handles ISO strings with milliseconds', () => {
			const thirtySecondsAgo = new Date(realNow - 30000);
			const isoString = thirtySecondsAgo.toISOString();
			expect(formatRelativeTime(isoString)).toBe('Just now');
		});
	});

	describe('boundary conditions', () => {
		it('handles exactly 1 minute ago', () => {
			const oneMinuteAgo = new Date(realNow - 60000);
			expect(formatRelativeTime(oneMinuteAgo)).toBe('1m ago');
		});

		it('handles exactly 59 seconds ago', () => {
			const fiftyNineSecondsAgo = new Date(realNow - 59000);
			expect(formatRelativeTime(fiftyNineSecondsAgo)).toBe('Just now');
		});

		it('handles exactly 60 minutes (1 hour) ago', () => {
			const oneHourAgo = new Date(realNow - 60 * 60000);
			expect(formatRelativeTime(oneHourAgo)).toBe('1h ago');
		});

		it('handles exactly 59 minutes ago', () => {
			const fiftyNineMinutesAgo = new Date(realNow - 59 * 60000);
			expect(formatRelativeTime(fiftyNineMinutesAgo)).toBe('59m ago');
		});

		it('handles exactly 24 hours (1 day) ago', () => {
			const oneDayAgo = new Date(realNow - 24 * 3600000);
			expect(formatRelativeTime(oneDayAgo)).toBe('1d ago');
		});

		it('handles exactly 23 hours ago', () => {
			const twentyThreeHoursAgo = new Date(realNow - 23 * 3600000);
			expect(formatRelativeTime(twentyThreeHoursAgo)).toBe('23h ago');
		});

		it('handles exactly 7 days ago', () => {
			const sevenDaysAgo = new Date(realNow - 7 * 86400000);
			const result = formatRelativeTime(sevenDaysAgo);
			// Should be formatted date, not "7d ago"
			expect(result).not.toContain('7d ago');
		});

		it('handles exactly 6 days ago', () => {
			const sixDaysAgo = new Date(realNow - 6 * 86400000);
			expect(formatRelativeTime(sixDaysAgo)).toBe('6d ago');
		});
	});

	describe('edge cases', () => {
		it('returns empty string for invalid Date objects', () => {
			const invalidDate = new Date('invalid');
			expect(formatRelativeTime(invalidDate)).toBe('');
		});

		it('returns empty string for invalid ISO strings', () => {
			expect(formatRelativeTime('not-a-date')).toBe('');
			expect(formatRelativeTime('')).toBe('');
		});

		it('returns empty string for malformed date strings', () => {
			expect(formatRelativeTime('2025-13-45')).toBe(''); // Invalid month/day
		});

		it('handles epoch timestamp (0)', () => {
			const epoch = new Date(0);
			const result = formatRelativeTime(epoch);
			expect(result).toBeTruthy();
			// Should be formatted date for Jan 1, 1970
			// The exact format depends on locale and timezone
			expect(result).not.toBe('Just now');
			expect(result).not.toContain('ago');
		});

		it('handles very old dates', () => {
			const oldDate = new Date('1970-01-01T00:00:00Z');
			const result = formatRelativeTime(oldDate);
			// Should be a formatted date
			expect(result).not.toBe('Just now');
			expect(result).not.toContain('ago');
		});

		it('handles future dates (negative difference)', () => {
			const futureDate = new Date(realNow + 300000); // 5 minutes in future
			// Future dates return formatted date since diff is negative
			const result = formatRelativeTime(futureDate);
			// The behavior for future dates is implementation-dependent
			// Just verify it doesn't crash and returns something
			expect(typeof result).toBe('string');
		});
	});

	describe('locale handling', () => {
		it('uses default locale for date formatting', () => {
			const oldDate = new Date('2025-01-15T12:00:00Z');
			const result = formatRelativeTime(oldDate);
			// Should contain month and day in some format
			expect(result.length).toBeGreaterThan(0);
			expect(result).not.toBe('Just now');
			expect(result).not.toContain('ago');
		});
	});

	describe('real-world scenarios', () => {
		it('formats recent message times correctly', () => {
			// A message sent 30 seconds ago
			const recent = new Date(realNow - 30000);
			expect(formatRelativeTime(recent)).toBe('Just now');

			// A message sent 5 minutes ago
			const fiveMinutes = new Date(realNow - 5 * 60000);
			expect(formatRelativeTime(fiveMinutes)).toBe('5m ago');

			// A message sent 2 hours ago
			const twoHours = new Date(realNow - 2 * 3600000);
			expect(formatRelativeTime(twoHours)).toBe('2h ago');

			// A message sent 21 hours ago
			const yesterday = new Date(realNow - 21 * 3600000);
			expect(formatRelativeTime(yesterday)).toBe('21h ago');
		});

		it('formats older message times correctly', () => {
			// A message from 3 days ago
			const threeDays = new Date(realNow - 3 * 86400000);
			expect(formatRelativeTime(threeDays)).toBe('3d ago');

			// A message from 8 days ago (should be formatted date)
			const lastWeek = new Date(realNow - 8 * 86400000);
			const result = formatRelativeTime(lastWeek);
			expect(result).not.toContain('8d ago');
			expect(result).not.toBe('Just now');
		});
	});

	describe('type handling', () => {
		it('accepts both Date and string inputs', () => {
			const fiveMinutesAgo = new Date(realNow - 5 * 60000);
			const dateObj = fiveMinutesAgo;
			const dateString = fiveMinutesAgo.toISOString();

			const result1 = formatRelativeTime(dateObj);
			const result2 = formatRelativeTime(dateString);

			expect(result1).toBe(result2);
			expect(result1).toBe('5m ago');
		});
	});
});
