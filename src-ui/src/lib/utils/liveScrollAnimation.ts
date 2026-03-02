/**
 * Live Scroll Animation System
 *
 * A "chasing" animation system that continuously recalculates the target
 * scroll position as content grows during streaming. This solves the problem
 * of animations targeting a fixed position that becomes stale as new content
 * arrives.
 *
 * Key Features:
 * - Animation "chases" the live bottom position as content grows
 * - User-priority animations always reach actual bottom
 * - Supports preemption via external control
 *
 * @module liveScrollAnimation
 */

import type { ScrollPriority } from './scrollPriorityQueue';
import { scheduleRead, scheduleWrite, type ScrollState } from './domBatcher';

// ============================================================================
// Types
// ============================================================================

/** Animation state */
interface AnimationState {
	/** Unique identifier for this animation */
	id: string;
	/** Thread ID this animation belongs to */
	threadId: string;
	/** Priority level */
	priority: ScrollPriority;
	/** Whether animation is actively running */
	isRunning: boolean;
	/** Whether animation should stop */
	shouldStop: boolean;
	/** Animation start time */
	startTime: number;
	/** Animation duration in ms */
	duration: number;
	/** Starting scroll position */
	startScrollTop: number;
	/** Target scroll position (updated live for background priority) */
	targetScrollTop: number;
	/** Whether to use animation or instant scroll */
	animated: boolean;
	/** Callback when animation completes naturally */
	onComplete?: () => void;
	/** Callback when animation is stopped/preempted */
	onStop?: () => void;
	/** RAF handle for cancellation */
	rafHandle?: number;
}

/** Options for starting a live animation */
export interface LiveAnimationOptions {
	/** Thread ID for the animation */
	threadId: string;
	/** Priority level (affects how animation targets are updated) */
	priority: ScrollPriority;
	/** Whether to animate or scroll instantly */
	animated?: boolean;
	/** Animation duration in ms (default: 300) */
	duration?: number;
	/** Callback when animation completes */
	onComplete?: () => void;
	/** Callback when animation is stopped */
	onStop?: () => void;
}

/** Result of starting an animation */
export interface AnimationStartResult {
	/** Animation ID */
	id: string;
	/** Whether animation started */
	started: boolean;
}

// ============================================================================
// Constants
// ============================================================================

/** Default animation duration in ms */
const DEFAULT_DURATION = 300;

/** Threshold for considering scroll "at bottom" */
const AT_BOTTOM_THRESHOLD = 5;

/** Easing function - ease-out-quad for smooth deceleration */
function easeOutQuad(t: number): number {
	return t * (2 - t);
}

// ============================================================================
// State
// ============================================================================

/** Map of animation ID to animation state */
const animations = new Map<string, AnimationState>();

/** Map of thread ID to active animation ID */
const threadAnimations = new Map<string, string>();

/** Counter for generating unique IDs */
let animationCounter = 0;

// ============================================================================
// Private Functions
// ============================================================================

/**
 * Generate a unique animation ID.
 */
function generateId(): string {
	return `live-anim-${++animationCounter}`;
}

/**
 * Get the current scroll state synchronously.
 * Used during animation loop for live target tracking.
 */
function getScrollStateSync(element: HTMLElement): ScrollState {
	const { scrollTop, scrollHeight, clientHeight } = element;
	const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
	const maxScroll = scrollHeight - clientHeight;
	return {
		scrollTop,
		scrollHeight,
		clientHeight,
		distanceFromBottom,
		isNearBottom: distanceFromBottom < AT_BOTTOM_THRESHOLD,
		maxScroll,
		progress: maxScroll > 0 ? scrollTop / maxScroll : 1
	};
}

/**
 * Calculate the target scroll position (bottom of content).
 */
function calculateTargetBottom(element: HTMLElement): number {
	return element.scrollHeight - element.clientHeight;
}

/**
 * Animation frame callback.
 */
function animationFrame(element: HTMLElement, state: AnimationState): void {
	// Check if we should stop
	if (state.shouldStop) {
		finalizeAnimation(state, false);
		return;
	}

	// Check if element is still in DOM
	if (!element.isConnected) {
		finalizeAnimation(state, false);
		return;
	}

	const now = performance.now();
	const elapsed = now - state.startTime;
	const progress = Math.min(elapsed / state.duration, 1);

	// For user/programmatic priority, use static target captured at start
	// For background priority, chase the live target
	if (state.priority === 'background') {
		// Update target to current bottom (chasing)
		state.targetScrollTop = calculateTargetBottom(element);
	}

	// Calculate eased progress
	const easedProgress = easeOutQuad(progress);

	// Calculate new scroll position
	const newScrollTop =
		state.startScrollTop + (state.targetScrollTop - state.startScrollTop) * easedProgress;

	// Apply scroll
	element.scrollTop = newScrollTop;

	if (progress < 1) {
		// Continue animation
		state.rafHandle = requestAnimationFrame(() => animationFrame(element, state));
	} else {
		// Animation complete
		finalizeAnimation(state, true);
	}
}

/**
 * Finalize an animation (complete or stop).
 */
