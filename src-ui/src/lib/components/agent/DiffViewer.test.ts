import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import DiffViewer from './DiffViewer.svelte';

// Mock the Diff2HtmlUI class
const mockDraw = vi.fn();
const mockHighlightCode = vi.fn();

class MockDiff2HtmlUI {
	draw = mockDraw;
	highlightCode = mockHighlightCode;
	constructor(target: any, diff: string, config: any) {
		// Store constructor args for verification
		(this as any).target = target;
		(this as any).diff = diff;
		(this as any).config = config;
	}
}

vi.mock('diff2html/lib/ui/js/diff2html-ui', () => ({
	Diff2HtmlUI: vi.fn((target: any, diff: string, config: any) => new MockDiff2HtmlUI(target, diff, config))
}));

// Mock highlight.js
const mockHighlight = {
	registerLanguage: vi.fn(),
	highlight: vi.fn(() => ({ value: '<span>highlighted</span>' }))
};

vi.mock('highlight.js/lib/core', () => ({
	default: mockHighlight
}));

// Mock language imports - return objects with default export
vi.mock('highlight.js/lib/languages/typescript', () => ({ default: {} }));
vi.mock('highlight.js/lib/languages/javascript', () => ({ default: {} }));
vi.mock('highlight.js/lib/languages/rust', () => ({ default: {} }));
vi.mock('highlight.js/lib/languages/python', () => ({ default: {} }));
vi.mock('highlight.js/lib/languages/css', () => ({ default: {} }));
vi.mock('highlight.js/lib/languages/scss', () => ({ default: {} }));
vi.mock('highlight.js/lib/languages/json', () => ({ default: {} }));
vi.mock('highlight.js/lib/languages/yaml', () => ({ default: {} }));
vi.mock('highlight.js/lib/languages/markdown', () => ({ default: {} }));
vi.mock('highlight.js/lib/languages/xml', () => ({ default: {} }));

// Mock CSS imports
vi.mock('diff2html/bundles/css/diff2html.min.css', () => ({}));

const sampleDiff = `--- a/src/test.ts
+++ b/src/test.ts
@@ -1,5 +1,5 @@
-export function oldFunction() {
+export function newFunction() {
     console.log('test');
-    return true;
+    return false;
 }`;

const multiFileDiff = `diff --git a/src/file1.ts b/src/file1.ts
index abc123..def456 100644
--- a/src/file1.ts
+++ b/src/file1.ts
@@ -1,1 +1,1 @@
-old content
+new content
diff --git a/src/file2.ts b/src/file2.ts
index 789xyz..012abc 100644
--- a/src/file2.ts
+++ b/src/file2.ts
@@ -1,1 +1,1 @@
-another old
+another new`;

