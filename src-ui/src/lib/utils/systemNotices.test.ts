import { describe, expect, it } from 'vitest';
import { buildContextNoticeContent, parseSystemNoticeFromMessage } from './systemNotices';

describe('systemNotices', () => {
	it('builds subtask transition copy without message counts', () => {
		const content = buildContextNoticeContent({
			type: 'context_compacted',
			sessionId: 'session-1',
			conversationId: 'conversation-1',
			reason: 'subtask_transition',
			beforeCount: 16,
			afterCount: 17,
			beforeTokens: 538,
			afterTokens: 507,
			timestamp: '2026-03-27T12:00:00.000Z'
		});

		expect(content).toContain('Context updated');
		expect(content).toContain('current subtask');
		expect(content).toContain('538 -> 507 tokens');
		expect(content).not.toContain('16 -> 17 messages');
	});

	it('parses persisted system notices back into structured metadata', () => {
		const parsed = parseSystemNoticeFromMessage(
			'system',
			'Context compacted\n\nOlder context was condensed to preserve room for the next steps.\n\n1,024 -> 768 tokens'
		);

		expect(parsed).toEqual({
			type: 'context_update',
			reason: 'token_limit',
			beforeCount: 0,
			afterCount: 0,
			beforeTokens: 1024,
			afterTokens: 768
		});
	});

	it('omits the tokens tail when before and after are equal', () => {
		const content = buildContextNoticeContent({
			type: 'context_compacted',
			sessionId: 'session-1',
			conversationId: 'conversation-1',
			reason: 'subtask_transition',
			beforeCount: 10,
			afterCount: 10,
			beforeTokens: 967,
			afterTokens: 967,
			timestamp: '2026-04-14T12:00:00.000Z'
		});

		expect(content).toContain('Context updated');
		expect(content).not.toContain('tokens');
		expect(content).not.toContain('->');
	});

	it('parses notices without a tokens tail (new format)', () => {
		const parsed = parseSystemNoticeFromMessage(
			'system',
			'Context updated\n\nThe agent reorganized its working context for the current subtask.'
		);

		expect(parsed).toEqual({
			type: 'context_update',
			reason: 'subtask_transition',
			beforeCount: 0,
			afterCount: 0,
			beforeTokens: 0,
			afterTokens: 0
		});
	});
});
