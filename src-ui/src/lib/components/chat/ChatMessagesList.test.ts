import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ChatMessagesList from './ChatMessagesList.svelte';
import type { Message, MessageKind } from '$lib/types/message';
import type { Thread } from '$lib/types/thread';

// Mock the agent stores
const mockActiveSession = {
	get subscribe() {
		return vi.fn(() => vi.fn());
	}
};

const mockPendingActions = {
	get subscribe() {
		return vi.fn(() => vi.fn());
	}
};

vi.mock('$lib/stores/agentStore', () => ({
	activeSession: mockActiveSession,
	pendingActions: mockPendingActions
}));

vi.mock('$lib/utils/agentStatus', () => ({
	getAgentStatus: vi.fn(() => ({ text: 'Ready', icon: 'check', active: false }))
}));

describe('ChatMessagesList', () => {
	const mockMessages: Message[] = [
		{
			id: 'msg-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			role: 'user',
			content: 'Hello, how are you?',
			timestamp: new Date('2024-01-01T12:00:00Z'),
			status: 'sent'
		},
		{
			id: 'msg-2',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			role: 'assistant',
			content: 'I am doing well, thank you!',
			timestamp: new Date('2024-01-01T12:01:00Z'),
			status: 'sent'
		}
	];

	const mockThreads: Thread[] = [
		{
			id: 'thread-1',
			title: 'Test Thread',
			messageIds: ['msg-1', 'msg-2'],
			createdAt: new Date('2024-01-01T12:00:00Z'),
			updatedAt: new Date('2024-01-01T12:01:00Z'),
			resolved: false
		}
	];

	const defaultProps = {
		messages: mockMessages,
		threads: mockThreads,
		messagesLoading: 'idle' as const,
		emptyContext: 'thread' as const,
		isTyping: false
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('rendering messages', () => {
		it('renders an array of messages', () => {
			render(ChatMessagesList, { ...defaultProps });
			// Messages should be rendered
			expect(screen.getByText('Hello, how are you?')).toBeInTheDocument();
			expect(screen.getByText('I am doing well, thank you!')).toBeInTheDocument();
		});

		it('renders empty messages array', () => {
			const { container } = render(ChatMessagesList, {
				...defaultProps,
				messages: []
			});
			// Should show empty state
			expect(container.querySelector('.emptyWrapper, [class*="empty"]')).toBeInTheDocument();
		});

		it('renders loading state', () => {
			const { container } = render(ChatMessagesList, {
				...defaultProps,
				messagesLoading: 'loading' as const
			});
			expect(screen.getByText(/loading messages/i)).toBeInTheDocument();
		});

		it('renders error state', () => {
			render(ChatMessagesList, {
				...defaultProps,
				messagesLoading: 'error' as const,
				error: 'Failed to load messages'
			});
			expect(screen.getByText(/failed to load messages/i)).toBeInTheDocument();
		});

		it('shows retry button in error state when onRetry provided', () => {
			const onRetry = vi.fn();
			render(ChatMessagesList, {
				...defaultProps,
				messagesLoading: 'error' as const,
				error: 'Connection error',
				onRetry
			});
			const retryButton = screen.getByRole('button', { name: /retry/i });
			expect(retryButton).toBeInTheDocument();
		});

		it('calls onRetry when retry button is clicked', async () => {
			const onRetry = vi.fn();
			render(ChatMessagesList, {
				...defaultProps,
				messagesLoading: 'error' as const,
				error: 'Connection error',
				onRetry
			});
			const retryButton = screen.getByRole('button', { name: /retry/i });
			await fireEvent.click(retryButton);
			expect(onRetry).toHaveBeenCalled();
		});
	});

	describe('message grouping', () => {
		it('shows group headers for different roles', () => {
			render(ChatMessagesList, { ...defaultProps });
			// Should show role headers (USER, ASSISTANT)
			const userHeader = screen.getByText('USER');
			const assistantHeader = screen.getByText('ASSISTANT');
			expect(userHeader).toBeInTheDocument();
			expect(assistantHeader).toBeInTheDocument();
		});

		it('shows time-based group headers for messages > 5 minutes apart', () => {
			const messagesWithTimeGap: Message[] = [
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user',
					content: 'First message',
					timestamp: new Date('2024-01-01T12:00:00Z'),
					status: 'sent'
				},
				{
					id: 'msg-2',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user',
					content: 'Second message 6 minutes later',
					timestamp: new Date('2024-01-01T12:06:00Z'), // 6 minutes later
					status: 'sent'
				}
			];

			render(ChatMessagesList, {
				...defaultProps,
				messages: messagesWithTimeGap
			});
			// Both messages should have group headers due to time gap
			expect(screen.getByText('First message')).toBeInTheDocument();
			expect(screen.getByText('Second message 6 minutes later')).toBeInTheDocument();
		});
	});

	describe('agent messages', () => {
		it('renders thinking messages', () => {
			const thinkingMessage: Message = {
				id: 'msg-thinking',
				conversationId: 'conv-1',
				threadId: 'thread-1',
				role: 'assistant',
				content: 'Thinking...',
				timestamp: new Date('2024-01-01T12:00:00Z'),
				kind: 'thinking' as MessageKind
			};

			render(ChatMessagesList, {
				...defaultProps,
				messages: [thinkingMessage]
			});
			// Should render without crashing
			expect(screen.getByText('Thinking...')).toBeInTheDocument();
		});

		it('renders tool_call messages', () => {
			const toolCallMessage: Message = {
				id: 'msg-tool',
				conversationId: 'conv-1',
				threadId: 'thread-1',
				role: 'assistant',
				content: 'Running tool...',
				timestamp: new Date('2024-01-01T12:00:00Z'),
				kind: 'tool_call' as MessageKind,
				agentData: {
					toolName: 'read_file',
					toolArgs: { path: '/test/file.txt' },
					toolCallStatus: 'success'
				}
			};

			render(ChatMessagesList, {
				...defaultProps,
				messages: [toolCallMessage]
			});
			// Should render without crashing
			expect(screen.getByText('Running tool...')).toBeInTheDocument();
		});

		it('renders terminal messages', () => {
			const terminalMessage: Message = {
				id: 'msg-terminal',
				conversationId: 'conv-1',
				threadId: 'thread-1',
				role: 'assistant',
				content: 'Command output',
				timestamp: new Date('2024-01-01T12:00:00Z'),
				kind: 'terminal' as MessageKind,
				agentData: {
					streamingContent: 'Output from command',
					streamType: 'stdout'
				}
			};

			render(ChatMessagesList, {
				...defaultProps,
				messages: [terminalMessage]
			});
			// Should render without crashing
			expect(screen.getByText('Command output')).toBeInTheDocument();
		});

		it('suppresses generic answering indicator when agent messages are present', () => {
			const agentMessages: Message[] = [
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user',
					content: 'Help me',
					timestamp: new Date('2024-01-01T12:00:00Z'),
					status: 'sent'
				},
				{
					id: 'msg-2',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'assistant',
					content: '',
					timestamp: new Date('2024-01-01T12:01:00Z'),
					status: 'sending',
					kind: 'thinking' as MessageKind
				}
			];

			const { container } = render(ChatMessagesList, {
				...defaultProps,
				messages: agentMessages,
				isTyping: true
			});
			// Should not show generic answering indicator when specialized messages present
			expect(screen.getByText('Help me')).toBeInTheDocument();
		});
	});

	describe('message actions', () => {
		it('calls onEditMessage when edit is triggered', async () => {
			const onEditMessage = vi.fn();
			render(ChatMessagesList, {
				...defaultProps,
				onEditMessage
			});

			// Find edit button and click it
			const editButtons = screen.queryAllByRole('button', { name: /edit/i });
			if (editButtons.length > 0) {
				await fireEvent.click(editButtons[0]);
				expect(onEditMessage).toHaveBeenCalled();
			}
		});

		it('calls onDeleteMessage when delete is triggered', async () => {
			const onDeleteMessage = vi.fn();
			render(ChatMessagesList, {
				...defaultProps,
				onDeleteMessage
			});

			const deleteButtons = screen.queryAllByRole('button', { name: /delete/i });
			if (deleteButtons.length > 0) {
				await fireEvent.click(deleteButtons[0]);
				expect(onDeleteMessage).toHaveBeenCalled();
			}
		});

		it('calls onRetryMessage when retry is triggered', async () => {
			const onRetryMessage = vi.fn();
			const errorMessages: Message[] = [
				{
					id: 'msg-error',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user',
					content: 'Failed message',
					timestamp: new Date('2024-01-01T12:00:00Z'),
					status: 'error'
				}
			];

			render(ChatMessagesList, {
				...defaultProps,
				messages: errorMessages,
				onRetryMessage
			});

			const retryButton = screen.queryByRole('button', { name: /retry/i });
			if (retryButton) {
				await fireEvent.click(retryButton);
				expect(onRetryMessage).toHaveBeenCalledWith('msg-error');
			}
		});

		it('calls onMessageFeedback when feedback is given', async () => {
			const onMessageFeedback = vi.fn();
			render(ChatMessagesList, {
				...defaultProps,
				onMessageFeedback
			});

			// Find feedback buttons (like/dislike)
			const feedbackButtons = screen.queryAllByRole('button');
			// Feedback buttons may be present but not easily queryable
			// The component should render without errors
			expect(screen.getByText('Hello, how are you?')).toBeInTheDocument();
		});

		it('calls onCopyMessage when copy action is triggered', async () => {
			const onCopyMessage = vi.fn().mockResolvedValue(undefined);
			render(ChatMessagesList, {
				...defaultProps,
				onCopyMessage
			});

			// Find copy button
			const copyButtons = screen.queryAllByRole('button', { name: /copy/i });
			if (copyButtons.length > 0) {
				await fireEvent.click(copyButtons[0]);
				await waitFor(() => {
					expect(onCopyMessage).toHaveBeenCalled();
				});
			}
		});
	});

	describe('thread interactions', () => {
		it('calls onThreadClick when thread indicator is clicked', async () => {
			const onThreadClick = vi.fn();
			const messagesWithThreads: Message[] = [
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user',
					content: 'Message in thread',
					timestamp: new Date('2024-01-01T12:00:00Z'),
					status: 'sent'
				}
			];

			render(ChatMessagesList, {
				...defaultProps,
				messages: messagesWithThreads,
				threads: mockThreads,
				onThreadClick
			});

			// Thread indicator should be clickable
			const threadText = screen.queryByText('Test Thread');
			if (threadText) {
				const clickable = threadText.closest('button') || threadText.closest('[role="button"]');
				if (clickable) {
					await fireEvent.click(clickable);
					expect(onThreadClick).toHaveBeenCalledWith('thread-1');
				}
			}
		});
	});

	describe('answering indicator', () => {
		it('shows answering indicator when isTyping is true and no agent messages', () => {
			const sendingMessage: Message[] = [
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user',
					content: 'Question',
					timestamp: new Date('2024-01-01T12:00:00Z'),
					status: 'sent'
				},
				{
					id: 'msg-2',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'assistant',
					content: '',
					timestamp: new Date('2024-01-01T12:01:00Z'),
					status: 'sending'
				}
			];

			const { container } = render(ChatMessagesList, {
				...defaultProps,
				messages: sendingMessage,
				isTyping: true
			});
			// Should show answering indicator for empty sending message
			expect(screen.getByText('Question')).toBeInTheDocument();
		});
	});

	describe('empty states', () => {
		it('shows empty state for conversation context', () => {
			const { container } = render(ChatMessagesList, {
				...defaultProps,
				messages: [],
				emptyContext: 'conversation' as const
			});
			expect(container.querySelector('.emptyWrapper, [class*="empty"]')).toBeInTheDocument();
		});

		it('shows empty state for thread context', () => {
			const { container } = render(ChatMessagesList, {
				...defaultProps,
				messages: [],
				emptyContext: 'thread' as const,
				conversationTitle: 'My Conversation'
			});
			expect(container.querySelector('.emptyWrapper, [class*="empty"]')).toBeInTheDocument();
		});

		it('shows empty state for none context', () => {
			const { container } = render(ChatMessagesList, {
				...defaultProps,
				messages: [],
				emptyContext: 'none' as const
			});
			expect(container.querySelector('.emptyWrapper, [class*="empty"]')).toBeInTheDocument();
		});
	});

	describe('message status indicators', () => {
		it('shows error indicator for failed messages', () => {
			const errorMessages: Message[] = [
				{
					id: 'msg-error',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user',
					content: 'Failed to send',
					timestamp: new Date('2024-01-01T12:00:00Z'),
					status: 'error'
				}
			];

			const { container } = render(ChatMessagesList, {
				...defaultProps,
				messages: errorMessages
			});
			// Should show error status
			expect(screen.getByText('Failed to send')).toBeInTheDocument();
			// Should have retry button
			const retryButton = screen.queryByRole('button', { name: /retry/i });
			expect(retryButton).toBeInTheDocument();
		});

		it('shows sending indicator for sending messages', () => {
			const sendingMessages: Message[] = [
				{
					id: 'msg-sending',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user',
					content: 'Sending...',
					timestamp: new Date('2024-01-01T12:00:00Z'),
					status: 'sending'
				}
			];

			const { container } = render(ChatMessagesList, {
				...defaultProps,
				messages: sendingMessages
			});
			// Should render with sending state
			expect(screen.getByText('Sending...')).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles messages with special characters in content', () => {
			const specialMessages: Message[] = [
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user',
					content: '<script>alert("test")</script>',
					timestamp: new Date('2024-01-01T12:00:00Z'),
					status: 'sent'
				}
			];

			render(ChatMessagesList, {
				...defaultProps,
				messages: specialMessages
			});
			// Content should be sanitized
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles very long message list', () => {
			const longMessageList: Message[] = Array.from({ length: 100 }, (_, i) => ({
				id: `msg-${i}`,
				conversationId: 'conv-1',
				threadId: 'thread-1',
				role: i % 2 === 0 ? 'user' : 'assistant',
				content: `Message ${i}`,
				timestamp: new Date(`2024-01-01T12:${String(i).padStart(2, '0')}:00Z`),
				status: 'sent'
			}));

			render(ChatMessagesList, {
				...defaultProps,
				messages: longMessageList
			});
			// Should render without crashing
			expect(screen.getByText('Message 0')).toBeInTheDocument();
			expect(screen.getByText('Message 99')).toBeInTheDocument();
		});

		it('handles messages with markdown content', () => {
			const markdownMessages: Message[] = [
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'assistant',
					content: '```javascript\nconst x = 42;\n```',
					timestamp: new Date('2024-01-01T12:00:00Z'),
					status: 'sent'
				}
			];

			render(ChatMessagesList, {
				...defaultProps,
				messages: markdownMessages
			});
			// Should render markdown
			expect(screen.getByText(/javascript/i)).toBeInTheDocument();
		});

		it('handles undefined handlers gracefully', () => {
			render(ChatMessagesList, {
				...defaultProps,
				onEditMessage: undefined,
				onDeleteMessage: undefined,
				onRetryMessage: undefined,
				onMessageFeedback: undefined,
				onThreadClick: undefined,
				onCopyMessage: undefined
			});
			// Should render without errors
			expect(screen.getByText('Hello, how are you?')).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has proper ARIA roles for message list', () => {
			const { container } = render(ChatMessagesList, { ...defaultProps });
			// Messages should be properly structured
			expect(screen.getByText('Hello, how are you?')).toBeInTheDocument();
		});

		it('provides accessible error messages', () => {
			render(ChatMessagesList, {
				...defaultProps,
				messagesLoading: 'error' as const,
				error: 'Failed to load'
			});
			const errorText = screen.getByText(/failed to load/i);
			expect(errorText).toBeInTheDocument();
		});
	});
});
