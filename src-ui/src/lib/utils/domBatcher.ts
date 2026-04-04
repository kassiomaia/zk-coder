/**
 * DOM Batcher Utility
 *
 * Batches DOM reads and writes to prevent layout thrashing.
 * Uses requestAnimationFrame to ensure all reads happen before writes
 * in each frame, minimizing forced reflows.
 *
 * @module domBatcher
 */

import { createLogger } from './logger';

const log = createLogger('DOMBatcher');

// ============================================================================
// Types
// ============================================================================

/** A queued DOM read operation */
interface ReadOperation {
	/** Unique identifier for this operation */
	id: string;
	/** The function to execute that reads from DOM */
	execute: () => void;
	/** Thread ID this operation belongs to (for cancellation) */
	threadId?: string;
}

/** A queued DOM write operation */
interface WriteOperation {
	/** Unique identifier for this operation */
	id: string;
	/** The function to execute that writes to DOM */
	execute: () => void;
	/** Thread ID this operation belongs to (for cancellation) */
	threadId?: string;
}

/** Scroll state measurement result */
export interface ScrollState {
	/** Current scroll position from top */
	scrollTop: number;
	/** Total scrollable height */
	scrollHeight: number;
	/** Visible height of the element */
	clientHeight: number;
	/** Distance from bottom (scrollHeight - scrollTop - clientHeight) */
	distanceFromBottom: number;
	/** Whether element is near bottom (within threshold) */
	isNearBottom: boolean;
	/** Maximum scrollable distance */
	maxScroll: number;
	/** Scroll progress as percentage (0-1) */
	progress: number;
}

// ============================================================================
// Constants
// ============================================================================

/** Threshold in pixels for considering element "near bottom" */
const NEAR_BOTTOM_THRESHOLD = 30;

// ============================================================================
// State
// ============================================================================

/** Queue of pending read operations */
const readQueue: ReadOperation[] = [];

/** Queue of pending write operations */
const writeQueue: WriteOperation[] = [];

/** Whether a frame is already scheduled */
let frameScheduled = false;

/** Counter for generating unique IDs */
let operationCounter = 0;

// ============================================================================
// Private Functions
// ============================================================================

/**
 * Generate a unique operation ID.
 */
function generateId(): string {
	return `dom-op-${++operationCounter}`;
}

/**
 * Process all queued operations in a single frame.
 * Reads are processed first, then writes, to prevent layout thrashing.
 */
function processFrame(): void {
	// Process all reads first
	const reads = readQueue.splice(0);
	for (const op of reads) {
		try {
			op.execute();
		} catch (error) {
			log.error('[DOMBatcher] Read operation failed:', error);
		}
	}

	// Then process all writes
	const writes = writeQueue.splice(0);
	for (const op of writes) {
		try {
			op.execute();
		} catch (error) {
			log.error('[DOMBatcher] Write operation failed:', error);
		}
	}

	frameScheduled = false;

	// If more operations were queued during processing, schedule another frame
	if (readQueue.length > 0 || writeQueue.length > 0) {
		scheduleFrame();
	}
}

/**
 * Schedule a frame to process queued operations.
 */
