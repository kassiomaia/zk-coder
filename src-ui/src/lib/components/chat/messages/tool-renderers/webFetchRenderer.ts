/**
 * Web-fetch renderer for the three aliases `web_request` / `web_fetch` / `webfetch`.
 */
import { Globe } from 'lucide-svelte';
import type { AnyToolRenderer } from './types';

interface WebFetchArgs {
	url?: string;
}

export const webFetchRenderer: AnyToolRenderer = {
	icon: Globe,
	getSummary: (args) => {
		const a = args as WebFetchArgs | undefined;
		return { title: 'Fetched URL', subtitle: a?.url };
	},
	getOutcome: (_result, status) => {
		if (status === 'pending') return 'Executing...';
		if (status === 'error') return 'Action failed';
		return 'Completed';
	}
};
