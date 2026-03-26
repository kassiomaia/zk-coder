import { render, screen } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import EmptyThreadState from './EmptyThreadState.svelte';

describe('EmptyThreadState', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering - none context', () => {
		it('renders container', () => {
			const { container } = render(EmptyThreadState, {
				context: 'none'
			});
			const containerEl = container.querySelector('[class*="container"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('displays correct headline for none context', () => {
			render(EmptyThreadState, {
				context: 'none'
			});
			expect(screen.getByText('Select or create a conversation')).toBeInTheDocument();
		});

		it('displays correct hint for none context', () => {
			render(EmptyThreadState, {
				context: 'none'
			});
			expect(
				screen.getByText('Choose a conversation from the sidebar to get started')
			).toBeInTheDocument();
		});

		it('displays action hint for none context', () => {
			render(EmptyThreadState, {
				context: 'none'
			});
			expect(screen.getByText('Or create a new conversation to get started')).toBeInTheDocument();
		});
	});

	describe('rendering - conversation context', () => {
		it('displays correct headline for conversation context', () => {
			render(EmptyThreadState, {
				context: 'conversation'
			});
			expect(screen.getByText('Start a new thread')).toBeInTheDocument();
		});

		it('displays correct hint for conversation context', () => {
			render(EmptyThreadState, {
				context: 'conversation'
			});
			expect(
				screen.getByText('Create a thread from the sidebar or send a message')
			).toBeInTheDocument();
		});

		it('includes conversation title in subtext when provided', () => {
			render(EmptyThreadState, {
				context: 'conversation',
				conversationTitle: 'My Project'
			});
			expect(screen.getByText('in "My Project"')).toBeInTheDocument();
		});
	});

	describe('rendering - thread context', () => {
		it('displays correct headline for thread context', () => {
			render(EmptyThreadState, {
				context: 'thread'
			});
			expect(screen.getByText('Start a conversation')).toBeInTheDocument();
		});

		it('displays correct hint for thread context', () => {
			render(EmptyThreadState, {
				context: 'thread'
			});
			expect(screen.getByText('Type a message below to begin')).toBeInTheDocument();
		});
	});

	describe('input activity state', () => {
		it('shows "Ready when you are" when hasInputActivity is true', () => {
			render(EmptyThreadState, {
				context: 'thread',
				hasInputActivity: true
			});
			expect(screen.getByText('Ready when you are')).toBeInTheDocument();
		});

		it('shows "Press Enter to send" when hasInputActivity is true', () => {
			render(EmptyThreadState, {
				context: 'thread',
				hasInputActivity: true
			});
			expect(screen.getByText('Press Enter to send')).toBeInTheDocument();
		});

		it('applies active class when hasInputActivity is true', () => {
			const { container } = render(EmptyThreadState, {
				context: 'thread',
				hasInputActivity: true
			});
			const containerEl = container.querySelector('.active, [class*="active"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('hides action hint when hasInputActivity is true', () => {
			render(EmptyThreadState, {
				context: 'none',
				hasInputActivity: true
			});
			expect(
				screen.queryByText('Or create a new conversation to get started')
			).not.toBeInTheDocument();
		});
	});

	describe('icons', () => {
		it('renders icon for none context', () => {
			const { container } = render(EmptyThreadState, {
				context: 'none'
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});

		it('renders icon for conversation context', () => {
			const { container } = render(EmptyThreadState, {
				context: 'conversation'
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});

		it('renders icon for thread context', () => {
			const { container } = render(EmptyThreadState, {
				context: 'thread'
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});
	});

	describe('showSecondary prop', () => {
		it('renders secondary elements when showSecondary is true', () => {
			const { container } = render(EmptyThreadState, {
				context: 'thread',
				showSecondary: true
			});
			const containerEl = container.querySelector('[class*="container"]');
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has aria-live="polite"', () => {
			const { container } = render(EmptyThreadState);
			const liveEl = container.querySelector('[aria-live="polite"]');
			expect(liveEl).toBeInTheDocument();
		});

		it('has aria-atomic="true"', () => {
			const { container } = render(EmptyThreadState);
			const atomicEl = container.querySelector('[aria-atomic="true"]');
			expect(atomicEl).toBeInTheDocument();
		});
	});

	describe('CSS classes', () => {
		it('applies context-specific class', () => {
			const { container } = render(EmptyThreadState, {
				context: 'thread'
			});
			const containerEl = container.querySelector('.thread, [class*="thread"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('applies conversation context class', () => {
			const { container } = render(EmptyThreadState, {
				context: 'conversation'
			});
			const containerEl = container.querySelector('.conversation, [class*="conversation"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('applies none context class', () => {
			const { container } = render(EmptyThreadState, {
				context: 'none'
			});
			const containerEl = container.querySelector('.none, [class*="none"]');
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles undefined context (defaults to none)', () => {
			render(EmptyThreadState);
			expect(screen.getByText('Select or create a conversation')).toBeInTheDocument();
		});

		it('handles empty conversation title', () => {
			render(EmptyThreadState, {
				context: 'conversation',
				conversationTitle: ''
			});
			// Should still render without error
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles long conversation title', () => {
			const longTitle = 'A'.repeat(200);
			render(EmptyThreadState, {
				context: 'conversation',
				conversationTitle: longTitle
			});
			expect(screen.getByText(`in "${longTitle}"`)).toBeInTheDocument();
		});
	});
});
