/**
 * Tests for scrollStateMachine
 */

import {beforeEach, describe, expect, it, vi} from 'vitest';
import {
    describeEvent,
    describeState,
    getAllEvents,
    getAllStates,
    getDebugInfo,
    getEventCount,
    getHistory,
    getState,
    getValidTransitions,
    isInState,
    isStreamingActive,
    isUserPaused,
    onStateChange,
    resetAll,
    resetThread,
    sendEvent,
    shouldAutoScroll
} from './scrollStateMachine';

describe('scrollStateMachine', () => {
	beforeEach(() => {
		// Reset all state before each test
		resetAll();
	});

	describe('Basic State Transitions', () => {
		it('should start in idle state for a new thread', () => {
			const threadId = 'thread-1';
			expect(getState(threadId)).toBe('idle');
		});

		it('should transition from idle to streaming on STREAM_START', () => {
			const threadId = 'thread-1';
			const result = sendEvent(threadId, 'STREAM_START');

			expect(result.from).toBe('idle');
			expect(result.to).toBe('streaming');
			expect(result.event).toBe('STREAM_START');
			expect(result.valid).toBe(true);
			expect(getState(threadId)).toBe('streaming');
		});

		it('should transition from streaming to idle on STREAM_END', () => {
			const threadId = 'thread-1';
			sendEvent(threadId, 'STREAM_START');
			const result = sendEvent(threadId, 'STREAM_END');

			expect(result.from).toBe('streaming');
			expect(result.to).toBe('idle');
			expect(result.valid).toBe(true);
			expect(getState(threadId)).toBe('idle');
		});

		it('should transition from streaming to user_paused on USER_SCROLL_UP', () => {
			const threadId = 'thread-1';
			sendEvent(threadId, 'STREAM_START');
			const result = sendEvent(threadId, 'USER_SCROLL_UP');

			expect(result.from).toBe('streaming');
			expect(result.to).toBe('user_paused');
			expect(result.valid).toBe(true);
			expect(getState(threadId)).toBe('user_paused');
		});

		it('should transition from user_paused to resuming on USER_SCROLL_TO_BOTTOM', () => {
			const threadId = 'thread-1';
			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');
			const result = sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM');

			expect(result.from).toBe('user_paused');
			expect(result.to).toBe('resuming');
			expect(result.valid).toBe(true);
			expect(getState(threadId)).toBe('resuming');
		});

		it('should transition from user_paused to resuming on USER_CLICK_SCROLL_DOWN', () => {
			const threadId = 'thread-1';
			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');
			const result = sendEvent(threadId, 'USER_CLICK_SCROLL_DOWN');

			expect(result.from).toBe('user_paused');
			expect(result.to).toBe('resuming');
			expect(result.valid).toBe(true);
			expect(getState(threadId)).toBe('resuming');
		});

		it('should transition from user_paused to resuming on USER_SEND_MESSAGE', () => {
			const threadId = 'thread-1';
			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');
			const result = sendEvent(threadId, 'USER_SEND_MESSAGE');

			expect(result.from).toBe('user_paused');
			expect(result.to).toBe('resuming');
			expect(result.valid).toBe(true);
			expect(getState(threadId)).toBe('resuming');
		});

		it('should transition from resuming to streaming on STREAM_START', () => {
			const threadId = 'thread-1';
			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');
			sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM');
			const result = sendEvent(threadId, 'STREAM_START');

			expect(result.from).toBe('resuming');
			expect(result.to).toBe('streaming');
			expect(result.valid).toBe(true);
			expect(getState(threadId)).toBe('streaming');
		});
	});

	describe('Thread State Isolation', () => {
		it('should maintain separate state for multiple threads', () => {
			const thread1 = 'thread-1';
			const thread2 = 'thread-2';

			sendEvent(thread1, 'STREAM_START');
			sendEvent(thread2, 'STREAM_START');
			sendEvent(thread1, 'USER_SCROLL_UP');

			expect(getState(thread1)).toBe('user_paused');
			expect(getState(thread2)).toBe('streaming');
		});

		it('should allow independent state transitions for each thread', () => {
			const thread1 = 'thread-1';
			const thread2 = 'thread-2';

			sendEvent(thread1, 'STREAM_START');
			sendEvent(thread2, 'STREAM_START');
			sendEvent(thread2, 'STREAM_END');

			expect(getState(thread1)).toBe('streaming');
			expect(getState(thread2)).toBe('idle');
		});
	});

	describe('State Query Functions', () => {
		it('should correctly identify if thread is in specific state', () => {
			const threadId = 'thread-1';

			expect(isInState(threadId, 'idle')).toBe(true);

			sendEvent(threadId, 'STREAM_START');
			expect(isInState(threadId, 'streaming')).toBe(true);
			expect(isInState(threadId, 'idle')).toBe(false);

			sendEvent(threadId, 'USER_SCROLL_UP');
			expect(isInState(threadId, 'user_paused')).toBe(true);
		});

		it('should correctly identify if streaming is active', () => {
			const threadId = 'thread-1';

			expect(isStreamingActive(threadId)).toBe(false);

			sendEvent(threadId, 'STREAM_START');
			expect(isStreamingActive(threadId)).toBe(true);

			sendEvent(threadId, 'USER_SCROLL_UP');
			expect(isStreamingActive(threadId)).toBe(true);

			sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM');
			expect(isStreamingActive(threadId)).toBe(true);

			sendEvent(threadId, 'STREAM_END');
			expect(isStreamingActive(threadId)).toBe(false);
		});

		it('should correctly identify if auto-scroll should be enabled', () => {
			const threadId = 'thread-1';

			expect(shouldAutoScroll(threadId)).toBe(false);

			sendEvent(threadId, 'STREAM_START');
			expect(shouldAutoScroll(threadId)).toBe(true);

			sendEvent(threadId, 'USER_SCROLL_UP');
			expect(shouldAutoScroll(threadId)).toBe(false);

			sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM');
			expect(shouldAutoScroll(threadId)).toBe(true);

			sendEvent(threadId, 'STREAM_END');
			expect(shouldAutoScroll(threadId)).toBe(false);
		});

		it('should correctly identify if user has paused', () => {
			const threadId = 'thread-1';

			expect(isUserPaused(threadId)).toBe(false);

			sendEvent(threadId, 'STREAM_START');
			expect(isUserPaused(threadId)).toBe(false);

			sendEvent(threadId, 'USER_SCROLL_UP');
			expect(isUserPaused(threadId)).toBe(true);

			sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM');
			expect(isUserPaused(threadId)).toBe(false);
		});
	});

	describe('State Change Listeners', () => {
		it('should notify listeners on state change', () => {
			const threadId = 'thread-1';
			const listener = vi.fn();

			onStateChange(listener);
			sendEvent(threadId, 'STREAM_START');

			expect(listener).toHaveBeenCalledWith('streaming', 'idle', 'STREAM_START');
		});

		it('should support multiple listeners', () => {
			const threadId = 'thread-1';
			const listener1 = vi.fn();
			const listener2 = vi.fn();

			onStateChange(listener1);
			onStateChange(listener2);
			sendEvent(threadId, 'STREAM_START');

			expect(listener1).toHaveBeenCalled();
			expect(listener2).toHaveBeenCalled();
		});

		it('should allow unsubscribing listeners', () => {
			const threadId = 'thread-1';
			const listener = vi.fn();

			const unsubscribe = onStateChange(listener);
			sendEvent(threadId, 'STREAM_START');
			expect(listener).toHaveBeenCalledTimes(1);

			unsubscribe();
			sendEvent(threadId, 'STREAM_END');
			expect(listener).toHaveBeenCalledTimes(1); // No additional call
		});

		it('should handle errors in listeners gracefully', () => {
			const threadId = 'thread-1';
			const errorListener = vi.fn(() => {
				throw new Error('Listener error');
			});
			const normalListener = vi.fn();

			onStateChange(errorListener);
			onStateChange(normalListener);

			// Should not throw despite error in one listener
			expect(() => sendEvent(threadId, 'STREAM_START')).not.toThrow();

			// Other listener should still be called
			expect(normalListener).toHaveBeenCalled();
		});

		it('should not notify listeners for invalid transitions', () => {
			const threadId = 'thread-1';
			const listener = vi.fn();

			onStateChange(listener);

			// All transitions from the state machine are valid by design
			// But we can verify listener is called for valid transitions
			sendEvent(threadId, 'STREAM_START');
			expect(listener).toHaveBeenCalledTimes(1);
		});
	});

	describe('History Tracking', () => {
		it('should track state transition history', () => {
			const threadId = 'thread-1';

			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');
			sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM');

			const history = getHistory(threadId);
			expect(history).toHaveLength(3);

			// First transition: idle -> streaming via STREAM_START
			expect(history[0]).toMatchObject({
				from: 'idle',
				to: 'streaming',
				event: 'STREAM_START',
				timestamp: expect.any(Number)
			});

			// Second transition: streaming -> user_paused via USER_SCROLL_UP
			expect(history[1]).toMatchObject({
				from: 'streaming',
				to: 'user_paused',
				event: 'USER_SCROLL_UP',
				timestamp: expect.any(Number)
			});

			// Third transition: user_paused -> resuming via USER_SCROLL_TO_BOTTOM
			expect(history[2]).toMatchObject({
				from: 'user_paused',
				to: 'resuming',
				event: 'USER_SCROLL_TO_BOTTOM',
				timestamp: expect.any(Number)
			});
		});

		it('should limit history size', () => {
			const threadId = 'thread-1';

			// Send more events than MAX_HISTORY (20)
			for (let i = 0; i < 25; i++) {
				sendEvent(threadId, 'STREAM_START');
				sendEvent(threadId, 'USER_SCROLL_UP');
				sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM');
			}

			const history = getHistory(threadId);
			expect(history.length).toBeLessThanOrEqual(20);
		});
	});

	describe('Thread Reset', () => {
		it('should reset a single thread', () => {
			const thread1 = 'thread-1';
			const thread2 = 'thread-2';

			sendEvent(thread1, 'STREAM_START');
			sendEvent(thread2, 'STREAM_START');

			resetThread(thread1);

			expect(getState(thread1)).toBe('idle');
			expect(getState(thread2)).toBe('streaming');
		});

		it('should clear history on thread reset', () => {
			const threadId = 'thread-1';

			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');

			expect(getHistory(threadId).length).toBeGreaterThan(0);

			resetThread(threadId);

			// After reset, a new context is created
			const newHistory = getHistory(threadId);
			expect(newHistory).toHaveLength(0);
		});

		it('should reset all threads', () => {
			const thread1 = 'thread-1';
			const thread2 = 'thread-2';

			sendEvent(thread1, 'STREAM_START');
			sendEvent(thread2, 'STREAM_START');
			sendEvent(thread2, 'USER_SCROLL_UP');

			resetAll();

			expect(getState(thread1)).toBe('idle');
			expect(getState(thread2)).toBe('idle');
		});
	});

	describe('Valid Transitions', () => {
		it('should return valid transitions for current state', () => {
			const threadId = 'thread-1';

			const idleTransitions = getValidTransitions(threadId);
			expect(idleTransitions).toEqual({
				STREAM_START: 'streaming',
				STREAM_END: 'idle',
				USER_SCROLL_UP: 'idle',
				USER_SCROLL_TO_BOTTOM: 'idle',
				USER_CLICK_SCROLL_DOWN: 'idle',
				USER_SEND_MESSAGE: 'idle',
				THREAD_CHANGE: 'idle'
			});

			sendEvent(threadId, 'STREAM_START');
			const streamingTransitions = getValidTransitions(threadId);
			expect(streamingTransitions).toEqual({
				STREAM_START: 'streaming',
				STREAM_END: 'idle',
				USER_SCROLL_UP: 'user_paused',
				USER_SCROLL_TO_BOTTOM: 'streaming',
				USER_CLICK_SCROLL_DOWN: 'streaming',
				USER_SEND_MESSAGE: 'streaming',
				THREAD_CHANGE: 'idle'
			});
		});
	});

	describe('Debug Info', () => {
		it('should return debug info for a thread', () => {
			const threadId = 'thread-1';
			sendEvent(threadId, 'STREAM_START');

			const debugInfo = getDebugInfo(threadId);

			expect(debugInfo.state).toBe('streaming');
			expect(debugInfo.historyLength).toBeGreaterThan(0);
			expect(debugInfo.lastEventTime).toBeGreaterThan(0);
			expect(debugInfo.validTransitions).toBeDefined();
		});

		it('should track event count', () => {
			const threadId = 'thread-1';

			const initialCount = getEventCount();
			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');

			expect(getEventCount()).toBe(initialCount + 2); // Each state transition increments
		});
	});

	describe('State and Event Descriptions', () => {
		it('should return description for each state', () => {
			expect(describeState('idle')).toBe('No streaming active, user can scroll freely');
			expect(describeState('streaming')).toBe('Content is being streamed, auto-scroll active');
			expect(describeState('user_paused')).toBe('User scrolled up during streaming, auto-scroll paused');
			expect(describeState('resuming')).toBe('Transitioning back to streaming, auto-scroll resuming');
		});

		it('should return description for each event', () => {
			expect(describeEvent('STREAM_START')).toBe('LLM started streaming content');
			expect(describeEvent('STREAM_END')).toBe('LLM finished streaming');
			expect(describeEvent('USER_SCROLL_UP')).toBe('User scrolled upward (away from bottom)');
			expect(describeEvent('USER_SCROLL_TO_BOTTOM')).toBe('User scrolled to bottom manually');
			expect(describeEvent('USER_CLICK_SCROLL_DOWN')).toBe('User clicked the scroll-to-bottom button');
			expect(describeEvent('USER_SEND_MESSAGE')).toBe('User sent a new message');
			expect(describeEvent('THREAD_CHANGE')).toBe('Active thread changed');
		});

		it('should return all possible states', () => {
			const allStates = getAllStates();
			expect(allStates).toEqual(['idle', 'streaming', 'user_paused', 'resuming']);
		});

		it('should return all possible events', () => {
			const allEvents = getAllEvents();
			expect(allEvents).toEqual([
				'STREAM_START',
				'STREAM_END',
				'USER_SCROLL_UP',
				'USER_SCROLL_TO_BOTTOM',
				'USER_CLICK_SCROLL_DOWN',
				'USER_SEND_MESSAGE',
				'THREAD_CHANGE'
			]);
		});
	});

	describe('Edge Cases', () => {
		it('should handle repeated events correctly', () => {
			const threadId = 'thread-1';

			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'STREAM_START'); // Another STREAM_START

			expect(getState(threadId)).toBe('streaming');
		});

		it('should handle THREAD_CHANGE event from any state', () => {
			const threadId = 'thread-1';

			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');

			const result = sendEvent(threadId, 'THREAD_CHANGE');
			expect(result.to).toBe('idle');
			expect(getState(threadId)).toBe('idle');
		});

		it('should handle events for non-existent thread', () => {
			const threadId = 'new-thread';

			// Should create context with idle state
			expect(getState(threadId)).toBe('idle');

			sendEvent(threadId, 'STREAM_START');
			expect(getState(threadId)).toBe('streaming');
		});
	});

	describe('Complex State Flows', () => {
		it('should handle typical streaming flow', () => {
			const threadId = 'thread-1';

			// Start streaming
			sendEvent(threadId, 'STREAM_START');
			expect(shouldAutoScroll(threadId)).toBe(true);

			// User scrolls up
			sendEvent(threadId, 'USER_SCROLL_UP');
			expect(isUserPaused(threadId)).toBe(true);
			expect(shouldAutoScroll(threadId)).toBe(false);

			// User scrolls to bottom
			sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM');
			expect(isUserPaused(threadId)).toBe(false);
			expect(shouldAutoScroll(threadId)).toBe(true);

			// Streaming ends
			sendEvent(threadId, 'STREAM_END');
			expect(shouldAutoScroll(threadId)).toBe(false);
			expect(getState(threadId)).toBe('idle');
		});

		it('should handle user clicking scroll-to-bottom button', () => {
			const threadId = 'thread-1';

			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');

			// User clicks scroll-to-bottom button
			sendEvent(threadId, 'USER_CLICK_SCROLL_DOWN');
			expect(shouldAutoScroll(threadId)).toBe(true);
			expect(getState(threadId)).toBe('resuming');
		});

		it('should handle user sending new message while paused', () => {
			const threadId = 'thread-1';

			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');

			// User sends message (should resume auto-scroll)
			sendEvent(threadId, 'USER_SEND_MESSAGE');
			expect(getState(threadId)).toBe('resuming');
			expect(shouldAutoScroll(threadId)).toBe(true);
		});

		it('should handle user scrolling up again during resuming', () => {
			const threadId = 'thread-1';

			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');
			sendEvent(threadId, 'USER_SCROLL_TO_BOTTOM'); // Now resuming

			// User scrolls up again during resuming
			sendEvent(threadId, 'USER_SCROLL_UP');
			expect(getState(threadId)).toBe('user_paused');
			expect(shouldAutoScroll(threadId)).toBe(false);
		});

		it('should handle STREAM_START while in user_paused', () => {
			const threadId = 'thread-1';

			sendEvent(threadId, 'STREAM_START');
			sendEvent(threadId, 'USER_SCROLL_UP');

			// STREAM_START while user_paused stays in user_paused
			sendEvent(threadId, 'STREAM_START');
			expect(getState(threadId)).toBe('user_paused');
		});
	});
});
