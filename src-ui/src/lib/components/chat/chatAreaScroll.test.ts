import {describe, expect, it} from 'vitest';
import {shouldForceInitialBottomScroll} from './chatAreaScroll';

describe('chatAreaScroll', () => {
	it('returns true when selected thread is pending and messages loaded successfully', () => {
		expect(
			shouldForceInitialBottomScroll({
				currentThreadId: 'thread-1',
				pendingThreadId: 'thread-1',
				messagesLoading: 'success'
			})
		).toBe(true);
	});

	it('returns false while loading has not completed', () => {
		expect(
			shouldForceInitialBottomScroll({
				currentThreadId: 'thread-1',
				pendingThreadId: 'thread-1',
				messagesLoading: 'loading'
			})
		).toBe(false);
	});

	it('returns false when pending thread does not match current thread', () => {
		expect(
			shouldForceInitialBottomScroll({
				currentThreadId: 'thread-2',
				pendingThreadId: 'thread-1',
				messagesLoading: 'success'
			})
		).toBe(false);
	});

	it('returns false when no current thread is selected', () => {
		expect(
			shouldForceInitialBottomScroll({
				currentThreadId: undefined,
				pendingThreadId: 'thread-1',
				messagesLoading: 'success'
			})
		).toBe(false);
	});
});
