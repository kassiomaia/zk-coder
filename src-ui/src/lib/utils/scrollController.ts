/**
 * Unified Scroll Controller
 *
 * A facade that coordinates all scroll-related subsystems:
 * - DOM Batcher: Batched DOM reads/writes
 * - Priority Queue: Priority-based operation scheduling
 * - Live Animation: Chasing animation for streaming
 * - State Machine: Explicit state management
 *
 * This controller provides a unified API for components to interact with
 * the scroll system without needing to understand the underlying complexity.
 *
 * @module scrollController
 */

import { scheduleRead, measureScrollState, applyScrollPosition } from './domBatcher';
import { cancelThread, hasPending, type ScrollPriority } from './scrollPriorityQueue';
import {
	startLiveAnimation,
	stop as stopAnimation,
	scrollToBottomInstant,
	updateLiveTarget,
	isRunning as isAnimationRunning
} from './liveScrollAnimation';
import {
	sendEvent,
	getState,
	shouldAutoScroll,
	isStreamingActive,
	onStateChange,
	resetThread as resetStateMachine,
	type ScrollState,
	type ScrollEvent
} from './scrollStateMachine';

// ============================================================================
// Types
// ============================================================================

/** Options for registering a thread */
export interface RegisterOptions {
	/** Initial auto-scroll state (default: true) */
	autoScrollEnabled?: boolean;
}

/** Thread context stored by the controller */
interface ThreadContext {
	/** The scrollable element */
	element: HTMLElement;
	/** Whether auto-scroll is enabled */
	autoScrollEnabled: boolean;
	/** Last known scroll height (for detecting content changes) */
	lastScrollHeight: number;
	/** Cleanup function for state listener */
	stateListenerCleanup?: () => void;
	/** Timestamp of last user scroll (for cooldown) */
	lastUserScrollTime: number;
	/** Last scroll top position (for detecting scroll direction) */
	lastScrollTop: number;
	/** Whether the last user scroll was upward (for extended cooldown) */
	wasScrollingUp: boolean;
	/** Flag to ignore scroll events from programmatic scrolls */
	ignoreScrollEvents: boolean;
	/** Whether user is currently dragging the scroll thumb */
	isDraggingThumb: boolean;
	/** Cleanup function for mouse event listeners */
	mouseListenersCleanup?: () => void;
}

/** Options for scroll to bottom */
export interface ScrollToBottomOptions {
	/** Priority level (default: 'programmatic') */
	priority?: ScrollPriority;
	/** Whether to animate (default: true) */
	animated?: boolean;
	/** Callback when scroll completes */
	onComplete?: () => void;
}

// ============================================================================
// Constants
// ============================================================================

/** Threshold for considering scroll "at bottom" */
const AT_BOTTOM_THRESHOLD = 50;

/** Threshold for detecting user scroll up */
const SCROLL_UP_THRESHOLD = 10;

/** Cooldown period after user scroll before auto-scroll can resume (ms) */
const USER_SCROLL_COOLDOWN = 150;

/** Extended cooldown when user is scrolling up (ms) - longer to prevent fighting */
const USER_SCROLL_UP_COOLDOWN = 500;

// ============================================================================
// State
// ============================================================================

/** Map of thread ID to context */
const threadContexts = new Map<string, ThreadContext>();

/** Counter for debug */
let operationCounter = 0;

// ============================================================================
// Private Functions
// ============================================================================

/**
 * Generate a unique operation ID.
 */
function generateOperationId(): string {
	return `ctrl-${++operationCounter}`;
}

/**
 * Execute a function with scroll events ignored (for programmatic scrolls).
 */
function withIgnoredScrollEvents(threadId: string, fn: () => void): void {
	const context = threadContexts.get(threadId);
	if (!context) {
		fn();
		return;
	}

	context.ignoreScrollEvents = true;
	try {
		fn();
	} finally {
		// Use setTimeout to ensure any pending scroll events are processed
		// before we start listening again
		setTimeout(() => {
			context.ignoreScrollEvents = false;
			// Update lastScrollTop to current position after programmatic scroll
			context.lastScrollTop = context.element.scrollTop;
		}, 0);
	}
}

