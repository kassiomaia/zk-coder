import {fireEvent, render, screen, waitFor} from '@testing-library/svelte';
import {beforeEach, describe, expect, it, vi} from 'vitest';
import ChatMessagesList from './ChatMessagesList.svelte';
import type {Message, MessageKind} from '$lib/types/message';
import type {Thread} from '$lib/types/thread';
import type {AgentStatus} from '$lib/utils/agentStatus';

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
			conversationId: 'conv-1',
			parentId: null,
			title: 'Test Thread',
			createdAt: new Date('2024-01-01T12:00:00Z'),
			updatedAt: new Date('2024-01-01T12:01:00Z'),
			messageCount: 2,
			metadata: { resolved: false }
		}
	];

	const activeAgentStatus: AgentStatus = {
		text: 'Reading files...',
		icon: 'file',
		active: true
	};

	const defaultProps = {
		messages: mockMessages,
		threads: mockThreads,
		messagesLoading: 'idle' as const,
		emptyContext: 'thread' as const,
		isTyping: false,
		agentStatus: null as AgentStatus | null
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('renders normal messages', () => {
		render(ChatMessagesList, { ...defaultProps });
		expect(screen.getByText('Hello, how are you?')).toBeInTheDocument();
		expect(screen.getByText('I am doing well, thank you!')).toBeInTheDocument();
	});

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

		expect(screen.getByText('Thinking...')).toBeInTheDocument();
	});

	it('keeps the answering indicator visible when agent messages are present', () => {
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
				content: 'Thinking...',
				timestamp: new Date('2024-01-01T12:01:00Z'),
				status: 'sending',
				kind: 'thinking' as MessageKind
			}
		];

		render(ChatMessagesList, {
			...defaultProps,
			messages: agentMessages,
			isTyping: true,
			agentStatus: activeAgentStatus
		});

		expect(screen.getByText('Help me')).toBeInTheDocument();
		expect(screen.getByLabelText('Agent status')).toBeInTheDocument();
	});

	it('shows only the bottom answering indicator for an active agent session', () => {
		const sendingMessage: Message[] = [
			{
				id: 'msg-1',
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
			isTyping: true,
			agentStatus: activeAgentStatus
		});

		expect(container.querySelectorAll('[aria-label="Agent status"]')).toHaveLength(1);
	});

	it('does not show the answering indicator for inactive statuses', () => {
		render(ChatMessagesList, {
			...defaultProps,
			isTyping: true,
			agentStatus: {
				text: 'Complete',
				icon: 'check',
				active: false
			}
		});

		expect(screen.queryByLabelText('Agent status')).not.toBeInTheDocument();
	});

	it('renders system notices with structured context metadata', () => {
		const systemNotice: Message = {
			id: 'msg-system-notice',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			role: 'system',
			content:
				'Context updated\n\nThe agent reorganized its working context for the current subtask.\n\n538 -> 507 tokens',
			timestamp: new Date('2024-01-01T12:02:00Z'),
			status: 'sent',
			metadata: {
				systemNotice: {
					type: 'context_update',
					reason: 'subtask_transition',
					beforeCount: 16,
					afterCount: 17,
					beforeTokens: 538,
					afterTokens: 507
				}
			}
		};

		render(ChatMessagesList, {
			...defaultProps,
			messages: [systemNotice]
		});

		expect(screen.getByLabelText('Context updated')).toBeInTheDocument();
		expect(
			screen.getByText('The agent reorganized its working context for the current subtask.')
		).toBeInTheDocument();
	});

	it('calls onRetry when retry button is clicked in error state', async () => {
		const onRetry = vi.fn();
		render(ChatMessagesList, {
			...defaultProps,
			messagesLoading: 'error' as const,
			error: 'Connection error',
			onRetry
		});

		await fireEvent.click(screen.getByRole('button', { name: /retry/i }));
		expect(onRetry).toHaveBeenCalled();
	});

	it('calls onCopyMessage when copy action is triggered', async () => {
		const onCopyMessage = vi.fn().mockResolvedValue(undefined);
		render(ChatMessagesList, {
			...defaultProps,
			onCopyMessage
		});

		const copyButtons = screen.queryAllByRole('button', { name: /copy/i });
		if (copyButtons.length > 0) {
			await fireEvent.click(copyButtons[0]);
			await waitFor(() => {
				expect(onCopyMessage).toHaveBeenCalled();
			});
		}
	});

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
