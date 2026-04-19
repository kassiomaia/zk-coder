/**
 * Shell-command renderer: `execute_command` and its `bash` alias.
 */
import { Terminal } from 'lucide-svelte';
import type { AnyToolRenderer } from './types';
import BashBody from '../BashBody.svelte';

interface BashArgs {
	command?: string;
}

interface BashResult {
	exit_code?: number;
}

export const bashRenderer: AnyToolRenderer = {
	icon: Terminal,
	getSummary: (args) => {
		const a = args as BashArgs | undefined;
		return { title: 'Ran command', subtitle: a?.command };
	},
	getOutcome: (result, status) => {
		if (status === 'pending') return 'Executing...';
		if (status === 'error') return 'Action failed';
		const res = result as BashResult | undefined;
		return res?.exit_code === 0
			? 'Command finished successfully'
			: `Failed with exit code ${res?.exit_code}`;
	},
	BodyComponent: BashBody
};
