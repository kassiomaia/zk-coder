/**
 * groupToolMessages — collapses consecutive tool_call messages sharing the
 * same non-undefined stepNumber into ToolGroup display nodes.
 *
 * Rules:
 * - Only `tool_call` messages with a defined `stepNumber` are candidates.
 * - Consecutive candidates with the SAME stepNumber form a run.
 * - Runs of length ≥ 2 are emitted as a ToolGroup.
 * - Runs of length 1 are emitted as the original Message unchanged.
 * - All non-tool_call messages pass through immediately without breaking runs
 *   — wait, actually non-tool_call messages DO break the run (different kind).
 */

import type { Message } from '$lib/types/message';

export interface ToolGroup {
	kind: 'tool_group';
	stepNumber: number;
	messages: Message[];
}

export type DisplayItem = Message | ToolGroup;

export function isToolGroup(item: DisplayItem): item is ToolGroup {
	return 'kind' in item && (item as ToolGroup).kind === 'tool_group';
}

/**
 * Returns a human-readable label for a group of tool_call messages.
 * Inspects `agentData.toolName` across all messages to produce a
 * category-specific phrase, falling back to "Ran N actions" for mixed groups.
 */
export function getGroupLabel(messages: Message[]): string {
	const n = messages.length;

	const readTools = new Set(['read', 'fs_read_file', 'read_file']);
	const writeTools = new Set(['write', 'edit', 'fs_write_file', 'edit_file', 'write_file']);
	const execTools = new Set(['execute_command', 'bash']);
	const searchTools = new Set(['glob', 'grep', 'fs_glob', 'fs_grep']);

	let category: 'read' | 'write' | 'exec' | 'search' | 'mixed' | null = null;

	for (const msg of messages) {
		const tool = msg.agentData?.toolName ?? '';

		let thisCat: 'read' | 'write' | 'exec' | 'search' | 'mixed';
		if (readTools.has(tool)) {
			thisCat = 'read';
		} else if (writeTools.has(tool)) {
			thisCat = 'write';
		} else if (execTools.has(tool)) {
			thisCat = 'exec';
		} else if (searchTools.has(tool)) {
			thisCat = 'search';
		} else {
			thisCat = 'mixed';
		}

		if (category === null) {
			category = thisCat;
		} else if (category !== thisCat) {
			category = 'mixed';
			break;
		}
	}

	switch (category) {
		case 'read':
			return `Read ${n} file${n !== 1 ? 's' : ''}`;
		case 'write':
			return `Modified ${n} file${n !== 1 ? 's' : ''}`;
		case 'exec':
			return `Ran ${n} command${n !== 1 ? 's' : ''}`;
		case 'search':
			return `Searched ${n} pattern${n !== 1 ? 's' : ''}`;
		default:
			return `Ran ${n} action${n !== 1 ? 's' : ''}`;
	}
}

/**
 * Collapses consecutive tool_call messages with the same non-undefined
 * stepNumber into ToolGroup nodes. All other messages pass through unchanged.
 * Groups of size 1 are NOT created — single tool calls render as before.
 */
export function groupToolMessages(messages: Message[]): DisplayItem[] {
	const result: DisplayItem[] = [];

	let runStep: number | undefined = undefined;
	let runBuffer: Message[] = [];

	function flushRun() {
		if (runBuffer.length === 0) return;
		if (runBuffer.length >= 2 && runStep !== undefined) {
			result.push({
				kind: 'tool_group',
				stepNumber: runStep,
				messages: [...runBuffer]
			});
		} else {
			// Single item — emit unchanged
			for (const msg of runBuffer) {
				result.push(msg);
			}
		}
		runBuffer = [];
		runStep = undefined;
	}

	for (const message of messages) {
		const isToolCall = message.kind === 'tool_call';
		const step = message.agentData?.stepNumber;

		if (isToolCall && step !== undefined) {
			if (runStep === undefined) {
				// Start a new run
				runStep = step;
				runBuffer.push(message);
			} else if (step === runStep) {
				// Continue the current run
				runBuffer.push(message);
			} else {
				// Different stepNumber — flush current run and start a new one
				flushRun();
				runStep = step;
				runBuffer.push(message);
			}
		} else {
			// Not a groupable tool_call — flush any pending run first
			flushRun();
			result.push(message);
		}
	}

	// Flush any trailing run
	flushRun();

	return result;
}