/**
 * Check if element is at or near bottom.
 */
async function isAtBottom(element: HTMLElement, threshold = AT_BOTTOM_THRESHOLD): Promise<boolean> {
	const state = await measureScrollState(element, threshold);
	return state.isNearBottom;
}

/**
 * Check if element is at bottom synchronously.
 */
function isAtBottomSync(element: HTMLElement, threshold = AT_BOTTOM_THRESHOLD): boolean {
	const distanceFromBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
	return distanceFromBottom <= threshold;
}

/**
 * Get the current scroll position synchronously.
 */
function getScrollPosition(element: HTMLElement): { scrollTop: number; scrollHeight: number } {
	return {
		scrollTop: element.scrollTop,
		scrollHeight: element.scrollHeight
	};
}

/**
 * Handle state machine transitions.
 */
function handleStateTransition(
	threadId: string,
	newState: ScrollState,
	previousState: ScrollState,
	event: ScrollEvent
): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	const { element } = context;

	// Update auto-scroll enabled based on state
	context.autoScrollEnabled = shouldAutoScroll(threadId);

	// Handle specific transitions
	if (previousState === 'user_paused' && newState === 'streaming') {
		// User scrolled back to bottom during streaming - resume auto-scroll
		context.autoScrollEnabled = true;
	}

	if (event === 'STREAM_END') {
		// Streaming ended - ensure we're at bottom
		if (context.autoScrollEnabled) {
			withIgnoredScrollEvents(threadId, () => {
				scrollToBottomInstant(element);
			});
		}
	}

	if (event === 'USER_SEND_MESSAGE') {
		// User sent message - force scroll to bottom
		context.autoScrollEnabled = true;
		withIgnoredScrollEvents(threadId, () => {
			scrollToBottomInstant(element);
		});
	}

	if (event === 'THREAD_CHANGE') {
		// Thread changed - reset everything
		stopAnimation(threadId);
	}
}

/**
 * Schedule a scroll to bottom operation.
 */
function scheduleScrollToBottom(threadId: string, options: ScrollToBottomOptions = {}): string {
	const context = threadContexts.get(threadId);
	if (!context) return '';

	const { element, autoScrollEnabled } = context;
	const priority = options.priority ?? 'programmatic';
	const animated = options.animated ?? true;

	// Don't scroll if auto-scroll is disabled (unless user priority)
	if (!autoScrollEnabled && priority !== 'user') {
		return '';
	}

	const opId = generateOperationId();

	// Execute directly - the queue was never being processed
	if (!element.isConnected) return '';

	if (animated) {
		// Set ignore flag for the duration of the animation
		context.ignoreScrollEvents = true;
		startLiveAnimation(element, {
			threadId,
			priority,
			animated: true,
			onComplete: () => {
				// Reset ignore flag after animation completes
				setTimeout(() => {
					if (context) {
						context.ignoreScrollEvents = false;
						context.lastScrollTop = context.element.scrollTop;
					}
				}, 0);
				options.onComplete?.();
			}
		});
	} else {
		withIgnoredScrollEvents(threadId, () => {
			scrollToBottomInstant(element);
		});
		options.onComplete?.();
	}

	return opId;
}

/**
 * Handle user scroll detection (internal implementation).
 */
