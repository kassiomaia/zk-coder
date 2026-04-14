/**
 * Tests for Tool Credentials API Client.
 *
 * Verifies command names and argument shapes — the backend command tests
 * in `commands/config/tool_credentials.rs` cover the semantics.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	deleteToolCredentials,
	getToolCredentials,
	hasToolCredentials,
	setToolCredentials,
	testWebSearchCredentials,
	WEB_SEARCH_TOOL_ID
} from '../toolCredentials';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

const mockInvoke = vi.mocked(invoke);

describe('toolCredentials API', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('WEB_SEARCH_TOOL_ID', () => {
		it('matches the Rust-side constant', () => {
			expect(WEB_SEARCH_TOOL_ID).toBe('search_web_search');
		});
	});

	describe('setToolCredentials', () => {
		it('invokes set_tool_credentials with flat args', async () => {
			mockInvoke.mockResolvedValue(undefined);
			await setToolCredentials(WEB_SEARCH_TOOL_ID, {
				apiKey: 'sk-abc',
				cx: 'cx-1'
			});
			expect(mockInvoke).toHaveBeenCalledWith('set_tool_credentials', {
				toolId: WEB_SEARCH_TOOL_ID,
				apiKey: 'sk-abc',
				cx: 'cx-1'
			});
		});
	});

	describe('getToolCredentials', () => {
		it('returns the stored value', async () => {
			mockInvoke.mockResolvedValue({ apiKey: 'sk', cx: 'cx' });
			const got = await getToolCredentials(WEB_SEARCH_TOOL_ID);
			expect(got).toEqual({ apiKey: 'sk', cx: 'cx' });
			expect(mockInvoke).toHaveBeenCalledWith('get_tool_credentials', {
				toolId: WEB_SEARCH_TOOL_ID
			});
		});

		it('passes through `null` on missing', async () => {
			mockInvoke.mockResolvedValue(null);
			const got = await getToolCredentials(WEB_SEARCH_TOOL_ID);
			expect(got).toBeNull();
		});
	});

	describe('deleteToolCredentials', () => {
		it('invokes delete_tool_credentials', async () => {
			mockInvoke.mockResolvedValue(undefined);
			await deleteToolCredentials(WEB_SEARCH_TOOL_ID);
			expect(mockInvoke).toHaveBeenCalledWith('delete_tool_credentials', {
				toolId: WEB_SEARCH_TOOL_ID
			});
		});
	});

	describe('hasToolCredentials', () => {
		it('returns the boolean from the backend', async () => {
			mockInvoke.mockResolvedValue(true);
			const has = await hasToolCredentials(WEB_SEARCH_TOOL_ID);
			expect(has).toBe(true);
			expect(mockInvoke).toHaveBeenCalledWith('has_tool_credentials', {
				toolId: WEB_SEARCH_TOOL_ID
			});
		});
	});

	describe('testWebSearchCredentials', () => {
		it('invokes the test command without args', async () => {
			mockInvoke.mockResolvedValue({
				valid: true,
				message: 'Credentials verified'
			});
			const result = await testWebSearchCredentials();
			expect(result.valid).toBe(true);
			expect(mockInvoke).toHaveBeenCalledWith(
				'test_web_search_credentials',
				undefined
			);
		});

		it('surfaces a validation failure without throwing', async () => {
			mockInvoke.mockResolvedValue({
				valid: false,
				message: 'Web search is not configured.'
			});
			const result = await testWebSearchCredentials();
			expect(result.valid).toBe(false);
			expect(result.message).toContain('not configured');
		});
	});
});
