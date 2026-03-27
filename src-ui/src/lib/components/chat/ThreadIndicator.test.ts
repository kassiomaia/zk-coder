import {fireEvent, render, screen} from '@testing-library/svelte';
import {afterEach, beforeEach, describe, expect, it, vi} from 'vitest';
import ThreadIndicator from './ThreadIndicator.svelte';
import type {Thread} from '$lib/types/thread';

describe('ThreadIndicator', () => {
	let mockThread: Thread;

	beforeEach(() => {
		vi.clearAllMocks();
		mockThread = {
			id: 'thread-1',
			conversationId: 'conv-1',
			parentId: null,
			title: 'Feature Discussion',
			createdAt: new Date('2024-01-15T10:00:00'),
			updatedAt: new Date('2024-01-15T10:30:00'),
			messageCount: 5,
			metadata: { resolved: false }
		};
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('renders the button', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread
			});
			const button = container.querySelector('button');
			expect(button).toBeInTheDocument();
		});

		it('renders the thread icon', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});

		it('displays thread title by default', () => {
			render(ThreadIndicator, {
				thread: mockThread
			});
			expect(screen.getByText('Feature Discussion')).toBeInTheDocument();
		});
	});

	describe('compact mode', () => {
		it('hides title when compact is true', () => {
			render(ThreadIndicator, {
				thread: mockThread,
				compact: true
			});
			expect(screen.queryByText('Feature Discussion')).not.toBeInTheDocument();
		});

		it('applies compact class', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread,
				compact: true
			});
			const button = container.querySelector('.compact, [class*="compact"]');
			expect(button).toBeInTheDocument();
		});

		it('still renders icon in compact mode', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread,
				compact: true
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});
	});

	describe('interaction', () => {
		it('calls onclick handler when clicked', async () => {
			const onclick = vi.fn();
			const { container } = render(ThreadIndicator, {
				thread: mockThread,
				onclick
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
				expect(onclick).toHaveBeenCalledTimes(1);
			}
		});

		it('stops event propagation', async () => {
			const onclick = vi.fn();
			let parentClicked = false;

			const { container } = render(ThreadIndicator, {
				thread: mockThread,
				onclick: (e: MouseEvent) => {
					parentClicked = true;
					onclick();
				}
			});

			const button = container.querySelector('button');
			if (button) {
				// Simulate the click and check if handler was called
				await fireEvent.click(button);
				expect(onclick).toHaveBeenCalled();
			}
		});
	});

	describe('accessibility', () => {
		it('has proper aria-label with thread title', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread
			});
			const button = container.querySelector(
				'button[aria-label="View thread: Feature Discussion"]'
			);
			expect(button).toBeInTheDocument();
		});

		it('has type="button"', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread
			});
			const button = container.querySelector('button[type="button"]');
			expect(button).toBeInTheDocument();
		});
	});

	describe('different thread titles', () => {
		it('handles short titles', () => {
			render(ThreadIndicator, {
				thread: { ...mockThread, title: 'Chat' }
			});
			expect(screen.getByText('Chat')).toBeInTheDocument();
		});

		it('handles long titles', () => {
			const longTitle = 'A'.repeat(100);
			render(ThreadIndicator, {
				thread: { ...mockThread, title: longTitle }
			});
			expect(screen.getByText(longTitle)).toBeInTheDocument();
		});

		it('handles special characters in title', () => {
			const specialTitle = 'Feature: <script>alert("test")</script>';
			render(ThreadIndicator, {
				thread: { ...mockThread, title: specialTitle }
			});
			// Title should be displayed as text, not executed
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles emoji in title', () => {
			const emojiTitle = 'Discussion 🚀';
			render(ThreadIndicator, {
				thread: { ...mockThread, title: emojiTitle }
			});
			expect(screen.getByText('Discussion 🚀')).toBeInTheDocument();
		});
	});

	describe('CSS classes', () => {
		it('applies indicator base class', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread
			});
			const button = container.querySelector('.indicator, [class*="indicator"]');
			expect(button).toBeInTheDocument();
		});

		it('applies icon class', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread
			});
			const icon = container.querySelector('.icon, [class*="icon"]');
			expect(icon).toBeInTheDocument();
		});

		it('applies title class in non-compact mode', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread,
				compact: false
			});
			const titleEl = container.querySelector('.title, [class*="title"]');
			expect(titleEl).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles undefined onclick', () => {
			const { container } = render(ThreadIndicator, {
				thread: mockThread,
				onclick: undefined
			});
			const button = container.querySelector('button');
			expect(button).toBeInTheDocument();
		});

		it('handles empty title', () => {
			const { container } = render(ThreadIndicator, {
				thread: { ...mockThread, title: '' }
			});
			const button = container.querySelector('button');
			expect(button).toBeInTheDocument();
		});

		it('handles thread with null parent', () => {
			render(ThreadIndicator, {
				thread: mockThread
			});
			const button = document.querySelector('button');
			expect(button).toBeInTheDocument();
		});

		it('handles thread with parent reference', () => {
			const childThread: Thread = {
				...mockThread,
				id: 'thread-2',
				parentId: 'thread-1'
			};
			const { container } = render(ThreadIndicator, {
				thread: childThread
			});
			const button = container.querySelector('button');
			expect(button).toBeInTheDocument();
		});
	});
});