function processUserScroll(threadId: string, element: HTMLElement): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	// Ignore scroll events from programmatic scrolls
	if (context.ignoreScrollEvents) {
		console.log('[ScrollController] processUserScroll: ignoring (programmatic scroll)');
		return;
	}

	const currentState = getState(threadId);
	const { scrollTop, scrollHeight } = getScrollPosition(element);

	// Detect scroll direction
	const scrollDelta = context.lastScrollTop - scrollTop; // positive = scrolled up, negative = scrolled down
	const isScrollingUp = scrollDelta > 2; // 2px threshold to ignore minor changes

	// Update tracking
	context.lastScrollTop = scrollTop;
	context.wasScrollingUp = isScrollingUp;

	// Check distance from bottom
	const distanceFromBottom = scrollHeight - scrollTop - element.clientHeight;

	console.log('[ScrollController] processUserScroll:', {
		threadId,
		currentState,
		distanceFromBottom,
		scrollDelta,
		isScrollingUp,
		isDraggingThumb: context.isDraggingThumb,
		threshold: AT_BOTTOM_THRESHOLD,
		autoScrollEnabled: context.autoScrollEnabled
	});

	// If user is actively dragging the scroll thumb and scrolling up, disable auto-scroll
	// This allows user to view older content during streaming
	if (context.isDraggingThumb && isScrollingUp) {
		if (context.autoScrollEnabled) {
			console.log('[ScrollController] DISABLING auto-scroll - user is dragging scroll thumb up');
			sendEvent(threadId, 'USER_SCROLL_UP');
			context.autoScrollEnabled = false;
			stopAnimation(threadId);
		}
		return;
	}

	// Handle re-enabling auto-scroll when user scrolls back to bottom from paused state
	if (currentState === 'user_paused') {
		if (distanceFromBottom <= AT_BOTTOM_THRESHOLD) {
			// User scrolled back to bottom while paused
			console.log('[ScrollController] RE-ENABLING auto-scroll - user scrolled to bottom');
			sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM');
			context.autoScrollEnabled = true;
		}
	}

	// Update last scroll height
	context.lastScrollHeight = scrollHeight;
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Register a scrollable element for a thread.
 *
 * @param threadId - Thread ID
 * @param element - Scrollable element
 * @param options - Registration options
 */
export function registerThreadElement(
	threadId: string,
	element: HTMLElement,
	options: RegisterOptions = {}
): void {
	// Unregister if already registered
	if (threadContexts.has(threadId)) {
		unregisterThread(threadId);
	}

	const context: ThreadContext = {
		element,
		autoScrollEnabled: options.autoScrollEnabled ?? true,
		lastScrollHeight: element.scrollHeight,
		lastUserScrollTime: 0,
		lastScrollTop: element.scrollTop,
		wasScrollingUp: false,
		ignoreScrollEvents: false,
		isDraggingThumb: false
	};

	threadContexts.set(threadId, context);

	// Set up state change listener
	const cleanup = onStateChange((state, previousState, event) => {
		handleStateTransition(threadId, state, previousState, event);
	});
	context.stateListenerCleanup = cleanup;

	// Set up mouse event listeners to detect scroll thumb dragging
	const handleMouseDown = (e: MouseEvent) => {
		// Check if click is on the scrollbar area (right side of element)
		const rect = element.getBoundingClientRect();
		const scrollbarWidth = rect.width - element.clientWidth;
		const isOnScrollbar = e.clientX >= rect.right - scrollbarWidth;

		if (isOnScrollbar) {
			context.isDraggingThumb = true;
			console.log('[ScrollController] User started dragging scroll thumb');
		}
	};

	const handleMouseUp = () => {
		if (context.isDraggingThumb) {
			context.isDraggingThumb = false;
			console.log('[ScrollController] User stopped dragging scroll thumb');
		}
	};

	const handleMouseLeave = () => {
		if (context.isDraggingThumb) {
			context.isDraggingThumb = false;
			console.log('[ScrollController] User left element while dragging');
		}
	};

	element.addEventListener('mousedown', handleMouseDown);
	window.addEventListener('mouseup', handleMouseUp);
	element.addEventListener('mouseleave', handleMouseLeave);

	context.mouseListenersCleanup = () => {
		element.removeEventListener('mousedown', handleMouseDown);
		window.removeEventListener('mouseup', handleMouseUp);
		element.removeEventListener('mouseleave', handleMouseLeave);
	};

	// Initialize state machine
	resetStateMachine(threadId);
}

/**
 * Unregister a thread element.
 *
 * @param threadId - Thread ID
 */