function finalizeAnimation(state: AnimationState, completed: boolean): void {
	state.isRunning = false;

	// Clear RAF handle
	if (state.rafHandle !== undefined) {
		cancelAnimationFrame(state.rafHandle);
		state.rafHandle = undefined;
	}

	// Clean up maps
	animations.delete(state.id);
	threadAnimations.delete(state.threadId);

	// Call appropriate callback
	if (completed) {
		state.onComplete?.();
	} else {
		state.onStop?.();
	}
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Start a live scroll animation.
 * The animation will scroll to the bottom of the element.
 * For 'background' priority, the target continuously updates as content grows.
 *
 * @param element - Element to scroll
 * @param options - Animation options
 * @returns Animation start result with ID
 */
export function startLiveAnimation(
	element: HTMLElement,
	options: LiveAnimationOptions
): AnimationStartResult {
	const id = generateId();
	const animated = options.animated ?? true;
	const duration = options.duration ?? DEFAULT_DURATION;

	// Stop any existing animation for this thread
	stop(options.threadId);

	// Get current scroll state
	const currentScrollTop = element.scrollTop;
	const targetScrollTop = calculateTargetBottom(element);

	// Check if already at bottom
	const distanceToBottom = targetScrollTop - currentScrollTop;
	if (distanceToBottom <= AT_BOTTOM_THRESHOLD) {
		// Already at bottom, no animation needed
		options.onComplete?.();
		return { id, started: false };
	}

	// If not animated, scroll instantly
	if (!animated) {
		element.scrollTop = targetScrollTop;
		options.onComplete?.();
		return { id, started: false };
	}

	// Create animation state
	const state: AnimationState = {
		id,
		threadId: options.threadId,
		priority: options.priority,
		isRunning: true,
		shouldStop: false,
		startTime: performance.now(),
		duration,
		startScrollTop: currentScrollTop,
		targetScrollTop,
		animated,
		onComplete: options.onComplete,
		onStop: options.onStop
	};

	// Store in maps
	animations.set(id, state);
	threadAnimations.set(options.threadId, id);

	// Start animation loop
	state.rafHandle = requestAnimationFrame(() => animationFrame(element, state));

	return { id, started: true };
}

/**
 * Update the live target for a background animation.
 * Call this when new content is added during streaming.
 *
 * @param element - Element being animated
 */
export function updateLiveTarget(element: HTMLElement): void {
	// Find the animation for this element's thread
	for (const [threadId, animId] of threadAnimations) {
		const state = animations.get(animId);
		if (state && state.priority === 'background' && state.isRunning) {
			// Update target - the animation loop will pick this up
			state.targetScrollTop = calculateTargetBottom(element);
		}
	}
}

/**
 * Stop an animation by ID.
 *
 * @param id - Animation ID to stop
 * @returns True if animation was stopped
 */
export function stopById(id: string): boolean {
	const state = animations.get(id);
	if (!state || !state.isRunning) return false;

	state.shouldStop = true;
	return true;
}

/**
 * Stop the animation for a specific thread.
 *
 * @param threadId - Thread ID to stop animation for
 * @returns True if an animation was stopped
 */
export function stop(threadId: string): boolean {
	const animId = threadAnimations.get(threadId);
	if (!animId) return false;

	return stopById(animId);
}

/**
 * Stop all animations.
 */
export function stopAll(): void {
	for (const state of animations.values()) {
		if (state.isRunning) {
			state.shouldStop = true;
		}
	}
}

/**
 * Check if an animation is running for a thread.
 *
 * @param threadId - Thread ID to check
 * @returns True if animation is running
 */
export function isRunning(threadId: string): boolean {
	const animId = threadAnimations.get(threadId);
	if (!animId) return false;
	const state = animations.get(animId);
	return state?.isRunning ?? false;
}

/**
 * Get the animation ID for a thread.
 *
 * @param threadId - Thread ID to get animation for
 * @returns Animation ID or undefined
 */
export function getAnimationId(threadId: string): string | undefined {
	return threadAnimations.get(threadId);
}

/**
 * Get the priority of the running animation for a thread.
 *
 * @param threadId - Thread ID to check
 * @returns Priority or undefined if no animation running
 */
export function getAnimationPriority(threadId: string): ScrollPriority | undefined {
	const animId = threadAnimations.get(threadId);
	if (!animId) return undefined;
	return animations.get(animId)?.priority;
}

/**
 * Scroll to bottom instantly (no animation).
 * Useful for initial positioning or when user preference indicates no motion.
 *
 * @param element - Element to scroll
 */
export function scrollToBottomInstant(element: HTMLElement): void {
	element.scrollTop = element.scrollHeight;
}

/**
 * Scroll to bottom with live animation (convenience method).
 * This is the recommended way to scroll during streaming.
 *
 * @param element - Element to scroll
 * @param threadId - Thread ID for tracking
 * @param priority - Priority level
 * @param onComplete - Optional completion callback
 * @returns Animation ID
 */
export function scrollToBottomLive(
	element: HTMLElement,
	threadId: string,
	priority: ScrollPriority,
	onComplete?: () => void
): string {
	const result = startLiveAnimation(element, {
		threadId,
		priority,
		animated: true,
		onComplete
	});
	return result.id;
}

/**
 * Get debug info for animations.
 */
export function getDebugInfo(): {
	activeAnimations: number;
	animations: Array<{
		id: string;
		threadId: string;
		priority: ScrollPriority;
		isRunning: boolean;
	}>;
} {
	return {
		activeAnimations: animations.size,
		animations: Array.from(animations.values()).map((s) => ({
			id: s.id,
			threadId: s.threadId,
			priority: s.priority,
			isRunning: s.isRunning
		}))
	};
}
