/**
 * Shared types mirroring the `search_web_search` tool's JSON contract.
 * Match `docs/tools/web_search/spec.md §4.4` exactly — any change here is
 * a wire-format break and needs a paired change on the Rust side.
 */

export interface WebSearchResult {
	title: string;
	url: string;
	snippet: string;
	source_domain: string;
}

export interface WebSearchOutput {
	query: string;
	count: number;
	results: WebSearchResult[];
	provider: 'google_custom_search';
	cached: boolean;
	filtered_by_policy?: boolean;
}

/** Every stable error code emitted by the tool; branches in the renderer. */
export type WebSearchErrorCode =
	| 'not_configured'
	| 'disabled_by_flag'
	| 'rate_limited'
	| 'unauthorized'
	| 'upstream_error'
	| 'invalid_arguments'
	| 'filtered_by_policy';

/** Shape produced by the tool on any failure. */
export interface WebSearchErrorBody {
	error: string;
	error_code?: WebSearchErrorCode;
	retryable?: boolean;
}

/** Narrowing helper used by the renderer + body component. */
export function isWebSearchError(
	value: unknown
): value is WebSearchErrorBody {
	return (
		typeof value === 'object' &&
		value !== null &&
		'error' in value &&
		typeof (value as { error: unknown }).error === 'string'
	);
}

export interface WebSearchArgs {
	query?: string;
	num_results?: number;
	language?: string;
	site?: string;
}
