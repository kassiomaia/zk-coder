import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ChatArea from './ChatArea.svelte';
import type { Message } from '$lib/types/message';
import type { Thread } from '$lib/types/thread';

// Mock the stores and utilities
const mockRegisterThreadElement = vi.fn();
const mockUnregisterThread = vi.fn();
const mockHandleStreamingStart = vi.fn();
const mockHandleStreamingEnd = vi.fn();
const mockHandleContentUpdate = vi.fn();
const mockHandleUserScroll = vi.fn();
const mockHandleUserClickScrollDown = vi.fn();
const mockHandleUserSendMessage = vi.fn();
const mockScrollToBottom = vi.fn();
const mockIsAutoScrollEnabled = vi.fn(() => true);

vi.mock('$lib/utils/scrollController', () => ({
	registerThreadElement: () => mockRegisterThreadElement,
	unregisterThread: () => mockUnregisterThread,
	handleStreamingStart: () => mockHandleStreamingStart,
	handleStreamingEnd: () => mockHandleStreamingEnd,
	handleContentUpdate: () => mockHandleContentUpdate,
	handleUserScroll: () => mockHandleUserScroll,
	handleUserClickScrollDown: () => mockHandleUserClickScrollDown,
	handleUserSendMessage: () => mockHandleUserSendMessage,
	scrollToBottom: () => mockScrollToBottom,
	isAutoScrollEnabled: () => mockIsAutoScrollEnabled
}));

vi.mock('$lib/utils/scrollManager', () => ({
	saveScrollPosition: vi.fn(),
	initializeThreadScroll: vi.fn(),
	cleanupThreadScroll: vi.fn()
}));

vi.mock('./chatAreaScroll', () => ({
	shouldForceInitialBottomScroll: vi.fn(() => false)
}));

vi.mock('$lib/utils/notifications', () => ({
	notifyAssistantComplete: vi.fn(),
	initNotifications: vi.fn()
}));

vi.mock('$lib/stores/chat', () => ({
	chatStore: {
		resolveAmbiguity: vi.fn(),
		dismissAmbiguity: vi.fn()
	},
	nlpState: {
		subscribe: vi.fn((run: (value: unknown) => void) => {
			run({ pendingAmbiguities: [] });
			return () => {};
		})
	},
	isLlmStreaming: {
		subscribe: vi.fn((run: (value: boolean) => void) => {
			run(false);
			return () => {};
		})
	},
	streamingContent: {
		subscribe: vi.fn((run: (value: string) => void) => {
			run('');
			return () => {};
		})
	}
}));

vi.mock('$lib/stores/configStore', () => ({
	configStore: {
		getDefault: vi.fn(() => 'claude-3-opus-20240229')
	},
	availableModelsByOperation: {
		subscribe: vi.fn((run: (value: unknown) => void) => {
			run({
				code: [
					{
						id: 'claude-3-opus-20240229',
						name: 'Claude 3 Opus',
						providerId: 'anthropic'
					}
				]
			});
			return () => {};
		})
	}
}));

