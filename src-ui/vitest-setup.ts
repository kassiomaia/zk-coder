import { afterEach, vi, expect, beforeEach } from 'vitest';
import { cleanup } from '@testing-library/svelte';
import * as matchers from '@testing-library/jest-dom/matchers';

// Extend Vitest's expect with jest-dom matchers
expect.extend(matchers);

// Mock all icon components
const iconMock = vi.fn(() => ({
	render: () => '<svg class="mock-icon" width="16" height="16"></svg>'
}));

vi.mock('$lib/icons', () => ({
	Icon: iconMock,
	CopyIcon: iconMock,
	EditIcon: iconMock,
	DeleteIcon: iconMock,
	CheckIcon: iconMock,
	RefreshIcon: iconMock,
	RefreshCcwIcon: iconMock,
	RenameIcon: iconMock,
	ArchiveIcon: iconMock,
	ThumbsUpIcon: iconMock,
	ThumbsDownIcon: iconMock,
	ChevronDownIcon: iconMock,
	ArrowDownIcon: iconMock,
	ArrowUpIcon: iconMock,
	PlusIcon: iconMock,
	CodeIcon: iconMock,
	SendIcon: iconMock,
	CloseIcon: iconMock,
	SearchIcon: iconMock,
	MoreIcon: iconMock,
	SettingsIcon: iconMock,
	MessageIcon: iconMock,
	MessageSquareIcon: iconMock,
	SpinnerIcon: iconMock,
	CheckCircleIcon: iconMock,
	AlertCircleIcon: iconMock,
	FileIcon: iconMock,
	PaperclipIcon: iconMock,
	UserIcon: iconMock,
	BoxIcon: iconMock
}));

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
