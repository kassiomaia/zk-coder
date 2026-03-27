import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ApprovalMessage from './ApprovalMessage.svelte';
import type { Message } from '$lib/types/message';

vi.mock('$lib/components/agent/DiffViewer.svelte', () => ({
	default: vi.fn().mockImplementation(() => ({
		$$render: () => '<div data-testid="diff-viewer">diff</div>'
	}))
}));

function buildApprovalMessage(): Message {
	return {
		id: 'approval-action-1',
		conversationId: 'conv-1',
		threadId: 'thread-1',
		role: 'assistant',
		content: 'Need approval to execute command.',
		timestamp: new Date('2026-03-27T20:00:00Z'),
		kind: 'approval',
		agentData: {
			sessionId: 'session-1',
			pendingAction: {
				id: 'action-1',
				toolName: 'system_execute',
				actionType: {
					type: 'execute_command',
					command: 'ls -la'
				},
				description: 'Execute `ls -la` in the workspace.',
				arguments: {},
				createdAt: '2026-03-27T20:00:00Z'
			},
			approvalResolved: false
		}
	};
}

describe('ApprovalMessage', () => {
	it('calls onApprove with session and action ids', async () => {
		const onApprove = vi.fn().mockResolvedValue(undefined);

		render(ApprovalMessage, {
			message: buildApprovalMessage(),
			onApprove,
			onDeny: vi.fn()
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Allow' }));

		expect(onApprove).toHaveBeenCalledWith('session-1', 'action-1');
	});

	it('calls onDeny with session and action ids', async () => {
		const onDeny = vi.fn().mockResolvedValue(undefined);

		render(ApprovalMessage, {
			message: buildApprovalMessage(),
			onApprove: vi.fn(),
			onDeny
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Deny' }));

		expect(onDeny).toHaveBeenCalledWith('session-1', 'action-1');
	});
});
