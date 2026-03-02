/**
 * Stream Throttler
 *
 * Throttles content updates during streaming to reduce Svelte reactive
 * update frequency. This prevents UI jank from too many reactive updates
 * while maintaining a responsive feel.
 *
 * Throttling Strategy:
 * - Minimum interval between updates (default: 50ms = 20fps feel)
 * - Maximum buffer time before forced flush (default: 100ms)
 * - Immediate flush for first chunk (responsive feel)
 * - Batched content accumulation
 *
 * @module streamThrottler
 */

// ============================================================================
// Types
// ============================================================================

/** Callback fired when throttled content is flushed */
export type FlushCallback = (content: string) => void;

/** State change callback */
export type StateChangeCallback = (state: ThrottlerState) => void;

/** Throttler state */
export interface ThrottlerState {
	/** Whether throttler is active (receiving content) */
	isActive: boolean;
	/** Total content received */
	totalContent: string;
	/** Buffered content waiting to be flushed */
	bufferedContent: string;
	/** Last flush timestamp */
	lastFlushTime: number;
	/** Number of chunks received */
	chunksReceived: number;
	/** Number of flushes performed */
	flushCount: number;
}

/** Options for creating a throttler */
export interface ThrottlerOptions {
	/** Minimum interval between flushes in ms (default: 50) */
	minInterval?: number;
	/** Maximum buffer time before forced flush in ms (default: 100) */
	maxBufferTime?: number;
	/** Whether to flush first chunk immediately (default: true) */
	flushFirstChunk?: boolean;
	/** Callback when content is flushed */
	onFlush?: FlushCallback;
	/** Callback when state changes */
	onStateChange?: StateChangeCallback;
}

/** Internal throttler instance */
interface ThrottlerInstance {
	/** Unique ID */
	id: string;
	/** Options */
	options: Required<ThrottlerOptions>;
	/** Current state */
	state: ThrottlerState;
	/** Pending flush timeout */
	flushTimeout: ReturnType<typeof setTimeout> | null;
	/** Maximum buffer timeout */
	maxBufferTimeout: ReturnType<typeof setTimeout> | null;
	/** Whether first chunk was received */
	receivedFirstChunk: boolean;
}

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_MIN_INTERVAL = 50; // 20fps feel
const DEFAULT_MAX_BUFFER_TIME = 100; // Force flush after 100ms

// ============================================================================
// State
// ============================================================================

/** Map of throttler ID to instance */
const throttlers = new Map<string, ThrottlerInstance>();

/** Counter for generating unique IDs */
let throttlerCounter = 0;

// ============================================================================
// Private Functions
// ============================================================================

/**
 * Generate a unique throttler ID.
 */
function generateId(): string {
	return `throttler-${++throttlerCounter}`;
}

/**
 * Get default options.
 */
function getDefaultOptions(): Required<ThrottlerOptions> {
	return {
		minInterval: DEFAULT_MIN_INTERVAL,
		maxBufferTime: DEFAULT_MAX_BUFFER_TIME,
		flushFirstChunk: true,
		onFlush: () => {},
		onStateChange: () => {}
	};
}

/**
 * Create initial state.
 */
function createInitialState(): ThrottlerState {
	return {
		isActive: false,
		totalContent: '',
		bufferedContent: '',
		lastFlushTime: 0,
		chunksReceived: 0,
		flushCount: 0
	};
}

/**
 * Perform a flush of buffered content.
 */
function performFlush(instance: ThrottlerInstance): void {
	const { state, options } = instance;

	// Clear timeouts
	if (instance.flushTimeout) {
		clearTimeout(instance.flushTimeout);
		instance.flushTimeout = null;
	}
	if (instance.maxBufferTimeout) {
		clearTimeout(instance.maxBufferTimeout);
		instance.maxBufferTimeout = null;
	}

	// Only flush if there's buffered content
	if (state.bufferedContent.length === 0) {
		return;
	}

	// Update state
	const flushedContent = state.bufferedContent;
	state.totalContent += flushedContent;
	state.bufferedContent = '';
	state.lastFlushTime = performance.now();
	state.flushCount++;

	// Call flush callback
	options.onFlush(flushedContent);

	// Notify state change
	options.onStateChange({ ...state });
}

/**
 * Schedule a flush after minInterval.
 */
function scheduleFlush(instance: ThrottlerInstance): void {
	const { state, options } = instance;

	// Clear existing timeout
	if (instance.flushTimeout) {
		clearTimeout(instance.flushTimeout);
	}

	// Calculate time until next allowed flush
	const timeSinceLastFlush = performance.now() - state.lastFlushTime;
	const timeUntilFlush = Math.max(0, options.minInterval - timeSinceLastFlush);

	instance.flushTimeout = setTimeout(() => {
		performFlush(instance);
	}, timeUntilFlush);
}

/**
 * Schedule max buffer timeout for forced flush.
 */
function scheduleMaxBufferTimeout(instance: ThrottlerInstance): void {
	const { options } = instance;

	// Clear existing timeout
	if (instance.maxBufferTimeout) {
		clearTimeout(instance.maxBufferTimeout);
	}

	instance.maxBufferTimeout = setTimeout(() => {
		performFlush(instance);
	}, options.maxBufferTime);
}

// ============================================================================
// Public API - Class-based Interface
// ============================================================================

