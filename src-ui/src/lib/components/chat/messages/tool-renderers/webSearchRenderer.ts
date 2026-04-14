/**
 * Renderer for the `search_web_search` tool.
 *
 * The header title stays "Searched web" regardless of status (the header
 * spinner already signals "in flight"), matching the spec decision in
 * plan.md Appendix B.2.1. The custom `BodyComponent` handles every
 * success / error / cached / filtered-by-policy branch.
 */
import { Globe } from 'lucide-svelte';
import type {
	WebSearchArgs,
	WebSearchErrorBody,
	WebSearchOutput
} from '$lib/types/webSearch';
import { isWebSearchError } from '$lib/types/webSearch';
import WebSearchBody from './WebSearchBody.svelte';
import type { ToolRenderer } from './types';

type Result = WebSearchOutput | WebSearchErrorBody;

export const webSearchRenderer: ToolRenderer<WebSearchArgs, Result> = {
	icon: Globe,
	getSummary: (args) => ({
		title: 'Searched web',
		subtitle: args?.query
	}),
	getOutcome: (result, status) => {
		if (status === 'pending') return 'Executing...';
		if (status === 'error' || isWebSearchError(result)) return 'Search failed';
		const out = result as WebSearchOutput | undefined;
		if (!out || out.count === 0) return 'No results';
		return out.cached
			? `Found ${out.count} results (cached)`
			: `Found ${out.count} results`;
	},
	BodyComponent: WebSearchBody
};
