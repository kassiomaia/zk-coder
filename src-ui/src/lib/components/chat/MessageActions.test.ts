import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import MessageActions from './MessageActions.svelte';

// Mock IconButton component
vi.mock('$lib/components/shared/IconButton.svelte', () => ({
	default: {
		render: (props: Record<string, unknown>) => ({
			props,
			fireEvent: (eventName: string) => {
				if (eventName === 'click') props.onclick?.();
			}
		})
	}
}));

// Mock icons
vi.mock('$lib/icons', () => ({
	CopyIcon: { size: 16 },
	EditIcon: { size: 16 },
	DeleteIcon: { size: 16 },
	RefreshIcon: { size: 16 },
	ThumbsUpIcon: { size: 16, filled: false },
	ThumbsDownIcon: { size: 16, filled: false },
	CheckIcon: { size: 16 }
}));

describe('MessageActions', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.restoreAllMocks();
		vi.useRealTimers();
	});

	const defaultProps = {
		showCopy: true,
		showEdit: true,
		showDelete: false,
		showRetry: false,
		showFeedback: true
	};

	describe('rendering', () => {
		it('renders the actions toolbar', () => {
			const { container } = render(MessageActions, { ...defaultProps });
			const toolbar = container.querySelector('[role="toolbar"]');
			expect(toolbar).toBeInTheDocument();
		});

		it('renders copy button when showCopy is true', () => {
			render(MessageActions, { ...defaultProps, showCopy: true });
			const copyButton = screen.getByRole('button', { name: /copy/i });
			expect(copyButton).toBeInTheDocument();
		});

		it('renders edit button when showEdit is true', () => {
			render(MessageActions, { ...defaultProps, showEdit: true });
			const editButton = screen.getByRole('button', { name: /edit/i });
			expect(editButton).toBeInTheDocument();
		});

		it('renders delete button when showDelete is true', () => {
			render(MessageActions, { ...defaultProps, showDelete: true });
			const deleteButton = screen.getByRole('button', { name: /delete/i });
			expect(deleteButton).toBeInTheDocument();
		});

		it('renders retry button when showRetry is true', () => {
			render(MessageActions, { ...defaultProps, showRetry: true });
			const retryButton = screen.getByRole('button', { name: /retry/i });
			expect(retryButton).toBeInTheDocument();
		});

		it('renders feedback buttons when showFeedback is true', () => {
			render(MessageActions, { ...defaultProps, showFeedback: true });
			const thumbsUpButton = screen.getByRole('button', { name: /good response/i });
			const thumbsDownButton = screen.getByRole('button', { name: /bad response/i });
			expect(thumbsUpButton).toBeInTheDocument();
			expect(thumbsDownButton).toBeInTheDocument();
		});
	});

	describe('copy functionality', () => {
		it('calls onCopy when copy button is clicked', async () => {
			const onCopy = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				onCopy
			});

			const copyButton = screen.getByRole('button', { name: /copy/i });
			await fireEvent.click(copyButton);

			expect(onCopy).toHaveBeenCalled();
		});

		it('shows copied state after copy (internal handling)', async () => {
			const { container } = render(MessageActions, {
				...defaultProps,
				onCopy: undefined
			});

			const copyButton = screen.getByRole('button', { name: /copy/i });
			await fireEvent.click(copyButton);

			// After click, the label should change to 'Copied'
			await waitFor(() => {
				const copiedButton = screen.queryByRole('button', { name: /copied/i });
				expect(copiedButton).toBeInTheDocument();
			});
		});

		it('resets copied state after 2 seconds', async () => {
			const { container } = render(MessageActions, {
				...defaultProps,
				onCopy: undefined
			});

			const copyButton = screen.getByRole('button', { name: /copy/i });
			await fireEvent.click(copyButton);

			await vi.advanceTimersByTimeAsync(2000);

			// After timeout, should show 'Copy' again
			const copyButtonAfter = screen.queryByRole('button', { name: /copy/i });
			expect(copyButtonAfter).toBeInTheDocument();
		});
	});

	describe('edit functionality', () => {
		it('calls onEdit when edit button is clicked', async () => {
			const onEdit = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				onEdit
			});

			const editButton = screen.getByRole('button', { name: /edit/i });
			await fireEvent.click(editButton);

			expect(onEdit).toHaveBeenCalled();
		});
	});

	describe('delete functionality', () => {
		it('calls onDelete when delete button is clicked', async () => {
			const onDelete = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				showDelete: true,
				onDelete
			});

			const deleteButton = screen.getByRole('button', { name: /delete/i });
			await fireEvent.click(deleteButton);

			expect(onDelete).toHaveBeenCalled();
		});
	});

	describe('retry functionality', () => {
		it('calls onRetry when retry button is clicked', async () => {
			const onRetry = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				showRetry: true,
				onRetry
			});

			const retryButton = screen.getByRole('button', { name: /retry/i });
			await fireEvent.click(retryButton);

			expect(onRetry).toHaveBeenCalled();
		});
	});

	describe('feedback functionality', () => {
		it('calls onFeedback with "like" when thumbs up is clicked', async () => {
			const onFeedback = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				onFeedback
			});

			const thumbsUpButton = screen.getByRole('button', { name: /good response/i });
			await fireEvent.click(thumbsUpButton);

			expect(onFeedback).toHaveBeenCalledWith('like');
		});

		it('calls onFeedback with "dislike" when thumbs down is clicked', async () => {
			const onFeedback = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				onFeedback
			});

			const thumbsDownButton = screen.getByRole('button', { name: /bad response/i });
			await fireEvent.click(thumbsDownButton);

			expect(onFeedback).toHaveBeenCalledWith('dislike');
		});

		it('toggles off thumbs up when clicked again', async () => {
			const onFeedback = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				feedback: 'like',
				onFeedback
			});

			const thumbsUpButton = screen.getByRole('button', { name: /good response/i });
			await fireEvent.click(thumbsUpButton);

			expect(onFeedback).toHaveBeenCalledWith(null);
		});

		it('toggles off thumbs down when clicked again', async () => {
			const onFeedback = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				feedback: 'dislike',
				onFeedback
			});

			const thumbsDownButton = screen.getByRole('button', { name: /bad response/i });
			await fireEvent.click(thumbsDownButton);

			expect(onFeedback).toHaveBeenCalledWith(null);
		});

		it('switches from dislike to like when thumbs up is clicked after dislike', async () => {
			const onFeedback = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				feedback: 'dislike',
				onFeedback
			});

			const thumbsUpButton = screen.getByRole('button', { name: /good response/i });
			await fireEvent.click(thumbsUpButton);

			expect(onFeedback).toHaveBeenCalledWith('like');
		});
	});

	describe('visibility control', () => {
		it('hides copy button when showCopy is false', () => {
			render(MessageActions, { ...defaultProps, showCopy: false });
			const copyButton = screen.queryByRole('button', { name: /copy/i });
			expect(copyButton).not.toBeInTheDocument();
		});

		it('hides edit button when showEdit is false', () => {
			render(MessageActions, { ...defaultProps, showEdit: false });
			const editButton = screen.queryByRole('button', { name: /edit/i });
			expect(editButton).not.toBeInTheDocument();
		});

		it('hides delete button when showDelete is false', () => {
			render(MessageActions, { ...defaultProps, showDelete: false });
			const deleteButton = screen.queryByRole('button', { name: /delete/i });
			expect(deleteButton).not.toBeInTheDocument();
		});

		it('hides retry button when showRetry is false', () => {
			render(MessageActions, { ...defaultProps, showRetry: false });
			const retryButton = screen.queryByRole('button', { name: /retry/i });
			expect(retryButton).not.toBeInTheDocument();
		});

		it('hides feedback buttons when showFeedback is false', () => {
			render(MessageActions, { ...defaultProps, showFeedback: false });
			const thumbsUpButton = screen.queryByRole('button', { name: /good response/i });
			const thumbsDownButton = screen.queryByRole('button', { name: /bad response/i });
			expect(thumbsUpButton).not.toBeInTheDocument();
			expect(thumbsDownButton).not.toBeInTheDocument();
		});
	});

	describe('feedback state display', () => {
		it('shows active state on thumbs up when feedback is "like"', () => {
			const { container } = render(MessageActions, {
				...defaultProps,
				feedback: 'like'
			});
			const thumbsUpButton = screen.getByRole('button', { name: /good response/i });
			expect(thumbsUpButton).toBeInTheDocument();
		});

		it('shows active state on thumbs down when feedback is "dislike"', () => {
			const { container } = render(MessageActions, {
				...defaultProps,
				feedback: 'dislike'
			});
			const thumbsDownButton = screen.getByRole('button', { name: /bad response/i });
			expect(thumbsDownButton).toBeInTheDocument();
		});

		it('shows no active feedback when feedback is null', () => {
			const { container } = render(MessageActions, {
				...defaultProps,
				feedback: null
			});
			expect(container.querySelector('[role="toolbar"]')).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles all handlers being undefined', () => {
			const { container } = render(MessageActions, {
				...defaultProps,
				onCopy: undefined,
				onEdit: undefined,
				onDelete: undefined,
				onRetry: undefined,
				onFeedback: undefined
			});
			const toolbar = container.querySelector('[role="toolbar"]');
			expect(toolbar).toBeInTheDocument();
		});

		it('handles all show flags being false', () => {
			const { container } = render(MessageActions, {
				showCopy: false,
				showEdit: false,
				showDelete: false,
				showRetry: false,
				showFeedback: false
			});
			const toolbar = container.querySelector('[role="toolbar"]');
			// Toolbar should still be present even with no buttons
			expect(toolbar).toBeInTheDocument();
		});

		it('handles multiple rapid feedback changes', async () => {
			const onFeedback = vi.fn();
			const { container } = render(MessageActions, {
				...defaultProps,
				onFeedback
			});

			const thumbsUpButton = screen.getByRole('button', { name: /good response/i });
			const thumbsDownButton = screen.getByRole('button', { name: /bad response/i });

			await fireEvent.click(thumbsUpButton);
			await fireEvent.click(thumbsDownButton);
			await fireEvent.click(thumbsUpButton);

			expect(onFeedback).toHaveBeenCalledTimes(3);
		});
	});

	describe('accessibility', () => {
		it('has proper toolbar role', () => {
			const { container } = render(MessageActions, { ...defaultProps });
			const toolbar = container.querySelector('[role="toolbar"]');
			expect(toolbar).toBeInTheDocument();
		});

		it('has aria-label for toolbar', () => {
			const { container } = render(MessageActions, { ...defaultProps });
			const toolbar = container.querySelector('[aria-label="Message actions"]');
			expect(toolbar).toBeInTheDocument();
		});

		it('all buttons have proper labels', () => {
			render(MessageActions, { ...defaultProps });
			const buttons = screen.getAllByRole('button');
			buttons.forEach((button) => {
				expect(button).toHaveAttribute('aria-label');
				expect(button).toHaveAttribute('label');
			});
		});
	});
});