export function unregisterThread(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	// Clean up animation
	stopAnimation(threadId);

	// Cancel pending operations
	cancelThread(threadId);

	// Clean up state listener
	context.stateListenerCleanup?.();

	// Clean up mouse event listeners
	context.mouseListenersCleanup?.();

	// Remove context
	threadContexts.delete(threadId);

	// Reset state machine
	resetStateMachine(threadId);
}

/**
 * Notify that streaming has started.
 *
 * @param threadId - Thread ID
 */
export function handleStreamingStart(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) {
		console.log('[ScrollController] handleStreamingStart: no context for thread', threadId);
		return;
	}

	console.log('[ScrollController] handleStreamingStart:', threadId);
	sendEvent(threadId, 'STREAM_START');
	context.autoScrollEnabled = true;
}

/**
 * Notify that streaming has ended.
 *
 * @param threadId - Thread ID
 */
export function handleStreamingEnd(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	sendEvent(threadId, 'STREAM_END');

	// Final scroll to bottom if auto-scroll is enabled
	if (context.autoScrollEnabled) {
		withIgnoredScrollEvents(threadId, () => {
			scrollToBottomInstant(context.element);
		});
	}
}

/**
 * Handle content update during streaming.
 * Call this when new content is added to the chat.
 *
 * @param threadId - Thread ID
 */
export function handleContentUpdate(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) {
		console.log('[ScrollController] handleContentUpdate: no context for thread', threadId);
		return;
	}

	const { element, autoScrollEnabled } = context;
	const currentState = getState(threadId);
	const streamingActive = isStreamingActive(threadId);
	const atBottom = isAtBottomSync(element);
	const distanceFromBottom = element.scrollHeight - element.scrollTop - element.clientHeight;

	console.log('[ScrollController] handleContentUpdate:', {
		threadId,
		autoScrollEnabled,
		streamingActive,
		currentState,
		atBottom,
		distanceFromBottom,
		scrollHeight: element.scrollHeight,
		scrollTop: element.scrollTop,
		clientHeight: element.clientHeight
	});

	// Always update lastScrollHeight
	context.lastScrollHeight = element.scrollHeight;

	// Update live target for running animations
	updateLiveTarget(element);

	// If auto-scroll is enabled and we're in streaming state, scroll to bottom
	// During streaming, auto-scroll stays enabled regardless of scroll position
	if (autoScrollEnabled && streamingActive) {
		if (atBottom) {
			// Already at bottom, just update position instantly
			console.log('[ScrollController] scrolling instantly to bottom');
			withIgnoredScrollEvents(threadId, () => {
				scrollToBottomInstant(element);
			});
		} else if (!isAnimationRunning(threadId)) {
			// Not at bottom and no animation running, start one
			console.log('[ScrollController] starting animation to bottom');
			scheduleScrollToBottom(threadId, { priority: 'background', animated: true });
		} else {
			console.log('[ScrollController] animation already running, target updated');
		}
	} else {
		console.log('[ScrollController] NOT scrolling:', { autoScrollEnabled, streamingActive });
	}
}

/**
 * Handle user scroll event.
 * Call this from the element's on:scroll handler.
 *
 * @param threadId - Thread ID
 */
export function handleUserScroll(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	processUserScroll(threadId, context.element);
}

/**
 * Handle user clicking the scroll-to-bottom button.
 *
 * @param threadId - Thread ID
 */
export function handleUserClickScrollDown(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	sendEvent(threadId, 'USER_CLICK_SCROLL_DOWN');
	context.autoScrollEnabled = true;
	// Clear cooldown and scroll direction since this is an explicit user action
	context.lastUserScrollTime = 0;
	context.wasScrollingUp = false;

	// Immediate scroll to bottom with high priority (uses scheduleScrollToBottom which handles ignore flag)
	scheduleScrollToBottom(threadId, { priority: 'user', animated: true });
}

/**
 * Handle user sending a message.
 *
 * @param threadId - Thread ID
 */
