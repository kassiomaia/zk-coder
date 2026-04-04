/**
 * Scroll State Machine
 *
 * Explicit state management for scroll behavior with clear transitions.
 * This ensures predictable behavior during streaming and user interactions.
 *
 * States:
 * - idle: No streaming, user can scroll freely
 * - streaming: Content is being streamed, auto-scroll may be active
 * - user_paused: User scrolled up during streaming, auto-scroll disabled
 * - resuming: Transitioning back to streaming after user scrolled to bottom
 *
 * Events:
 * - STREAM_START: LLM starts streaming content
 * - STREAM_END: LLM finishes streaming
 * - USER_SCROLL_UP: User scrolls upward (away from bottom)
 * - USER_SCROLL_TO_BOTTOM: User scrolls to bottom manually
 * - USER_CLICK_SCROLL_DOWN: User clicks scroll-to-bottom button
 * - USER_SEND_MESSAGE: User sends a new message
 * - THREAD_CHANGE: Active thread changes
 *
 * @module scrollStateMachine
 */

import { createLogger } from './logger';

const log = createLogger('ScrollStateMachine');

// ============================================================================
// Types
// ============================================================================

/** Possible states of the scroll state machine */
export type ScrollState = 'idle' | 'streaming' | 'user_paused' | 'resuming';

/** Events that trigger state transitions */
export type ScrollEvent =
	| 'STREAM_START'
	| 'STREAM_END'
	| 'USER_SCROLL_UP'
	| 'USER_SCROLL_TO_BOTTOM'
	| 'USER_CLICK_SCROLL_DOWN'
	| 'USER_SEND_MESSAGE'
	| 'THREAD_CHANGE';

/** State transition result */
export interface StateTransition {
	/** Previous state */
	from: ScrollState;
	/** New state */
	to: ScrollState;
	/** Event that triggered the transition */
	event: ScrollEvent;
	/** Whether the transition was valid */
	valid: boolean;
}

/** State change listener */
export type StateChangeListener = (
	state: ScrollState,
	previousState: ScrollState,
	event: ScrollEvent
) => void;

/** State machine context for a thread */
interface ThreadContext {
	/** Current state */
	state: ScrollState;
	/** State history for debugging */
	history: Array<{
		from: ScrollState;
		to: ScrollState;
		event: ScrollEvent;
		timestamp: number;
	}>;
	/** Last event timestamp */
	lastEventTime: number;
}

// ============================================================================
// Constants
// ============================================================================

/** Valid state transitions: state -> event -> new state */
const TRANSITIONS: Record<ScrollState, Record<ScrollEvent, ScrollState>> = {
	idle: {
		STREAM_START: 'streaming',
		STREAM_END: 'idle',
		USER_SCROLL_UP: 'idle',
		USER_SCROLL_TO_BOTTOM: 'idle',
		USER_CLICK_SCROLL_DOWN: 'idle',
		USER_SEND_MESSAGE: 'idle',
		THREAD_CHANGE: 'idle'
	},
	streaming: {
		STREAM_START: 'streaming',
		STREAM_END: 'idle',
		USER_SCROLL_UP: 'user_paused',
		USER_SCROLL_TO_BOTTOM: 'streaming',
		USER_CLICK_SCROLL_DOWN: 'streaming',
		USER_SEND_MESSAGE: 'streaming',
		THREAD_CHANGE: 'idle'
	},
	user_paused: {
		STREAM_START: 'user_paused',
		STREAM_END: 'idle',
		USER_SCROLL_UP: 'user_paused',
		USER_SCROLL_TO_BOTTOM: 'resuming',
		USER_CLICK_SCROLL_DOWN: 'resuming',
		USER_SEND_MESSAGE: 'resuming',
		THREAD_CHANGE: 'idle'
	},
	resuming: {
		STREAM_START: 'streaming',
		STREAM_END: 'idle',
		USER_SCROLL_UP: 'user_paused',
		USER_SCROLL_TO_BOTTOM: 'streaming',
		USER_CLICK_SCROLL_DOWN: 'streaming',
		USER_SEND_MESSAGE: 'streaming',
		THREAD_CHANGE: 'idle'
	}
};

/** Max history entries per thread */
const MAX_HISTORY = 20;

// ============================================================================
// State
// ============================================================================

/** Map of thread ID to context */
const threadContexts = new Map<string, ThreadContext>();

/** Set of state change listeners */
const stateListeners = new Set<StateChangeListener>();

/** Counter for debug */
let eventCounter = 0;

// ============================================================================
// Private Functions
// ============================================================================

/**
 * Get or create context for a thread.
 */
function getThreadContext(threadId: string): ThreadContext {
	let context = threadContexts.get(threadId);
	if (!context) {
		context = {
			state: 'idle',
			history: [],
			lastEventTime: 0
		};
		threadContexts.set(threadId, context);
	}
	return context;
}

/**
 * Add entry to history (for debugging).
 */
function addToHistory(
	context: ThreadContext,
	from: ScrollState,
	to: ScrollState,
	event: ScrollEvent
): void {
	context.history.push({
		from,
		to,
		event,
		timestamp: Date.now()
	});

	// Trim history if too long
	if (context.history.length > MAX_HISTORY) {
		context.history.shift();
	}
}

/**
 * Notify all listeners of state change.
 */
