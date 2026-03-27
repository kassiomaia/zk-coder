import {render, screen} from '@testing-library/svelte';
import {describe, expect, it} from 'vitest';
import SystemNoticeMessage from './SystemNoticeMessage.svelte';

describe('SystemNoticeMessage', () => {
	it('renders the subtask transition copy and token delta', () => {
		render(SystemNoticeMessage, {
			message: {
				id: 'message-1',
				conversationId: 'conversation-1',
				threadId: 'thread-1',
				role: 'system',
				content:
					'Context updated\n\nThe agent reorganized its working context for the current subtask.\n\n538 -> 507 tokens',
				timestamp: new Date('2026-03-27T12:00:00.000Z'),
				status: 'sent',
				metadata: {
					systemNotice: {
						type: 'context_update',
						reason: 'subtask_transition',
						beforeCount: 16,
						afterCount: 17,
						beforeTokens: 538,
						afterTokens: 507
					}
				}
			}
		});

		expect(screen.getByLabelText('Context updated')).toBeInTheDocument();
		expect(
			screen.getByText('The agent reorganized its working context for the current subtask.')
		).toBeInTheDocument();
		expect(screen.getByText('538 -> 507 tokens')).toBeInTheDocument();
	});

	it('renders the token-limit copy when the notice reason is token_limit', () => {
		render(SystemNoticeMessage, {
			message: {
				id: 'message-2',
				conversationId: 'conversation-1',
				threadId: 'thread-1',
				role: 'system',
				content:
					'Context compacted\n\nOlder context was condensed to preserve room for the next steps.\n\n1,024 -> 768 tokens',
				timestamp: new Date('2026-03-27T12:01:00.000Z'),
				status: 'sent',
				metadata: {
					systemNotice: {
						type: 'context_update',
						reason: 'token_limit',
						beforeCount: 40,
						afterCount: 13,
						beforeTokens: 1024,
						afterTokens: 768
					}
				}
			}
		});

		expect(screen.getByLabelText('Context compacted')).toBeInTheDocument();
		expect(
			screen.getByText('Older context was condensed to preserve room for the next steps.')
		).toBeInTheDocument();
		expect(screen.getByText('1,024 -> 768 tokens')).toBeInTheDocument();
	});
});
