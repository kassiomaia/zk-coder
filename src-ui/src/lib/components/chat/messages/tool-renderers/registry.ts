/**
 * Static lookup table for per-tool renderers.
 *
 * Why a static map and not a mutable `register()` API:
 * - No import-time side effects: each renderer module can be imported in
 *   isolation by component tests.
 * - Reviewers see the complete catalog in one place.
 * - Adding a tool is one import + one map entry — nothing else to grep for.
 *
 * Aliases (`read`, `write`, `glob`, `grep`, `bash`, `webfetch`) point at the
 * *same renderer instance* as the canonical name; tests rely on reference
 * equality to detect accidental divergence.
 */
import { agentPlanSubtasksRenderer } from './agentRenderers';
import { bashRenderer } from './bashRenderer';
import { defaultRenderer } from './defaultRenderer';
import {
	fsGlobRenderer,
	fsGrepRenderer,
	fsReadRenderer,
	fsWriteRenderer
} from './fsRenderers';
import {
	llmRequestRenderer,
	skillRenderer,
	taskRenderer
} from './miscRenderers';
import type { AnyToolRenderer } from './types';
import { webFetchRenderer } from './webFetchRenderer';
import { webSearchRenderer } from './webSearchRenderer';

const RENDERERS: Record<string, AnyToolRenderer> = {
	// fs — canonical + legacy aliases
	fs_read_file: fsReadRenderer,
	read_file: fsReadRenderer,
	read: fsReadRenderer,
	fs_write_file: fsWriteRenderer,
	write_file: fsWriteRenderer,
	write: fsWriteRenderer,
	fs_glob: fsGlobRenderer,
	glob: fsGlobRenderer,
	fs_grep: fsGrepRenderer,
	grep: fsGrepRenderer,

	// shell
	execute_command: bashRenderer,
	bash: bashRenderer,

	// web
	web_request: webFetchRenderer,
	web_fetch: webFetchRenderer,
	webfetch: webFetchRenderer,
	search_web_search: webSearchRenderer as unknown as AnyToolRenderer,

	// misc
	llm_request: llmRequestRenderer,
	skill: skillRenderer,
	task: taskRenderer,

	// agent orchestration
	agent_plan_subtasks: agentPlanSubtasksRenderer
};

/**
 * Returns the renderer registered for `toolName`, falling back to the
 * default renderer (which surfaces the raw tool name so the user still
 * has a clue what ran).
 */
export function getToolRenderer(toolName: string): AnyToolRenderer {
	return RENDERERS[toolName] ?? defaultRenderer;
}

/** Exposed for tests: returns the raw map so assertions can iterate it. */
export function _registryForTests(): Readonly<Record<string, AnyToolRenderer>> {
	return RENDERERS;
}

export { defaultRenderer };
