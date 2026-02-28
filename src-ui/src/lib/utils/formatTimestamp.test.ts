import { describe, it, expect } from 'vitest';
import { formatTimestamp } from './formatTimestamp.js';

describe('formatTimestamp', () => {
	it('formats a Date object', () => {
		const date = new Date('2024-01-15T10:30:00');
		const result = formatTimestamp(date);
		expect(result).toBeTruthy();
		expect(typeof result).toBe('string');
	});

	it('formats a number timestamp', () => {
		const timestamp = 1705315800000; // 2024-01-15T10:30:00
		const result = formatTimestamp(timestamp);
		expect(result).toBeTruthy();
		expect(typeof result).toBe('string');
	});

	it('returns consistent results for same input', () => {
		const date = new Date('2024-06-01T12:00:00');
		const result1 = formatTimestamp(date);
		const result2 = formatTimestamp(date);
		expect(result1).toBe(result2);
	});
});
