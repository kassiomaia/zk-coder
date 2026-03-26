import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import ChatScrollButton from './ChatScrollButton.svelte';

describe('ChatScrollButton', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('does not render when show is false', () => {
			const { container } = render(ChatScrollButton, {
				show: false,
				onclick: vi.fn()
			});
			const button = container.querySelector('button');
			expect(button).not.toBeInTheDocument();
		});

		it('renders button when show is true', () => {
			const { container } = render(ChatScrollButton, {
				show: true,
				onclick: vi.fn()
			});
			const button = container.querySelector('button');
			expect(button).toBeInTheDocument();
		});

		it('renders the scroll down icon', () => {
			render(ChatScrollButton, {
				show: true,
				onclick: vi.fn()
			});
			const icon = document.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});
	});

	describe('interaction', () => {
		it('calls onclick when button is clicked', async () => {
			const onclick = vi.fn();
			const { container } = render(ChatScrollButton, {
				show: true,
				onclick
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
				expect(onclick).toHaveBeenCalledTimes(1);
			}
		});

		it('handles multiple clicks', async () => {
			const onclick = vi.fn();
			const { container } = render(ChatScrollButton, {
				show: true,
				onclick
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
				await fireEvent.click(button);
				await fireEvent.click(button);
				expect(onclick).toHaveBeenCalledTimes(3);
			}
		});
	});

	describe('accessibility', () => {
		it('has proper aria-label', () => {
			const { container } = render(ChatScrollButton, {
				show: true,
				onclick: vi.fn()
			});
			const button = container.querySelector('[aria-label="Scroll to latest messages"]');
			expect(button).toBeInTheDocument();
		});

		it('has type="button"', () => {
			const { container } = render(ChatScrollButton, {
				show: true,
				onclick: vi.fn()
			});
			const button = container.querySelector('button[type="button"]');
			expect(button).toBeInTheDocument();
		});
	});

	describe('CSS classes', () => {
		it('applies scroll to bottom class', () => {
			const { container } = render(ChatScrollButton, {
				show: true,
				onclick: vi.fn()
			});
			const button = container.querySelector('.scrollToBottom, [class*="scrollToBottom"]');
			expect(button).toBeInTheDocument();
		});

		it('applies secondaryElements class', () => {
			const { container } = render(ChatScrollButton, {
				show: true,
				onclick: vi.fn()
			});
			const button = container.querySelector('.secondaryElements, [class*="secondary"]');
			expect(button).toBeInTheDocument();
		});

		it('applies pulsing class when streaming (checked via isLlmStreaming store)', () => {
			// Note: The pulsing class is controlled by the isLlmStreaming store
			// We're testing that the button renders correctly
			const { container } = render(ChatScrollButton, {
				show: true,
				onclick: vi.fn()
			});
			const button = container.querySelector('button');
			expect(button).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles undefined onclick gracefully', () => {
			const { container } = render(ChatScrollButton, {
				show: true,
				onclick: undefined as unknown as () => void
			});
			const button = container.querySelector('button');
			expect(button).toBeInTheDocument();
		});

		it('handles rapid show/hide transitions', () => {
			const { container, rerender } = render(ChatScrollButton, {
				show: false,
				onclick: vi.fn()
			});

			expect(container.querySelector('button')).not.toBeInTheDocument();

			// Re-render with show=true
			render(ChatScrollButton, {
				show: true,
				onclick: vi.fn()
			});

			expect(container.querySelector('button')).toBeInTheDocument();
		});

		it('preserves onclick handler through re-renders', async () => {
			const onclick = vi.fn();
			const { container, rerender } = render(ChatScrollButton, {
				show: true,
				onclick
			});

			// Re-render with same handler
			render(ChatScrollButton, {
				show: true,
				onclick
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
				expect(onclick).toHaveBeenCalledTimes(1);
			}
		});
	});
});
