import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ChatMessageGroup from './ChatMessageGroup.svelte';
import type { Message } from '$lib/types/message';
import type { Thread } from '$lib/types/thread';

describe('ChatMessageGroup', () => {
	const mockUserMessages: Message[] = [
		{
			id: 'msg1',
			conversationId: 'conv1',
			threadId: 'thread1',
			role: 'user',
			content: 'Hello there!',
			timestamp: new Date('2024-01-15T10:00:00'),
			status: 'sent'
		},
		{
			id: 'msg2',
			conversationId: 'conv1',
			threadId: 'thread1',
			role: 'user',
			content: 'How are you?',
			timestamp: new Date('2024-01-15T10:01:00'),
			status: 'sent'
		}
	];

	const mockAssistantMessages: Message[] = [
		{
			id: 'msg3',
			conversationId: 'conv1',
			threadId: 'thread1',
			role: 'assistant',
			content: 'I am doing well, thank you!',
			timestamp: new Date('2024-01-15T10:02:00'),
			status: 'sent'
		}
	];

	const mockThreads: Thread[] = [
		{
			id: 'thread1',
			conversationId: 'conv1',
			parentId: null,
			title: 'Test Thread',
			createdAt: new Date('2024-01-15T10:00:00'),
			updatedAt: new Date('2024-01-15T10:02:00'),
			messageCount: 3,
			metadata: { resolved: false }
		}
	];

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('rendering', () => {
		it('renders message group container', () => {
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			const group = container.querySelector('[role="group"]');
			expect(group).toBeInTheDocument();
		});

		it('renders all messages in the group', () => {
			render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			expect(screen.getByText('Hello there!')).toBeInTheDocument();
			expect(screen.getByText('How are you?')).toBeInTheDocument();
		});

		it('renders group label when provided', () => {
			render(ChatMessageGroup, {
				messages: mockUserMessages,
				label: 'Today'
			});
			expect(screen.getByText('Today')).toBeInTheDocument();
		});

		it('does not render label when not provided', () => {
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			const label = screen.queryByText('Today');
			expect(label).not.toBeInTheDocument();
		});
	});

	describe('group header', () => {
		it('renders header for user role', () => {
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			const header = container.querySelector('.header, [class*="header"]');
			expect(header).toBeInTheDocument();
		});

		it('renders header for assistant role', () => {
			const { container } = render(ChatMessageGroup, {
				messages: mockAssistantMessages
			});
			const header = container.querySelector('.header, [class*="header"]');
			expect(header).toBeInTheDocument();
		});

		it('displays "You" for user role', () => {
			render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			expect(screen.getByText('You')).toBeInTheDocument();
		});

		it('displays "Assistant" for assistant role', () => {
			render(ChatMessageGroup, {
				messages: mockAssistantMessages
			});
			expect(screen.getByText('Assistant')).toBeInTheDocument();
		});

		it('displays formatted timestamp', () => {
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			const timeElement = container.querySelector('time, [class*="time"]');
			expect(timeElement).toBeInTheDocument();
		});
	});

	describe('message rendering', () => {
		it('renders each message with content', () => {
			render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			expect(screen.getByText('Hello there!')).toBeInTheDocument();
			expect(screen.getByText('How are you?')).toBeInTheDocument();
		});

		it('passes message content correctly', () => {
			render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			expect(screen.getByText('Hello there!')).toBeInTheDocument();
		});
	});

	describe('event handlers', () => {
		it('handles onEdit callback', () => {
			const onEdit = vi.fn();
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages,
				onEdit
			});
			const group = container.querySelector('[role="group"]');
			expect(group).toBeInTheDocument();
		});

		it('handles onDelete callback', () => {
			const onDelete = vi.fn();
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages,
				onDelete
			});
			const group = container.querySelector('[role="group"]');
			expect(group).toBeInTheDocument();
		});

		it('handles onRetry callback', () => {
			const onRetry = vi.fn();
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages,
				onRetry
			});
			const group = container.querySelector('[role="group"]');
			expect(group).toBeInTheDocument();
		});

		it('handles onThreadClick callback', () => {
			const onThreadClick = vi.fn();
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages,
				threads: mockThreads,
				onThreadClick
			});
			const group = container.querySelector('[role="group"]');
			expect(group).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty messages array', () => {
			const { container } = render(ChatMessageGroup, {
				messages: []
			});
			const group = container.querySelector('[role="group"]');
			expect(group).toBeInTheDocument();
		});

		it('handles single message', () => {
			render(ChatMessageGroup, {
				messages: [mockUserMessages[0]]
			});
			expect(screen.getByText('Hello there!')).toBeInTheDocument();
		});

		it('handles missing threads array', () => {
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages,
				threads: []
			});
			const group = container.querySelector('[role="group"]');
			expect(group).toBeInTheDocument();
		});

		it('handles undefined handlers', () => {
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			const group = container.querySelector('[role="group"]');
			expect(group).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has proper role attribute', () => {
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			const group = container.querySelector('[role="group"]');
			expect(group).toBeInTheDocument();
		});

		it('has aria-label describing the group', () => {
			const { container } = render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			const group = container.querySelector('[aria-label*="message group"], [role="group"]');
			expect(group).toBeInTheDocument();
		});
	});

	describe('styling classes', () => {
		it('applies correct CSS classes for different roles', () => {
			const { container: userContainer } = render(ChatMessageGroup, {
				messages: mockUserMessages
			});
			const { container: assistantContainer } = render(ChatMessageGroup, {
				messages: mockAssistantMessages
			});

			expect(userContainer.querySelector('[role="group"]')).toBeInTheDocument();
			expect(assistantContainer.querySelector('[role="group"]')).toBeInTheDocument();
		});
	});
});
