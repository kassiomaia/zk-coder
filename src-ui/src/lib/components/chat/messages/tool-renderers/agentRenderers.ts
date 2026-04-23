/**
 * Renderers for `agent_*` tools — the LLM-facing orchestration primitives
 * (plan_subtasks, narrate, status, etc.). Only the tools whose argument
 * shape includes arrays/objects live here; string-only agent tools still
 * fall through to the default renderer safely after the guard added to
 * `defaultRenderer.ts`.
 */
import { ListChecks } from 'lucide-svelte';
import type { AnyToolRenderer } from './types';

interface PlanSubtasksArgs {
	subtasks?: Array<{ title?: string }>;
}

export const agentPlanSubtasksRenderer: AnyToolRenderer = {
	icon: ListChecks,
	getSummary: (args) => {
		const a = args as PlanSubtasksArgs | undefined;
		const count = a?.subtasks?.length ?? 0;
		const first = a?.subtasks?.[0]?.title;
		let subtitle: string | undefined;
		if (count === 0) {
			subtitle = undefined;
		} else if (count === 1) {
			subtitle = first;
		} else {
			subtitle = first ? `${first} (+${count - 1} more)` : `${count} subtasks`;
		}
		return { title: 'Planned subtasks', subtitle };
	},
	getOutcome: (_result, status) => {
		if (status === 'pending') return 'Planning...';
		if (status === 'error') return 'Planning failed';
		return 'Plan ready';
	}
};
