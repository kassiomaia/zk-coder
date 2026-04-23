import { beforeEach, describe, expect, it } from 'vitest';
import type { Message } from '$lib/types/message';
import { getGroupLabel, groupToolMessages, isToolGroup } from './groupToolMessages';

function msg(overrides: Partial<Message> & { id: string }): Message {
	return {
		id: overrides.id,
		conversationId: 'conv-1',
		threadId: 'thread-1',
		role: 'assistant',
		content: overrides.content ?? '',
		timestamp: new Date(0),
		...overrides
	};
}

function toolCall(id: string, stepNumber: number, toolName = 'read', content = ''): Message {
	return msg({
		id,
		role: 'assistant',
		kind: 'tool_call',
		content,
		agentData: { stepNumber, toolName }
	});
}

// Reset cross-test memoization by feeding an empty input once.
beforeEach(() => {
	groupToolMessages([]);
});

describe('groupToolMessages — grouping semantics', () => {
	it('returns empty for empty input', () => {
		expect(groupToolMessages([])).toEqual([]);
	});

	it('passes non-tool_call messages through unchanged', () => {
		const input = [msg({ id: 'a', content: 'hello', role: 'user' })];
		const output = groupToolMessages(input);
		expect(output).toHaveLength(1);
		expect(output[0]).toBe(input[0]);
	});

	it('passes a single tool_call through unchanged (no group)', () => {
		const input = [toolCall('t1', 1)];
		const output = groupToolMessages(input);
		expect(output).toHaveLength(1);
		expect(isToolGroup(output[0])).toBe(false);
		expect(output[0]).toBe(input[0]);
	});

	it('groups two consecutive tool_calls sharing a stepNumber', () => {
		const input = [toolCall('t1', 1), toolCall('t2', 1)];
		const output = groupToolMessages(input);
		expect(output).toHaveLength(1);
		expect(isToolGroup(output[0])).toBe(true);
		if (isToolGroup(output[0])) {
			expect(output[0].stepNumber).toBe(1);
			expect(output[0].messages).toHaveLength(2);
		}
	});

	it('does not group tool_calls with different stepNumbers', () => {
		const input = [toolCall('t1', 1), toolCall('t2', 2)];
		const output = groupToolMessages(input);
		expect(output).toHaveLength(2);
		expect(isToolGroup(output[0])).toBe(false);
		expect(isToolGroup(output[1])).toBe(false);
	});

	it('breaks a run on a non-tool_call in the middle', () => {
		const input = [
			toolCall('t1', 1),
			msg({ id: 'u', content: 'interrupt', role: 'user' }),
			toolCall('t2', 1)
		];
		const output = groupToolMessages(input);
		expect(output).toHaveLength(3);
		expect(output.every((item) => !isToolGroup(item))).toBe(true);
	});

	it('ignores tool_calls without stepNumber', () => {
		const input = [
			msg({ id: 't1', kind: 'tool_call', agentData: { toolName: 'read' } }),
			msg({ id: 't2', kind: 'tool_call', agentData: { toolName: 'read' } })
		];
		const output = groupToolMessages(input);
		expect(output).toHaveLength(2);
		expect(output.every((item) => !isToolGroup(item))).toBe(true);
	});
});

describe('groupToolMessages — memoization', () => {
	it('reuses output object identity for unchanged prefix', () => {
		const first = [msg({ id: 'a', content: 'hi' }), msg({ id: 'b', content: 'there' })];
		const out1 = groupToolMessages(first);

		// Append a new message — the prefix outputs should be reference-equal.
		const second = [...first, msg({ id: 'c', content: 'new' })];
		const out2 = groupToolMessages(second);

		expect(out2).toHaveLength(3);
		expect(out2[0]).toBe(out1[0]);
		expect(out2[1]).toBe(out1[1]);
	});

	it('reuses tool group identity when its source messages are unchanged', () => {
		const group = [toolCall('t1', 1, 'read'), toolCall('t2', 1, 'read')];
		const out1 = groupToolMessages(group);
		expect(isToolGroup(out1[0])).toBe(true);

		// Append a new unrelated message.
		const out2 = groupToolMessages([...group, msg({ id: 'x', content: 'later' })]);
		expect(out2).toHaveLength(2);
		expect(out2[0]).toBe(out1[0]);
	});

	it('invalidates the tail when the last message content changes (streaming)', () => {
		const initial = [msg({ id: 'a', content: 'stable' }), msg({ id: 'streaming', content: 'He' })];
		const out1 = groupToolMessages(initial);

		const growing = [
			initial[0],
			msg({ id: 'streaming', content: 'Hello' }) // same id, new content
		];
		const out2 = groupToolMessages(growing);

		expect(out2[0]).toBe(out1[0]); // stable prefix preserved
		expect(out2[1]).not.toBe(out1[1]); // streaming message reprocessed
		expect((out2[1] as Message).content).toBe('Hello');
	});

	it('discards all prior entries when the first message changes', () => {
		groupToolMessages([msg({ id: 'a', content: 'A' })]);
		const out = groupToolMessages([msg({ id: 'b', content: 'B' })]);
		expect(out).toHaveLength(1);
		expect((out[0] as Message).id).toBe('b');
	});
});

describe('getGroupLabel', () => {
	it('labels read-only groups', () => {
		expect(getGroupLabel([toolCall('a', 1, 'read'), toolCall('b', 1, 'read')])).toBe('Read 2 files');
	});

	it('labels write groups', () => {
		expect(getGroupLabel([toolCall('a', 1, 'write'), toolCall('b', 1, 'edit')])).toBe(
			'Modified 2 files'
		);
	});

	it('labels exec groups', () => {
		expect(getGroupLabel([toolCall('a', 1, 'execute_command')])).toBe('Ran 1 command');
	});

	it('labels search groups', () => {
		expect(getGroupLabel([toolCall('a', 1, 'grep'), toolCall('b', 1, 'glob')])).toBe(
			'Searched 2 patterns'
		);
	});

	it('labels mixed groups', () => {
		expect(getGroupLabel([toolCall('a', 1, 'read'), toolCall('b', 1, 'write')])).toBe(
			'Ran 2 actions'
		);
	});
});
