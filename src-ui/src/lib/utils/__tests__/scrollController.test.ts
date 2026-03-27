import {afterEach, beforeEach, describe, expect, it} from 'vitest';
import {
    clearAll,
    getThreadScrollState,
    handleContentUpdate,
    handleStreamingStart,
    handleUserScroll,
    isAutoScrollEnabled,
    registerThreadElement
} from '../scrollController';

interface ScrollMetrics {
	scrollTop: number;
	scrollHeight: number;
	clientHeight: number;
}

function setScrollMetrics(element: HTMLDivElement, metrics: ScrollMetrics): void {
	Object.defineProperty(element, 'scrollTop', {
		value: metrics.scrollTop,
		writable: true,
		configurable: true
	});
	Object.defineProperty(element, 'scrollHeight', {
		value: metrics.scrollHeight,
		writable: true,
		configurable: true
	});
	Object.defineProperty(element, 'clientHeight', {
		value: metrics.clientHeight,
		writable: true,
		configurable: true
	});
}

function createScrollableElement(metrics: ScrollMetrics): HTMLDivElement {
	const element = document.createElement('div');
	setScrollMetrics(element, metrics);
	document.body.appendChild(element);
	return element;
}

describe('scrollController', () => {
	beforeEach(() => {
		clearAll();
		document.body.innerHTML = '';
	});

	afterEach(() => {
		clearAll();
		document.body.innerHTML = '';
	});

	it('auto-scrolls to bottom during streaming updates when auto-scroll is enabled', () => {
		const threadId = 'thread-auto-follow';
		const element = createScrollableElement({
			scrollTop: 100,
			scrollHeight: 300,
			clientHeight: 200
		});

		registerThreadElement(threadId, element);
		handleStreamingStart(threadId);

		setScrollMetrics(element, {
			scrollTop: 100,
			scrollHeight: 340,
			clientHeight: 200
		});
		handleContentUpdate(threadId);

		expect(isAutoScrollEnabled(threadId)).toBe(true);
		expect(element.scrollTop).toBe(340);
	});

	it('pauses auto-scroll when user scrolls up during streaming', () => {
		const threadId = 'thread-user-pause';
		const element = createScrollableElement({
			scrollTop: 300,
			scrollHeight: 500,
			clientHeight: 200
		});

		registerThreadElement(threadId, element);
		handleStreamingStart(threadId);

		setScrollMetrics(element, {
			scrollTop: 180,
			scrollHeight: 500,
			clientHeight: 200
		});
		handleUserScroll(threadId);

		expect(isAutoScrollEnabled(threadId)).toBe(false);
		expect(getThreadScrollState(threadId)).toBe('user_paused');

		setScrollMetrics(element, {
			scrollTop: 180,
			scrollHeight: 560,
			clientHeight: 200
		});
		handleContentUpdate(threadId);

		expect(element.scrollTop).toBe(180);
	});

	it('resumes auto-scroll when user returns to bottom during streaming', () => {
		const threadId = 'thread-resume';
		const element = createScrollableElement({
			scrollTop: 300,
			scrollHeight: 500,
			clientHeight: 200
		});

		registerThreadElement(threadId, element);
		handleStreamingStart(threadId);

		setScrollMetrics(element, {
			scrollTop: 180,
			scrollHeight: 500,
			clientHeight: 200
		});
		handleUserScroll(threadId);
		expect(getThreadScrollState(threadId)).toBe('user_paused');

		setScrollMetrics(element, {
			scrollTop: 360,
			scrollHeight: 560,
			clientHeight: 200
		});
		handleUserScroll(threadId);

		expect(isAutoScrollEnabled(threadId)).toBe(true);
		expect(getThreadScrollState(threadId)).toBe('resuming');

		setScrollMetrics(element, {
			scrollTop: 360,
			scrollHeight: 600,
			clientHeight: 200
		});
		handleContentUpdate(threadId);

		expect(element.scrollTop).toBe(600);
	});
});
