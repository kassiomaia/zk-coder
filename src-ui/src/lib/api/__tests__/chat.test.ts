/**
 * Tests for Chat API Client
 */
import {beforeEach, describe, expect, it, vi} from 'vitest';
import {
    ChatApiError,
    createConversation,
    createThread,
    createThreadWithMessage,
    deleteConversation,
    deleteMessage,
    deleteThread,
    loadConversation,
    loadConversations,
    loadMessages,
    loadThreadMessages,
    loadThreads,
    renameConversation,
    renameThread,
    resolveThread,
    sendMessage,
    setConversationWorkspace,
    setThreadAgentSession,
    updateConversationSettings,
    updateMessageFeedback,
    updateMessageStatus
} from '../chat';
import {invoke} from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

const mockInvoke = vi.mocked(invoke);

describe('chat API', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	// ============================================================================
	// Conversation Commands
	// ============================================================================

	describe('loadConversations', () => {
		it('should return all conversations', async () => {
			const mockConversations = [
				{
					id: 'conv-1',
					title: 'Chat 1',
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T01:00:00Z',
					settings: {
						model: 'gpt-4',
						temperature: 0.7,
						maxTokens: 4096
					}
				},
				{
					id: 'conv-2',
					title: 'Chat 2',
					createdAt: '2024-01-02T00:00:00Z',
					updatedAt: '2024-01-02T01:00:00Z',
					settings: {
						model: 'gpt-4',
						temperature: 0.5,
						maxTokens: 2048
					}
				}
			];
			mockInvoke.mockResolvedValueOnce(mockConversations);

			const result = await loadConversations();

			expect(mockInvoke).toHaveBeenCalledWith('load_conversations', undefined);
			expect(result).toHaveLength(2);
			expect(result[0].title).toBe('Chat 1');
			expect(result[1].title).toBe('Chat 2');
		});

		it('should return empty array when no conversations', async () => {
			mockInvoke.mockResolvedValueOnce([]);

			const result = await loadConversations();

			expect(result).toEqual([]);
		});

		it('should convert date strings to Date objects', async () => {
			mockInvoke.mockResolvedValueOnce([
				{
					id: 'conv-1',
					title: 'Chat',
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T01:00:00Z',
					settings: {
						model: 'gpt-4',
						temperature: 0.7,
						maxTokens: 4096
					}
				}
			]);

			const result = await loadConversations();

			expect(result[0].createdAt).toBeInstanceOf(Date);
			expect(result[0].updatedAt).toBeInstanceOf(Date);
		});
	});

	describe('createConversation', () => {
		it('should create a new conversation', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'conv-new',
				title: 'New Chat',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T00:00:00Z',
				settings: {
					model: 'gpt-4',
					temperature: 0.7,
					maxTokens: 4096
				}
			});

			const result = await createConversation('New Chat');

			expect(mockInvoke).toHaveBeenCalledWith('create_conversation', {
				title: 'New Chat'
			});
			expect(result.id).toBe('conv-new');
			expect(result.title).toBe('New Chat');
		});
	});

	describe('loadConversation', () => {
		it('should load a single conversation', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'conv-1',
				title: 'My Chat',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T01:00:00Z',
				settings: {
					model: 'gpt-4',
					temperature: 0.7,
					maxTokens: 4096
				}
			});

			const result = await loadConversation('conv-1');

			expect(mockInvoke).toHaveBeenCalledWith('load_conversation', { id: 'conv-1' });
			expect(result.id).toBe('conv-1');
		});

		it('should throw ChatApiError for non-existent conversation', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('Conversation not found'));

			await expect(loadConversation('invalid-id')).rejects.toThrow(ChatApiError);
		});
	});

	describe('renameConversation', () => {
		it('should rename a conversation', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'conv-1',
				title: 'Updated Title',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-02T00:00:00Z',
				settings: {
					model: 'gpt-4',
					temperature: 0.7,
					maxTokens: 4096
				}
			});

			const result = await renameConversation('conv-1', 'Updated Title');

			expect(mockInvoke).toHaveBeenCalledWith('rename_conversation', {
				id: 'conv-1',
				title: 'Updated Title'
			});
			expect(result.title).toBe('Updated Title');
		});
	});

	describe('deleteConversation', () => {
		it('should delete a conversation', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await deleteConversation('conv-1');

			expect(mockInvoke).toHaveBeenCalledWith('delete_conversation', { id: 'conv-1' });
		});
	});

	describe('updateConversationSettings', () => {
		it('should call invoke with correct command and args', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'conv-123',
				title: 'Test Conversation',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T01:00:00Z',
				settings: {
					model: 'openai-gpt-4',
					temperature: 0.5,
					maxTokens: 2048,
					systemPrompt: 'Test prompt'
				}
			});

			const result = await updateConversationSettings('conv-123', {
				model: 'openai-gpt-4',
				temperature: 0.5,
				maxTokens: 2048,
				systemPrompt: 'Test prompt'
			});

			expect(mockInvoke).toHaveBeenCalledWith('update_conversation_settings', {
				id: 'conv-123',
				settings: {
					model: 'openai-gpt-4',
					temperature: 0.5,
					maxTokens: 2048,
					systemPrompt: 'Test prompt'
				}
			});

			expect(result.settings.model).toBe('openai-gpt-4');
			expect(result.settings.temperature).toBe(0.5);
		});

		it('should throw ChatApiError on failure', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('Database error'));

			await expect(
				updateConversationSettings('invalid-id', {
					model: 'test-model',
					temperature: 0.7,
					maxTokens: 4096
				})
			).rejects.toThrow();
		});

		it('should handle settings without system prompt', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'conv-123',
				title: 'Test',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T01:00:00Z',
				settings: {
					model: 'new-model',
					temperature: 0.7,
					maxTokens: 4096
				}
			});

			const result = await updateConversationSettings('conv-123', {
				model: 'new-model',
				temperature: 0.7,
				maxTokens: 4096
			});

			expect(result.settings.model).toBe('new-model');
			expect(result.settings.systemPrompt).toBeUndefined();
		});

		it('should return conversation with proper date conversion', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'conv-123',
				title: 'Test',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T01:00:00Z',
				settings: {
					model: 'test-model',
					temperature: 0.7,
					maxTokens: 4096
				}
			});

			const result = await updateConversationSettings('conv-123', {
				model: 'test-model',
				temperature: 0.7,
				maxTokens: 4096
			});

			expect(result.createdAt).toBeInstanceOf(Date);
			expect(result.updatedAt).toBeInstanceOf(Date);
		});
	});

	describe('setConversationWorkspace', () => {
		it('should set workspace root for conversation', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'conv-1',
				title: 'Chat',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T01:00:00Z',
				settings: {
					model: 'gpt-4',
					temperature: 0.7,
					maxTokens: 4096,
					workspaceRoot: '/workspace'
				}
			});

			const result = await setConversationWorkspace('conv-1', '/workspace');

			expect(mockInvoke).toHaveBeenCalledWith('set_conversation_workspace', {
				id: 'conv-1',
				workspaceRoot: '/workspace'
			});
			expect(result.settings.workspaceRoot).toBe('/workspace');
		});
	});

	// ============================================================================
	// Thread Commands
	// ============================================================================

	describe('loadThreads', () => {
		it('should load threads for a conversation', async () => {
			const mockThreads = [
				{
					id: 'thread-1',
					conversationId: 'conv-1',
					parentId: null,
					title: 'Thread 1',
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T01:00:00Z',
					messageCount: 5,
					metadata: {
						resolved: false,
						tags: ['bug'],
						priority: 'high' as const
					}
				}
			];
			mockInvoke.mockResolvedValueOnce(mockThreads);

			const result = await loadThreads('conv-1');

			expect(mockInvoke).toHaveBeenCalledWith('load_threads', {
				conversationId: 'conv-1'
			});
			expect(result).toHaveLength(1);
			expect(result[0].title).toBe('Thread 1');
		});
	});

	describe('createThread', () => {
		it('should create a new thread', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'thread-new',
				conversationId: 'conv-1',
				parentId: null,
				title: 'New Thread',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T00:00:00Z',
				messageCount: 0,
				metadata: {
					resolved: false,
					tags: [],
					priority: 'medium' as const
				}
			});

			const result = await createThread('conv-1', 'New Thread');

			expect(mockInvoke).toHaveBeenCalledWith('create_thread', {
				conversationId: 'conv-1',
				title: 'New Thread'
			});
			expect(result.id).toBe('thread-new');
		});
	});

	describe('resolveThread', () => {
		it('should mark thread as resolved', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await resolveThread('thread-1', true);

			expect(mockInvoke).toHaveBeenCalledWith('resolve_thread', {
				id: 'thread-1',
				resolved: true
			});
		});

		it('should reopen thread', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await resolveThread('thread-1', false);

			expect(mockInvoke).toHaveBeenCalledWith('resolve_thread', {
				id: 'thread-1',
				resolved: false
			});
		});
	});

	describe('deleteThread', () => {
		it('should delete a thread', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await deleteThread('thread-1');

			expect(mockInvoke).toHaveBeenCalledWith('delete_thread', { id: 'thread-1' });
		});
	});

	describe('renameThread', () => {
		it('should rename a thread', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'thread-1',
				conversationId: 'conv-1',
				parentId: null,
				title: 'Updated Title',
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T02:00:00Z',
				messageCount: 5,
				metadata: {
					resolved: false,
					tags: [],
					priority: 'medium' as const
				}
			});

			const result = await renameThread('thread-1', 'Updated Title');

			expect(mockInvoke).toHaveBeenCalledWith('rename_thread', {
				id: 'thread-1',
				title: 'Updated Title'
			});
			expect(result.title).toBe('Updated Title');
		});
	});

	describe('setThreadAgentSession', () => {
		it('should set agent session for thread', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await setThreadAgentSession('thread-1', 'session-123');

			expect(mockInvoke).toHaveBeenCalledWith('set_thread_agent_session', {
				id: 'thread-1',
				agentSessionId: 'session-123'
			});
		});

		it('should clear agent session for thread', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await setThreadAgentSession('thread-1', null);

			expect(mockInvoke).toHaveBeenCalledWith('set_thread_agent_session', {
				id: 'thread-1',
				agentSessionId: null
			});
		});
	});

	// ============================================================================
	// Message Commands
	// ============================================================================

	describe('loadMessages', () => {
		it('should load messages for a conversation', async () => {
			const mockMessages = [
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user' as const,
					content: 'Hello',
					timestamp: '2024-01-01T00:00:00Z',
					status: 'sent' as const
				},
				{
					id: 'msg-2',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'assistant' as const,
					content: 'Hi there!',
					timestamp: '2024-01-01T00:01:00Z',
					status: 'delivered' as const
				}
			];
			mockInvoke.mockResolvedValueOnce(mockMessages);

			const result = await loadMessages('conv-1');

			expect(mockInvoke).toHaveBeenCalledWith('load_messages', {
				conversationId: 'conv-1'
			});
			expect(result).toHaveLength(2);
			expect(result[0].role).toBe('user');
			expect(result[1].role).toBe('assistant');
		});
	});

	describe('loadThreadMessages', () => {
		it('should load messages for a thread', async () => {
			const mockMessages = [
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user' as const,
					content: 'Hello',
					timestamp: '2024-01-01T00:00:00Z',
					status: 'sent' as const
				}
			];
			mockInvoke.mockResolvedValueOnce(mockMessages);

			const result = await loadThreadMessages('thread-1');

			expect(mockInvoke).toHaveBeenCalledWith('load_thread_messages', {
				threadId: 'thread-1'
			});
			expect(result).toHaveLength(1);
		});
	});

	describe('sendMessage', () => {
		it('should send a message', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'msg-new',
				conversationId: 'conv-1',
				threadId: 'thread-1',
				role: 'user' as const,
				content: 'Hello',
				timestamp: '2024-01-01T00:00:00Z',
				status: 'sent' as const
			});

			const result = await sendMessage({
				conversationId: 'conv-1',
				threadId: 'thread-1',
				role: 'user',
				content: 'Hello'
			});

			expect(mockInvoke).toHaveBeenCalledWith('send_message', {
				conversationId: 'conv-1',
				content: 'Hello',
				role: 'user',
				threadId: 'thread-1'
			});
			expect(result.id).toBe('msg-new');
		});

		it('should support assistant messages', async () => {
			mockInvoke.mockResolvedValueOnce({
				id: 'msg-2',
				conversationId: 'conv-1',
				threadId: 'thread-1',
				role: 'assistant' as const,
				content: 'Response',
				timestamp: '2024-01-01T00:01:00Z',
				status: 'delivered' as const
			});

			const result = await sendMessage({
				conversationId: 'conv-1',
				threadId: 'thread-1',
				role: 'assistant',
				content: 'Response'
			});

			expect(result.role).toBe('assistant');
		});
	});

	describe('createThreadWithMessage', () => {
		it('should create thread with message atomically', async () => {
			mockInvoke.mockResolvedValueOnce({
				thread: {
					id: 'thread-new',
					conversationId: 'conv-1',
					parentId: null,
					title: 'New Thread',
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T00:00:00Z',
					messageCount: 1,
					metadata: {
						resolved: false,
						tags: [],
						priority: 'medium' as const
					}
				},
				message: {
					id: 'msg-new',
					conversationId: 'conv-1',
					threadId: 'thread-new',
					role: 'user' as const,
					content: 'Hello',
					timestamp: '2024-01-01T00:00:00Z',
					status: 'sent' as const
				}
			});

			const result = await createThreadWithMessage({
				conversationId: 'conv-1',
				threadTitle: 'New Thread',
				content: 'Hello',
				role: 'user'
			});

			expect(mockInvoke).toHaveBeenCalledWith('create_thread_with_message', {
				conversationId: 'conv-1',
				threadTitle: 'New Thread',
				content: 'Hello',
				role: 'user'
			});
			expect(result.thread.id).toBe('thread-new');
			expect(result.message.id).toBe('msg-new');
		});
	});

	describe('deleteMessage', () => {
		it('should delete a message', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await deleteMessage('msg-1');

			expect(mockInvoke).toHaveBeenCalledWith('delete_message', { id: 'msg-1' });
		});
	});

	describe('updateMessageStatus', () => {
		it('should update message status', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await updateMessageStatus('msg-1', 'delivered');

			expect(mockInvoke).toHaveBeenCalledWith('update_message_status', {
				id: 'msg-1',
				status: 'delivered'
			});
		});

		it('should support all status values', async () => {
			const statuses: Array<'sending' | 'sent' | 'delivered' | 'error'> = [
				'sending',
				'sent',
				'delivered',
				'error'
			];

			for (const status of statuses) {
				mockInvoke.mockResolvedValueOnce(undefined);
				await updateMessageStatus('msg-1', status);
				expect(mockInvoke).toHaveBeenLastCalledWith('update_message_status', {
					id: 'msg-1',
					status
				});
			}
		});
	});

	describe('updateMessageFeedback', () => {
		it('should set like feedback', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await updateMessageFeedback('msg-1', 'like');

			expect(mockInvoke).toHaveBeenCalledWith('update_message_feedback', {
				id: 'msg-1',
				feedback: 'like'
			});
		});

		it('should set dislike feedback', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await updateMessageFeedback('msg-1', 'dislike');

			expect(mockInvoke).toHaveBeenCalledWith('update_message_feedback', {
				id: 'msg-1',
				feedback: 'dislike'
			});
		});

		it('should clear feedback', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await updateMessageFeedback('msg-1', null);

			expect(mockInvoke).toHaveBeenCalledWith('update_message_feedback', {
				id: 'msg-1',
				feedback: null
			});
		});
	});

	// ============================================================================
	// Conversion Helpers Tests
	// ============================================================================

	describe('Date conversion', () => {
		it('should handle null threadId in messages', async () => {
			mockInvoke.mockResolvedValueOnce([
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: null,
					role: 'user' as const,
					content: 'Hello',
					timestamp: '2024-01-01T00:00:00Z',
					status: 'sent' as const
				}
			]);

			const result = await loadMessages('conv-1');

			expect(result[0].threadId).toBe('');
		});

		it('should convert thread parentId from null', async () => {
			mockInvoke.mockResolvedValueOnce([
				{
					id: 'thread-1',
					conversationId: 'conv-1',
					parentId: null,
					title: 'Thread',
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T01:00:00Z',
					messageCount: 0,
					metadata: {
						resolved: false,
						tags: [],
						priority: 'medium' as const
					}
				}
			]);

			const result = await loadThreads('conv-1');

			expect(result[0].parentId).toBeNull();
		});

		it('should preserve thread parentId with value', async () => {
			mockInvoke.mockResolvedValueOnce([
				{
					id: 'thread-2',
					conversationId: 'conv-1',
					parentId: 'thread-1',
					title: 'Child Thread',
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T01:00:00Z',
					messageCount: 0,
					metadata: {
						resolved: false,
						tags: [],
						priority: 'medium' as const
					}
				}
			]);

			const result = await loadThreads('conv-1');

			expect(result[0].parentId).toBe('thread-1');
		});
	});

	describe('Metadata conversion', () => {
		it('should handle missing feedback in message metadata', async () => {
			mockInvoke.mockResolvedValueOnce([
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'assistant' as const,
					content: 'Response',
					timestamp: '2024-01-01T00:00:00Z',
					status: 'delivered' as const,
					metadata: {
						model: 'gpt-4',
						tokens: 50
					}
				}
			]);

			const result = await loadMessages('conv-1');

			expect(result[0].metadata?.model).toBe('gpt-4');
			expect(result[0].metadata?.tokens).toBe(50);
			expect(result[0].metadata?.feedback).toBeUndefined();
		});

		it('should handle missing metadata', async () => {
			mockInvoke.mockResolvedValueOnce([
				{
					id: 'msg-1',
					conversationId: 'conv-1',
					threadId: 'thread-1',
					role: 'user' as const,
					content: 'Hello',
					timestamp: '2024-01-01T00:00:00Z',
					status: 'sent' as const
				}
			]);

			const result = await loadMessages('conv-1');

			expect(result[0].metadata).toBeUndefined();
		});

		it('should handle thread metadata with all fields', async () => {
			mockInvoke.mockResolvedValueOnce([
				{
					id: 'thread-1',
					conversationId: 'conv-1',
					parentId: null,
					title: 'Thread',
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T01:00:00Z',
					messageCount: 5,
					metadata: {
						resolved: false,
						tags: ['bug', 'high-priority'],
						priority: 'high' as const,
						intent: 'debug',
						summary: 'Fix login bug',
						agentSessionId: 'session-123'
					}
				}
			]);

			const result = await loadThreads('conv-1');

			expect(result[0].metadata.resolved).toBe(false);
			expect(result[0].metadata.tags).toEqual(['bug', 'high-priority']);
			expect(result[0].metadata.priority).toBe('high');
			expect(result[0].metadata.intent).toBe('debug');
			expect(result[0].metadata.summary).toBe('Fix login bug');
			expect(result[0].metadata.agentSessionId).toBe('session-123');
		});
	});
});
