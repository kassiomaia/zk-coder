import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WebSearchPolicy from './WebSearchPolicy.svelte';
import {
	getWorkspaceSearchPolicy,
	setWorkspaceSearchPolicy
} from '$lib/api/workspacePolicy';

vi.mock('$lib/api/workspacePolicy', () => ({
	getWorkspaceSearchPolicy: vi.fn(),
	setWorkspaceSearchPolicy: vi.fn(),
	deleteWorkspaceSearchPolicy: vi.fn()
}));

const mockGet = vi.mocked(getWorkspaceSearchPolicy);
const mockSet = vi.mocked(setWorkspaceSearchPolicy);

describe('WebSearchPolicy', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockGet.mockResolvedValue({ allowlist: [], denylist: [] });
		mockSet.mockResolvedValue(undefined);
	});

	async function renderPolicy(path = '/home/me/project') {
		const view = render(WebSearchPolicy, {
			workspacePath: path,
			workspaceDisplayPath: '~/project'
		});
		await waitFor(() => expect(mockGet).toHaveBeenCalled());
		return view;
	}

	it('renders empty lists for a new workspace', async () => {
		await renderPolicy();
		expect(screen.queryAllByRole('listitem')).toHaveLength(0);
	});

	it('shows a disabled empty state when no workspace is active', () => {
		render(WebSearchPolicy, { workspacePath: '' });
		expect(screen.getByText(/no workspace active/i)).toBeInTheDocument();
		expect(mockGet).not.toHaveBeenCalled();
	});

	it('adds a valid domain to the allowlist on Enter', async () => {
		await renderPolicy();
		const input = screen.getByTestId('allow-input');
		await fireEvent.input(input, { target: { value: 'github.com' } });
		await fireEvent.keyDown(input, { key: 'Enter' });
		await waitFor(() => {
			expect(screen.getByText('github.com')).toBeInTheDocument();
		});
	});

	it('rejects invalid domain syntax with an inline error', async () => {
		await renderPolicy();
		const input = screen.getByTestId('allow-input');
		await fireEvent.input(input, { target: { value: 'https://bad' } });
		await fireEvent.keyDown(input, { key: 'Enter' });
		expect(await screen.findByRole('alert')).toHaveTextContent(/scheme/i);
		// The chip should not be added.
		expect(screen.queryByText('https://bad')).not.toBeInTheDocument();
	});

	it('removes a domain via its chip dismiss button', async () => {
		mockGet.mockResolvedValue({
			allowlist: ['github.com', 'svelte.dev'],
			denylist: []
		});
		await renderPolicy();
		const remove = await screen.findByRole('button', {
			name: /Remove github\.com from allowlist/
		});
		await fireEvent.click(remove);
		await waitFor(() => {
			expect(
				screen.queryByRole('button', {
					name: /Remove github\.com from allowlist/
				})
			).not.toBeInTheDocument();
		});
	});

	it('warns when the same entry appears in both lists', async () => {
		mockGet.mockResolvedValue({
			allowlist: ['example.com'],
			denylist: ['example.com']
		});
		await renderPolicy();
		const warning = await screen.findByTestId('conflict-warning');
		expect(warning.textContent).toContain('example.com');
	});

	it('calls set_workspace_search_policy on save', async () => {
		await renderPolicy();
		const input = screen.getByTestId('allow-input');
		await fireEvent.input(input, { target: { value: 'github.com' } });
		await fireEvent.keyDown(input, { key: 'Enter' });
		const save = screen.getByRole('button', { name: /Save policy/ });
		await waitFor(() => expect(save).not.toBeDisabled());
		await fireEvent.click(save);
		await waitFor(() => {
			expect(mockSet).toHaveBeenCalledWith(
				'/home/me/project',
				['github.com'],
				[]
			);
		});
	});
});
