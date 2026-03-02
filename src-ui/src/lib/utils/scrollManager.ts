/**
 * Scroll Manager
 * Manages scroll positions for chat threads with session-only memory.
 */

/** Threshold in pixels to determine if user is "near bottom" */
const BOTTOM_THRESHOLD = 100;

/** Delay in ms before auto-scroll resumes after user stops scrolling */
const RESUME_DELAY = 500;

/** Session-only storage for scroll positions per thread */
const scrollPositions = new Map<string, number>();

/** Track if auto-scroll is enabled per thread */
const autoScrollEnabled = new Map<string, boolean>();

/** Track active resume timeouts per thread */
const resumeTimeouts = new Map<string, ReturnType<typeof setTimeout>>();

/** Track if user is actively scrolling */
let userIsScrolling = false;

/**
 * Check if element is scrolled to near bottom.
 */
export function isNearBottom(element: HTMLElement): boolean {
	const { scrollTop, scrollHeight, clientHeight } = element;
	const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
	return distanceFromBottom < BOTTOM_THRESHOLD;
}

/**
 * Get the current scroll position as a percentage (0-1).
 */
export function getScrollProgress(element: HTMLElement): number {
	const { scrollTop, scrollHeight, clientHeight } = element;
	const maxScroll = scrollHeight - clientHeight;
	if (maxScroll <= 0) return 1;
	return scrollTop / maxScroll;
}

/**
 * Save scroll position for a thread.
 */
export function saveScrollPosition(threadId: string, element: HTMLElement): void {
	scrollPositions.set(threadId, element.scrollTop);
}

/**
 * Restore scroll position for a thread.
 * Returns true if position was restored, false if no saved position exists.
 */
export function restoreScrollPosition(threadId: string, element: HTMLElement): boolean {
	const savedPosition = scrollPositions.get(threadId);
	if (savedPosition !== undefined) {
		element.scrollTop = savedPosition;
		return true;
	}
	return false;
}

/**
 * Check if a thread has a saved scroll position.
 */
export function hasSavedPosition(threadId: string): boolean {
	return scrollPositions.has(threadId);
}

/**
 * Clear saved scroll position for a thread.
 */
export function clearScrollPosition(threadId: string): void {
	scrollPositions.delete(threadId);
}

/**
 * Clear all saved scroll positions.
 */
export function clearAllScrollPositions(): void {
	scrollPositions.clear();
}

/**
 * Check if auto-scroll is enabled for a thread.
 */
export function isAutoScrollEnabled(threadId: string): boolean {
	return autoScrollEnabled.get(threadId) !== false;
}

/**
 * Enable auto-scroll for a thread.
 */
export function enableAutoScroll(threadId: string): void {
	autoScrollEnabled.set(threadId, true);
}

/**
 * Disable auto-scroll for a thread.
 */
export function disableAutoScroll(threadId: string): void {
	autoScrollEnabled.set(threadId, false);
	// Clear any pending resume timeout
	const existingTimeout = resumeTimeouts.get(threadId);
	if (existingTimeout) {
		clearTimeout(existingTimeout);
		resumeTimeouts.delete(threadId);
	}
}

/**
 * Schedule auto-scroll resume after user stops scrolling.
 */
export function scheduleAutoScrollResume(
	threadId: string,
	callback: () => void
): void {
	// Clear any existing timeout
	const existingTimeout = resumeTimeouts.get(threadId);
	if (existingTimeout) {
		clearTimeout(existingTimeout);
	}

	// Schedule new resume check
	const timeout = setTimeout(() => {
		resumeTimeouts.delete(threadId);
		callback();
	}, RESUME_DELAY);

	resumeTimeouts.set(threadId, timeout);
}

/**
 * Cancel any pending auto-scroll resume.
 */
export function cancelAutoScrollResume(threadId: string): void {
	const existingTimeout = resumeTimeouts.get(threadId);
	if (existingTimeout) {
		clearTimeout(existingTimeout);
		resumeTimeouts.delete(threadId);
	}
}

/**
 * Set user scrolling state.
 */
export function setUserScrolling(value: boolean): void {
	userIsScrolling = value;
}

/**
 * Check if user is currently scrolling.
 */
export function isUserScrolling(): boolean {
	return userIsScrolling;
}

/** Duration for smooth scroll animation in ms */
const SCROLL_DURATION = 300;

/** Easing function for smooth scroll animation */
function easeOutQuad(t: number): number {
	return t * (2 - t);
}

/**
 * Scroll to bottom smoothly using custom animation for precise timing control.
 */
export function scrollToBottom(element: HTMLElement, smooth = true): void {
	if (!smooth) {
		// Instant scroll
		element.scrollTop = element.scrollHeight;
		return;
	}

	// Custom smooth scroll animation
	const startScrollTop = element.scrollTop;
	const targetScrollTop = element.scrollHeight - element.clientHeight;
	const distance = targetScrollTop - startScrollTop;

	if (distance <= 0) {
		// Already at bottom
		return;
	}

	const startTime = performance.now();
	const duration = SCROLL_DURATION;

	function animateScroll(currentTime: number) {
		const elapsed = currentTime - startTime;
		const progress = Math.min(elapsed / duration, 1);
		const easedProgress = easeOutQuad(progress);
		const newScrollTop = startScrollTop + distance * easedProgress;

		element.scrollTop = newScrollTop;

		if (progress < 1) {
			requestAnimationFrame(animateScroll);
		}
	}

	requestAnimationFrame(animateScroll);
}

/**
 * Handle scroll event - determines if user is scrolling up or near bottom.
 * Returns an object with the current scroll state.
 */
export function handleScrollEvent(
	element: HTMLElement,
	threadId: string,
	isStreaming: boolean
): {
	isNearBottom: boolean;
	shouldAutoScroll: boolean;
	userScrolledUp: boolean;
} {
	const nearBottom = isNearBottom(element);
	const wasAutoScrollEnabled = isAutoScrollEnabled(threadId);

	// If user scrolled and is no longer near bottom, they scrolled up
	const userScrolledUp = !nearBottom && wasAutoScrollEnabled;

	if (userScrolledUp) {
		// User scrolled up - disable auto-scroll
		disableAutoScroll(threadId);
		setUserScrolling(true);
	} else if (nearBottom) {
		// User is at bottom - enable auto-scroll
		enableAutoScroll(threadId);
		setUserScrolling(false);
	}

	return {
		isNearBottom: nearBottom,
		shouldAutoScroll: isAutoScrollEnabled(threadId),
		userScrolledUp
	};
}

/**
 * Initialize scroll state for a new thread.
 * By default, auto-scroll is enabled for new threads.
 */
export function initializeThreadScroll(threadId: string): void {
	if (!autoScrollEnabled.has(threadId)) {
		autoScrollEnabled.set(threadId, true);
	}
}

/**
 * Cleanup scroll state for a thread.
 */
export function cleanupThreadScroll(threadId: string): void {
	cancelAutoScrollResume(threadId);
}
