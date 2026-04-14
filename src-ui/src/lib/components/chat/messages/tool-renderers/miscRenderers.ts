/**
 * Small one-shot renderers: `llm_request`, `skill`, `task`. Grouped into
 * one file because each one is only a handful of lines and splitting them
 * would add file-level noise without clarifying responsibilities.
 */
import { Code, Eye, PlayCircle } from 'lucide-svelte';
import type { AnyToolRenderer } from './types';

interface SkillArgs {
	name?: string;
}

interface TaskArgs {
	description?: string;
}

function completedOrStatus(status: string, done = 'Completed'): string {
	if (status === 'pending') return 'Executing...';
	if (status === 'error') return 'Action failed';
	return done;
}

export const llmRequestRenderer: AnyToolRenderer = {
	icon: Code,
	getSummary: (_args) => ({ title: 'LLM request' }),
	getOutcome: (_result, status) => completedOrStatus(status)
};

export const skillRenderer: AnyToolRenderer = {
	icon: PlayCircle,
	getSummary: (args) => {
		const a = args as SkillArgs | undefined;
		return { title: 'Loaded skill', subtitle: a?.name };
	},
	getOutcome: (_result, status) => completedOrStatus(status)
};

export const taskRenderer: AnyToolRenderer = {
	icon: Eye,
	getSummary: (args) => {
		const a = args as TaskArgs | undefined;
		return { title: 'Spawned agent', subtitle: a?.description };
	},
	getOutcome: (_result, status) => completedOrStatus(status)
};