function notifyListeners(state: ScrollState, previousState: ScrollState, event: ScrollEvent): void {
	stateListeners.forEach((listener) => {
		try {
			listener(state, previousState, event);
		} catch (error) {
			log.error('[ScrollStateMachine] Listener error:', error);
		}
	});
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Send an event to the state machine for a specific thread.
 *
 * @param threadId - Thread ID
 * @param event - Event to send
 * @returns State transition result
 */
export function sendEvent(threadId: string, event: ScrollEvent): StateTransition {
	const context = getThreadContext(threadId);
	const currentState = context.state;
	const transitions = TRANSITIONS[currentState];
	const newState = transitions[event];

	const valid = newState !== undefined;

	if (valid) {
		const previousState = currentState;
		context.state = newState!;
		context.lastEventTime = Date.now();
		addToHistory(context, previousState, newState!, event);

		// Notify listeners
		notifyListeners(newState!, previousState, event);

		eventCounter++;
	}

	return {
		from: currentState,
		to: valid ? newState! : currentState,
		event,
		valid
	};
}

/**
 * Get the current state for a thread.
 *
 * @param threadId - Thread ID
 * @returns Current state
 */
export function getState(threadId: string): ScrollState {
	return getThreadContext(threadId).state;
}

/**
 * Check if a thread is in a specific state.
 *
 * @param threadId - Thread ID
 * @param state - State to check
 * @returns True if in that state
 */
export function isInState(threadId: string, state: ScrollState): boolean {
	return getThreadContext(threadId).state === state;
}

/**
 * Check if streaming is active (streaming or user_paused or resuming).
 *
 * @param threadId - Thread ID
 * @returns True if streaming is active
 */
export function isStreamingActive(threadId: string): boolean {
	const state = getThreadContext(threadId).state;
	return state === 'streaming' || state === 'user_paused' || state === 'resuming';
}

/**
 * Check if auto-scroll should be enabled.
 * Auto-scroll is enabled in 'streaming' and 'resuming' states.
 *
 * @param threadId - Thread ID
 * @returns True if auto-scroll should be enabled
 */
export function shouldAutoScroll(threadId: string): boolean {
	const state = getThreadContext(threadId).state;
	return state === 'streaming' || state === 'resuming';
}

/**
 * Check if user has paused auto-scroll.
 *
 * @param threadId - Thread ID
 * @returns True if user has paused
 */
export function isUserPaused(threadId: string): boolean {
	return getThreadContext(threadId).state === 'user_paused';
}

/**
 * Reset state for a thread (e.g., on thread change).
 *
 * @param threadId - Thread ID
 */
export function resetThread(threadId: string): void {
	threadContexts.delete(threadId);
}

/**
 * Reset all threads.
 */
export function resetAll(): void {
	threadContexts.clear();
}

/**
 * Add a state change listener.
 *
 * @param listener - Callback function
 * @returns Unsubscribe function
 */
export function onStateChange(listener: StateChangeListener): () => void {
	stateListeners.add(listener);
	return () => stateListeners.delete(listener);
}

/**
 * Get valid transitions from current state.
 *
 * @param threadId - Thread ID
 * @returns Map of events to resulting states
 */
export function getValidTransitions(threadId: string): Record<ScrollEvent, ScrollState> {
	const state = getThreadContext(threadId).state;
	return { ...TRANSITIONS[state] };
}

/**
 * Get state history for debugging.
 *
 * @param threadId - Thread ID
 * @returns Array of state transitions
 */
export function getHistory(threadId: string): Array<{
	from: ScrollState;
	to: ScrollState;
	event: ScrollEvent;
	timestamp: number;
}> {
	return [...getThreadContext(threadId).history];
}

/**
 * Get debug info for a thread.
 *
 * @param threadId - Thread ID
 * @returns Debug information
 */
export function getDebugInfo(threadId: string): {
	state: ScrollState;
	historyLength: number;
	lastEventTime: number;
	validTransitions: Record<ScrollEvent, ScrollState>;
} {
	const context = getThreadContext(threadId);
	return {
		state: context.state,
		historyLength: context.history.length,
		lastEventTime: context.lastEventTime,
		validTransitions: getValidTransitions(threadId)
	};
}

/**
 * Get total event count (for debugging).
 */
export function getEventCount(): number {
	return eventCounter;
}

// ============================================================================
// State Description Helpers
// ============================================================================

/**
 * Get a human-readable description of a state.
 */
export function describeState(state: ScrollState): string {
	const descriptions: Record<ScrollState, string> = {
		idle: 'No streaming active, user can scroll freely',
		streaming: 'Content is being streamed, auto-scroll active',
		user_paused: 'User scrolled up during streaming, auto-scroll paused',
		resuming: 'Transitioning back to streaming, auto-scroll resuming'
	};
	return descriptions[state];
}

/**
 * Get a human-readable description of an event.
 */
export function describeEvent(event: ScrollEvent): string {
	const descriptions: Record<ScrollEvent, string> = {
		STREAM_START: 'LLM started streaming content',
		STREAM_END: 'LLM finished streaming',
		USER_SCROLL_UP: 'User scrolled upward (away from bottom)',
		USER_SCROLL_TO_BOTTOM: 'User scrolled to bottom manually',
		USER_CLICK_SCROLL_DOWN: 'User clicked the scroll-to-bottom button',
		USER_SEND_MESSAGE: 'User sent a new message',
		THREAD_CHANGE: 'Active thread changed'
	};
	return descriptions[event];
}

/**
 * Get all possible states.
 */
export function getAllStates(): ScrollState[] {
	return ['idle', 'streaming', 'user_paused', 'resuming'];
}

/**
 * Get all possible events.
 */
export function getAllEvents(): ScrollEvent[] {
	return [
		'STREAM_START',
		'STREAM_END',
		'USER_SCROLL_UP',
		'USER_SCROLL_TO_BOTTOM',
		'USER_CLICK_SCROLL_DOWN',
		'USER_SEND_MESSAGE',
		'THREAD_CHANGE'
	];
}
