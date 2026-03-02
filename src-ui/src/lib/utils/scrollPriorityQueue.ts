/**
 * Scroll Priority Queue
 *
 * A priority-based queue for scroll operations with preemption support.
 * Higher priority operations (user actions) can preempt lower priority
 * operations (background/streaming) mid-animation.
 *
 * Priority Order (highest to lowest):
 * 1. user - Manual scroll, button click, send message
 * 2. programmatic - Auto-scroll triggered by app
 * 3. background - Streaming content updates
 *
 * @module scrollPriorityQueue
 */

// ============================================================================
// Types
// ============================================================================

/** Priority levels for scroll operations */
export type ScrollPriority = 'user' | 'programmatic' | 'background';

/** Numeric priority values for comparison (higher = more important) */
const PRIORITY_VALUES: Record<ScrollPriority, number> = {
	user: 3,
	programmatic: 2,
	background: 1
};

/** A scroll operation in the queue */
export interface ScrollOperation {
	/** Unique identifier for this operation */
	id: string;
	/** Thread ID this operation belongs to */
	threadId: string;
	/** Priority level */
	priority: ScrollPriority;
	/** The scroll action to perform */
	action: () => void | Promise<void>;
	/** Callback when operation completes */
	onComplete?: () => void;
	/** Callback when operation is cancelled/preempted */
	onCancel?: () => void;
	/** Whether this operation is currently executing */
	isExecuting?: boolean;
	/** Timestamp when operation was queued */
	timestamp: number;
}

/** Options for enqueueing a scroll operation */
export interface EnqueueOptions {
	/** Thread ID for the operation */
	threadId: string;
	/** Priority level (default: 'programmatic') */
	priority?: ScrollPriority;
	/** Callback when operation completes */
	onComplete?: () => void;
	/** Callback when operation is cancelled */
	onCancel?: () => void;
}

/** Result of enqueue operation */
export interface EnqueueResult {
	/** Operation ID */
	id: string;
	/** Whether lower priority operations were preempted */
	preempted: boolean;
	/** IDs of operations that were preempted */
	preemptedIds: string[];
}

// ============================================================================
// State
// ============================================================================

/** Map of thread ID to operation queue */
const threadQueues = new Map<string, ScrollOperation[]>();

/** Map of operation ID to operation (for quick lookup) */
const operationMap = new Map<string, ScrollOperation>();

/** Currently executing operations by thread */
const executingOperations = new Map<string, ScrollOperation>();

/** Counter for generating unique IDs */
let operationCounter = 0;

/** Set of priority change listeners */
const priorityListeners = new Set<(threadId: string, operation: ScrollOperation) => void>();

// ============================================================================
// Private Functions
// ============================================================================

/**
 * Generate a unique operation ID.
 */
function generateId(): string {
	return `scroll-op-${++operationCounter}`;
}

/**
 * Get or create a queue for a thread.
 */
function getThreadQueue(threadId: string): ScrollOperation[] {
	let queue = threadQueues.get(threadId);
	if (!queue) {
		queue = [];
		threadQueues.set(threadId, queue);
	}
	return queue;
}

/**
 * Sort queue by priority (highest first), then by timestamp (oldest first within same priority).
 */
function sortQueue(queue: ScrollOperation[]): void {
	queue.sort((a, b) => {
		const priorityDiff = PRIORITY_VALUES[b.priority] - PRIORITY_VALUES[a.priority];
		if (priorityDiff !== 0) return priorityDiff;
		return a.timestamp - b.timestamp;
	});
}

/**
 * Preempt (cancel) all operations with lower priority than the given one.
 */