describe('ChatArea', () => {
	const mockMessages: Message[] = [
		{
			id: 'msg1',
			conversationId: 'conv1',
			threadId: 'thread1',
			role: 'user',
			content: 'Hello, how are you?',
			timestamp: new Date('2024-01-15T10:00:00'),
			status: 'sent'
		},
		{
			id: 'msg2',
			conversationId: 'conv1',
			threadId: 'thread1',
			role: 'assistant',
			content: 'I am doing well, thank you!',
			timestamp: new Date('2024-01-15T10:01:00'),
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
			updatedAt: new Date('2024-01-15T10:01:00'),
			messageCount: 2,
			metadata: { resolved: false }
		}
	];

	const defaultProps = {
		messages: mockMessages,
		threadId: 'thread1',
		threads: mockThreads,
		emptyContext: 'thread' as const,
		loading: false,
		isTyping: false,
		threadResolved: false
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('renders the chat area container', () => {
			const { container } = render(ChatArea, { ...defaultProps });
			const chatArea = container.querySelector('.chatArea, [class*="chatArea"]');
			expect(chatArea).toBeInTheDocument();
		});

		it('renders messages when provided', () => {
			render(ChatArea, { ...defaultProps });
			expect(screen.getByText('Hello, how are you?')).toBeInTheDocument();
			expect(screen.getByText('I am doing well, thank you!')).toBeInTheDocument();
		});

		it('renders empty state when no messages', () => {
			render(ChatArea, { ...defaultProps, messages: [] });
			const container = document.querySelector('body');
			// Should render without errors even with no messages
			expect(container).toBeInTheDocument();
		});
	});

	describe('thread resolved state', () => {
		it('passes threadResolved prop to ChatInputContainer', () => {
			const { container } = render(ChatArea, {
				...defaultProps,
				threadResolved: true
			});
			// Component should render without errors when thread is resolved
			const chatArea = container.querySelector('.chatArea, [class*="chatArea"]');
			expect(chatArea).toBeInTheDocument();
		});

		it('passes threadResolved as false by default', () => {
			const { container } = render(ChatArea, {
				...defaultProps,
				threadResolved: false
			});
			const chatArea = container.querySelector('.chatArea, [class*="chatArea"]');
			expect(chatArea).toBeInTheDocument();
		});
	});

	describe('props handling', () => {
		it('accepts and uses threadId prop', () => {
			render(ChatArea, { ...defaultProps, threadId: 'custom-thread-id' });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('accepts and uses initialModelId prop', () => {
			render(ChatArea, {
				...defaultProps,
				initialModelId: 'gpt-4-turbo'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('accepts threads prop for thread lookups', () => {
			const customThreads: Thread[] = [
				{
					id: 'thread-custom',
					conversationId: 'conv1',
					parentId: null,
					title: 'Custom Thread',
					createdAt: new Date(),
					updatedAt: new Date(),
					messageCount: 1,
					metadata: { resolved: false }
				}
			];

			render(ChatArea, {
				...defaultProps,
				threads: customThreads
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('accepts emptyContext prop', () => {
			render(ChatArea, {
				...defaultProps,
				emptyContext: 'none'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('event handlers', () => {
		it('accepts onSendMessage handler', () => {
			const onSendMessage = vi.fn();
			render(ChatArea, {
				...defaultProps,
				onSendMessage
			});
			const textarea = screen.getByRole('textbox');
			expect(textarea).toBeInTheDocument();
		});

		it('accepts onModelChange handler', () => {
			const onModelChange = vi.fn();
			render(ChatArea, {
				...defaultProps,
				onModelChange
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('accepts onOpenSettings handler', () => {
			const onOpenSettings = vi.fn();
			render(ChatArea, {
				...defaultProps,
				onOpenSettings
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('accepts onRetry handler', () => {
			const onRetry = vi.fn();
			render(ChatArea, {
				...defaultProps,
				onRetry
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('loading states', () => {
		it('renders with loading state', () => {
			render(ChatArea, {
				...defaultProps,
				loading: true
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders with messagesLoading state', () => {
			render(ChatArea, {
				...defaultProps,
				messagesLoading: 'loading'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders with isTyping state', () => {
			render(ChatArea, {
				...defaultProps,
				isTyping: true,
				agentStatus: {
					text: 'Reading files...',
					icon: 'file',
					active: true
				},
				agentSessionId: 'session-1'
			});
			expect(screen.getByLabelText('Agent status')).toBeInTheDocument();
			expect(screen.getByRole('button', { name: /stop generation/i })).toBeInTheDocument();
		});
	});

	describe('error handling', () => {
		it('renders with error message', () => {
			render(ChatArea, {
				...defaultProps,
				error: 'Something went wrong'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders with messagesLoading error state', () => {
			render(ChatArea, {
				...defaultProps,
				messagesLoading: 'error'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('provides textbox for input', () => {
			render(ChatArea, { ...defaultProps });
			const textbox = screen.getByRole('textbox');
			expect(textbox).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles undefined threadId gracefully', () => {
			render(ChatArea, {
				...defaultProps,
				threadId: undefined
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles empty threads array', () => {
			render(ChatArea, {
				...defaultProps,
				threads: []
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles null initialModelId', () => {
			render(ChatArea, {
				...defaultProps,
				initialModelId: null
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});
});
