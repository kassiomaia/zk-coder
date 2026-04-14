import { afterEach, vi, expect, beforeEach } from 'vitest';
import { cleanup } from '@testing-library/svelte';
import * as matchers from '@testing-library/jest-dom/matchers';

// Extend Vitest's expect with jest-dom matchers
expect.extend(matchers);

// The project previously mocked `$lib/icons` with a plain `vi.fn` factory, which
// is not a valid Svelte 5 component constructor. Every component that rendered
// an icon inside a button ended up with an empty slot where the icon should be,
// hiding the surrounding button from accessibility queries and breaking ~350
// component tests. The real icons are lightweight SVG components with no
// external dependencies, so we let them render in the jsdom environment.

// Cleanup after each test
afterEach(() => cleanup());

// Define mocks that need to be fresh for each test
const createMatchMediaMock = () =>
	vi.fn().mockImplementation((query) => ({
		matches: false,
		media: query,
		onchange: null,
		addListener: vi.fn(),
		removeListener: vi.fn(),
		addEventListener: vi.fn(),
		removeEventListener: vi.fn(),
		dispatchEvent: vi.fn()
	}));

// Setup before each test
beforeEach(() => {
	// Mock window.matchMedia for responsive tests
	const mockMatchMedia = createMatchMediaMock();
	Object.defineProperty(window, 'matchMedia', {
		writable: true,
		configurable: true,
		value: mockMatchMedia
	});
	global.matchMedia = mockMatchMedia;
});

// Mock ResizeObserver
global.ResizeObserver = vi.fn().mockImplementation(() => ({
	observe: vi.fn(),
	unobserve: vi.fn(),
	disconnect: vi.fn()
}));

// Mock IntersectionObserver
global.IntersectionObserver = vi.fn().mockImplementation(() => ({
	observe: vi.fn(),
	unobserve: vi.fn(),
	disconnect: vi.fn()
}));

// Mock requestAnimationFrame
global.requestAnimationFrame = vi.fn((cb) => setTimeout(cb, 16));
global.cancelAnimationFrame = vi.fn(clearTimeout);

// Polyfill scrollIntoView for jsdom
Element.prototype.scrollIntoView = vi.fn();

// Polyfill scrolling methods for jsdom
Element.prototype.scrollTo = vi.fn();
window.scrollTo = vi.fn();

// Polyfill Web Animations API for jsdom. Svelte 5 uses `element.animate()`
// for its built-in transitions and some component libraries call it during
// focus/mount. jsdom does not implement WAAPI, so without this shim every
// component test that triggers an animation throws
// `TypeError: element.animate is not a function`.
Element.prototype.animate = vi.fn().mockImplementation(() => ({
	play: vi.fn(),
	pause: vi.fn(),
	cancel: vi.fn(),
	finish: vi.fn(),
	reverse: vi.fn(),
	updatePlaybackRate: vi.fn(),
	addEventListener: vi.fn(),
	removeEventListener: vi.fn(),
	dispatchEvent: vi.fn(),
	finished: Promise.resolve(),
	ready: Promise.resolve(),
	onfinish: null,
	oncancel: null,
	onremove: null,
	currentTime: 0,
	effect: null,
	id: '',
	pending: false,
	playState: 'finished',
	playbackRate: 1,
	startTime: 0,
	timeline: null,
	replaceState: 'active'
}));

// Mock getBoundingClientRect for scroll-related calculations
Element.prototype.getBoundingClientRect = vi.fn(() => ({
	top: 0,
	left: 0,
	bottom: 0,
	right: 0,
	width: 0,
	height: 0,
	x: 0,
	y: 0
}));