function scheduleFrame(): void {
	if (!frameScheduled) {
		frameScheduled = true;
		requestAnimationFrame(processFrame);
	}
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Schedule a DOM read operation.
 * The operation will be executed in the next animation frame,
 * before any write operations.
 *
 * @param fn - Function that reads from DOM
 * @param threadId - Optional thread ID for cancellation
 * @returns Operation ID for cancellation
 */
export function scheduleRead(fn: () => void, threadId?: string): string {
	const id = generateId();
	readQueue.push({ id, execute: fn, threadId });
	scheduleFrame();
	return id;
}

/**
 * Schedule a DOM write operation.
 * The operation will be executed in the next animation frame,
 * after all read operations have completed.
 *
 * @param fn - Function that writes to DOM
 * @param threadId - Optional thread ID for cancellation
 * @returns Operation ID for cancellation
 */
export function scheduleWrite(fn: () => void, threadId?: string): string {
	const id = generateId();
	writeQueue.push({ id, execute: fn, threadId });
	scheduleFrame();
	return id;
}

/**
 * Schedule a read followed immediately by a write.
 * This is a convenience method for common read-then-write patterns.
 *
 * @param readFn - Function that reads from DOM and returns data
 * @param writeFn - Function that receives read data and writes to DOM
 * @param threadId - Optional thread ID for cancellation
 * @returns Operation ID for cancellation
 */
export function scheduleReadThenWrite<T>(
	readFn: () => T,
	writeFn: (data: T) => void,
	threadId?: string
): string {
	const id = generateId();
	let readData: T;

	readQueue.push({
		id,
		execute: () => {
			readData = readFn();
		},
		threadId
	});

	writeQueue.push({
		id,
		execute: () => {
			writeFn(readData);
		},
		threadId
	});

	scheduleFrame();
	return id;
}

/**
 * Measure scroll state of an element in a batched read.
 * This is more efficient than directly accessing scroll properties
 * as it avoids triggering layout recalculations.
 *
 * @param element - Element to measure
 * @param threshold - Threshold for near-bottom detection (default: 30px)
 * @returns Promise resolving to scroll state
 */
export function measureScrollState(
	element: HTMLElement,
	threshold: number = NEAR_BOTTOM_THRESHOLD
): Promise<ScrollState> {
	return new Promise((resolve) => {
		scheduleRead(() => {
			const { scrollTop, scrollHeight, clientHeight } = element;
			const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
			const maxScroll = scrollHeight - clientHeight;
			const progress = maxScroll > 0 ? scrollTop / maxScroll : 1;

			resolve({
				scrollTop,
				scrollHeight,
				clientHeight,
				distanceFromBottom,
				isNearBottom: distanceFromBottom < threshold,
				maxScroll,
				progress
			});
		});
	});
}

/**
 * Measure scroll state synchronously (use sparingly).
 * Only use when you absolutely need the value immediately.
 * Can cause layout thrashing if used frequently.
 *
 * @param element - Element to measure
 * @param threshold - Threshold for near-bottom detection
 * @returns Scroll state measurement
 */
export function measureScrollStateSync(
	element: HTMLElement,
	threshold: number = NEAR_BOTTOM_THRESHOLD
): ScrollState {
	const { scrollTop, scrollHeight, clientHeight } = element;
	const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
	const maxScroll = scrollHeight - clientHeight;
	const progress = maxScroll > 0 ? scrollTop / maxScroll : 1;

	return {
		scrollTop,
		scrollHeight,
		clientHeight,
		distanceFromBottom,
		isNearBottom: distanceFromBottom < threshold,
		maxScroll,
		progress
	};
}

/**
 * Apply a scroll position in a batched write.
 *
 * @param element - Element to scroll
 * @param scrollTop - Target scroll position
 */
export function applyScrollPosition(element: HTMLElement, scrollTop: number): string {
	return scheduleWrite(() => {
		element.scrollTop = scrollTop;
	});
}

/**
 * Cancel a pending operation by ID.
 *
 * @param id - Operation ID to cancel
 * @returns True if operation was found and cancelled
 */
export function cancelOperation(id: string): boolean {
	const readIndex = readQueue.findIndex((op) => op.id === id);
	if (readIndex !== -1) {
		readQueue.splice(readIndex, 1);
		return true;
	}

	const writeIndex = writeQueue.findIndex((op) => op.id === id);
	if (writeIndex !== -1) {
		writeQueue.splice(writeIndex, 1);
		return true;
	}

	return false;
}

/**
 * Cancel all operations for a specific thread.
 *
 * @param threadId - Thread ID to cancel operations for
 */
export function cancelThreadOperations(threadId: string): void {
	// Remove from read queue
	for (let i = readQueue.length - 1; i >= 0; i--) {
		if (readQueue[i].threadId === threadId) {
			readQueue.splice(i, 1);
		}
	}

	// Remove from write queue
	for (let i = writeQueue.length - 1; i >= 0; i--) {
		if (writeQueue[i].threadId === threadId) {
			writeQueue.splice(i, 1);
		}
	}
}

/**
 * Clear all pending operations.
 */
export function clearAll(): void {
	readQueue.length = 0;
	writeQueue.length = 0;
}

/**
 * Check if there are pending operations.
 */
export function hasPendingOperations(): boolean {
	return readQueue.length > 0 || writeQueue.length > 0;
}

/**
 * Get count of pending operations.
 */
export function getPendingCount(): { reads: number; writes: number } {
	return {
		reads: readQueue.length,
		writes: writeQueue.length
	};
}

/**
 * Execute a callback immediately, bypassing the batcher.
 * Use sparingly - only when immediate DOM access is required.
 *
 * @param fn - Function to execute
 */
export function executeImmediate<T>(fn: () => T): T {
	return fn();
}
