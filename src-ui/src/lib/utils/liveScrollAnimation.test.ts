/**
 * Tests for liveScrollAnimation
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	getAnimationId,
	getAnimationPriority,
	getDebugInfo,
	scrollToBottomInstant,
	scrollToBottomLive,
	startLiveAnimation,
	stop,
	stopAll,
	stopById,
	updateLiveTarget
} from './liveScrollAnimation';

describe('liveScrollAnimation', () => {
	let testElement: HTMLElement;

	beforeEach(() => {
		// Reset state
		stopAll();

		// Create test element
		testElement = document.createElement('div');
		testElement.style.height = '200px';
		testElement.style.overflow = 'auto';
		testElement.innerHTML = '<div style="height: 1000px;">Content</div>';
		document.body.appendChild(testElement);
	});

	afterEach(() => {
		stopAll();

		if (testElement && testElement.parentNode) {
			document.body.removeChild(testElement);
		}
	});

	describe('startLiveAnimation', () => {
		it('should return animation result with ID', () => {
			testElement.scrollTop = 0;

			const result = startLiveAnimation(testElement, {
				threadId: 'test-1',
				priority: 'background'
			});

			expect(result.id).toMatch(/^live-anim-\d+$/);
			expect(typeof result.started).toBe('boolean');
		});

		it('should not start animation if already at bottom', () => {
			const onComplete = vi.fn();

			// Scroll to bottom
			testElement.scrollTop = testElement.scrollHeight - testElement.clientHeight;

			const result = startLiveAnimation(testElement, {
				threadId: 'test-2',
				priority: 'background',
				onComplete
			});

			expect(result.started).toBe(false);
			expect(onComplete).toHaveBeenCalled();
		});

		it('should respect animated option for instant scroll', () => {
			testElement.scrollTop = 0;
			const onComplete = vi.fn();

			const result = startLiveAnimation(testElement, {
				threadId: 'test-3',
				priority: 'background',
				animated: false,
				onComplete
			});

			expect(result.started).toBe(false);
			expect(testElement.scrollTop).toBe(testElement.scrollHeight - testElement.clientHeight);
			expect(onComplete).toHaveBeenCalled();
		});

		it('should start animation when not at bottom', () => {
			testElement.scrollTop = 0;

			const result = startLiveAnimation(testElement, {
				threadId: 'test-4',
				priority: 'background'
			});

			expect(result.started).toBe(true);
		});

		it('should store animation in debug info', () => {
			testElement.scrollTop = 0;

			const result = startLiveAnimation(testElement, {
				threadId: 'test-5',
				priority: 'background'
			});

			const debugInfo = getDebugInfo();
			const anim = debugInfo.animations.find((a) => a.id === result.id);

			expect(anim).toBeDefined();
			expect(anim?.threadId).toBe('test-5');
			expect(anim?.priority).toBe('background');
		});

		it('should replace existing animation for same thread', () => {
			testElement.scrollTop = 0;

			const result1 = startLiveAnimation(testElement, {
				threadId: 'test-6',
				priority: 'background'
			});

			const result2 = startLiveAnimation(testElement, {
				threadId: 'test-6',
				priority: 'user'
			});

			expect(result1.id).not.toBe(result2.id);
			expect(getAnimationId('test-6')).toBe(result2.id);
		});
	});

	describe('Animation Thresholds', () => {
		it('should not start animation when within threshold of bottom', () => {
			// At threshold (5)
			testElement.scrollTop = testElement.scrollHeight - testElement.clientHeight - 5;
			const onComplete = vi.fn();

			const result = startLiveAnimation(testElement, {
				threadId: 'test-7',
				priority: 'background',
				onComplete
			});

			expect(result.started).toBe(false);
			expect(onComplete).toHaveBeenCalled();
		});

		it('should start animation when just outside threshold', () => {
			// Just above threshold
			testElement.scrollTop = testElement.scrollHeight - testElement.clientHeight - 6;

			const result = startLiveAnimation(testElement, {
				threadId: 'test-8',
				priority: 'background'
			});

			expect(result.started).toBe(true);
		});

		it('should handle zero maxScroll gracefully', () => {
			// Create element with no scrollable content
			const smallElement = document.createElement('div');
			smallElement.style.height = '200px';
			smallElement.innerHTML = '<div style="height: 50px;">Small</div>';
			document.body.appendChild(smallElement);

			try {
				const result = startLiveAnimation(smallElement, {
					threadId: 'test-9',
					priority: 'background'
				});

				// Should not animate since there's nowhere to scroll
				expect(result.started).toBe(false);
			} finally {
				document.body.removeChild(smallElement);
			}
		});
	});

	describe('Query Functions', () => {
		it('should get animation ID for thread', () => {
			testElement.scrollTop = 0;

			expect(getAnimationId('test-10')).toBeUndefined();

			const result = startLiveAnimation(testElement, {
				threadId: 'test-10',
				priority: 'background'
			});

			expect(getAnimationId('test-10')).toBe(result.id);
		});

		it('should get animation priority for thread', () => {
			testElement.scrollTop = 0;

			expect(getAnimationPriority('test-11')).toBeUndefined();

			startLiveAnimation(testElement, {
				threadId: 'test-11',
				priority: 'user'
			});

			expect(getAnimationPriority('test-11')).toBe('user');
		});

		it('should return undefined for non-existent thread', () => {
			expect(getAnimationId('non-existent')).toBeUndefined();
			expect(getAnimationPriority('non-existent')).toBeUndefined();
		});
	});

	describe('Debug Info', () => {
		it('should return debug info with active animations count', () => {
			testElement.scrollTop = 0;

			const beforeCount = getDebugInfo().activeAnimations;

			startLiveAnimation(testElement, {
				threadId: 'test-12',
				priority: 'background'
			});

			expect(getDebugInfo().activeAnimations).toBe(beforeCount + 1);
		});

		it('should include all animations in debug info', () => {
			testElement.scrollTop = 0;

			startLiveAnimation(testElement, {
				threadId: 'test-13',
				priority: 'background'
			});

			startLiveAnimation(testElement, {
				threadId: 'test-14',
				priority: 'user'
			});

			const debugInfo = getDebugInfo();

			expect(debugInfo.activeAnimations).toBeGreaterThanOrEqual(2);
		});

		it('should include correct animation properties in debug info', () => {
			testElement.scrollTop = 0;

			startLiveAnimation(testElement, {
				threadId: 'test-15',
				priority: 'programmatic'
			});

			const debugInfo = getDebugInfo();
			const anim = debugInfo.animations.find((a) => a.threadId === 'test-15');

			expect(anim).toMatchObject({
				id: expect.any(String),
				threadId: 'test-15',
				priority: 'programmatic',
				isRunning: true
			});
		});
	});

	describe('Stopping Animations', () => {
		it('should return true when stopping existing animation by ID', () => {
			testElement.scrollTop = 0;

			const result = startLiveAnimation(testElement, {
				threadId: 'test-16',
				priority: 'background'
			});

			const stopped = stopById(result.id);
			expect(stopped).toBe(true);
		});

		it('should return false when stopping non-existent animation by ID', () => {
			const stopped = stopById('non-existent-id');
			expect(stopped).toBe(false);
		});

		it('should return true when stopping animation for thread', () => {
			testElement.scrollTop = 0;

			startLiveAnimation(testElement, {
				threadId: 'test-17',
				priority: 'background'
			});

			const stopped = stop('test-17');
			expect(stopped).toBe(true);
		});

		it('should return false when stopping animation for non-existent thread', () => {
			const stopped = stop('non-existent-thread');
			expect(stopped).toBe(false);
		});

		it('should set shouldStop flag on all animations when stopAll is called', () => {
			testElement.scrollTop = 0;

			startLiveAnimation(testElement, {
				threadId: 'test-18',
				priority: 'background'
			});

			startLiveAnimation(testElement, {
				threadId: 'test-19',
				priority: 'user'
			});

			stopAll();

			// After stopAll, animations should still be in the system until RAF processes them
			const debugInfo = getDebugInfo();
			expect(debugInfo.activeAnimations).toBeGreaterThan(0);
		});
	});

	describe('Live Target Updates', () => {
		it('should handle updateLiveTarget call', () => {
			testElement.scrollTop = 0;

			startLiveAnimation(testElement, {
				threadId: 'test-20',
				priority: 'background'
			});

			// Simulate content growth
			testElement.innerHTML = '<div style="height: 2000px;">More content</div>';

			// Should not throw
			expect(() => updateLiveTarget(testElement)).not.toThrow();
		});

		it('should handle updateLiveTarget with no animations', () => {
			expect(() => updateLiveTarget(testElement)).not.toThrow();
		});
	});

	describe('Convenience Functions', () => {
		it('should scroll to bottom instantly', () => {
			testElement.scrollTop = 0;

			scrollToBottomInstant(testElement);

			expect(testElement.scrollTop).toBe(testElement.scrollHeight);
		});

		it('should return animation ID from scrollToBottomLive', () => {
			testElement.scrollTop = 0;

			const id = scrollToBottomLive(testElement, 'test-21', 'background');

			expect(id).toMatch(/^live-anim-\d+$/);
		});

		it('should create animation with correct settings via scrollToBottomLive', () => {
			testElement.scrollTop = 0;

			scrollToBottomLive(testElement, 'test-22', 'user');

			expect(getAnimationPriority('test-22')).toBe('user');
		});
	});

	describe('Multiple Threads', () => {
		it('should handle animations for multiple threads independently', () => {
			testElement.scrollTop = 0;

			const result1 = startLiveAnimation(testElement, {
				threadId: 'test-23',
				priority: 'background'
			});

			const result2 = startLiveAnimation(testElement, {
				threadId: 'test-24',
				priority: 'user'
			});

			expect(result1.id).not.toBe(result2.id);
			expect(getAnimationId('test-23')).toBe(result1.id);
			expect(getAnimationId('test-24')).toBe(result2.id);
		});

		it('should only stop animation for specified thread', () => {
			testElement.scrollTop = 0;

			startLiveAnimation(testElement, {
				threadId: 'test-25',
				priority: 'background'
			});

			startLiveAnimation(testElement, {
				threadId: 'test-26',
				priority: 'user'
			});

			stop('test-25');

			// test-26 should still have an animation
			expect(getAnimationId('test-26')).toBeTruthy();
		});
	});

	describe('Animation Options', () => {
		it('should use custom duration', () => {
			testElement.scrollTop = 0;

			const result = startLiveAnimation(testElement, {
				threadId: 'test-27',
				priority: 'background',
				duration: 500
			});

			expect(result.started).toBe(true);
		});

		it('should handle zero duration', () => {
			testElement.scrollTop = 0;

			const result = startLiveAnimation(testElement, {
				threadId: 'test-28',
				priority: 'background',
				duration: 0
			});

			expect(result.started).toBe(true);
		});

		it('should accept callbacks without error', () => {
			testElement.scrollTop = 0;
			const onComplete = vi.fn();
			const onStop = vi.fn();

			const result = startLiveAnimation(testElement, {
				threadId: 'test-29',
				priority: 'background',
				onComplete,
				onStop
			});

			expect(result.started).toBe(true);
		});
	});

	describe('Animation ID Generation', () => {
		it('should generate incrementing unique IDs', () => {
			testElement.scrollTop = 0;

			const id1 = startLiveAnimation(testElement, {
				threadId: 'test-30',
				priority: 'background'
			}).id;

			const id2 = startLiveAnimation(testElement, {
				threadId: 'test-31',
				priority: 'background'
			}).id;

			const id3 = startLiveAnimation(testElement, {
				threadId: 'test-32',
				priority: 'background'
			}).id;

			expect(id1).not.toBe(id2);
			expect(id2).not.toBe(id3);

			const num1 = parseInt(id1.split('-')[2]);
			const num2 = parseInt(id2.split('-')[2]);
			const num3 = parseInt(id3.split('-')[2]);

			expect(num2).toBe(num1 + 1);
			expect(num3).toBe(num2 + 1);
		});
	});

	describe('Priority Levels', () => {
		it('should accept all priority levels', () => {
			testElement.scrollTop = 0;

			const userResult = startLiveAnimation(testElement, {
				threadId: 'test-33',
				priority: 'user'
			});
			expect(userResult.started).toBe(true);

			const progResult = startLiveAnimation(testElement, {
				threadId: 'test-34',
				priority: 'programmatic'
			});
			expect(progResult.started).toBe(true);

			const bgResult = startLiveAnimation(testElement, {
				threadId: 'test-35',
				priority: 'background'
			});
			expect(bgResult.started).toBe(true);
		});
	});
});
