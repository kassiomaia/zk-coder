/**
 * Tests for Chat Store - Conversation Settings
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the API module
vi.mock('$lib/api/chat', () => ({
	loadConversations: vi.fn(() => []),
	createConversation: vi.fn(),
	renameConversation: vi.fn(),
	deleteConversation: vi.fn(),
	updateConversationSettings: vi.fn(),
	loadThreads: vi.fn(() => []),
	createThread: vi.fn(),
	resolveThread: vi.fn(),
	deleteThread: vi.fn(),
	renameThread: vi.fn(),
	loadThreadMessages: vi.fn(() => []),
	sendMessage: vi.fn(),
	deleteMessage: vi.fn(),
	updateMessageStatus: vi.fn(),
	createThreadWithMessage: vi.fn()
}));

// Mock the NLP API module
vi.mock('$lib/api/nlp', () => ({
	classifyIntent: vi.fn(),
	addContextMessage: vi.fn(),
	detectAmbiguities: vi.fn(),
	resolveAmbiguity: vi.fn(),
	applyResolutions: vi.fn(),
	createDebouncedClassifier: vi.fn(() => ({ classify: vi.fn(), cancel: vi.fn() })),
	isBackendAvailable: vi.fn(() => Promise.resolve(false))
}));

import { updateConversationSettings } from '$lib/api/chat';

describe('chatStore - updateConversationSettings', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('API function', () => {
		it('should call updateConversationSettings API with correct parameters', async () => {
			const mockUpdate = vi.mocked(updateConversationSettings);
			mockUpdate.mockResolvedValueOnce({
				id: 'conv-123',
				title: 'Test',
				createdAt: new Date(),
				updatedAt: new Date(),
				settings: {
					model: 'new-model',
					temperature: 0.5,
					maxTokens: 2048,
					systemPrompt: undefined
				}
			});

			const result = await updateConversationSettings('conv-123', {
				model: 'new-model',
				temperature: 0.5,
				maxTokens: 2048
			});

			expect(mockUpdate).toHaveBeenCalledWith('conv-123', {
				model: 'new-model',
				temperature: 0.5,
				maxTokens: 2048
			});
			expect(result.settings.model).toBe('new-model');
		});

		it('should handle API errors', async () => {
			const mockUpdate = vi.mocked(updateConversationSettings);
			mockUpdate.mockRejectedValueOnce(new Error('Conversation not found'));

			await expect(
				updateConversationSettings('invalid-id', {
					model: 'test-model',
					temperature: 0.7,
					maxTokens: 4096
				})
			).rejects.toThrow();
		});
	});
});
