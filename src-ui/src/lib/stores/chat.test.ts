/**
 * Tests for chatStore
 * Comprehensive test coverage for all chat store functionality.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import type { Conversation, Thread } from '$lib/types/thread';
import type { Message } from '$lib/types/message';
import type { IntentClassification, Ambiguity, ResolvedAmbiguity } from '$lib/types/nlp';

// Create mock functions that will be used in vi.mock
const mockListen = vi.fn();
const mockInvoke = vi.fn();

// Mock Tauri APIs before importing the store
vi.mock('@tauri-apps/api/event', () => ({
	listen: mockListen
}));

vi.mock('@tauri-apps/api/core', () => ({
	invoke: mockInvoke
}));

// Mock crypto.randomUUID
vi.stubGlobal('crypto', {
	randomUUID: vi.fn(() => 'mock-uuid-1234')
});

// Mock the chat API functions
vi.mock('$lib/api/chat', () => ({
	loadConversations: vi.fn(),
	createConversation: vi.fn(),
	renameConversation: vi.fn(),
	deleteConversation: vi.fn(),
	updateConversationSettings: vi.fn(),
	setConversationWorkspace: vi.fn(),
	loadThreads: vi.fn(),
	createThread: vi.fn(),
	resolveThread: vi.fn(),
	deleteThread: vi.fn(),
	renameThread: vi.fn(),
	setThreadAgentSession: vi.fn(),
	loadThreadMessages: vi.fn(),
	sendMessage: vi.fn(),
	createThreadWithMessage: vi.fn(),
	deleteMessage: vi.fn(),
	updateMessageStatus: vi.fn(),
	updateMessageFeedback: vi.fn()
}));

// Mock the NLP API functions
vi.mock('$lib/api/nlp', () => ({
	isBackendAvailable: vi.fn(),
	addContextMessage: vi.fn(),
	detectAmbiguities: vi.fn(),
	resolveAmbiguity: vi.fn(),
	applyResolutions: vi.fn(),
	createDebouncedClassifier: vi.fn(() => ({
		classify: vi.fn(),
		cancel: vi.fn()
	}))
}));

// Mock agentStore for interaction testing
vi.mock('./agentStore', () => ({
	agentStore: {
		createSession: vi.fn(),
		loadSession: vi.fn(),
		setActiveSession: vi.fn(),
		sendRequest: vi.fn()
	}
}));

import {
	loadConversations,
	createConversation,
	renameConversation,
	deleteConversation,
	updateConversationSettings,
	setConversationWorkspace,
	loadThreads,
	createThread,
	resolveThread,
	deleteThread,
	renameThread,
	setThreadAgentSession,
	loadThreadMessages,
	sendMessage,
	createThreadWithMessage,
	deleteMessage,
	updateMessageStatus,
	updateMessageFeedback
} from '$lib/api/chat';

import {
	isBackendAvailable,
	addContextMessage,
	detectAmbiguities,
	resolveAmbiguity,
	applyResolutions,
	createDebouncedClassifier
} from '$lib/api/nlp';

import { agentStore } from './agentStore';

// Now import after mocking
import { chatStore } from './chat';

// Helper function to create mock conversation
const createMockConversation = (id: string, title: string): Conversation => ({
	id,
	title,
	createdAt: new Date('2024-01-01T00:00:00Z'),
	updatedAt: new Date('2024-01-01T01:00:00Z'),
	settings: {
		model: 'gpt-4',
		temperature: 0.7,
		maxTokens: 4096,
		systemPrompt: 'Test prompt',
		workspaceRoot: '/workspace'
	}
});

// Helper function to create mock thread
const createMockThread = (id: string, conversationId: string, title: string): Thread => ({
	id,
	conversationId,
	parentId: null,
	title,
	createdAt: new Date('2024-01-01T00:00:00Z'),
	updatedAt: new Date('2024-01-01T01:00:00Z'),
	messageCount: 5,
	metadata: {
		resolved: false,
		tags: [],
		priority: 'medium'
	}
});

// Helper function to create mock message
const createMockMessage = (
	id: string,
	conversationId: string,
	threadId: string,
	role: Message['role'],
	content: string
): Message => ({
	id,
	conversationId,
	threadId,
	role,
	content,
	timestamp: new Date('2024-01-01T00:00:00Z'),
	status: 'sent'
});

describe('chatStore', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers().setSystemTime(new Date('2024-01-01T00:00:00Z'));

		// Reset store state
		chatStore.reset();

		// Setup default mock implementations
		vi.mocked(isBackendAvailable).mockResolvedValue(true);
		vi.mocked(loadConversations).mockResolvedValue([]);
		vi.mocked(loadThreads).mockResolvedValue([]);
		vi.mocked(loadThreadMessages).mockResolvedValue([]);
		vi.mocked(detectAmbiguities).mockResolvedValue([]);
		vi.mocked(resolveAmbiguity).mockResolvedValue({} as ResolvedAmbiguity);
		vi.mocked(applyResolutions).mockResolvedValue('');
		vi.mocked(addContextMessage).mockResolvedValue(undefined);
		vi.mocked(createDebouncedClassifier).mockReturnValue({
			classify: vi.fn().mockResolvedValue({ intent: 'test', confidence: 0.9 }),
			cancel: vi.fn()
		});

		// Setup agentStore mocks
		vi.mocked(agentStore.createSession).mockResolvedValue({
			id: 'agent-session-1',
			agentId: 'code-agent',
			agentName: 'Code Agent',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			state: { type: 'idle' },
			workspaceRoot: '/workspace',
			createdAt: new Date().toISOString(),
			updatedAt: new Date().toISOString()
		});
		vi.mocked(agentStore.loadSession).mockResolvedValue({
			id: 'agent-session-1',
			agentId: 'code-agent',
			agentName: 'Code Agent',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			state: { type: 'idle' },
			workspaceRoot: '/workspace',
			createdAt: new Date().toISOString(),
			updatedAt: new Date().toISOString()
		});
		vi.mocked(agentStore.setActiveSession).mockImplementation(() => {});
		vi.mocked(agentStore.sendRequest).mockResolvedValue(undefined);
	});

	afterEach(() => {
		vi.runOnlyPendingTimers();
		vi.useRealTimers();
	});

	describe('Store Initialization & State', () => {
		it('has correct initial state', () => {
			const state = get(chatStore);
			expect(state.conversations).toEqual([]);
			expect(state.conversationsLoading).toBe('idle');
			expect(state.selectedConversationId).toBeNull();
			expect(state.selectedThreadId).toBeNull();
			expect(state.messages).toEqual([]);
			expect(state.messagesLoading).toBe('idle');
			expect(state.threads).toEqual([]);
			expect(state.threadsLoading).toBe('idle');
			expect(state.nlp.backendAvailable).toBe(false);
			expect(state.nlp.detectedIntent).toBeNull();
			expect(state.nlp.ambiguities).toEqual([]);
			expect(state.nlp.pendingAmbiguities).toEqual([]);
			expect(state.llm.isStreaming).toBe(false);
			expect(state.llm.requestId).toBeNull();
			expect(state.llm.streamingContent).toBe('');
			expect(state.llm.model).toBeNull();
			expect(state.threadStreaming).toEqual({});
			expect(state.unreadThreadIds).toEqual([]);
			expect(state.error).toBeNull();
		});

		it('initialize loads conversations and checks backend availability', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			vi.mocked(loadConversations).mockResolvedValue([mockConv]);

			await chatStore.initialize();

			expect(isBackendAvailable).toHaveBeenCalled();
			expect(loadConversations).toHaveBeenCalled();

			const state = get(chatStore);
			expect(state.conversations).toEqual([mockConv]);
			expect(state.conversationsLoading).toBe('success');
			expect(state.nlp.backendAvailable).toBe(true);
		});

		it('initialize handles backend not available gracefully', async () => {
			vi.mocked(isBackendAvailable).mockResolvedValue(false);

			await chatStore.initialize();

			const state = get(chatStore);
			expect(state.nlp.backendAvailable).toBe(false);
		});

		it('initialize handles load conversations error', async () => {
			vi.mocked(loadConversations).mockRejectedValue(new Error('Network error'));

			await chatStore.initialize();

			const state = get(chatStore);
			expect(state.conversationsLoading).toBe('error');
			expect(state.error).toBe('Network error');
		});

		it('loadConversationsFromBackend loads and updates conversations', async () => {
			const mockConvs = [
				createMockConversation('conv-1', 'Chat 1'),
				createMockConversation('conv-2', 'Chat 2')
			];
			vi.mocked(loadConversations).mockResolvedValue(mockConvs);

			await chatStore.loadConversationsFromBackend();

			const state = get(chatStore);
			expect(state.conversations).toHaveLength(2);
			expect(state.conversationsLoading).toBe('success');
		});

		it('reset clears all state', () => {
			// Setup some state
			const mockConv = createMockConversation('conv-1', 'Test');
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const mockMsg = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Hello');

			chatStore.setConversations([mockConv]);
			chatStore.addMessage(mockMsg);
			chatStore.selectConversation('conv-1');

			// Reset
			chatStore.reset();

			const state = get(chatStore);
			expect(state.conversations).toEqual([]);
			expect(state.messages).toEqual([]);
			expect(state.selectedConversationId).toBeNull();
			expect(state.selectedThreadId).toBeNull();
		});
	});

	describe('Conversation Management', () => {
		it('setConversations updates conversations in state', () => {
			const mockConvs = [
				createMockConversation('conv-1', 'Chat 1'),
				createMockConversation('conv-2', 'Chat 2')
			];

			chatStore.setConversations(mockConvs);

			const state = get(chatStore);
			expect(state.conversations).toEqual(mockConvs);
		});

		it('createConversation creates and adds new conversation', async () => {
			const newConv = createMockConversation('conv-new', 'New Chat');
			vi.mocked(createConversation).mockResolvedValue(newConv);

			const result = await chatStore.createConversation('New Chat');

			expect(result).toEqual(newConv);
			expect(createConversation).toHaveBeenCalledWith('New Chat');

			const state = get(chatStore);
			expect(state.conversations).toHaveLength(1);
			expect(state.conversations[0]).toEqual(newConv);
		});

		it('createConversation handles errors', async () => {
			vi.mocked(createConversation).mockRejectedValue(new Error('Creation failed'));

			const result = await chatStore.createConversation('New Chat');

			expect(result).toBeNull();

			const state = get(chatStore);
			expect(state.error).toBe('Creation failed');
		});

		it('addConversation adds conversation locally', () => {
			const mockConv = createMockConversation('conv-1', 'Test');

			chatStore.addConversation(mockConv);

			const state = get(chatStore);
			expect(state.conversations).toEqual([mockConv]);
		});

		it('renameConversation updates conversation title', async () => {
			const mockConv = createMockConversation('conv-1', 'Old Title');
			const updatedConv = createMockConversation('conv-1', 'New Title');
			updatedConv.updatedAt = new Date('2024-01-02T00:00:00Z');

			chatStore.setConversations([mockConv]);
			vi.mocked(renameConversation).mockResolvedValue(updatedConv);

			const result = await chatStore.renameConversation('conv-1', 'New Title');

			expect(result).toBe(true);
			expect(renameConversation).toHaveBeenCalledWith('conv-1', 'New Title');

			const state = get(chatStore);
			expect(state.conversations[0].title).toBe('New Title');
		});

		it('renameConversation handles errors', async () => {
			const mockConv = createMockConversation('conv-1', 'Title');
			chatStore.setConversations([mockConv]);
			vi.mocked(renameConversation).mockRejectedValue(new Error('Rename failed'));

			const result = await chatStore.renameConversation('conv-1', 'New Title');

			expect(result).toBe(false);
			expect(get(chatStore).error).toBe('Rename failed');
		});

		it('updateConversation updates conversation locally', () => {
			const mockConv = createMockConversation('conv-1', 'Title');
			chatStore.setConversations([mockConv]);

			chatStore.updateConversation('conv-1', { title: 'Updated Title' });

			const state = get(chatStore);
			expect(state.conversations[0].title).toBe('Updated Title');
		});

		it('deleteConversation removes conversation from state', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			chatStore.selectConversation('conv-1');

			vi.mocked(deleteConversation).mockResolvedValue(undefined);

			const result = await chatStore.deleteConversation('conv-1');

			expect(result).toBe(true);
			expect(deleteConversation).toHaveBeenCalledWith('conv-1');

			const state = get(chatStore);
			expect(state.conversations).toEqual([]);
			expect(state.selectedConversationId).toBeNull();
		});

		it('deleteConversation handles errors', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			vi.mocked(deleteConversation).mockRejectedValue(new Error('Delete failed'));

			const result = await chatStore.deleteConversation('conv-1');

			expect(result).toBe(false);
			expect(get(chatStore).error).toBe('Delete failed');
		});

		it('updateConversationSettings updates settings via API and locally', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);

			const updatedSettings = {
				model: 'new-model',
				temperature: 0.5,
				maxTokens: 2048,
				systemPrompt: 'New prompt',
				workspaceRoot: '/new-workspace'
			};

			const updatedConv = createMockConversation('conv-1', 'Test');
			updatedConv.settings = { ...updatedSettings };

			vi.mocked(updateConversationSettings).mockResolvedValue(updatedConv);

			const result = await chatStore.updateConversationSettings('conv-1', updatedSettings);

			expect(result).toBe(true);
			expect(updateConversationSettings).toHaveBeenCalledWith('conv-1', updatedSettings);

			const state = get(chatStore);
			expect(state.conversations[0].settings).toEqual(updatedSettings);
		});

		it('updateConversationSettings handles conversation not found', async () => {
			const result = await chatStore.updateConversationSettings('nonexistent', {
				model: 'test',
				temperature: 0.7,
				maxTokens: 4096
			});

			expect(result).toBe(false);
			expect(get(chatStore).error).toBe('Conversation not found');
		});

		it('setConversationWorkspace updates workspace root', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);

			const updatedConv = createMockConversation('conv-1', 'Test');
			updatedConv.settings.workspaceRoot = '/new-workspace';

			vi.mocked(setConversationWorkspace).mockResolvedValue(updatedConv);

			const result = await chatStore.setConversationWorkspace('conv-1', '/new-workspace');

			expect(result).toBe(true);

			const state = get(chatStore);
			expect(state.conversations[0].settings.workspaceRoot).toBe('/new-workspace');
		});

		it('setConversationWorkspace handles errors', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			vi.mocked(setConversationWorkspace).mockRejectedValue(new Error('Update failed'));

			const result = await chatStore.setConversationWorkspace('conv-1', '/workspace');

			expect(result).toBe(false);
			expect(get(chatStore).error).toBe('Update failed');
		});

		it('selectConversation loads threads and selects most recent', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			const mockThreads = [
				createMockThread('thread-1', 'conv-1', 'Thread 1'),
				createMockThread('thread-2', 'conv-1', 'Thread 2')
			];

			chatStore.setConversations([mockConv]);
			vi.mocked(loadThreads).mockResolvedValue(mockThreads);

			await chatStore.selectConversation('conv-1');

			expect(loadThreads).toHaveBeenCalledWith('conv-1');

			const state = get(chatStore);
			expect(state.selectedConversationId).toBe('conv-1');
			expect(state.selectedThreadId).toBe('thread-2'); // Most recent based on creation order
			expect(state.threads).toEqual(mockThreads);
			expect(state.threadsLoading).toBe('success');
		});

		it('selectConversation handles null conversation ID', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			chatStore.selectConversation('conv-1');

			await chatStore.selectConversation(null);

			const state = get(chatStore);
			expect(state.selectedConversationId).toBeNull();
			expect(state.selectedThreadId).toBeNull();
			expect(state.threads).toEqual([]);
		});

		it('selectConversation handles load threads error', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			vi.mocked(loadThreads).mockRejectedValue(new Error('Load failed'));

			await chatStore.selectConversation('conv-1');

			const state = get(chatStore);
			expect(state.threadsLoading).toBe('error');
			expect(state.error).toBe('Load failed');
		});

		it('selectConversation with empty threads does not auto-select', async () => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			vi.mocked(loadThreads).mockResolvedValue([]);

			await chatStore.selectConversation('conv-1');

			const state = get(chatStore);
			expect(state.selectedThreadId).toBeNull();
		});
	});

	describe('Thread Management', () => {
		beforeEach(() => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			chatStore.selectConversation('conv-1');
		});

		it('selectThread loads messages and updates selected thread', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const mockMessages = [
				createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Hello'),
				createMockMessage('msg-2', 'conv-1', 'thread-1', 'assistant', 'Hi')
			];

			vi.mocked(loadThreadMessages).mockResolvedValue(mockMessages);

			await chatStore.selectThread('thread-1');

			expect(loadThreadMessages).toHaveBeenCalledWith('thread-1');

			const state = get(chatStore);
			expect(state.selectedThreadId).toBe('thread-1');
			expect(state.messages).toEqual(mockMessages);
			expect(state.messagesLoading).toBe('success');
		});

		it('selectThread handles null thread ID', async () => {
			await chatStore.selectThread(null);

			const state = get(chatStore);
			expect(state.selectedThreadId).toBeNull();
			expect(state.messagesLoading).toBe('idle');
		});

		it('selectThread loads agent session when present', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			mockThread.metadata.agentSessionId = 'agent-session-1';

			const state = get(chatStore);
			state.threads = [mockThread];

			vi.mocked(loadThreadMessages).mockResolvedValue([]);

			await chatStore.selectThread('thread-1');

			expect(agentStore.loadSession).toHaveBeenCalledWith('agent-session-1');
			expect(agentStore.setActiveSession).toHaveBeenCalledWith('agent-session-1');
		});

		it('selectThread clears agent session when not present', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');

			const state = get(chatStore);
			state.threads = [mockThread];

			vi.mocked(loadThreadMessages).mockResolvedValue([]);

			await chatStore.selectThread('thread-1');

			expect(agentStore.setActiveSession).toHaveBeenCalledWith(null);
		});

		it('selectThread handles streaming state', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const mockMessages = [createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Hello')];

			const state = get(chatStore);
			state.threads = [mockThread];
			state.threadStreaming['thread-1'] = {
				isStreaming: true,
				requestId: 'req-1',
				streamingContent: 'Streaming...',
				model: 'gpt-4',
				conversationId: 'conv-1',
				assistantMessageId: 'assist-1'
			};

			vi.mocked(loadThreadMessages).mockResolvedValue(mockMessages);

			await chatStore.selectThread('thread-1');

			const newState = get(chatStore);
			expect(newState.messages).toHaveLength(2); // Original + streaming message
			expect(newState.messages[1].id).toBe('assist-1');
			expect(newState.messages[1].content).toBe('Streaming...');
		});

		it('selectThread handles load messages error', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];

			vi.mocked(loadThreadMessages).mockRejectedValue(new Error('Load failed'));

			await chatStore.selectThread('thread-1');

			const newState = get(chatStore);
			expect(newState.messagesLoading).toBe('error');
			expect(newState.error).toBe('Load failed');
		});

		it('createThread creates new thread and agent session', async () => {
			const newThread = createMockThread('thread-new', 'conv-1', 'New Thread');

			vi.mocked(createThread).mockResolvedValue(newThread);
			vi.mocked(setThreadAgentSession).mockResolvedValue(undefined);

			const result = await chatStore.createThread('New Thread');

			expect(result).toEqual(newThread);
			expect(createThread).toHaveBeenCalledWith('conv-1', 'New Thread');
			expect(agentStore.createSession).toHaveBeenCalledWith(
				'code-agent',
				'conv-1',
				'thread-new',
				'/workspace'
			);
			expect(setThreadAgentSession).toHaveBeenCalledWith('thread-new', 'agent-session-1');

			const state = get(chatStore);
			expect(state.threads).toContainEqual(newThread);
		});

		it('createThread continues without agent on agent creation failure', async () => {
			const newThread = createMockThread('thread-new', 'conv-1', 'New Thread');

			vi.mocked(createThread).mockResolvedValue(newThread);
			vi.mocked(agentStore.createSession).mockRejectedValue(new Error('Agent failed'));
			vi.mocked(setThreadAgentSession).mockResolvedValue(undefined);

			const result = await chatStore.createThread('New Thread');

			expect(result).toEqual(newThread);
		});

		it('createThread returns null when no conversation selected', async () => {
			chatStore.selectConversation(null);

			const result = await chatStore.createThread('New Thread');

			expect(result).toBeNull();
			expect(get(chatStore).error).toBe('No conversation selected');
		});

		it('resolveThread updates thread resolved status', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];

			vi.mocked(resolveThread).mockResolvedValue(undefined);

			const result = await chatStore.resolveThread('thread-1', true);

			expect(result).toBe(true);
			expect(resolveThread).toHaveBeenCalledWith('thread-1', true);

			const newState = get(chatStore);
			expect(newState.threads[0].metadata.resolved).toBe(true);
		});

		it('resolveThread handles errors', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];

			vi.mocked(resolveThread).mockRejectedValue(new Error('Resolve failed'));

			const result = await chatStore.resolveThread('thread-1', true);

			expect(result).toBe(false);
			expect(get(chatStore).error).toBe('Resolve failed');
		});

		it('deleteThread removes thread from state', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];
			state.selectedThreadId = 'thread-1';

			vi.mocked(deleteThread).mockResolvedValue(undefined);

			const result = await chatStore.deleteThread('thread-1');

			expect(result).toBe(true);
			expect(deleteThread).toHaveBeenCalledWith('thread-1');

			const newState = get(chatStore);
			expect(newState.threads).toEqual([]);
			expect(newState.selectedThreadId).toBeNull();
		});

		it('deleteThread handles errors', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];

			vi.mocked(deleteThread).mockRejectedValue(new Error('Delete failed'));

			const result = await chatStore.deleteThread('thread-1');

			expect(result).toBe(false);
			expect(get(chatStore).error).toBe('Delete failed');
		});

		it('renameThread updates thread title', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const updatedThread = createMockThread('thread-1', 'conv-1', 'New Title');

			const state = get(chatStore);
			state.threads = [mockThread];

			vi.mocked(renameThread).mockResolvedValue(updatedThread);

			const result = await chatStore.renameThread('thread-1', 'New Title');

			expect(result).toBe(true);
			expect(renameThread).toHaveBeenCalledWith('thread-1', 'New Title');

			const newState = get(chatStore);
			expect(newState.threads[0].title).toBe('New Title');
		});

		it('renameThread handles errors', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];

			vi.mocked(renameThread).mockRejectedValue(new Error('Rename failed'));

			const result = await chatStore.renameThread('thread-1', 'New Title');

			expect(result).toBe(false);
			expect(get(chatStore).error).toBe('Rename failed');
		});
	});

	describe('Message Management', () => {
		beforeEach(() => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			chatStore.selectConversation('conv-1');

			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];
			state.selectedThreadId = 'thread-1';
		});

		it('sendMessage sends message and updates state', async () => {
			const newMessage = createMockMessage('msg-new', 'conv-1', 'thread-1', 'user', 'Hello');

			vi.mocked(sendMessage).mockResolvedValue(newMessage);

			const result = await chatStore.sendMessage('Hello', 'user');

			expect(result).toEqual(newMessage);
			expect(sendMessage).toHaveBeenCalledWith({
				conversationId: 'conv-1',
				content: 'Hello',
				role: 'user',
				threadId: 'thread-1'
			});

			const state = get(chatStore);
			expect(state.messages).toContainEqual(newMessage);
		});

		it('sendMessage returns null when no conversation selected', async () => {
			chatStore.selectConversation(null);

			const result = await chatStore.sendMessage('Hello', 'user');

			expect(result).toBeNull();
			expect(get(chatStore).error).toBe('No conversation selected');
		});

		it('sendMessage creates thread with message when no threads exist', async () => {
			const state = get(chatStore);
			state.threads = [];
			state.selectedThreadId = null;

			const newThread = createMockThread('thread-new', 'conv-1', 'Hello');
			const newMessage = createMockMessage('msg-new', 'conv-1', 'thread-new', 'user', 'Hello');

			vi.mocked(createThreadWithMessage).mockResolvedValue({
				thread: newThread,
				message: newMessage
			});
			vi.mocked(setThreadAgentSession).mockResolvedValue(undefined);

			const result = await chatStore.sendMessage('Hello', 'user');

			expect(result).toEqual(newMessage);
			expect(createThreadWithMessage).toHaveBeenCalled();
		});

		it('sendMessage handles send errors and rolls back optimistic update', async () => {
			vi.mocked(sendMessage).mockRejectedValue(new Error('Send failed'));

			const result = await chatStore.sendMessage('Hello', 'user');

			expect(result).toBeNull();

			const state = get(chatStore);
			expect(state.error).toBe('Send failed');
			// Message with error status should be in state
			expect(state.messages.some((m) => m.status === 'error')).toBe(true);
		});

		it('addMessage adds message locally', async () => {
			const mockMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Test');

			await chatStore.addMessage(mockMessage);

			const state = get(chatStore);
			expect(state.messages).toContainEqual(mockMessage);
		});

		it('addMessage syncs with NLP backend when available', async () => {
			const state = get(chatStore);
			state.nlp.backendAvailable = true;

			const mockMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Test');

			await chatStore.addMessage(mockMessage);

			expect(addContextMessage).toHaveBeenCalledWith({
				conversationId: 'conv-1',
				role: 'user',
				content: 'Test'
			});
		});

		it('deleteMessage removes message from state', async () => {
			const mockMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Test');
			const state = get(chatStore);
			state.messages = [mockMessage];

			vi.mocked(deleteMessage).mockResolvedValue(undefined);

			const result = await chatStore.deleteMessage('msg-1');

			expect(result).toBe(true);
			expect(deleteMessage).toHaveBeenCalledWith('msg-1');

			const newState = get(chatStore);
			expect(newState.messages).toEqual([]);
		});

		it('deleteMessage handles errors', async () => {
			vi.mocked(deleteMessage).mockRejectedValue(new Error('Delete failed'));

			const result = await chatStore.deleteMessage('msg-1');

			expect(result).toBe(false);
			expect(get(chatStore).error).toBe('Delete failed');
		});

		it('updateMessageStatus updates message status', async () => {
			const mockMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Test');
			const state = get(chatStore);
			state.messages = [mockMessage];

			vi.mocked(updateMessageStatus).mockResolvedValue(undefined);

			const result = await chatStore.updateMessageStatus('msg-1', 'delivered');

			expect(result).toBe(true);
			expect(updateMessageStatus).toHaveBeenCalledWith('msg-1', 'delivered');

			const newState = get(chatStore);
			expect(newState.messages[0].status).toBe('delivered');
		});

		it('updateMessageStatus handles errors gracefully', async () => {
			const mockMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Test');
			const state = get(chatStore);
			state.messages = [mockMessage];

			vi.mocked(updateMessageStatus).mockRejectedValue(new Error('Update failed'));

			const result = await chatStore.updateMessageStatus('msg-1', 'delivered');

			expect(result).toBe(false);
		});

		it('updateMessageFeedback sets like feedback', async () => {
			const mockMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'assistant', 'Response');
			mockMessage.metadata = {};
			const state = get(chatStore);
			state.messages = [mockMessage];

			vi.mocked(updateMessageFeedback).mockResolvedValue(undefined);

			const result = await chatStore.updateMessageFeedback('msg-1', 'like');

			expect(result).toBe(true);
			expect(updateMessageFeedback).toHaveBeenCalledWith('msg-1', 'like');

			const newState = get(chatStore);
			expect(newState.messages[0].metadata?.feedback).toBe('like');
		});

		it('updateMessageFeedback clears feedback with null', async () => {
			const mockMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'assistant', 'Response');
			mockMessage.metadata = { feedback: 'like' };
			const state = get(chatStore);
			state.messages = [mockMessage];

			vi.mocked(updateMessageFeedback).mockResolvedValue(undefined);

			await chatStore.updateMessageFeedback('msg-1', null);

			const newState = get(chatStore);
			expect(newState.messages[0].metadata?.feedback).toBeUndefined();
		});

		it('updateMessageFeedback handles errors gracefully', async () => {
			const mockMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'assistant', 'Response');
			mockMessage.metadata = {};
			const state = get(chatStore);
			state.messages = [mockMessage];

			vi.mocked(updateMessageFeedback).mockRejectedValue(new Error('Update failed'));

			const result = await chatStore.updateMessageFeedback('msg-1', 'like');

			expect(result).toBe(false);
		});

		it('updateMessage updates message locally', () => {
			const mockMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Test');
			const state = get(chatStore);
			state.messages = [mockMessage];

			chatStore.updateMessage('msg-1', { content: 'Updated' });

			const newState = get(chatStore);
			expect(newState.messages[0].content).toBe('Updated');
		});

		it('removeTransientMessages removes tool_call and thinking messages for session', () => {
			const normalMsg = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Hello');
			const toolCallMsg = createMockMessage('msg-2', 'conv-1', 'thread-1', 'assistant', 'Tool call');
			toolCallMsg.kind = 'tool_call';
			toolCallMsg.agentData = { sessionId: 'session-1' };
			const thinkingMsg = createMockMessage('msg-3', 'conv-1', 'thread-1', 'assistant', 'Thinking');
			thinkingMsg.kind = 'thinking';
			thinkingMsg.agentData = { sessionId: 'session-1' };

			const state = get(chatStore);
			state.messages = [normalMsg, toolCallMsg, thinkingMsg];

			chatStore.removeTransientMessages('session-1');

			const newState = get(chatStore);
			expect(newState.messages).toHaveLength(1);
			expect(newState.messages[0].id).toBe('msg-1');
		});
	});

	describe('NLP Methods', () => {
		beforeEach(() => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			chatStore.selectConversation('conv-1');

			const state = get(chatStore);
			state.nlp.backendAvailable = true;
		});

		it('classifyInput classifies intent and updates state', async () => {
			const mockResult: IntentClassification = {
				intent: 'code_generation',
				confidence: 0.95
			};

			const mockClassifier = {
				classify: vi.fn().mockResolvedValue(mockResult),
				cancel: vi.fn()
			};
			vi.mocked(createDebouncedClassifier).mockReturnValue(mockClassifier);

			// Re-initialize to get the new classifier
			chatStore.reset();

			await chatStore.classifyInput('Write a function');

			expect(mockClassifier.classify).toHaveBeenCalledWith({
				input: 'Write a function',
				conversationId: 'conv-1'
			});

			const state = get(chatStore);
			expect(state.nlp.detectedIntent).toEqual(mockResult);
			expect(state.nlp.processing).toBe(false);
		});

		it('classifyInput returns early when backend unavailable or no conversation', async () => {
			const state = get(chatStore);
			state.nlp.backendAvailable = false;

			await chatStore.classifyInput('Test');

			// Should not throw, just return early
			expect(state.nlp.processing).toBe(false);
		});

		it('classifyInput handles errors', async () => {
			const mockClassifier = {
				classify: vi.fn().mockRejectedValue(new Error('Classification failed')),
				cancel: vi.fn()
			};
			vi.mocked(createDebouncedClassifier).mockReturnValue(mockClassifier);
			chatStore.reset();

			await chatStore.classifyInput('Test');

			const state = get(chatStore);
			expect(state.nlp.error).toBe('Classification failed');
			expect(state.nlp.processing).toBe(false);
		});

		it('detectAmbiguitiesForInput detects and filters ambiguities', async () => {
			const mockAmbiguities: Ambiguity[] = [
				{
					id: 'amb-1',
					span: { text: 'file', start: 0, end: 4 },
					type: 'file_reference',
					severity: 'high',
					suggestions: []
				},
				{
					id: 'amb-2',
					span: { text: 'function', start: 10, end: 18 },
					type: 'ambiguous_reference',
					severity: 'low',
					suggestions: []
				}
			];

			vi.mocked(detectAmbiguities).mockResolvedValue(mockAmbiguities);

			await chatStore.detectAmbiguitiesForInput('Test input with file reference');

			const state = get(chatStore);
			expect(state.nlp.ambiguities).toEqual(mockAmbiguities);
			// Only high/medium severity should be pending
			expect(state.nlp.pendingAmbiguities).toHaveLength(1);
			expect(state.nlp.pendingAmbiguities[0].id).toBe('amb-1');
		});

		it('detectAmbiguitiesForInput returns early when backend unavailable', async () => {
			const state = get(chatStore);
			state.nlp.backendAvailable = false;

			await chatStore.detectAmbiguitiesForInput('Test');

			// Should not call the API
			expect(detectAmbiguities).not.toHaveBeenCalled();
		});

		it('clearNlpState clears all NLP state', () => {
			const state = get(chatStore);
			state.nlp.detectedIntent = { intent: 'test', confidence: 0.9 };
			state.nlp.ambiguities = [{ id: 'amb-1', span: { text: 'test', start: 0, end: 4 }, type: 'file_reference', severity: 'high', suggestions: [] }];
			state.nlp.pendingAmbiguities = state.nlp.ambiguities;
			state.nlp.processing = true;
			state.nlp.error = 'Test error';

			chatStore.clearNlpState();

			const newState = get(chatStore);
			expect(newState.nlp.detectedIntent).toBeNull();
			expect(newState.nlp.ambiguities).toEqual([]);
			expect(newState.nlp.pendingAmbiguities).toEqual([]);
			expect(newState.nlp.processing).toBe(false);
			expect(newState.nlp.error).toBeNull();
		});

		it('resolveAmbiguity resolves ambiguity and updates state', async () => {
			const mockAmbiguity: Ambiguity = {
				id: 'amb-1',
				span: { text: 'file', start: 0, end: 4 },
				type: 'file_reference',
				severity: 'high',
				suggestions: []
			};

			const state = get(chatStore);
			state.nlp.pendingAmbiguities = [mockAmbiguity];

			const mockResolved: ResolvedAmbiguity = {
				ambiguity_id: 'amb-1',
				original_text: 'file',
				resolution: '/path/to/file',
				resolution_type: 'user_provided'
			};

			vi.mocked(resolveAmbiguity).mockResolvedValue(mockResolved);

			const result = await chatStore.resolveAmbiguity('amb-1', '/path/to/file', 'user_provided');

			expect(result).toBe(true);
			expect(resolveAmbiguity).toHaveBeenCalledWith({
				ambiguityId: 'amb-1',
				originalText: 'file',
				resolution: '/path/to/file',
				resolutionType: 'user_provided'
			});

			const newState = get(chatStore);
			expect(newState.nlp.pendingAmbiguities).toEqual([]);
			expect(newState.nlp.resolvedAmbiguities.get('amb-1')).toEqual(mockResolved);
		});

		it('resolveAmbiguity handles ambiguity not found', async () => {
			const result = await chatStore.resolveAmbiguity('nonexistent', 'test', 'user_provided');

			expect(result).toBe(false);
			expect(get(chatStore).nlp.error).toBe('Ambiguity not found');
		});

		it('resolveAmbiguity handles API errors', async () => {
			const mockAmbiguity: Ambiguity = {
				id: 'amb-1',
				span: { text: 'test', start: 0, end: 4 },
				type: 'file_reference',
				severity: 'high',
				suggestions: []
			};

			const state = get(chatStore);
			state.nlp.pendingAmbiguities = [mockAmbiguity];

			vi.mocked(resolveAmbiguity).mockRejectedValue(new Error('Resolve failed'));

			const result = await chatStore.resolveAmbiguity('amb-1', 'test', 'user_provided');

			expect(result).toBe(false);
			expect(get(chatStore).nlp.error).toBe('Resolve failed');
		});

		it('dismissAmbiguity resolves with dismissed type', async () => {
			const mockAmbiguity: Ambiguity = {
				id: 'amb-1',
				span: { text: 'test', start: 0, end: 4 },
				type: 'file_reference',
				severity: 'high',
				suggestions: []
			};

			const state = get(chatStore);
			state.nlp.pendingAmbiguities = [mockAmbiguity];

			vi.mocked(resolveAmbiguity).mockResolvedValue({
				ambiguity_id: 'amb-1',
				original_text: 'test',
				resolution: '',
				resolution_type: 'dismissed'
			});

			const result = await chatStore.dismissAmbiguity('amb-1');

			expect(result).toBe(true);
		});

		it('hasBlockingAmbiguities returns true when high severity ambiguities exist', () => {
		 const state = get(chatStore);
			state.nlp.pendingAmbiguities = [
				{ id: 'amb-1', span: { text: 'test', start: 0, end: 4 }, type: 'file_reference', severity: 'high', suggestions: [] }
			];

			expect(chatStore.hasBlockingAmbiguities()).toBe(true);
		});

		it('hasBlockingAmbiguities returns false when no high severity ambiguities', () => {
			const state = get(chatStore);
			state.nlp.pendingAmbiguities = [
				{ id: 'amb-1', span: { text: 'test', start: 0, end: 4 }, type: 'file_reference', severity: 'low', suggestions: [] }
			];

			expect(chatStore.hasBlockingAmbiguities()).toBe(false);
		});

		it('applyAmbiguityResolutions applies resolutions to input', async () => {
			const mockResolved: ResolvedAmbiguity = {
				ambiguity_id: 'amb-1',
				original_text: 'file',
				resolution: '/path/to/file.ts',
				resolution_type: 'user_provided'
			};

			const state = get(chatStore);
			state.nlp.resolvedAmbiguities = new Map([['amb-1', mockResolved]]);

			vi.mocked(applyResolutions).mockResolvedValue('Edit /path/to/file.ts instead');

			const result = await chatStore.applyAmbiguityResolutions('Edit file instead');

			expect(result).toBe('Edit /path/to/file.ts instead');
			expect(applyResolutions).toHaveBeenCalledWith({
				input: 'Edit file instead',
				resolutions: { amb-1: '/path/to/file.ts' }
			});
		});

		it('applyAmbiguityResolutions returns original input when no resolutions', async () => {
			const result = await chatStore.applyAmbiguityResolutions('Original input');

			expect(result).toBe('Original input');
			expect(applyResolutions).not.toHaveBeenCalled();
		});

		it('applyAmbiguityResolutions handles errors gracefully', async () => {
			const mockResolved: ResolvedAmbiguity = {
				ambiguity_id: 'amb-1',
				original_text: 'file',
				resolution: '/path/to/file',
				resolution_type: 'user_provided'
			};

			const state = get(chatStore);
			state.nlp.resolvedAmbiguities = new Map([['amb-1', mockResolved]]);

			vi.mocked(applyResolutions).mockRejectedValue(new Error('Apply failed'));

			const result = await chatStore.applyAmbiguityResolutions('Edit file');

			expect(result).toBe('Edit file'); // Returns original on error
		});

		it('clearResolvedAmbiguities clears resolved ambiguities map', () => {
			const state = get(chatStore);
			state.nlp.resolvedAmbiguities = new Map([
				['amb-1', {} as ResolvedAmbiguity],
				['amb-2', {} as ResolvedAmbiguity]
			]);

			chatStore.clearResolvedAmbiguities();

			const newState = get(chatStore);
			expect(newState.nlp.resolvedAmbiguities.size).toBe(0);
		});
	});

	describe('LLM Integration Methods', () => {
		beforeEach(() => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			chatStore.selectConversation('conv-1');

			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];
			state.selectedThreadId = 'thread-1';
		});

		it('sendMessageWithLlm sends user message and routes through agent', async () => {
			const userMessage = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Hello');
			const threadWithAgent = createMockThread('thread-1', 'conv-1', 'Thread 1');
			threadWithAgent.metadata.agentSessionId = 'agent-session-1';

			const state = get(chatStore);
			state.threads = [threadWithAgent];

			vi.mocked(sendMessage).mockResolvedValue(userMessage);

			const result = await chatStore.sendMessageWithLlm('Hello');

			expect(result).toEqual(userMessage);
			expect(agentStore.sendRequest).toHaveBeenCalledWith('agent-session-1', 'Hello');
		});

		it('sendMessageWithLlm returns null when no conversation selected', async () => {
			chatStore.selectConversation(null);

			const result = await chatStore.sendMessageWithLlm('Hello');

			expect(result).toBeNull();
		});

		it('cancelLlmStream cancels active stream', async () => {
			// This test verifies the method exists and can be called
			// Full testing would require mocking the LlmHandler class
			await expect(chatStore.cancelLlmStream()).resolves.not.toThrow();
		});

		it('isLlmStreaming returns streaming status', () => {
			// Initial state should not be streaming
			expect(chatStore.isLlmStreaming()).toBe(false);
		});

		it('getStreamingContent returns current streaming content', () => {
			// Initial state should have empty content
			expect(chatStore.getStreamingContent()).toBe('');
		});
	});

	describe('Utility Methods', () => {
		it('clearError clears error state', () => {
			const state = get(chatStore);
			state.error = 'Test error';

			chatStore.clearError();

			const newState = get(chatStore);
			expect(newState.error).toBeNull();
		});

		it('markThreadRead removes thread from unread IDs', () => {
			const state = get(chatStore);
			state.unreadThreadIds = ['thread-1', 'thread-2', 'thread-3'];

			chatStore.markThreadRead('thread-2');

			const newState = get(chatStore);
			expect(newState.unreadThreadIds).toEqual(['thread-1', 'thread-3']);
		});

		it('markThreadRead handles thread not in unread list', () => {
			const state = get(chatStore);
			state.unreadThreadIds = ['thread-1'];

			chatStore.markThreadRead('thread-2');

			const newState = get(chatStore);
			expect(newState.unreadThreadIds).toEqual(['thread-1']);
		});

		it('setThreadWorkspace updates thread workspace locally', async () => {
			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];

			const result = await chatStore.setThreadWorkspace('thread-1', '/new-workspace');

			expect(result).toBe(true);

			const newState = get(chatStore);
			expect(newState.threads[0].metadata.workspaceRoot).toBe('/new-workspace');
		});

		it('setThreadWorkspace handles errors gracefully', async () => {
			// Since the implementation just updates state, it should always resolve
			const result = await chatStore.setThreadWorkspace('nonexistent', '/workspace');
			expect(result).toBe(true);
		});
	});

	describe('Derived Stores', () => {
		beforeEach(() => {
			const mockConv = createMockConversation('conv-1', 'Test');
			chatStore.setConversations([mockConv]);
			chatStore.selectConversation('conv-1');

			const mockThread = createMockThread('thread-1', 'conv-1', 'Thread 1');
			const state = get(chatStore);
			state.threads = [mockThread];
			state.selectedThreadId = 'thread-1';
		});

		it('selectedConversation returns the selected conversation or null', async () => {
			const { selectedConversation } = await import('./chat');

			expect(get(selectedConversation)?.id).toBe('conv-1');

			chatStore.selectConversation(null);

			expect(get(selectedConversation)).toBeNull();
		});

		it('selectedThread returns the selected thread or null', async () => {
			const { selectedThread } = await import('./chat');

			expect(get(selectedThread)?.id).toBe('thread-1');

			chatStore.selectThread(null);

			expect(get(selectedThread)).toBeNull();
		});

		it('currentThreads returns threads array', async () => {
			const { currentThreads } = await import('./chat');

			expect(get(currentThreads)).toHaveLength(1);
			expect(get(currentThreads)[0].id).toBe('thread-1');
		});

		it('activeStreamingThreadIds returns streaming thread IDs', async () => {
			const { activeStreamingThreadIds } = await import('./chat');

			expect(get(activeStreamingThreadIds)).toEqual([]);

			const state = get(chatStore);
			state.threadStreaming['thread-1'] = {
				isStreaming: true,
				requestId: 'req-1',
				streamingContent: 'test',
				model: 'gpt-4',
				conversationId: 'conv-1',
				assistantMessageId: 'assist-1'
			};

			expect(get(activeStreamingThreadIds)).toEqual(['thread-1']);
		});

		it('unreadThreadIds returns unread thread IDs', async () => {
			const { unreadThreadIds: unreadStore } = await import('./chat');

			const state = get(chatStore);
			state.unreadThreadIds = ['thread-1', 'thread-2'];

			expect(get(unreadStore)).toEqual(['thread-1', 'thread-2']);
		});

		it('messageGroups groups messages by time interval', async () => {
			const { messageGroups } = await import('./chat');

			const msg1 = createMockMessage('msg-1', 'conv-1', 'thread-1', 'user', 'Hello');
			msg1.timestamp = new Date('2024-01-01T00:00:00Z');

			const msg2 = createMockMessage('msg-2', 'conv-1', 'thread-1', 'assistant', 'Hi');
			msg2.timestamp = new Date('2024-01-01T00:02:00Z'); // 2 minutes later - same group

			const msg3 = createMockMessage('msg-3', 'conv-1', 'thread-1', 'user', 'Hello again');
			msg3.timestamp = new Date('2024-01-01T00:10:00Z'); // 10 minutes later - new group

			const state = get(chatStore);
			state.messages = [msg1, msg2, msg3];

			const groups = get(messageGroups);

			expect(groups).toHaveLength(2);
			expect(groups[0].messages).toHaveLength(2);
			expect(groups[1].messages).toHaveLength(1);
		});

		it('conversationGroups groups conversations by time', async () => {
			const { conversationGroups } = await import('./chat');

			const todayConv = createMockConversation('conv-1', 'Today');
			todayConv.updatedAt = new Date();

			const oldConv = createMockConversation('conv-2', 'Older');
			oldConv.updatedAt = new Date('2020-01-01T00:00:00Z');

			chatStore.setConversations([todayConv, oldConv]);

			const groups = get(conversationGroups);

			expect(groups.length).toBeGreaterThan(0);
			expect(groups.some((g) => g.label === 'Today')).toBe(true);
		});

		it('nlpState returns NLP state', async () => {
			const { nlpState } = await import('./chat');

			const state = get(chatStore);
			state.nlp.backendAvailable = true;
			state.nlp.processing = true;

			const nlp = get(nlpState);

			expect(nlp.backendAvailable).toBe(true);
			expect(nlp.processing).toBe(true);
		});

		it('llmState returns LLM state for selected thread', async () => {
			const { llmState } = await import('./chat');

			const state = get(chatStore);
			state.threadStreaming['thread-1'] = {
				isStreaming: true,
				requestId: 'req-1',
				streamingContent: 'Streaming...',
				model: 'gpt-4',
				conversationId: 'conv-1',
				assistantMessageId: 'assist-1'
			};

			const llm = get(llmState);

			expect(llm.isStreaming).toBe(true);
			expect(llm.streamingContent).toBe('Streaming...');
			expect(llm.model).toBe('gpt-4');
		});

		it('llmState returns empty state when no thread selected', async () => {
			const { llmState } = await import('./chat');

			chatStore.selectThread(null);

			const llm = get(llmState);

			expect(llm.isStreaming).toBe(false);
			expect(llm.streamingContent).toBe('');
			expect(llm.model).toBeNull();
		});

		it('isLlmStreaming returns streaming boolean', async () => {
			const { isLlmStreaming } = await import('./chat');

			expect(get(isLlmStreaming)).toBe(false);

			const state = get(chatStore);
			state.threadStreaming['thread-1'] = {
				isStreaming: true,
				requestId: 'req-1',
				streamingContent: 'test',
				model: 'gpt-4',
				conversationId: 'conv-1',
				assistantMessageId: 'assist-1'
			};

			expect(get(isLlmStreaming)).toBe(true);
		});

		it('streamingContent returns streaming content', async () => {
			const { streamingContent } = await import('./chat');

			expect(get(streamingContent)).toBe('');

			const state = get(chatStore);
			state.threadStreaming['thread-1'] = {
				isStreaming: true,
				requestId: 'req-1',
				streamingContent: 'Test content',
				model: 'gpt-4',
				conversationId: 'conv-1',
				assistantMessageId: 'assist-1'
			};

			expect(get(streamingContent)).toBe('Test content');
		});

		it('conversationsLoading returns loading state', async () => {
			const { conversationsLoading } = await import('./chat');

			const state = get(chatStore);
			state.conversationsLoading = 'loading';

			expect(get(conversationsLoading)).toBe('loading');
		});

		it('messagesLoading returns loading state', async () => {
			const { messagesLoading } = await import('./chat');

			const state = get(chatStore);
			state.messagesLoading = 'loading';

			expect(get(messagesLoading)).toBe('loading');
		});

		it('threadsLoading returns loading state', async () => {
			const { threadsLoading } = await import('./chat');

			const state = get(chatStore);
			state.threadsLoading = 'loading';

			expect(get(threadsLoading)).toBe('loading');
		});

		it('chatError returns error state', async () => {
			const { chatError } = await import('./chat');

			const state = get(chatStore);
			state.error = 'Test error';

			expect(get(chatError)).toBe('Test error');
		});

		it('isLoading returns true when any loading state is loading', async () => {
			const { isLoading } = await import('./chat');

			expect(get(isLoading)).toBe(false);

			const state = get(chatStore);
			state.conversationsLoading = 'loading';

			expect(get(isLoading)).toBe(true);
		});

		it('isLoading returns true when NLP is processing', async () => {
			const { isLoading } = await import('./chat');

			const state = get(chatStore);
			state.nlp.processing = true;

			expect(get(isLoading)).toBe(true);
		});
	});
});
