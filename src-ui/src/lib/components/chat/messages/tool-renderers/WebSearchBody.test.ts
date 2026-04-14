import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import WebSearchBody from './WebSearchBody.svelte';
import type {
	WebSearchErrorBody,
	WebSearchOutput
} from '$lib/types/webSearch';

// Mock the shell wrapper so rendering `WebSearchResults` inside the body
// doesn't try to reach the Tauri plugin in jsdom.
vi.mock('$lib/api/shell', () => ({
	openUrl: vi.fn(),
	isOpenableUrl: () => true
}));

function successBody(
	overrides: Partial<WebSearchOutput> = {}
): WebSearchOutput {
	return {
		query: 'rust',
		count: 2,
		results: [
			{
				title: 'Rust',
				url: 'https://rust-lang.org/',
				snippet: 'A language empowering everyone.',
				source_domain: 'rust-lang.org'
			},
			{
				title: 'async-trait',
				url: 'https://crates.io/crates/async-trait',
				snippet: 'Procedural macro.',
				source_domain: 'crates.io'
			}
		],
		provider: 'google_custom_search',
		cached: false,
		...overrides
	};
}

describe('WebSearchBody', () => {
	it('renders nothing while the call is pending', () => {
		render(WebSearchBody, {
			args: { query: 'x' },
			result: undefined,
			status: 'pending'
		});
		expect(
			screen.queryByTestId('web-search-results')
		).not.toBeInTheDocument();
	});

	it('renders results on a successful non-empty output', () => {
		render(WebSearchBody, {
			args: { query: 'rust' },
			result: successBody(),
			status: 'success'
		});
		expect(screen.getByTestId('web-search-results')).toBeInTheDocument();
		expect(screen.getAllByTestId('web-search-result')).toHaveLength(2);
	});

	it('renders the policy banner when filtered_by_policy and count > 0', () => {
		render(WebSearchBody, {
			args: { query: 'rust' },
			result: successBody({ filtered_by_policy: true }),
			status: 'success'
		});
		expect(screen.getByTestId('ws-banner-filtered')).toBeInTheDocument();
	});

	it('renders the policy banner when count is zero due to filtering', () => {
		render(WebSearchBody, {
			args: { query: 'rust' },
			result: successBody({ count: 0, results: [], filtered_by_policy: true }),
			status: 'success'
		});
		expect(screen.getByTestId('ws-banner-policy')).toBeInTheDocument();
		expect(screen.queryByTestId('web-search-results')).not.toBeInTheDocument();
	});

	it('renders the empty banner when count is zero and no policy filter', () => {
		render(WebSearchBody, {
			args: { query: 'rust' },
			result: successBody({ count: 0, results: [] }),
			status: 'success'
		});
		expect(screen.getByTestId('ws-banner-empty')).toBeInTheDocument();
	});

	it('renders the cached banner when cached=true', () => {
		render(WebSearchBody, {
			args: { query: 'rust' },
			result: successBody({ cached: true }),
			status: 'success'
		});
		expect(screen.getByTestId('ws-banner-cached')).toBeInTheDocument();
		expect(screen.getByTestId('web-search-results')).toBeInTheDocument();
	});

	it('renders the not-configured banner for that error code', () => {
		const err: WebSearchErrorBody = {
			error: 'Web search is not configured.',
			error_code: 'not_configured',
			retryable: false
		};
		render(WebSearchBody, {
			args: { query: 'rust' },
			result: err,
			status: 'error'
		});
		expect(
			screen.getByTestId('ws-banner-not-configured')
		).toBeInTheDocument();
		expect(
			screen.getByRole('button', { name: /Open settings/i })
		).toBeInTheDocument();
	});

	it('renders a generic error banner for other error codes', () => {
		const err: WebSearchErrorBody = {
			error: 'Web search quota exceeded.',
			error_code: 'rate_limited',
			retryable: true
		};
		render(WebSearchBody, {
			args: { query: 'rust' },
			result: err,
			status: 'error'
		});
		expect(screen.getByTestId('ws-banner-error')).toBeInTheDocument();
		expect(
			screen.queryByTestId('ws-banner-not-configured')
		).not.toBeInTheDocument();
	});
});
