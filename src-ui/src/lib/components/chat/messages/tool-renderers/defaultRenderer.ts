/**
 * Fallback renderer used when no registered renderer matches the tool name.
 * Shows the tool name as the title and the first *primitive* argument value
 * as the subtitle. Arrays and objects are skipped — rendering them inline
 * produces `[object Object],...` via Svelte's implicit string coercion; the
 * expanded Parameters panel already shows the full JSON via `JSON.stringify`.
 */
import { Code } from 'lucide-svelte';
import type { AnyToolRenderer } from './types';

function firstPrimitiveValue(args: unknown): string | undefined {
	if (!args || typeof args !== 'object') return undefined;
	const first = Object.values(args as Record<string, unknown>)[0];
	if (first === null || first === undefined) return undefined;
	const t = typeof first;
	if (t === 'string') return first as string;
	if (t === 'number' || t === 'boolean') return String(first);
	return undefined;
}

export const defaultRenderer: AnyToolRenderer = {
	icon: Code,
	getSummary: (args, toolName) => ({
		title: toolName,
		subtitle: firstPrimitiveValue(args)
	}),
	getOutcome: (_result, status) => {
		if (status === 'pending') return 'Executing...';
		if (status === 'error') return 'Action failed';
		return 'Completed';
	}
};
