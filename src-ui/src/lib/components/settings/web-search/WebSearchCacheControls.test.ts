import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WebSearchCacheControls from './WebSearchCacheControls.svelte';
import {
	clearWebSearchCache,
	getWebSearchCacheStats
} from '$lib/api/webSearchCache';

vi.mock('$lib/api/webSearchCache', () => ({
	getWebSearchCacheStats: vi.fn(),
	clearWebSearchCache: vi.fn()
}));

const mockStats = vi.mocked(getWebSearchCacheStats);
const mockClear = vi.mocked(clearWebSearchCache);

describe('WebSearchCacheControls', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockStats.mockResolvedValue({ entries: 47, defaultTtlSecs: 86_400 });
		mockClear.mockResolvedValue(0);
	});

	async function renderCache(path = '/ws') {
		const view = render(WebSearchCacheControls, { workspacePath: path });
		await waitFor(() => expect(mockStats).toHaveBeenCalled());
		return view;
	}

	it('renders the entry count', async () => {
		await renderCache();
		await waitFor(() => {
			expect(screen.getByText('47')).toBeInTheDocument();
		});
	});

	it('formats the default TTL from seconds', async () => {
		mockStats.mockResolvedValue({ entries: 0, defaultTtlSecs: 86_400 });
		await renderCache();
		await waitFor(() => {
			expect(screen.getByText('1d')).toBeInTheDocument();
		});
	});

	it('calls clear_web_search_cache with undefined on Clear all', async () => {
		await renderCache();
		mockClear.mockResolvedValue(47);
		const all = screen.getByRole('button', { name: /Clear all/ });
		await fireEvent.click(all);
		await waitFor(() => {
			expect(mockClear).toHaveBeenCalledWith(undefined);
		});
	});

	it('calls clear_web_search_cache with the workspace path on Clear this workspace', async () => {
		await renderCache('/ws-a');
		mockClear.mockResolvedValue(3);
		const one = screen.getByRole('button', { name: /Clear this workspace/ });
		await fireEvent.click(one);
		await waitFor(() => {
			expect(mockClear).toHaveBeenCalledWith('/ws-a');
		});
	});

	it('shows the removed count after clear', async () => {
		await renderCache();
		mockStats.mockResolvedValueOnce({ entries: 0, defaultTtlSecs: 86_400 });
		mockClear.mockResolvedValue(12);
		await fireEvent.click(screen.getByRole('button', { name: /Clear all/ }));
		await waitFor(() => {
			const info = screen.getByTestId('cache-info');
			expect(info.textContent).toContain('12');
		});
	});

	it('disables the per-workspace button when no workspace is active', async () => {
		render(WebSearchCacheControls, { workspacePath: '' });
		await waitFor(() => expect(mockStats).toHaveBeenCalled());
		const one = screen.getByRole('button', { name: /Clear this workspace/ });
		expect(one).toBeDisabled();
	});
});
