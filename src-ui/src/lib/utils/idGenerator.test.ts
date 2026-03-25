import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { generateUniqueId, generatePopoverId } from './idGenerator';

describe('idGenerator', () => {
	describe('generateUniqueId', () => {
		it('generates unique IDs with default prefix', () => {
			const id1 = generateUniqueId();
			const id2 = generateUniqueId();

			expect(id1).toBeTruthy();
			expect(id2).toBeTruthy();
			expect(id1).not.toBe(id2);
			expect(id1).toMatch(/^id-/);
		});

		it('generates unique IDs with custom prefix', () => {
			const id1 = generateUniqueId('button');
			const id2 = generateUniqueId('modal');

			expect(id1).toMatch(/^button-/);
			expect(id2).toMatch(/^modal-/);
		});

		it('generates unique IDs with special character prefixes', () => {
			const id = generateUniqueId('test-prefix_123');
			expect(id).toMatch(/^test-prefix_123-/);
		});

		it('generates cryptographically secure IDs when crypto API is available', () => {
			const id = generateUniqueId('test');

			// crypto.randomUUID() generates UUIDs in format xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
			// The full ID will be "test-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
			if (typeof crypto !== 'undefined' && crypto.randomUUID) {
				expect(id).toMatch(/^test-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
			} else {
				// Fallback format: prefix-timestamp-counter
				expect(id).toMatch(/^test-\d+-\d+$/);
			}
		});

		it('generates different IDs on rapid successive calls', () => {
			const ids = new Set<string>();
			const count = 100;

			for (let i = 0; i < count; i++) {
				ids.add(generateUniqueId());
			}

			expect(ids.size).toBe(count);
		});

		it('handles empty prefix', () => {
			const id = generateUniqueId('');
			expect(id).toBeTruthy();
			expect(id.length).toBeGreaterThan(0);
		});

		it('handles very long prefixes', () => {
			const longPrefix = 'a'.repeat(100);
			const id = generateUniqueId(longPrefix);
			expect(id).toMatch(new RegExp(`^${longPrefix}-`));
		});
	});

	describe('generateUniqueId fallback behavior', () => {
		let originalCrypto: typeof crypto | undefined;

		beforeEach(() => {
			// Store original crypto object
			originalCrypto = (globalThis as any).crypto;
		});

		afterEach(() => {
			// Restore crypto object
			if (originalCrypto) {
				(globalThis as any).crypto = originalCrypto;
			} else {
				delete (globalThis as any).crypto;
			}
		});

		it('uses fallback when crypto.randomUUID is not available', () => {
			// Remove crypto API to test fallback
			delete (globalThis as any).crypto;

			const id1 = generateUniqueId('test');
			const id2 = generateUniqueId('test');

			// Fallback format: prefix-timestamp-counter
			expect(id1).toMatch(/^test-\d+-\d+$/);
			expect(id2).toMatch(/^test-\d+-\d+$/);

			// IDs should still be unique due to counter
			expect(id1).not.toBe(id2);
		});

		it('fallback increments counter correctly', () => {
			delete (globalThis as any).crypto;

			const id1 = generateUniqueId();
			const id2 = generateUniqueId();
			const id3 = generateUniqueId();

			// Extract counter values (last number after last dash)
			const counter1 = parseInt(id1.split('-').pop() || '0', 10);
			const counter2 = parseInt(id2.split('-').pop() || '0', 10);
			const counter3 = parseInt(id3.split('-').pop() || '0', 10);

			expect(counter2).toBe(counter1 + 1);
			expect(counter3).toBe(counter2 + 1);
		});

		it('fallback includes timestamp for uniqueness', () => {
			delete (globalThis as any).crypto;

			const id = generateUniqueId('fallback');
			const parts = id.split('-');

			// Format: prefix-timestamp-counter
			expect(parts[0]).toBe('fallback');
			expect(parts[1]).toMatch(/^\d+$/); // timestamp
			expect(parts[2]).toMatch(/^\d+$/); // counter

			const timestamp = parseInt(parts[1], 10);
			expect(timestamp).toBeGreaterThan(0);
			expect(timestamp).lessThanOrEqual(Date.now());
		});
	});

	describe('generatePopoverId', () => {
		it('generates IDs with "popover" prefix', () => {
			const id = generatePopoverId();
			expect(id).toMatch(/^popover-/);
		});

		it('generates unique popover IDs', () => {
			const id1 = generatePopoverId();
			const id2 = generatePopoverId();

			expect(id1).not.toBe(id2);
		});

		it('is consistent with generateUniqueId("popover")', () => {
			// Both should produce the same format
			const popoverId = generatePopoverId();
			const uniqueId = generateUniqueId('popover');

			expect(popoverId).toMatch(/^popover-/);
			expect(uniqueId).toMatch(/^popover-/);

			// They should be unique from each other
			expect(popoverId).not.toBe(uniqueId);
		});

		it('can generate multiple popover IDs for different popovers', () => {
			const ids = new Set<string>();

			for (let i = 0; i < 10; i++) {
				ids.add(generatePopoverId());
			}

			expect(ids.size).toBe(10);
		});
	});

	describe('uniqueness across different prefixes', () => {
		it('generates unique IDs even with different prefixes', () => {
			const ids = [
				generateUniqueId('a'),
				generateUniqueId('b'),
				generateUniqueId('c'),
				generatePopoverId()
			];

			const uniqueIds = new Set(ids);
			expect(uniqueIds.size).toBe(4);
		});
	});

	describe('ID format and structure', () => {
		it('always includes prefix separator', () => {
			const id1 = generateUniqueId('test');
			const id2 = generateUniqueId();

			expect(id1).toContain('-');
			expect(id2).toContain('-');

			// Prefix should be separated by a dash from the unique part
			expect(id1.split('-')[0]).toBe('test');
			expect(id2.split('-')[0]).toBe('id');
		});

		it('never generates empty IDs', () => {
			for (let i = 0; i < 100; i++) {
				const id = generateUniqueId();
				expect(id.length).toBeGreaterThan(0);
				expect(id.trim()).toBe(id);
			}
		});

		it('handles Unicode characters in prefix', () => {
			const id = generateUniqueId('test-中文-🚀');
			expect(id).toMatch(/^test-中文-🚀-/);
		});
	});
});
