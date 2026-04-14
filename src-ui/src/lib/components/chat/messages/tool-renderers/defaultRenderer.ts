/**
 * Fallback renderer used when no registered renderer matches the tool name.
 * Preserves the original behavior: show the tool name as the title and the
 * first argument value as the subtitle.
 */
import { Code } from 'lucide-svelte';
import type { AnyToolRenderer } from './types';

export const defaultRenderer: AnyToolRenderer = {
	icon: Code,
	getSummary: (args, toolName) => ({
		title: toolName,
		subtitle:
			args && typeof args === 'object'
				? (Object.values(args as Record<string, unknown>)[0] as string | undefined)
				: undefined
	}),
	getOutcome: (_result, status) => {
		if (status === 'pending') return 'Executing...';
		if (status === 'error') return 'Action failed';
		return 'Completed';
	}
};
