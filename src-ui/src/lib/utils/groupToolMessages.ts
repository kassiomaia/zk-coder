/**
 * groupToolMessages — collapses consecutive tool_call messages sharing the
 * same non-undefined stepNumber into ToolGroup display nodes.
 *
 * Rules:
 * - Only `tool_call` messages with a defined `stepNumber` are candidates.
 * - Consecutive candidates with the SAME stepNumber form a run.
 * - Runs of length ≥ 2 are emitted as a ToolGroup.
 * - Runs of length 1 are emitted as the original Message unchanged.
 * - Non-tool_call messages break a run (pass through as-is).
 *
 * Memoization: callers invoke this function on every reactive update to the
 * `messages` array. During streaming that means once per token. To keep the
 * output reference-stable (so Svelte's keyed {#each} can skip rerendering
 * unchanged subtrees), we reuse prior output entries whose source messages
 * have not changed. Stability is checked by (id, content) pair — strings are
 * immutable in JS, so a content reference change is exactly what streaming
 * updates produce.
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
 * Pure grouping algorithm. Returns each display item paired with the ids of
 * the source messages that produced it, which lets the memoized wrapper know
 * how much of the input each output entry consumed.
 */
interface ComputedEntry {
	item: DisplayItem;
	sourceIds: string[];
}

function computeEntries(messages: Message[], startIndex: number): ComputedEntry[] {
	const entries: ComputedEntry[] = [];

	let runStep: number | undefined = undefined;
	let runBuffer: Message[] = [];

	function flushRun() {
		if (runBuffer.length === 0) return;
		if (runBuffer.length >= 2 && runStep !== undefined) {
			entries.push({
				item: {
					kind: 'tool_group',
					stepNumber: runStep,
					messages: [...runBuffer]
				},
				sourceIds: runBuffer.map((m) => m.id)
			});
		} else {
			for (const msg of runBuffer) {
				entries.push({ item: msg, sourceIds: [msg.id] });
			}
		}
		runBuffer = [];
		runStep = undefined;
	}

	for (let i = startIndex; i < messages.length; i++) {
		const message = messages[i];
		const isToolCall = message.kind === 'tool_call';
		const step = message.agentData?.stepNumber;

		if (isToolCall && step !== undefined) {
			if (runStep === undefined) {
				runStep = step;
				runBuffer.push(message);
			} else if (step === runStep) {
				runBuffer.push(message);
			} else {
				flushRun();
				runStep = step;
				runBuffer.push(message);
			}
		} else {
			flushRun();
			entries.push({ item: message, sourceIds: [message.id] });
		}
	}

	flushRun();
	return entries;
}

/**
 * Memoization state. Reset implicitly whenever the input prefix no longer
 * matches (different thread, edits, etc.), so there is no leak risk — the
 * cache is a performance hint, never a source of truth.
 */
let memoEntries: ComputedEntry[] = [];
let memoContents: Map<string, string> = new Map();

/**
 * Collapses consecutive tool_call messages with the same non-undefined
 * stepNumber into ToolGroup nodes. All other messages pass through unchanged.
 * Groups of size 1 are NOT created — single tool calls render as before.
 */
export function groupToolMessages(messages: Message[]): DisplayItem[] {
	// Walk prior entries; reuse each whose sources still match the head of the new input.
	const reusedEntries: ComputedEntry[] = [];
	let msgIndex = 0;

	for (const entry of memoEntries) {
		if (msgIndex + entry.sourceIds.length > messages.length) break;
		let allMatch = true;
		for (let k = 0; k < entry.sourceIds.length; k++) {
			const srcId = entry.sourceIds[k];
			const current = messages[msgIndex + k];
			if (current.id !== srcId || current.content !== memoContents.get(srcId)) {
				allMatch = false;
				break;
			}
		}
		if (!allMatch) break;
		reusedEntries.push(entry);
		msgIndex += entry.sourceIds.length;
	}

	// Compute the tail that didn't match.
	const tailEntries = msgIndex < messages.length ? computeEntries(messages, msgIndex) : [];

	// Rebuild memo to cover exactly the current input. Entries track source
	// messages in input order, so we walk messages in parallel rather than
	// searching for each id.
	const newEntries = reusedEntries.concat(tailEntries);
	memoEntries = newEntries;
	memoContents = new Map();
	let contentCursor = 0;
	for (const entry of newEntries) {
		for (let k = 0; k < entry.sourceIds.length; k++) {
			const msg = messages[contentCursor++];
			memoContents.set(msg.id, msg.content);
		}
	}

	return newEntries.map((e) => e.item);
}
