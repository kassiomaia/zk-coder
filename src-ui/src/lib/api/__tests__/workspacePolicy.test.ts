/**
 * Tests for the Workspace Search Policy API wrappers.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	deleteWorkspaceSearchPolicy,
	getWorkspaceSearchPolicy,
	setWorkspaceSearchPolicy
} from '../workspacePolicy';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

const mockInvoke = vi.mocked(invoke);

describe('workspacePolicy API', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('getWorkspaceSearchPolicy', () => {
		it('passes the workspace path and returns the stored lists', async () => {
			mockInvoke.mockResolvedValue({
				allowlist: ['github.com'],
				denylist: ['pinterest.com']
			});
			const policy = await getWorkspaceSearchPolicy('/home/me/project');
			expect(policy.allowlist).toEqual(['github.com']);
			expect(policy.denylist).toEqual(['pinterest.com']);
			expect(mockInvoke).toHaveBeenCalledWith(
				'get_workspace_search_policy',
				{ workspacePath: '/home/me/project' }
			);
		});

		it('returns empty lists when the backend reports no row', async () => {
			mockInvoke.mockResolvedValue({ allowlist: [], denylist: [] });
			const policy = await getWorkspaceSearchPolicy('/other');
			expect(policy.allowlist).toEqual([]);
			expect(policy.denylist).toEqual([]);
		});
	});

	describe('setWorkspaceSearchPolicy', () => {
		it('sends the lists verbatim', async () => {
			mockInvoke.mockResolvedValue(undefined);
			await setWorkspaceSearchPolicy(
				'/ws',
				['github.com', '.rust-lang.org'],
				['pinterest.com']
			);
			expect(mockInvoke).toHaveBeenCalledWith(
				'set_workspace_search_policy',
				{
					workspacePath: '/ws',
					allowlist: ['github.com', '.rust-lang.org'],
					denylist: ['pinterest.com']
				}
			);
		});

		it('accepts empty lists (clears both sides)', async () => {
			mockInvoke.mockResolvedValue(undefined);
			await setWorkspaceSearchPolicy('/ws', [], []);
			expect(mockInvoke).toHaveBeenCalledWith(
				'set_workspace_search_policy',
				{ workspacePath: '/ws', allowlist: [], denylist: [] }
			);
		});
	});

	describe('deleteWorkspaceSearchPolicy', () => {
		it('invokes delete_workspace_search_policy', async () => {
			mockInvoke.mockResolvedValue(undefined);
			await deleteWorkspaceSearchPolicy('/ws');
			expect(mockInvoke).toHaveBeenCalledWith(
				'delete_workspace_search_policy',
				{ workspacePath: '/ws' }
			);
		});
	});
});