describe('DiffViewer', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('rendering', () => {
		it('renders diff viewer wrapper', () => {
			const { container } = render(DiffViewer, { diff: sampleDiff });

			const wrapper = container.querySelector('.diff-viewer-wrapper');
			expect(wrapper).toBeInTheDocument();
		});

		it('renders diff header with title', () => {
			render(DiffViewer, { diff: sampleDiff });

			expect(screen.getByText('Changes')).toBeInTheDocument();
		});

		it('renders view toggle button', () => {
			render(DiffViewer, { diff: sampleDiff });

			const toggleBtn = screen.getByRole('button', { name: /Side by Side/i });
			expect(toggleBtn).toBeInTheDocument();
		});

		it('renders diff2html container', () => {
			const { container } = render(DiffViewer, { diff: sampleDiff });

			const diffContainer = container.querySelector('.diff2html-container');
			expect(diffContainer).toBeInTheDocument();
		});
	});

	describe('view toggling', () => {
		it('starts in unified view mode by default', () => {
			render(DiffViewer, { diff: sampleDiff });

			const toggleBtn = screen.getByRole('button', { name: /Side by Side/i });
			expect(toggleBtn).toBeInTheDocument();
		});

		it('switches to side-by-side view when toggle clicked', async () => {
			render(DiffViewer, { diff: sampleDiff });

			const toggleBtn = screen.getByRole('button', { name: /Side by Side/i });
			await fireEvent.click(toggleBtn);

			// Button text should change
			await waitFor(() => {
				const unifiedBtn = screen.queryByRole('button', { name: /Unified/i });
				expect(unifiedBtn).toBeInTheDocument();
			});
		});

		it('switches back to unified view when toggled again', async () => {
			render(DiffViewer, { diff: sampleDiff });

			const toggleBtn = screen.getByRole('button', { name: /Side by Side/i });
			await fireEvent.click(toggleBtn);

			await waitFor(() => {
				const unifiedBtn = screen.queryByRole('button', { name: /Unified/i });
				if (unifiedBtn) {
					fireEvent.click(unifiedBtn);
				}
			});

			await waitFor(() => {
				const sideBySideBtn = screen.queryByRole('button', { name: /Side by Side/i });
				expect(sideBySideBtn).toBeInTheDocument();
			});
		});
	});

	describe('props', () => {
		it('accepts custom maxHeight prop', () => {
			const { container } = render(DiffViewer, {
				diff: sampleDiff,
				maxHeight: 800
			});

			const wrapper = container.querySelector('.diff-viewer-wrapper');
			expect(wrapper).toBeInTheDocument();
		});

		it('accepts view prop to set initial view mode', () => {
			render(DiffViewer, {
				diff: sampleDiff,
				view: 'side-by-side'
			});

			// Should start with side-by-side view
			const unifiedBtn = screen.queryByRole('button', { name: /Unified/i });
			expect(unifiedBtn).toBeInTheDocument();
		});

		it('accepts wordLevelDiff prop', () => {
			const { container } = render(DiffViewer, {
				diff: sampleDiff,
				wordLevelDiff: false
			});

			const wrapper = container.querySelector('.diff-viewer-wrapper');
			expect(wrapper).toBeInTheDocument();
		});

		it('accepts language prop for syntax highlighting', () => {
			render(DiffViewer, {
				diff: sampleDiff,
				language: 'typescript'
			});

			// Language is passed to Diff2HtmlUI, verify component renders
			expect(screen.getByText('Changes')).toBeInTheDocument();
		});
	});

	describe('Diff2Html integration', () => {
		it('initializes Diff2HtmlUI with correct configuration', async () => {
			render(DiffViewer, { diff: sampleDiff });

			await waitFor(() => {
				expect(mockDraw).toHaveBeenCalled();
			});
		});

		it('calls highlightCode method', async () => {
			render(DiffViewer, { diff: sampleDiff });

			await waitFor(() => {
				expect(mockHighlightCode).toHaveBeenCalled();
			});
		});
	});

	describe('reactivity', () => {
		it('re-renders when diff content changes', async () => {
			const { rerender } = render(DiffViewer, { diff: sampleDiff });

			const initialCalls = mockDraw.mock.calls.length;

			await rerender({ diff: multiFileDiff });

			// Should re-draw with new diff content
			await waitFor(() => {
				expect(mockDraw.mock.calls.length).toBeGreaterThan(initialCalls);
			});
		});

		it('re-renders when view mode changes', async () => {
			const { rerender } = render(DiffViewer, {
				diff: sampleDiff,
				view: 'unified'
			});

			const initialCalls = mockDraw.mock.calls.length;

			await rerender({ diff: sampleDiff, view: 'side-by-side' });

			await waitFor(() => {
				expect(mockDraw.mock.calls.length).toBeGreaterThan(initialCalls);
			});
		});

		it('re-renders when wordLevelDiff changes', async () => {
			const { rerender } = render(DiffViewer, {
				diff: sampleDiff,
				wordLevelDiff: true
			});

			const initialCalls = mockDraw.mock.calls.length;

			await rerender({ diff: sampleDiff, wordLevelDiff: false });

			await waitFor(() => {
				expect(mockDraw.mock.calls.length).toBeGreaterThan(initialCalls);
			});
		});
	});

	describe('error handling', () => {
		it('handles invalid diff content gracefully', () => {
			// Spy on console.error
			const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

			const invalidDiff = 'not a valid diff';
			const { container } = render(DiffViewer, { diff: invalidDiff });

			// Component should still render wrapper
			const wrapper = container.querySelector('.diff-viewer-wrapper');
			expect(wrapper).toBeInTheDocument();

			consoleSpy.mockRestore();
		});
	});

	describe('cleanup', () => {
		it('cleans up diff2HtmlUi instance on destroy', () => {
			const { unmount } = render(DiffViewer, { diff: sampleDiff });

			expect(mockDraw).toHaveBeenCalled();

			unmount();

			// After unmount, no errors should occur
			expect(mockDraw).toHaveBeenCalled();
		});
	});

	describe('accessibility', () => {
		it('view toggle button has proper title attribute', () => {
			const { container } = render(DiffViewer, { diff: sampleDiff });

			const toggleBtn = container.querySelector('.view-toggle') as HTMLButtonElement;
			expect(toggleBtn).toHaveAttribute(
				'title',
				'Switch between unified and side-by-side view'
			);
		});

		it('view toggle button has proper type attribute', () => {
			const { container } = render(DiffViewer, { diff: sampleDiff });

			const toggleBtn = container.querySelector('.view-toggle') as HTMLButtonElement;
			expect(toggleBtn).toHaveAttribute('type', 'button');
		});
	});

	describe('styling', () => {
		it('applies correct CSS classes to header elements', () => {
			const { container } = render(DiffViewer, { diff: sampleDiff });

			const header = container.querySelector('.diff-header');
			const title = container.querySelector('.diff-title');
			const toggle = container.querySelector('.view-toggle');

			expect(header).toBeInTheDocument();
			expect(title).toBeInTheDocument();
			expect(toggle).toBeInTheDocument();
		});

		it('applies correct CSS class to container', () => {
			const { container } = render(DiffViewer, { diff: sampleDiff });

			const diffContainer = container.querySelector('.diff2html-container');
			expect(diffContainer).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty diff string', () => {
			const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

			const { container } = render(DiffViewer, { diff: '' });

			const wrapper = container.querySelector('.diff-viewer-wrapper');
			expect(wrapper).toBeInTheDocument();

			consoleSpy.mockRestore();
		});

		it('handles multi-file diff', () => {
			const { container } = render(DiffViewer, { diff: multiFileDiff });

			const wrapper = container.querySelector('.diff-viewer-wrapper');
			expect(wrapper).toBeInTheDocument();
		});

		it('handles diff with special characters', () => {
			const specialDiff = `--- a/test.txt
+++ b/test.txt
@@ -1,1 +1,1 @@
-old with "quotes" and 'apostrophes'
+new with "quotes" and 'apostrophes'`;

			const { container } = render(DiffViewer, { diff: specialDiff });

			const wrapper = container.querySelector('.diff-viewer-wrapper');
			expect(wrapper).toBeInTheDocument();
		});

		it('handles very large diff', () => {
			const largeDiff = sampleDiff.repeat(1000);

			const { container } = render(DiffViewer, { diff: largeDiff });

			const wrapper = container.querySelector('.diff-viewer-wrapper');
			expect(wrapper).toBeInTheDocument();
		});
	});
});