function preemptLowerPriority(threadId: string, priority: ScrollPriority): string[] {
	const queue = getThreadQueue(threadId);
	const preemptedIds: string[] = [];
	const minPriority = PRIORITY_VALUES[priority];

	// Cancel currently executing operation if lower priority
	const executing = executingOperations.get(threadId);
	if (executing && PRIORITY_VALUES[executing.priority] < minPriority) {
		executing.onCancel?.();
		executingOperations.delete(threadId);
		operationMap.delete(executing.id);
		preemptedIds.push(executing.id);
	}

	// Remove lower priority operations from queue
	for (let i = queue.length - 1; i >= 0; i--) {
		const op = queue[i];
		if (PRIORITY_VALUES[op.priority] < minPriority) {
			op.onCancel?.();
			operationMap.delete(op.id);
			preemptedIds.push(op.id);
			queue.splice(i, 1);
		}
	}

	return preemptedIds;
}

/**
 * Notify listeners of priority change.
 */
function notifyPriorityChange(threadId: string, operation: ScrollOperation): void {
	priorityListeners.forEach((listener) => {
		try {
			listener(threadId, operation);
		} catch (error) {
			console.error('[ScrollPriorityQueue] Listener error:', error);
		}
	});
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Enqueue a scroll operation.
 * Higher priority operations will preempt lower priority ones.
 *
 * @param action - The scroll action to perform
 * @param options - Enqueue options
 * @returns Enqueue result with operation ID and preemption info
 */
export function enqueue(action: () => void | Promise<void>, options: EnqueueOptions): EnqueueResult {
	const id = generateId();
	const priority = options.priority ?? 'programmatic';

	const operation: ScrollOperation = {
		id,
		threadId: options.threadId,
		priority,
		action,
		onComplete: options.onComplete,
		onCancel: options.onCancel,
		timestamp: Date.now()
	};

	// Store in maps
	operationMap.set(id, operation);

	// Preempt lower priority operations
	const preemptedIds = preemptLowerPriority(options.threadId, priority);

	// Add to queue
	const queue = getThreadQueue(options.threadId);
	queue.push(operation);
	sortQueue(queue);

	// Notify if this is a user priority operation
	if (priority === 'user') {
		notifyPriorityChange(options.threadId, operation);
	}

	return { id, preempted: preemptedIds.length > 0, preemptedIds };
}

/**
 * Cancel a specific operation by ID.
 *
 * @param id - Operation ID to cancel
 * @returns True if operation was found and cancelled
 */
export function cancel(id: string): boolean {
	const operation = operationMap.get(id);
	if (!operation) return false;

	// If currently executing, cancel it
	if (executingOperations.get(operation.threadId)?.id === id) {
		operation.onCancel?.();
		executingOperations.delete(operation.threadId);
		operationMap.delete(id);
		return true;
	}

	// Remove from queue
	const queue = threadQueues.get(operation.threadId);
	if (queue) {
		const index = queue.findIndex((op) => op.id === id);
		if (index !== -1) {
			operation.onCancel?.();
			queue.splice(index, 1);
			operationMap.delete(id);
			return true;
		}
	}

	return false;
}

/**
 * Cancel all operations for a specific thread.
 *
 * @param threadId - Thread ID to cancel operations for
 */
export function cancelThread(threadId: string): void {
	// Cancel executing operation
	const executing = executingOperations.get(threadId);
	if (executing) {
		executing.onCancel?.();
		executingOperations.delete(threadId);
		operationMap.delete(executing.id);
	}

	// Cancel all queued operations
	const queue = threadQueues.get(threadId);
	if (queue) {
		for (const op of queue) {
			op.onCancel?.();
			operationMap.delete(op.id);
		}
		queue.length = 0;
	}
}

/**
 * Get the next operation to execute for a thread.
 * This removes the operation from the queue and marks it as executing.
 *
 * @param threadId - Thread ID to get next operation for
 * @returns The next operation, or undefined if queue is empty
 */
export function getNext(threadId: string): ScrollOperation | undefined {
	const queue = getThreadQueue(threadId);
	if (queue.length === 0) return undefined;

	const operation = queue.shift()!;
	operation.isExecuting = true;
	executingOperations.set(threadId, operation);

	return operation;
}

/**
 * Mark an operation as complete.
 * This removes it from the executing map and calls its onComplete callback.
 *
 * @param id - Operation ID that completed
 */
export function complete(id: string): void {
	const operation = operationMap.get(id);
	if (!operation) return;

	// Only complete if it's the currently executing operation
	if (executingOperations.get(operation.threadId)?.id === id) {
		executingOperations.delete(operation.threadId);
		operation.onComplete?.();
		operationMap.delete(id);
	}
}

/**
 * Check if a thread has pending operations.
 *
 * @param threadId - Thread ID to check
 * @returns True if there are pending operations
 */
export function hasPending(threadId: string): boolean {
	const queue = threadQueues.get(threadId);
	return (queue?.length ?? 0) > 0 || executingOperations.has(threadId);
}

/**
 * Check if a thread has an executing operation.
 *
 * @param threadId - Thread ID to check
 * @returns True if there is an executing operation
 */
export function isExecuting(threadId: string): boolean {
	return executingOperations.has(threadId);
}

/**
 * Get the currently executing operation for a thread.
 *
 * @param threadId - Thread ID to get executing operation for
 * @returns The executing operation, or undefined
 */
export function getExecuting(threadId: string): ScrollOperation | undefined {
	return executingOperations.get(threadId);
}

/**
 * Get the count of pending operations for a thread.
 *
 * @param threadId - Thread ID to get count for
 * @returns Number of pending operations
 */
export function getPendingCount(threadId: string): number {
	const queue = threadQueues.get(threadId);
	return queue?.length ?? 0;
}

/**
 * Get the highest priority of pending operations for a thread.
 *
 * @param threadId - Thread ID to check
 * @returns The highest priority, or null if no pending operations
 */
export function getHighestPriority(threadId: string): ScrollPriority | null {
	const queue = threadQueues.get(threadId);
	if (!queue || queue.length === 0) return null;

	// Queue is sorted, so first item has highest priority
	return queue[0].priority;
}

/**
 * Check if a new operation with given priority should preempt current operations.
 *
 * @param threadId - Thread ID to check
 * @param priority - Priority of the new operation
 * @returns True if the new operation would preempt
 */
export function wouldPreempt(threadId: string, priority: ScrollPriority): boolean {
	const executing = executingOperations.get(threadId);
	if (executing && PRIORITY_VALUES[executing.priority] < PRIORITY_VALUES[priority]) {
		return true;
	}

	const highest = getHighestPriority(threadId);
	if (highest && PRIORITY_VALUES[highest] < PRIORITY_VALUES[priority]) {
		return true;
	}

	return false;
}

/**
 * Add a listener for priority changes (when user priority operation is added).
 *
 * @param listener - Callback function
 * @returns Unsubscribe function
 */
export function onPriorityChange(
	listener: (threadId: string, operation: ScrollOperation) => void
): () => void {
	priorityListeners.add(listener);
	return () => priorityListeners.delete(listener);
}

/**
 * Clear all operations for all threads.
 */
export function clearAll(): void {
	// Cancel all executing operations
	for (const [threadId, operation] of executingOperations) {
		operation.onCancel?.();
	}
	executingOperations.clear();

	// Cancel all queued operations
	for (const [threadId, queue] of threadQueues) {
		for (const op of queue) {
			op.onCancel?.();
		}
	}
	threadQueues.clear();
	operationMap.clear();
}

/**
 * Get debug info for a thread.
 *
 * @param threadId - Thread ID to get info for
 * @returns Debug information
 */
export function getDebugInfo(threadId: string): {
	executing: ScrollOperation | undefined;
	pending: number;
	highestPriority: ScrollPriority | null;
} {
	return {
		executing: executingOperations.get(threadId),
		pending: getPendingCount(threadId),
		highestPriority: getHighestPriority(threadId)
	};
}
