/**
 * Tests for Chat API Client
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { updateConversationSettings } from '../chat';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

import { invoke } from '@tauri-apps/api/core';

describe('chat API', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('updateConversationSettings', () => {
		it('should call invoke with correct command and args', async () => {
			const mockInvoke = vi.mocked(invoke);
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
			const mockInvoke = vi.mocked(invoke);
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
			const mockInvoke = vi.mocked(invoke);
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
			const mockInvoke = vi.mocked(invoke);
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
});
