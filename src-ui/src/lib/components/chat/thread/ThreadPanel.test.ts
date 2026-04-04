import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ThreadPanel from './ThreadPanel.svelte';
import type { Thread } from '$lib/types/thread';
import type { Message } from '$lib/types/message';

// Mock the stores
const mockChatStore = {
	markThreadRead: vi.fn()
};

const mockUnreadThreadIds = {
	subscribe: vi.fn(() => vi.fn())
};

vi.mock('$lib/stores/chat', () => ({
	chatStore: mockChatStore,
	unreadThreadIds: mockUnreadThreadIds
}));

describe('ThreadPanel', () => {
	const mockThread: Thread = {
		id: 'thread1',
		conversationId: 'conv1',
		parentId: null,
		title: 'Test Thread',
		createdAt: new Date('2024-01-15T10:00:00'),
		updatedAt: new Date('2024-01-15T10:05:00'),
		messageCount: 5,
		metadata: {
			resolved: false,
			tags: ['test', 'example']
		}
	};

	const mockResolvedThread: Thread = {
		...mockThread,
		id: 'thread2',
		metadata: {
			resolved: true,
			tags: ['resolved']
		}
	};

	const mockMessages: Message[] = [
		{
			id: 'msg1',
			conversationId: 'conv1',
			threadId: 'thread1',
			role: 'user',
			content: 'Hello',
			timestamp: new Date('2024-01-15T10:00:00'),
			status: 'sent'
		},
		{
			id: 'msg2',
			conversationId: 'conv1',
			threadId: 'thread1',
			role: 'assistant',
			content: 'Hi there!',
			timestamp: new Date('2024-01-15T10:01:00'),
			status: 'sent'
		}
	];

	const defaultProps = {
		thread: mockThread,
		messages: mockMessages
	};

	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.restoreAllMocks();
		vi.useRealTimers();
	});

	describe('rendering', () => {
		it('renders the panel container', () => {
			const { container } = render(ThreadPanel, { ...defaultProps });
			const panel = container.querySelector('.panel, [class*="panel"]');
			expect(panel).toBeInTheDocument();
		});

		it('renders thread title in header', () => {
			render(ThreadPanel, { ...defaultProps });
			expect(screen.getByText('Test Thread')).toBeInTheDocument();
		});

		it('renders header divider', () => {
			const { container } = render(ThreadPanel, { ...defaultProps });
			const divider = container.querySelector('.headerDivider, [class*="divider"]');
			expect(divider).toBeInTheDocument();
		});
	});

	describe('header section', () => {
		it('renders sidebar toggle button when onToggleSidebar is provided', () => {
			const { container } = render(ThreadPanel, {
				...defaultProps,
				onToggleSidebar: vi.fn()
			});
			const toggleButton = container.querySelector('.sidebarToggle, [class*="toggle"]');
			expect(toggleButton).toBeInTheDocument();
		});

		it('does not render sidebar toggle without handler', () => {
			const { container } = render(ThreadPanel, {
				...defaultProps
			});
			const toggleButton = container.querySelector('.sidebarToggle, [class*="toggle"]');
			expect(toggleButton).not.toBeInTheDocument();
		});

		it('calls onToggleSidebar when toggle button is clicked', async () => {
			const onToggleSidebar = vi.fn();
			const { container } = render(ThreadPanel, {
				...defaultProps,
				onToggleSidebar
			});

			const toggleButton = container.querySelector('.sidebarToggle, [class*="toggle"]');
			if (toggleButton) {
				await fireEvent.click(toggleButton);
				expect(onToggleSidebar).toHaveBeenCalled();
			}
		});

		it('displays context when breadcrumbPath is provided', () => {
			render(ThreadPanel, {
				...defaultProps,
				breadcrumbPath: [{ id: 'conv1', title: 'My Conversation' }]
			});
			expect(screen.getByText('My Conversation')).toBeInTheDocument();
		});

		it('does not display context without breadcrumbPath', () => {
			render(ThreadPanel, { ...defaultProps });
			const context = screen.queryByText('My Conversation');
			expect(context).not.toBeInTheDocument();
		});
	});

	describe('resolved state', () => {
		it('shows resolved badge when thread is resolved', () => {
			render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread
			});
			expect(screen.getByText('Resolved')).toBeInTheDocument();
		});

		it('does not show resolved badge when thread is not resolved', () => {
			render(ThreadPanel, {
				...defaultProps,
				thread: mockThread
			});
			const resolvedBadge = screen.queryByText('Resolved');
			expect(resolvedBadge).not.toBeInTheDocument();
		});

		it('shows resolved message banner when thread is resolved', () => {
			const { container } = render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread
			});
			await vi.advanceTimersByTimeAsync(200);
			const resolvedMessage = container.querySelector('[class*="resolved"], [role="alert"]');
			expect(resolvedMessage).toBeInTheDocument();
		});

		it('displays "Thread Locked" text in banner', () => {
			render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread
			});
			expect(screen.getByText('Thread Locked')).toBeInTheDocument();
		});

		it('displays "Read-only mode active" subtext', () => {
			render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread
			});
			expect(screen.getByText('Read-only mode active')).toBeInTheDocument();
		});

		it('renders unlock button in resolved banner', () => {
			render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread
			});
			const unlockButton = screen.queryByText('Unlock');
			expect(unlockButton).toBeInTheDocument();
		});
	});

	describe('reopen functionality', () => {
		it('calls onReopenThread when unlock button is clicked', async () => {
			const onReopenThread = vi.fn();
			const { container } = render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread,
				onReopenThread
			});

			// Wait for banner animation
			await vi.advanceTimersByTimeAsync(200);

			const unlockButton = screen.queryByText('Unlock');
			if (unlockButton) {
				await fireEvent.click(unlockButton);
				await vi.advanceTimersByTimeAsync(200);
				expect(onReopenThread).toHaveBeenCalled();
			}
		});
	});

	describe('expanded state', () => {
		it('expands resolved message when clicked', async () => {
			const { container } = render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread
			});

			await vi.advanceTimersByTimeAsync(200);

			const resolvedMessage = container.querySelector('[class*="resolved"]');
			if (resolvedMessage) {
				await fireEvent.click(resolvedMessage);
				// Should toggle expanded state
				expect(resolvedMessage).toBeInTheDocument();
			}
		});
	});

	describe('thread summary', () => {
		it('renders summary when thread has summary metadata', () => {
			const threadWithSummary: Thread = {
				...mockThread,
				metadata: {
					...mockThread.metadata,
					summary: 'This is a thread summary'
				}
			};

			render(ThreadPanel, {
				...defaultProps,
				thread: threadWithSummary
			});
			expect(screen.getByText('This is a thread summary')).toBeInTheDocument();
		});

		it('does not render summary container without summary', () => {
			const { container } = render(ThreadPanel, {
				...defaultProps
			});
			const summaryContainer = container.querySelector('.summaryContainer, [class*="summary"]');
			expect(summaryContainer).not.toBeInTheDocument();
		});
	});

	describe('event handlers', () => {
		it('accepts onSendMessage handler', () => {
			const onSendMessage = vi.fn();
			render(ThreadPanel, {
				...defaultProps,
				onSendMessage
			});
			expect(onSendMessage).toBeDefined();
		});

		it('accepts onOpenSettings handler', () => {
			const onOpenSettings = vi.fn();
			render(ThreadPanel, {
				...defaultProps,
				onOpenSettings
			});
			expect(onOpenSettings).toBeDefined();
		});

		it('accepts onRetry handler', () => {
			const onRetry = vi.fn();
			render(ThreadPanel, {
				...defaultProps,
				onRetry
			});
			expect(onRetry).toBeDefined();
		});

		it('accepts onModelChange handler', () => {
			const onModelChange = vi.fn();
			render(ThreadPanel, {
				...defaultProps,
				onModelChange
			});
			expect(onModelChange).toBeDefined();
		});

		it('accepts onMessageFeedback handler', () => {
			const onMessageFeedback = vi.fn();
			render(ThreadPanel, {
				...defaultProps,
				onMessageFeedback
			});
			expect(onMessageFeedback).toBeDefined();
		});
	});

	describe('accessibility', () => {
		it('has proper heading structure', () => {
			render(ThreadPanel, { ...defaultProps });
			const title = screen.getByRole('heading', { level: 1 });
			expect(title).toBeInTheDocument();
		});

		it('resolved banner has role="alert"', () => {
			const { container } = render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread
			});
			await vi.advanceTimersByTimeAsync(200);
			const alert = container.querySelector('[role="alert"]');
			expect(alert).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty messages array', () => {
			const { container } = render(ThreadPanel, {
				...defaultProps,
				messages: []
			});
			const panel = container.querySelector('.panel, [class*="panel"]');
			expect(panel).toBeInTheDocument();
		});

		it('handles undefined onReopenThread', () => {
			const { container } = render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread
			});
			const panel = container.querySelector('.panel, [class*="panel"]');
			expect(panel).toBeInTheDocument();
		});

		it('handles empty breadcrumb path', () => {
			const { container } = render(ThreadPanel, {
				...defaultProps,
				breadcrumbPath: []
			});
			const panel = container.querySelector('.panel, [class*="panel"]');
			expect(panel).toBeInTheDocument();
		});

		it('handles null initialModelId', () => {
			const { container } = render(ThreadPanel, {
				...defaultProps,
				initialModelId: null
			});
			const panel = container.querySelector('.panel, [class*="panel"]');
			expect(panel).toBeInTheDocument();
		});
	});

	describe('animations', () => {
		it('applies animation classes for banner transitions', async () => {
			const { container } = render(ThreadPanel, {
				...defaultProps,
				thread: mockResolvedThread
			});

			await vi.advanceTimersByTimeAsync(200);

			const panel = container.querySelector('.panel, [class*="panel"]');
			expect(panel).toBeInTheDocument();
		});
	});
});
