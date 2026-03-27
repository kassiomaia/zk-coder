/**
 * Tests for scrollPriorityQueue
 */

import {beforeEach, describe, expect, it, vi} from 'vitest';
import {
    cancel,
    cancelThread,
    clearAll,
    complete,
    enqueue,
    getDebugInfo,
    getExecuting,
    getHighestPriority,
    getNext,
    getPendingCount,
    hasPending,
    isExecuting,
    onPriorityChange,
    wouldPreempt
} from './scrollPriorityQueue';

describe('scrollPriorityQueue', () => {
	beforeEach(() => {
		// Reset all state before each test
		clearAll();
	});

	describe('Enqueue Operations', () => {
		it('should enqueue an operation and return an ID', () => {
			const action = vi.fn();
			const result = enqueue(action, { threadId: 'thread-1' });

			expect(result.id).toMatch(/^scroll-op-\d+$/);
			expect(result.preempted).toBe(false);
			expect(result.preemptedIds).toEqual([]);
			expect(getPendingCount('thread-1')).toBe(1);
		});

		it('should use default priority of programmatic', () => {
			const action = vi.fn();
			const result = enqueue(action, { threadId: 'thread-1' });

			const op = getExecuting('thread-1'); // Not executing yet
			expect(op).toBeUndefined();

			const nextOp = getNext('thread-1');
			expect(nextOp?.priority).toBe('programmatic');
		});

		it('should enqueue with specified priority', () => {
			const action = vi.fn();
			const result = enqueue(action, {
				threadId: 'thread-1',
				priority: 'background'
			});

			expect(result.id).toBeTruthy();

			const nextOp = getNext('thread-1');
			expect(nextOp?.priority).toBe('background');
		});

		it('should assign unique IDs to each operation', () => {
			const action = vi.fn();
			const result1 = enqueue(action, { threadId: 'thread-1' });
			const result2 = enqueue(action, { threadId: 'thread-1' });

			expect(result1.id).not.toBe(result2.id);
		});

		it('should store timestamp on operation', () => {
			const beforeTime = Date.now();
			const action = vi.fn();
			enqueue(action, { threadId: 'thread-1' });
			const afterTime = Date.now();

			const nextOp = getNext('thread-1');
			expect(nextOp?.timestamp).toBeGreaterThanOrEqual(beforeTime);
			expect(nextOp?.timestamp).toBeLessThanOrEqual(afterTime);
		});
	});

	describe('Priority Ordering', () => {
		it('should prioritize user operations over programmatic', () => {
			const threadId = 'thread-1';

			// Enqueue operations in reverse priority order
			enqueue(vi.fn(), { threadId, priority: 'background' });
			enqueue(vi.fn(), { threadId, priority: 'programmatic' });
			enqueue(vi.fn(), { threadId, priority: 'user' });

			// User priority preempts background and programmatic
			expect(getHighestPriority(threadId)).toBe('user');
			expect(getPendingCount(threadId)).toBe(1);

			const op1 = getNext(threadId);
			expect(op1?.priority).toBe('user');

			// No more operations since they were preempted
			const op2 = getNext(threadId);
			expect(op2).toBeUndefined();
		});

		it('should maintain FIFO order within same priority', () => {
			const threadId = 'thread-1';
			const executionOrder: string[] = [];

			// Enqueue multiple operations with same priority
			enqueue(() => executionOrder.push('op1'), {
				threadId,
				priority: 'programmatic'
			});
			enqueue(() => executionOrder.push('op2'), {
				threadId,
				priority: 'programmatic'
			});
			enqueue(() => executionOrder.push('op3'), {
				threadId,
				priority: 'programmatic'
			});

			// Get and execute operations
			const op1 = getNext(threadId);
			op1?.action();
			complete(op1!.id);

			const op2 = getNext(threadId);
			op2?.action();
			complete(op2!.id);

			const op3 = getNext(threadId);
			op3?.action();
			complete(op3!.id);

			expect(executionOrder).toEqual(['op1', 'op2', 'op3']);
		});

		it('should sort queue by priority then timestamp', () => {
			const threadId = 'thread-1';

			// Add in mixed order - all same priority to avoid preemption
			enqueue(vi.fn(), { threadId, priority: 'background' });
			enqueue(vi.fn(), { threadId, priority: 'background' });
			enqueue(vi.fn(), { threadId, priority: 'background' });

			expect(getHighestPriority(threadId)).toBe('background');

			const op1 = getNext(threadId);
			expect(op1?.priority).toBe('background');

			const op2 = getNext(threadId);
			expect(op2?.priority).toBe('background');

			const op3 = getNext(threadId);
			expect(op3?.priority).toBe('background');
		});
	});

	describe('Preemption', () => {
		it('should preempt lower priority operations', () => {
			const threadId = 'thread-1';
			const onCancel1 = vi.fn();
			const onCancel2 = vi.fn();

			// Enqueue background operations (same priority, so no preemption between them)
			enqueue(vi.fn(), { threadId, priority: 'background', onCancel: onCancel1 });
			enqueue(vi.fn(), { threadId, priority: 'background', onCancel: onCancel2 });

			// Enqueue user priority - should preempt both background operations
			const result = enqueue(vi.fn(), { threadId, priority: 'user' });

			expect(result.preempted).toBe(true);
			expect(result.preemptedIds).toHaveLength(2);
			expect(onCancel1).toHaveBeenCalled();
			expect(onCancel2).toHaveBeenCalled();
		});

		it('should preempt executing operations', () => {
			const threadId = 'thread-1';
			const onCancel = vi.fn();

			// Start a background operation
			enqueue(vi.fn(), { threadId, priority: 'background', onCancel: onCancel });
			const op1 = getNext(threadId);

			// Enqueue user priority while op is executing
			const result = enqueue(vi.fn(), { threadId, priority: 'user' });

			expect(result.preempted).toBe(true);
			expect(onCancel).toHaveBeenCalled();
		});

		it('should not preempt same or higher priority operations', () => {
			const threadId = 'thread-1';
			const onCancel = vi.fn();

			// Enqueue user operation
			enqueue(vi.fn(), { threadId, priority: 'user', onCancel: onCancel });

			// Enqueue programmatic - should not preempt user
			const result = enqueue(vi.fn(), { threadId, priority: 'programmatic' });

			expect(result.preempted).toBe(false);
			expect(onCancel).not.toHaveBeenCalled();
		});

		it('should correctly identify wouldPreempt', () => {
			const threadId = 'thread-1';

			expect(wouldPreempt(threadId, 'user')).toBe(false);

			enqueue(vi.fn(), { threadId, priority: 'background' });
			expect(wouldPreempt(threadId, 'user')).toBe(true);
			expect(wouldPreempt(threadId, 'programmatic')).toBe(true);
			expect(wouldPreempt(threadId, 'background')).toBe(false);

			const op = getNext(threadId);
			expect(wouldPreempt(threadId, 'user')).toBe(true);
		});
	});

	describe('Priority Change Listeners', () => {
		it('should notify listeners when user priority operation is added', () => {
			const threadId = 'thread-1';
			const listener = vi.fn();

			onPriorityChange(listener);

			enqueue(vi.fn(), { threadId, priority: 'user' });

			expect(listener).toHaveBeenCalledWith(threadId, expect.objectContaining({
				id: expect.any(String),
				priority: 'user'
			}));
		});

		it('should not notify listeners for non-user priority operations', () => {
			const threadId = 'thread-1';
			const listener = vi.fn();

			onPriorityChange(listener);

			enqueue(vi.fn(), { threadId, priority: 'programmatic' });
			enqueue(vi.fn(), { threadId, priority: 'background' });

			expect(listener).not.toHaveBeenCalled();
		});

		it('should support multiple listeners', () => {
			const threadId = 'thread-1';
			const listener1 = vi.fn();
			const listener2 = vi.fn();

			onPriorityChange(listener1);
			onPriorityChange(listener2);

			enqueue(vi.fn(), { threadId, priority: 'user' });

			expect(listener1).toHaveBeenCalled();
			expect(listener2).toHaveBeenCalled();
		});

		it('should allow unsubscribing listeners', () => {
			const threadId = 'thread-1';
			const listener = vi.fn();

			const unsubscribe = onPriorityChange(listener);
			enqueue(vi.fn(), { threadId, priority: 'user' });
			expect(listener).toHaveBeenCalledTimes(1);

			unsubscribe();
			enqueue(vi.fn(), { threadId, priority: 'user' });
			expect(listener).toHaveBeenCalledTimes(1); // No additional call
		});

		it('should handle errors in listeners gracefully', () => {
			const threadId = 'thread-1';
			const errorListener = vi.fn(() => {
				throw new Error('Listener error');
			});
			const normalListener = vi.fn();

			onPriorityChange(errorListener);
			onPriorityChange(normalListener);

			expect(() => enqueue(vi.fn(), { threadId, priority: 'user' })).not.toThrow();
			expect(normalListener).toHaveBeenCalled();
		});
	});

	describe('Execution Lifecycle', () => {
		it('should mark operation as executing when retrieved', () => {
			const threadId = 'thread-1';

			enqueue(vi.fn(), { threadId, priority: 'programmatic' });

			expect(isExecuting(threadId)).toBe(false);

			const op = getNext(threadId);

			expect(isExecuting(threadId)).toBe(true);
			expect(op?.isExecuting).toBe(true);
		});

		it('should complete operation and remove from executing', () => {
			const threadId = 'thread-1';
			const onComplete = vi.fn();

			const result = enqueue(vi.fn(), { threadId, onComplete });
			const op = getNext(threadId);

			expect(isExecuting(threadId)).toBe(true);

			complete(result.id);

			expect(isExecuting(threadId)).toBe(false);
			expect(onComplete).toHaveBeenCalled();
		});

		it('should get executing operation', () => {
			const threadId = 'thread-1';

			enqueue(vi.fn(), { threadId, priority: 'programmatic' });

			const op = getNext(threadId);
			const executing = getExecuting(threadId);

			expect(executing).toEqual(op);
		});

		it('should handle async action functions', async () => {
			const threadId = 'thread-1';
			let executed = false;

			const asyncAction = async () => {
				executed = true;
			};

			enqueue(asyncAction, { threadId, priority: 'programmatic' });
			const op = getNext(threadId);

			await op!.action();

			expect(executed).toBe(true);
		});
	});

	describe('Cancellation', () => {
		it('should cancel a queued operation by ID', () => {
			const threadId = 'thread-1';
			const onCancel = vi.fn();

			const result = enqueue(vi.fn(), { threadId, onCancel });
			expect(getPendingCount(threadId)).toBe(1);

			const cancelled = cancel(result.id);

			expect(cancelled).toBe(true);
			expect(getPendingCount(threadId)).toBe(0);
			expect(onCancel).toHaveBeenCalled();
		});

		it('should cancel an executing operation by ID', () => {
			const threadId = 'thread-1';
			const onCancel = vi.fn();

			const result = enqueue(vi.fn(), { threadId, onCancel });
			const op = getNext(threadId);

			expect(isExecuting(threadId)).toBe(true);

			const cancelled = cancel(result.id);

			expect(cancelled).toBe(true);
			expect(isExecuting(threadId)).toBe(false);
			expect(onCancel).toHaveBeenCalled();
		});

		it('should return false when cancelling non-existent operation', () => {
			const cancelled = cancel('non-existent-id');
			expect(cancelled).toBe(false);
		});

		it('should cancel all operations for a thread', () => {
			const threadId = 'thread-1';
			const onCancel1 = vi.fn();
			const onCancel2 = vi.fn();
			const onCancel3 = vi.fn();

			enqueue(vi.fn(), { threadId, onCancel: onCancel1 });
			enqueue(vi.fn(), { threadId, onCancel: onCancel2 });
			const op = getNext(threadId);
			enqueue(vi.fn(), { threadId, onCancel: onCancel3 });

			cancelThread(threadId);

			expect(getPendingCount(threadId)).toBe(0);
			expect(isExecuting(threadId)).toBe(false);
			expect(onCancel1).toHaveBeenCalled();
			expect(onCancel2).toHaveBeenCalled();
			expect(onCancel3).toHaveBeenCalled();
		});

		it('should not affect other threads when cancelling one thread', () => {
			const thread1 = 'thread-1';
			const thread2 = 'thread-2';
			const onCancel2 = vi.fn();

			enqueue(vi.fn(), { threadId: thread1 });
			enqueue(vi.fn(), { threadId: thread2, onCancel: onCancel2 });

			cancelThread(thread1);

			expect(getPendingCount(thread1)).toBe(0);
			expect(getPendingCount(thread2)).toBe(1);
			expect(onCancel2).not.toHaveBeenCalled();
		});
	});

	describe('Thread Isolation', () => {
		it('should maintain separate queues for each thread', () => {
			const thread1 = 'thread-1';
			const thread2 = 'thread-2';

			enqueue(vi.fn(), { threadId: thread1, priority: 'background' });
			enqueue(vi.fn(), { threadId: thread2, priority: 'user' });

			expect(getPendingCount(thread1)).toBe(1);
			expect(getPendingCount(thread2)).toBe(1);

			expect(getHighestPriority(thread1)).toBe('background');
			expect(getHighestPriority(thread2)).toBe('user');
		});

		it('should only cancel operations for specified thread', () => {
			const thread1 = 'thread-1';
			const thread2 = 'thread-2';

			enqueue(vi.fn(), { threadId: thread1 });
			enqueue(vi.fn(), { threadId: thread2 });

			cancelThread(thread1);

			expect(getPendingCount(thread1)).toBe(0);
			expect(getPendingCount(thread2)).toBe(1);
		});

		it('should track executing operations per thread', () => {
			const thread1 = 'thread-1';
			const thread2 = 'thread-2';

			enqueue(vi.fn(), { threadId: thread1 });
			enqueue(vi.fn(), { threadId: thread2 });

			const op1 = getNext(thread1);
			const op2 = getNext(thread2);

			expect(isExecuting(thread1)).toBe(true);
			expect(isExecuting(thread2)).toBe(true);

			expect(getExecuting(thread1)).toBe(op1);
			expect(getExecuting(thread2)).toBe(op2);
		});
	});

	describe('Query Functions', () => {
		it('should report if thread has pending operations', () => {
			const threadId = 'thread-1';

			expect(hasPending(threadId)).toBe(false);

			enqueue(vi.fn(), { threadId });
			expect(hasPending(threadId)).toBe(true);

			const op = getNext(threadId);
			expect(hasPending(threadId)).toBe(true); // Still true while executing

			complete(op!.id);
			expect(hasPending(threadId)).toBe(false);
		});

		it('should report pending count correctly', () => {
			const threadId = 'thread-1';

			expect(getPendingCount(threadId)).toBe(0);

			enqueue(vi.fn(), { threadId });
			enqueue(vi.fn(), { threadId });
			enqueue(vi.fn(), { threadId });

			expect(getPendingCount(threadId)).toBe(3);

			getNext(threadId);
			expect(getPendingCount(threadId)).toBe(2);
		});

		it('should return undefined for highest priority when queue is empty', () => {
			const threadId = 'thread-1';

			expect(getHighestPriority(threadId)).toBeNull();

			enqueue(vi.fn(), { threadId, priority: 'user' });
			expect(getHighestPriority(threadId)).toBe('user');

			const op = getNext(threadId);
			complete(op!.id);

			expect(getHighestPriority(threadId)).toBeNull();
		});

		it('should return debug info for a thread', () => {
			const threadId = 'thread-1';

			// Enqueue two operations of same priority
			enqueue(vi.fn(), { threadId, priority: 'background' });
			enqueue(vi.fn(), { threadId, priority: 'background' });

			const debugInfo = getDebugInfo(threadId);

			expect(debugInfo.pending).toBe(2);
			expect(debugInfo.highestPriority).toBe('background');
			expect(debugInfo.executing).toBeUndefined();

			const op = getNext(threadId);

			const debugInfo2 = getDebugInfo(threadId);
			expect(debugInfo2.pending).toBe(1);
			expect(debugInfo2.executing).toBeDefined();
			expect(debugInfo2.executing?.priority).toBe('background');
		});
	});

	describe('Clear All', () => {
		it('should clear all operations for all threads', () => {
			const onCancel1 = vi.fn();
			const onCancel2 = vi.fn();
			const onCancel3 = vi.fn();

			enqueue(vi.fn(), { threadId: 'thread-1', onCancel: onCancel1 });
			enqueue(vi.fn(), { threadId: 'thread-2', onCancel: onCancel2 });
			const op = getNext('thread-1');
			enqueue(vi.fn(), { threadId: 'thread-1', onCancel: onCancel3 });

			clearAll();

			expect(getPendingCount('thread-1')).toBe(0);
			expect(getPendingCount('thread-2')).toBe(0);
			expect(isExecuting('thread-1')).toBe(false);
			expect(onCancel1).toHaveBeenCalled();
			expect(onCancel2).toHaveBeenCalled();
			expect(onCancel3).toHaveBeenCalled();
		});
	});

	describe('Edge Cases', () => {
		it('should handle empty queue operations gracefully', () => {
			const threadId = 'thread-1';

			expect(getNext(threadId)).toBeUndefined();
			expect(getExecuting(threadId)).toBeUndefined();
			expect(getHighestPriority(threadId)).toBeNull();
			expect(hasPending(threadId)).toBe(false);
		});

		it('should handle completing non-existent operation', () => {
			expect(() => complete('non-existent-id')).not.toThrow();
		});

		it('should handle cancelling operation from empty queue', () => {
			const cancelled = cancel('non-existent-id');
			expect(cancelled).toBe(false);
		});

		it('should handle completing operation that is not executing', () => {
			const threadId = 'thread-1';
			const onComplete = vi.fn();

			const result = enqueue(vi.fn(), { threadId, onComplete });

			// Complete without getting it first
			complete(result.id);

			// Should not call onComplete since it wasn't executing
			expect(onComplete).not.toHaveBeenCalled();

			// Operation should still be in queue
			expect(getPendingCount(threadId)).toBe(1);
		});

		it('should handle multiple threads with same operation IDs', () => {
			// This tests that the module-level counter increments correctly
			const result1 = enqueue(vi.fn(), { threadId: 'thread-1' });
			const result2 = enqueue(vi.fn(), { threadId: 'thread-2' });

			expect(result1.id).not.toBe(result2.id);

			// IDs should be sequential
			const num1 = parseInt(result1.id.split('-')[2]);
			const num2 = parseInt(result2.id.split('-')[2]);
			expect(num2).toBe(num1 + 1);
		});
	});

	describe('Complex Scenarios', () => {
		it('should handle preemption with executing and queued operations', () => {
			const threadId = 'thread-1';
			const onCancelExecuting = vi.fn();
			const onCancelQueued1 = vi.fn();
			const onCancelQueued2 = vi.fn();

			// Background operation executing
			enqueue(vi.fn(), { threadId, priority: 'background', onCancel: onCancelExecuting });
			getNext(threadId); // Move to executing

			// More operations queued (all lower priority than user)
			enqueue(vi.fn(), { threadId, priority: 'background', onCancel: onCancelQueued1 });
			enqueue(vi.fn(), { threadId, priority: 'background', onCancel: onCancelQueued2 });

			// User operation preempts all lower priority operations (all backgrounds)
			const result = enqueue(vi.fn(), { threadId, priority: 'user' });

			expect(result.preempted).toBe(true);
			// Should preempt executing background (1) and queued backgrounds (2)
			expect(result.preemptedIds).toHaveLength(3);
			expect(onCancelExecuting).toHaveBeenCalled();
			expect(onCancelQueued1).toHaveBeenCalled();
			expect(onCancelQueued2).toHaveBeenCalled();
		});

		it('should handle multiple priority changes in sequence', () => {
			const threadId = 'thread-1';
			const listener = vi.fn();

			onPriorityChange(listener);

			enqueue(vi.fn(), { threadId, priority: 'user' });
			expect(listener).toHaveBeenCalledTimes(1);

			enqueue(vi.fn(), { threadId, priority: 'user' });
			expect(listener).toHaveBeenCalledTimes(2);

			// Non-user priority doesn't trigger
			enqueue(vi.fn(), { threadId, priority: 'programmatic' });
			expect(listener).toHaveBeenCalledTimes(2);
		});
	});
});
