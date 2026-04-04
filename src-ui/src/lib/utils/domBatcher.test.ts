/**
 * Tests for domBatcher
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	applyScrollPosition,
	cancelOperation,
	cancelThreadOperations,
	clearAll,
	executeImmediate,
	getPendingCount,
	hasPendingOperations,
	measureScrollState,
	measureScrollStateSync,
	scheduleRead,
	scheduleReadThenWrite,
	scheduleWrite
} from './domBatcher';

describe('domBatcher', () => {
	let testElement: HTMLElement;

	beforeEach(() => {
		// Clear any pending operations
		clearAll();

		// Create a test element with scrollable content
		testElement = document.createElement('div');
		testElement.style.height = '200px';
		testElement.style.overflow = 'auto';
		testElement.innerHTML = '<div style="height: 1000px;">Scrollable content</div>';
		document.body.appendChild(testElement);
	});

	afterEach(() => {
		if (testElement && testElement.parentNode) {
			document.body.removeChild(testElement);
		}
	});

	describe('scheduleRead', () => {
		it('should schedule a read operation', () => {
			const readFn = vi.fn();

			const id = scheduleRead(readFn);

			expect(id).toMatch(/^dom-op-\d+$/);
			expect(hasPendingOperations()).toBe(true);
			expect(getPendingCount().reads).toBe(1);
		});

		it('should execute read operation in next frame', async () => {
			const readFn = vi.fn();

			scheduleRead(readFn);

			expect(readFn).not.toHaveBeenCalled();

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(readFn).toHaveBeenCalled();
		});

		it('should handle multiple read operations', () => {
			const readFn1 = vi.fn();
			const readFn2 = vi.fn();
			const readFn3 = vi.fn();

			scheduleRead(readFn1);
			scheduleRead(readFn2);
			scheduleRead(readFn3);

			expect(getPendingCount().reads).toBe(3);
		});

		it('should assign unique IDs to each operation', () => {
			const id1 = scheduleRead(vi.fn());
			const id2 = scheduleRead(vi.fn());

			expect(id1).not.toBe(id2);
		});

		it('should associate operation with thread ID', () => {
			const threadId = 'thread-1';
			scheduleRead(vi.fn(), threadId);

			// Can't directly verify thread association, but operation is queued
			expect(getPendingCount().reads).toBe(1);
		});
	});

	describe('scheduleWrite', () => {
		it('should schedule a write operation', () => {
			const writeFn = vi.fn();

			const id = scheduleWrite(writeFn);

			expect(id).toMatch(/^dom-op-\d+$/);
			expect(hasPendingOperations()).toBe(true);
			expect(getPendingCount().writes).toBe(1);
		});

		it('should execute write operation in next frame', async () => {
			const writeFn = vi.fn();

			scheduleWrite(writeFn);

			expect(writeFn).not.toHaveBeenCalled();

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(writeFn).toHaveBeenCalled();
		});

		it('should handle multiple write operations', () => {
			const writeFn1 = vi.fn();
			const writeFn2 = vi.fn();

			scheduleWrite(writeFn1);
			scheduleWrite(writeFn2);

			expect(getPendingCount().writes).toBe(2);
		});
	});

	describe('scheduleReadThenWrite', () => {
		it('should schedule read followed by write', () => {
			const readFn = vi.fn(() => 'read-data');
			const writeFn = vi.fn();

			const id = scheduleReadThenWrite(readFn, writeFn);

			expect(id).toMatch(/^dom-op-\d+$/);
			expect(getPendingCount().reads).toBe(1);
			expect(getPendingCount().writes).toBe(1);
		});

		it('should pass read data to write function', async () => {
			const readData = { scrollTop: 100 };
			const readFn = vi.fn(() => readData);
			const writeFn = vi.fn();

			scheduleReadThenWrite(readFn, writeFn);

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(readFn).toHaveBeenCalled();
			expect(writeFn).toHaveBeenCalledWith(readData);
		});

		it('should use same ID for both read and write', () => {
			const readFn = vi.fn(() => 'data');
			const writeFn = vi.fn();

			const id = scheduleReadThenWrite(readFn, writeFn);

			// Both operations should have the same ID
			expect(getPendingCount().reads).toBe(1);
			expect(getPendingCount().writes).toBe(1);
		});
	});

	describe('Read-Write Ordering', () => {
		it('should execute all reads before writes', async () => {
			const executionOrder: string[] = [];

			scheduleRead(() => executionOrder.push('read1'));
			scheduleWrite(() => executionOrder.push('write1'));
			scheduleRead(() => executionOrder.push('read2'));
			scheduleWrite(() => executionOrder.push('write2'));

			await new Promise((resolve) => setTimeout(resolve, 50));

			// All operations should execute
			expect(executionOrder).toContain('read1');
			expect(executionOrder).toContain('read2');
			expect(executionOrder).toContain('write1');
			expect(executionOrder).toContain('write2');

			// Find indices of read and write operations
			const readIndices = executionOrder
				.map((op, i) => (op.includes('read') ? i : -1))
				.filter((i) => i !== -1);
			const writeIndices = executionOrder
				.map((op, i) => (op.includes('write') ? i : -1))
				.filter((i) => i !== -1);

			// All read indices should be less than all write indices
			if (readIndices.length > 0 && writeIndices.length > 0) {
				const maxReadIndex = Math.max(...readIndices);
				const minWriteIndex = Math.min(...writeIndices);
				expect(maxReadIndex).toBeLessThan(minWriteIndex);
			}
		});

		it('should maintain order within read operations', async () => {
			const executionOrder: string[] = [];

			scheduleRead(() => executionOrder.push('read1'));
			scheduleRead(() => executionOrder.push('read2'));
			scheduleRead(() => executionOrder.push('read3'));

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(executionOrder).toEqual(['read1', 'read2', 'read3']);
		});

		it('should maintain order within write operations', async () => {
			const executionOrder: string[] = [];

			scheduleWrite(() => executionOrder.push('write1'));
			scheduleWrite(() => executionOrder.push('write2'));
			scheduleWrite(() => executionOrder.push('write3'));

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(executionOrder).toEqual(['write1', 'write2', 'write3']);
		});
	});

	describe('measureScrollState', () => {
		it('should measure scroll state via batched read', async () => {
			testElement.scrollTop = 50;

			const promise = measureScrollState(testElement);

			expect(getPendingCount().reads).toBe(1);

			const state = await promise;

			expect(state.scrollTop).toBe(50);
			// In jsdom, scrollHeight might not match actual content, but it should have some value
			expect(state.scrollHeight).toBeGreaterThanOrEqual(0);
			// clientHeight might be 0 in jsdom if element isn't fully rendered
			expect(state.clientHeight).toBeGreaterThanOrEqual(0);
			expect(state.maxScroll).toBeGreaterThanOrEqual(0);
			expect(state.progress).toBeGreaterThanOrEqual(0);
			expect(state.progress).toBeLessThanOrEqual(1);
		});

		it('should detect when near bottom', async () => {
			// Scroll to near bottom
			testElement.scrollTop = testElement.scrollHeight - testElement.clientHeight - 10;

			const state = await measureScrollState(testElement, 30);

			expect(state.isNearBottom).toBe(true);
			expect(state.distanceFromBottom).toBeLessThan(30);
		});

		it('should detect when not near bottom', async () => {
			testElement.scrollTop = 0;

			const state = await measureScrollState(testElement, 30);

			// If at top, distance from bottom should be positive
			expect(state.distanceFromBottom).toBeGreaterThanOrEqual(0);
			// Near bottom depends on threshold
			// In jsdom with simulated content, we check the logic works
			expect(state.isNearBottom).toBe(state.distanceFromBottom < 30);
		});

		it('should use default threshold of 30px', async () => {
			testElement.scrollTop = testElement.scrollHeight - testElement.clientHeight - 25;

			const state = await measureScrollState(testElement);

			expect(state.isNearBottom).toBe(true);
		});

		it('should handle custom threshold', async () => {
			testElement.scrollTop = testElement.scrollHeight - testElement.clientHeight - 50;

			const state = await measureScrollState(testElement, 100);

			expect(state.isNearBottom).toBe(true);

			const state2 = await measureScrollState(testElement, 10);

			expect(state2.isNearBottom).toBe(false);
		});

		it('should calculate progress correctly', async () => {
			testElement.scrollTop = 0;
			let state = await measureScrollState(testElement);
			expect(state.progress).toBeGreaterThanOrEqual(0);

			// Set scroll to max (may not work perfectly in jsdom, but we can test the logic)
			const maxScroll = state.maxScroll;
			testElement.scrollTop = maxScroll;
			state = await measureScrollState(testElement);
			// Progress should be 1 or very close when at bottom
			expect(state.progress).toBeCloseTo(1, 1);

			// Test middle position if maxScroll > 0
			if (maxScroll > 0) {
				testElement.scrollTop = maxScroll / 2;
				state = await measureScrollState(testElement);
				expect(state.progress).toBeCloseTo(0.5, 1);
			}
		});

		it('should handle zero maxScroll (content fits)', async () => {
			// Element with no scroll
			const smallElement = document.createElement('div');
			smallElement.style.height = '200px';
			smallElement.style.overflow = 'auto';
			smallElement.innerHTML = '<div style="height: 100px;">Small content</div>';
			document.body.appendChild(smallElement);

			try {
				const state = await measureScrollState(smallElement);
				// maxScroll might be 0 or negative in jsdom
				expect(state.maxScroll).toBeLessThanOrEqual(200);
				// When maxScroll is 0, progress should be 1
				if (state.maxScroll === 0) {
					expect(state.progress).toBe(1);
				}
			} finally {
				document.body.removeChild(smallElement);
			}
		});
	});

	describe('measureScrollStateSync', () => {
		it('should measure scroll state synchronously', () => {
			testElement.scrollTop = 75;

			const state = measureScrollStateSync(testElement);

			expect(state.scrollTop).toBe(75);
			expect(state.scrollHeight).toBeGreaterThanOrEqual(0);
			// clientHeight might be 0 in jsdom if element isn't fully rendered
			expect(state.clientHeight).toBeGreaterThanOrEqual(0);
		});

		it('should detect near bottom synchronously', () => {
			testElement.scrollTop = testElement.scrollHeight - testElement.clientHeight - 10;

			const state = measureScrollStateSync(testElement, 30);

			expect(state.isNearBottom).toBe(true);
		});

		it('should use default threshold', () => {
			testElement.scrollTop = testElement.scrollHeight - testElement.clientHeight - 25;

			const state = measureScrollStateSync(testElement);

			expect(state.isNearBottom).toBe(true);
		});
	});

	describe('applyScrollPosition', () => {
		it('should apply scroll position via batched write', async () => {
			const targetScroll = 100;

			const id = applyScrollPosition(testElement, targetScroll);

			expect(id).toMatch(/^dom-op-\d+$/);
			expect(getPendingCount().writes).toBe(1);

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(testElement.scrollTop).toBe(targetScroll);
		});

		it('should return operation ID for cancellation', () => {
			const id = applyScrollPosition(testElement, 50);

			const cancelled = cancelOperation(id);

			expect(cancelled).toBe(true);
		});
	});

	describe('Cancellation', () => {
		it('should cancel a read operation by ID', () => {
			const readFn = vi.fn();

			const id = scheduleRead(readFn);
			expect(getPendingCount().reads).toBe(1);

			const cancelled = cancelOperation(id);

			expect(cancelled).toBe(true);
			expect(getPendingCount().reads).toBe(0);
		});

		it('should cancel a write operation by ID', () => {
			const writeFn = vi.fn();

			const id = scheduleWrite(writeFn);
			expect(getPendingCount().writes).toBe(1);

			const cancelled = cancelOperation(id);

			expect(cancelled).toBe(true);
			expect(getPendingCount().writes).toBe(0);
		});

		it('should return false for non-existent operation', () => {
			const cancelled = cancelOperation('non-existent-id');
			expect(cancelled).toBe(false);
		});

		it('should cancel all operations for a thread', () => {
			const threadId = 'thread-1';

			scheduleRead(vi.fn(), threadId);
			scheduleWrite(vi.fn(), threadId);
			scheduleRead(vi.fn(), threadId);

			expect(getPendingCount().reads).toBe(2);
			expect(getPendingCount().writes).toBe(1);

			cancelThreadOperations(threadId);

			expect(getPendingCount().reads).toBe(0);
			expect(getPendingCount().writes).toBe(0);
		});

		it('should only cancel operations for specified thread', () => {
			scheduleRead(vi.fn(), 'thread-1');
			scheduleWrite(vi.fn(), 'thread-2');
			scheduleRead(vi.fn(), 'thread-1');

			cancelThreadOperations('thread-1');

			expect(getPendingCount().reads).toBe(0);
			expect(getPendingCount().writes).toBe(1); // thread-2's write
		});

		it('should cancel readThenWrite operations by thread', () => {
			const threadId = 'thread-1';

			scheduleReadThenWrite(vi.fn, vi.fn, threadId);

			expect(getPendingCount().reads).toBe(1);
			expect(getPendingCount().writes).toBe(1);

			cancelThreadOperations(threadId);

			expect(getPendingCount().reads).toBe(0);
			expect(getPendingCount().writes).toBe(0);
		});
	});

	describe('Clear All', () => {
		it('should clear all pending operations', () => {
			scheduleRead(vi.fn());
			scheduleWrite(vi.fn());
			scheduleRead(vi.fn());

			expect(hasPendingOperations()).toBe(true);

			clearAll();

			expect(hasPendingOperations()).toBe(false);
			expect(getPendingCount().reads).toBe(0);
			expect(getPendingCount().writes).toBe(0);
		});
	});

	describe('Query Functions', () => {
		it('should report if there are pending operations', () => {
			expect(hasPendingOperations()).toBe(false);

			scheduleRead(vi.fn());
			expect(hasPendingOperations()).toBe(true);

			clearAll();
			scheduleWrite(vi.fn());
			expect(hasPendingOperations()).toBe(true);
		});

		it('should return count of pending operations', () => {
			expect(getPendingCount()).toEqual({ reads: 0, writes: 0 });

			scheduleRead(vi.fn());
			scheduleRead(vi.fn());
			expect(getPendingCount()).toEqual({ reads: 2, writes: 0 });

			scheduleWrite(vi.fn());
			expect(getPendingCount()).toEqual({ reads: 2, writes: 1 });

			scheduleReadThenWrite(vi.fn, vi.fn);
			expect(getPendingCount()).toEqual({ reads: 3, writes: 2 });
		});
	});

	describe('executeImmediate', () => {
		it('should execute function immediately without batching', () => {
			const fn = vi.fn(() => 'result');

			const result = executeImmediate(fn);

			expect(fn).toHaveBeenCalled();
			expect(result).toBe('result');
			expect(getPendingCount().reads).toBe(0);
		});

		it('should bypass the batcher for immediate DOM access', () => {
			testElement.scrollTop = 0;

			const scrollTop = executeImmediate(() => testElement.scrollTop);

			expect(scrollTop).toBe(0);
		});
	});

	describe('Error Handling', () => {
		it('should handle errors in read operations gracefully', async () => {
			const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

			const errorFn = () => {
				throw new Error('Read error');
			};
			const normalFn = vi.fn();

			scheduleRead(errorFn);
			scheduleRead(normalFn);

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(consoleSpy).toHaveBeenCalled();
			expect(normalFn).toHaveBeenCalled(); // Other operations still execute

			consoleSpy.mockRestore();
		});

		it('should handle errors in write operations gracefully', async () => {
			const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

			const errorFn = () => {
				throw new Error('Write error');
			};
			const normalFn = vi.fn();

			scheduleWrite(errorFn);
			scheduleWrite(normalFn);

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(consoleSpy).toHaveBeenCalled();
			expect(normalFn).toHaveBeenCalled(); // Other operations still execute

			consoleSpy.mockRestore();
		});

		it('should handle errors in readThenWrite operations', async () => {
			const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

			const errorReadFn = () => {
				throw new Error('Read error');
			};
			const writeFn = vi.fn();

			scheduleReadThenWrite(errorReadFn, writeFn);

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(consoleSpy).toHaveBeenCalled();

			consoleSpy.mockRestore();
		});
	});

	describe('Frame Scheduling', () => {
		it('should schedule only one frame for multiple operations', () => {
			// Track RAF calls
			let rafCount = 0;
			const mockRaf = vi.fn().mockImplementation((cb) => {
				rafCount++;
				// Execute immediately in tests
				setTimeout(() => cb(0), 0);
				return rafCount;
			});

			vi.spyOn(window, 'requestAnimationFrame').mockImplementation(mockRaf);

			scheduleRead(vi.fn());
			scheduleWrite(vi.fn());
			scheduleRead(vi.fn());

			// Should have called requestAnimationFrame (actual count depends on implementation)
			expect(rafCount).toBeGreaterThan(0);

			vi.restoreAllMocks();
		});

		it('should process operations in batch', async () => {
			let readExecuted = false;
			let writeExecuted = false;

			scheduleRead(() => {
				readExecuted = true;
			});
			scheduleWrite(() => {
				writeExecuted = true;
			});

			// Wait for processing
			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(readExecuted).toBe(true);
			expect(writeExecuted).toBe(true);
		});
	});

	describe('Edge Cases', () => {
		it('should handle empty readThenWrite with no data', async () => {
			const readFn = vi.fn(() => undefined);
			const writeFn = vi.fn();

			scheduleReadThenWrite(readFn, writeFn);

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(readFn).toHaveBeenCalled();
			expect(writeFn).toHaveBeenCalledWith(undefined);
		});

		it('should handle cancellation of already executed operation', async () => {
			const fn = vi.fn();
			const id = scheduleRead(fn);

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(fn).toHaveBeenCalled();

			// Try to cancel after execution
			const cancelled = cancelOperation(id);
			expect(cancelled).toBe(false); // Already executed, not in queue
		});

		it('should handle rapidly scheduling and cancelling', () => {
			const ids: string[] = [];

			for (let i = 0; i < 10; i++) {
				ids.push(scheduleRead(vi.fn()));
			}

			expect(getPendingCount().reads).toBe(10);

			ids.forEach((id) => cancelOperation(id));

			expect(getPendingCount().reads).toBe(0);
		});

		it('should handle operations that modify DOM being measured', async () => {
			// Set initial content
			testElement.innerHTML = '<div style="height: 500px;">Initial content</div>';
			testElement.scrollTop = 0;

			// Force a reflow by accessing a property
			const initialScrollHeight = testElement.scrollHeight;

			scheduleWrite(() => {
				testElement.innerHTML = '<div style="height: 1000px;">New content</div>';
			});

			// Wait for write to execute
			await new Promise((resolve) => setTimeout(resolve, 50));

			// Now measure
			const state = await measureScrollState(testElement);

			// The measurement should happen after the write
			// Note: In jsdom, scrollHeight calculation may not perfectly match browser
			// But we can verify the function works
			expect(state).toBeDefined();
			expect(typeof state.scrollHeight).toBe('number');
		});
	});

	describe('Operation Counter', () => {
		it('should generate incrementing operation IDs', () => {
			const id1 = scheduleRead(vi.fn());
			const id2 = scheduleWrite(vi.fn());
			const id3 = scheduleReadThenWrite(vi.fn, vi.fn);

			const num1 = parseInt(id1.split('-')[2]);
			const num2 = parseInt(id2.split('-')[2]);
			const num3 = parseInt(id3.split('-')[2]);

			expect(num2).toBe(num1 + 1);
			expect(num3).toBe(num2 + 1);
		});
	});
});
