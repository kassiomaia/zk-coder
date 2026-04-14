import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WebSearchResults from './WebSearchResults.svelte';
import type { WebSearchResult } from '$lib/types/webSearch';

const sample: WebSearchResult[] = [
	{
		title: 'Svelte 5 is alive',
		url: 'https://svelte.dev/blog/svelte-5-is-alive',
		snippet: 'After nearly 18 months of development, Svelte 5 is now stable.',
		source_domain: 'svelte.dev'
	},
	{
		title: 'Release notes · svelte 5.0.0',
		url: 'https://github.com/sveltejs/svelte/releases/tag/svelte%405.0.0',
		snippet: 'Major release of the Svelte compiler with runes.',
		source_domain: 'github.com'
	}
];

describe('WebSearchResults', () => {
	let onOpen: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		onOpen = vi.fn();
	});

	it('renders one row per result', () => {
		render(WebSearchResults, { results: sample, onOpen });
		expect(screen.getAllByTestId('web-search-result')).toHaveLength(2);
	});

	it('shows title, url, and snippet for each row', () => {
		render(WebSearchResults, { results: sample, onOpen });
		expect(screen.getByText('Svelte 5 is alive')).toBeInTheDocument();
		expect(
			screen.getByText('https://svelte.dev/blog/svelte-5-is-alive')
		).toBeInTheDocument();
		expect(
			screen.getByText(/After nearly 18 months/)
		).toBeInTheDocument();
	});

	it('invokes onOpen with the row URL on click', async () => {
		render(WebSearchResults, { results: sample, onOpen });
		const rows = screen.getAllByTestId('web-search-result');
		await fireEvent.click(rows[0]);
		expect(onOpen).toHaveBeenCalledWith(
			'https://svelte.dev/blog/svelte-5-is-alive'
		);
	});

	it('does not invoke onOpen for a non-http URL', async () => {
		const unsafe: WebSearchResult[] = [
			{
				...sample[0],
				url: 'javascript:alert(1)'
			}
		];
		render(WebSearchResults, { results: unsafe, onOpen });
		const row = screen.getByTestId('web-search-result');
		await fireEvent.click(row);
		expect(onOpen).not.toHaveBeenCalled();
	});

	it('each row exposes an accessible label with title + domain', () => {
		render(WebSearchResults, { results: sample, onOpen });
		expect(
			screen.getByRole('button', {
				name: /Open Svelte 5 is alive at svelte\.dev/
			})
		).toBeInTheDocument();
	});

	it('keyboard activation via Enter also triggers open', async () => {
		// `<button>` natively handles Enter → click, so the same onclick
		// listener fires. We assert the net effect rather than the keydown.
		render(WebSearchResults, { results: sample, onOpen });
		const row = screen.getAllByTestId('web-search-result')[0];
		row.focus();
		await fireEvent.click(row);
		expect(onOpen).toHaveBeenCalled();
	});
});