export function handleUserSendMessage(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	sendEvent(threadId, 'USER_SEND_MESSAGE');
	context.autoScrollEnabled = true;
	// Clear cooldown and scroll direction since this is an explicit user action
	context.lastUserScrollTime = 0;
	context.wasScrollingUp = false;

	// Immediate scroll to bottom (use wrapper to ignore scroll events)
	withIgnoredScrollEvents(threadId, () => {
		scrollToBottomInstant(context.element);
	});
}

/**
 * Handle thread change (switching to a different thread).
 *
 * @param oldThreadId - Previous thread ID (if any)
 * @param newThreadId - New thread ID
 */
export function handleThreadChange(oldThreadId: string | null, newThreadId: string): void {
	// Notify state machine about thread change
	if (oldThreadId) {
		sendEvent(oldThreadId, 'THREAD_CHANGE');
	}

	// The new thread should already be registered
	// Reset its state
	resetStateMachine(newThreadId);
}

/**
 * Scroll to bottom programmatically.
 *
 * @param threadId - Thread ID
 * @param options - Scroll options
 */
export function scrollToBottom(threadId: string, options: ScrollToBottomOptions = {}): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	const { element } = context;
	const animated = options.animated ?? true;
	const priority = options.priority ?? 'programmatic';

	if (animated) {
		scheduleScrollToBottom(threadId, { ...options, priority });
	} else {
		withIgnoredScrollEvents(threadId, () => {
			scrollToBottomInstant(element);
		});
		options.onComplete?.();
	}
}

/**
 * Check if auto-scroll is enabled for a thread.
 *
 * @param threadId - Thread ID
 * @returns True if auto-scroll is enabled
 */
export function isAutoScrollEnabled(threadId: string): boolean {
	const context = threadContexts.get(threadId);
	return context?.autoScrollEnabled ?? false;
}

/**
 * Get the current scroll state for a thread.
 *
 * @param threadId - Thread ID
 * @returns Current scroll state
 */
export function getThreadScrollState(threadId: string): ScrollState {
	return getState(threadId);
}

/**
 * Check if thread is currently streaming.
 *
 * @param threadId - Thread ID
 * @returns True if streaming is active
 */
export function isThreadStreaming(threadId: string): boolean {
	return isStreamingActive(threadId);
}

/**
 * Check if a thread has pending scroll operations.
 *
 * @param threadId - Thread ID
 * @returns True if there are pending operations
 */
export function hasPendingOperations(threadId: string): boolean {
	return hasPending(threadId);
}

/**
 * Force enable auto-scroll for a thread.
 *
 * @param threadId - Thread ID
 */
export function enableAutoScroll(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	context.autoScrollEnabled = true;
	// Clear cooldown and scroll direction since this is typically called for explicit user actions
	context.lastUserScrollTime = 0;
	context.wasScrollingUp = false;
}

/**
 * Force disable auto-scroll for a thread.
 *
 * @param threadId - Thread ID
 */
export function disableAutoScroll(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	context.autoScrollEnabled = false;
	stopAnimation(threadId);
}

/**
 * Reset scroll lock state (e.g., after thread switch).
 *
 * @param threadId - Thread ID
 */
export function resetScrollLock(threadId: string): void {
	const context = threadContexts.get(threadId);
	if (!context) return;

	context.autoScrollEnabled = true;
	context.lastScrollHeight = context.element.scrollHeight;
	resetStateMachine(threadId);
	stopAnimation(threadId);
}

/**
 * Get debug info for a thread.
 *
 * @param threadId - Thread ID
 * @returns Debug information
 */
export function getDebugInfo(threadId: string): {
	hasContext: boolean;
	state: ScrollState;
	autoScrollEnabled: boolean;
	hasPendingOps: boolean;
	isAnimating: boolean;
} {
	const context = threadContexts.get(threadId);

	return {
		hasContext: !!context,
		state: getState(threadId),
		autoScrollEnabled: context?.autoScrollEnabled ?? false,
		hasPendingOps: hasPending(threadId),
		isAnimating: isAnimationRunning(threadId)
	};
}

/**
 * Clear all thread contexts (for cleanup).
 */
export function clearAll(): void {
	for (const threadId of threadContexts.keys()) {
		unregisterThread(threadId);
	}
}
