/**
 * Pure-function tests for the `search_web_search` renderer. Keeps the
 * spec-frozen summary and outcome strings pinned so any copy tweak is a
 * deliberate change — test copy is part of the product.
 */
import { describe, expect, it } from 'vitest';
import { webSearchRenderer } from './webSearchRenderer';
import { getToolRenderer } from './registry';
import type { WebSearchErrorBody, WebSearchOutput } from '$lib/types/webSearch';

const successFive: WebSearchOutput = {
	query: 'rust',
	count: 5,
	results: [],
	provider: 'google_custom_search',
	cached: false
};

describe('webSearchRenderer', () => {
	it('is registered under the canonical tool name', () => {
		expect(getToolRenderer('search_web_search')).toBe(
			webSearchRenderer as unknown as ReturnType<typeof getToolRenderer>
		);
	});

	it('header title is "Searched web"', () => {
		const summary = webSearchRenderer.getSummary(
			{ query: "what's the latest svelte version?" },
			'search_web_search'
		);
		expect(summary.title).toBe('Searched web');
	});

	it('subtitle is the query string', () => {
		const summary = webSearchRenderer.getSummary(
			{ query: 'rust 1.78 release date' },
			'search_web_search'
		);
		expect(summary.subtitle).toBe('rust 1.78 release date');
	});

	it('outcome is "Executing..." when pending', () => {
		expect(webSearchRenderer.getOutcome(undefined, 'pending')).toBe('Executing...');
	});

	it('outcome is "No results" when count is zero', () => {
		expect(
			webSearchRenderer.getOutcome({ ...successFive, count: 0 }, 'success')
		).toBe('No results');
	});

	it('outcome is "Found N results" for a non-cached success', () => {
		expect(webSearchRenderer.getOutcome(successFive, 'success')).toBe(
			'Found 5 results'
		);
	});

	it('outcome appends "(cached)" when cached is true', () => {
		expect(
			webSearchRenderer.getOutcome({ ...successFive, cached: true }, 'success')
		).toBe('Found 5 results (cached)');
	});

	it('outcome is "Search failed" when status is error', () => {
		expect(webSearchRenderer.getOutcome(undefined, 'error')).toBe('Search failed');
	});

	it('treats error-shaped results as a failure even when status says success', () => {
		// A result body with an `error` field is surfaced as a failure body by
		// the Rust layer (see spec §4.4). The renderer must detect this shape
		// so the LLM's tool-call loop stays coherent regardless of `is_error`.
		const errorBody: WebSearchErrorBody = {
			error: 'Web search is not configured.',
			error_code: 'not_configured',
			retryable: false
		};
		expect(webSearchRenderer.getOutcome(errorBody, 'success')).toBe('Search failed');
	});

	it('registers a BodyComponent', () => {
		expect(webSearchRenderer.BodyComponent).toBeDefined();
	});
});
