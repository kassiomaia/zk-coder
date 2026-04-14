/**
 * Tests for the Web Search Cache API wrappers.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	clearWebSearchCache,
	getWebSearchCacheStats
} from '../webSearchCache';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

const mockInvoke = vi.mocked(invoke);

describe('webSearchCache API', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('getWebSearchCacheStats', () => {
		it('returns entries + default TTL from the backend', async () => {
			mockInvoke.mockResolvedValue({ entries: 17, defaultTtlSecs: 86_400 });
			const stats = await getWebSearchCacheStats();
			expect(stats).toEqual({ entries: 17, defaultTtlSecs: 86_400 });
			expect(mockInvoke).toHaveBeenCalledWith(
				'web_search_cache_stats',
				undefined
			);
		});
	});

	describe('clearWebSearchCache', () => {
		it('clears everything when no workspace path is given', async () => {
			mockInvoke.mockResolvedValue(42);
			const removed = await clearWebSearchCache();
			expect(removed).toBe(42);
			expect(mockInvoke).toHaveBeenCalledWith('clear_web_search_cache', {
				workspacePath: null
			});
		});

		it('forwards a workspace path when provided', async () => {
			mockInvoke.mockResolvedValue(3);
			const removed = await clearWebSearchCache('/home/me/project');
			expect(removed).toBe(3);
			expect(mockInvoke).toHaveBeenCalledWith('clear_web_search_cache', {
				workspacePath: '/home/me/project'
			});
		});
	});
});
