import type {AgentEvent} from '$lib/types/agent';
import type {ContextUpdateNotice, MessageRole, SystemNoticeReason} from '$lib/types/message';

type ContextCompactedEvent = Extract<AgentEvent, { type: 'context_compacted' }>;

interface NoticeCopy {
	title: string;
	body: string;
}

const CONTEXT_NOTICE_TITLES: Record<SystemNoticeReason, NoticeCopy> = {
	token_limit: {
		title: 'Context compacted',
		body: 'Older context was condensed to preserve room for the next steps.'
	},
	subtask_transition: {
		title: 'Context updated',
		body: 'The agent reorganized its working context for the current subtask.'
	}
};

export function getContextNoticeCopy(reason: SystemNoticeReason): NoticeCopy {
	return CONTEXT_NOTICE_TITLES[reason];
}

export function buildContextNoticeContent(event: ContextCompactedEvent): string {
	const copy = getContextNoticeCopy(event.reason);
	return `${copy.title}\n\n${copy.body}\n\n${event.beforeTokens.toLocaleString()} -> ${event.afterTokens.toLocaleString()} tokens`;
}

export function parseSystemNoticeFromMessage(
	role: MessageRole,
	content: string
): ContextUpdateNotice | undefined {
	if (role !== 'system') {
		return undefined;
	}

	const normalized = content.trim();
	const match = normalized.match(
		/^(Context updated|Context compacted)\n\n([\s\S]+?)\n\n([\d,]+) -> ([\d,]+) tokens$/u
	);

	if (!match) {
		return undefined;
	}

	const [, title, , beforeTokensRaw, afterTokensRaw] = match;
	const reason: SystemNoticeReason =
		title === 'Context compacted' ? 'token_limit' : 'subtask_transition';

	return {
		type: 'context_update',
		reason,
		beforeCount: 0,
		afterCount: 0,
		beforeTokens: parseLocaleNumber(beforeTokensRaw),
		afterTokens: parseLocaleNumber(afterTokensRaw)
	};
}

function parseLocaleNumber(value: string): number {
	return Number.parseInt(value.replaceAll(',', ''), 10);
}