/**
 * StreamThrottler class for instance-based usage.
 * Preferred for component-level throttling.
 */
export class StreamThrottler {
	private instance: ThrottlerInstance;

	constructor(options: ThrottlerOptions = {}) {
		const id = generateId();
		this.instance = {
			id,
			options: { ...getDefaultOptions(), ...options },
			state: createInitialState(),
			flushTimeout: null,
			maxBufferTimeout: null,
			receivedFirstChunk: false
		};
		throttlers.set(id, this.instance);
	}

	/**
	 * Push content to the throttler.
	 * Content will be buffered and flushed according to throttling rules.
	 *
	 * @param content - Content chunk to add
	 */
	push(content: string): void {
		const { state, options } = this.instance;

		state.isActive = true;
		state.chunksReceived++;
		state.bufferedContent += content;

		// Notify state change
		options.onStateChange({ ...state });

		// First chunk handling
		if (!this.instance.receivedFirstChunk && options.flushFirstChunk) {
			this.instance.receivedFirstChunk = true;
			performFlush(this.instance);
			return;
		}

		// Schedule flush if not already scheduled
		if (!this.instance.flushTimeout) {
			scheduleFlush(this.instance);
		}

		// Ensure max buffer timeout is set
		if (!this.instance.maxBufferTimeout) {
			scheduleMaxBufferTimeout(this.instance);
		}
	}

	/**
	 * Force immediate flush of buffered content.
	 */
	flush(): void {
		performFlush(this.instance);
	}

	/**
	 * Reset the throttler state.
	 * Clears all buffers and timeouts.
	 */
	reset(): void {
		const { options } = this.instance;

		// Clear timeouts
		if (this.instance.flushTimeout) {
			clearTimeout(this.instance.flushTimeout);
			this.instance.flushTimeout = null;
		}
		if (this.instance.maxBufferTimeout) {
			clearTimeout(this.instance.maxBufferTimeout);
			this.instance.maxBufferTimeout = null;
		}

		// Reset state
		this.instance.state = createInitialState();
		this.instance.receivedFirstChunk = false;

		// Notify state change
		options.onStateChange({ ...this.instance.state });
	}

	/**
	 * Get current state.
	 */
	getState(): ThrottlerState {
		return { ...this.instance.state };
	}

	/**
	 * Get total accumulated content.
	 */
	getContent(): string {
		return this.instance.state.totalContent + this.instance.state.bufferedContent;
	}

	/**
	 * Check if throttler is active.
	 */
	isActive(): boolean {
		return this.instance.state.isActive;
	}

	/**
	 * Check if there's pending content to flush.
	 */
	hasPendingContent(): boolean {
		return this.instance.state.bufferedContent.length > 0;
	}

	/**
	 * Destroy the throttler.
	 * Clears all resources.
	 */
	destroy(): void {
		this.flush();
		throttlers.delete(this.instance.id);
	}
}

// ============================================================================
// Public API - Functional Interface (for thread-based throttling)
// ============================================================================

/** Map of thread ID to throttler instance */
const threadThrottlers = new Map<string, StreamThrottler>();

/**
 * Create or get a throttler for a specific thread.
 *
 * @param threadId - Thread ID
 * @param options - Throttler options (only used on creation)
 * @returns StreamThrottler instance
 */
export function getThreadThrottler(threadId: string, options?: ThrottlerOptions): StreamThrottler {
	let throttler = threadThrottlers.get(threadId);
	if (!throttler) {
		throttler = new StreamThrottler(options);
		threadThrottlers.set(threadId, throttler);
	}
	return throttler;
}

/**
 * Push content to a thread's throttler.
 *
 * @param threadId - Thread ID
 * @param content - Content to push
 * @param options - Options for throttler creation
 */
export function pushToThread(threadId: string, content: string, options?: ThrottlerOptions): void {
	const throttler = getThreadThrottler(threadId, options);
	throttler.push(content);
}

/**
 * Flush a thread's throttler.
 *
 * @param threadId - Thread ID
 */
export function flushThread(threadId: string): void {
	const throttler = threadThrottlers.get(threadId);
	throttler?.flush();
}

/**
 * Reset a thread's throttler.
 *
 * @param threadId - Thread ID
 */
export function resetThread(threadId: string): void {
	const throttler = threadThrottlers.get(threadId);
	throttler?.reset();
}

/**
 * Get content from a thread's throttler.
 *
 * @param threadId - Thread ID
 * @returns Total content or empty string
 */
export function getThreadContent(threadId: string): string {
	const throttler = threadThrottlers.get(threadId);
	return throttler?.getContent() ?? '';
}

/**
 * Remove a thread's throttler.
 *
 * @param threadId - Thread ID
 */
export function removeThreadThrottler(threadId: string): void {
	const throttler = threadThrottlers.get(threadId);
	if (throttler) {
		throttler.destroy();
		threadThrottlers.delete(threadId);
	}
}

/**
 * Clear all thread throttlers.
 */
export function clearAllThreadThrottlers(): void {
	for (const throttler of threadThrottlers.values()) {
		throttler.destroy();
	}
	threadThrottlers.clear();
}

/**
 * Get all active thread IDs.
 */
export function getActiveThreadIds(): string[] {
	return Array.from(threadThrottlers.keys());
}
