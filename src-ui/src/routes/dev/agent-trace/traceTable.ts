import type { ExecutionSnapshot, RuntimeTrace } from '$lib/types/agent';

export type TableRow = {
	id: string;
	time: string;
	rawTime: number;
	deltaMs: number | null;
	session: string;
	task: string;
	step: number;
	phase: string;
	transition: string;
	action: string;
	actor: 'LLM' | 'Tools' | 'Runtime';
	status: 'active' | 'completed' | 'failed' | 'cancelled';
	duration: string;
	tokens: string;
	tool: string;
	model: string;
	summary: string;
};

export const defaultWidths: Record<string, number> = {
	time: 100,
	delta: 84,
	step: 60,
	phase: 112,
	action: 176,
	actor: 80,
	status: 104,
	duration: 92,
	tokens: 108,
	tool: 132,
	summary: 420
};

type TracePayload = Record<string, unknown> & {
	to?: string;
	from?: string;
	phase?: string;
	durationMs?: number;
	totalTokens?: number;
	inputTokens?: number;
	outputTokens?: number;
	toolNames?: string[];
	tool?: string;
	model?: string;
	error?: string;
	reason?: string;
	message?: string;
};

export function formatTime(value: string): string {
	return new Date(value).toLocaleTimeString([], {
		hour12: false,
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit'
	});
}

export function summarizeTrace(trace: RuntimeTrace): string {
	const payload = trace.payloadJson;

	if (typeof payload === 'string') {
		return payload;
	}

	if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
		return trace.traceType;
	}

	const parsed = payload as TracePayload;

	if (typeof parsed.error === 'string') return parsed.error;
	if (typeof parsed.reason === 'string') return parsed.reason;
	if (typeof parsed.message === 'string') return parsed.message;
	if (Array.isArray(parsed.toolNames)) return parsed.toolNames.join(', ');
	if (typeof parsed.model === 'string') return parsed.model;

	return trace.traceType;
}

export function buildRow(
	trace: RuntimeTrace,
	index: number,
	allTraces: RuntimeTrace[],
	snapshot: ExecutionSnapshot | null
): TableRow {
	const payload =
		trace.payloadJson && typeof trace.payloadJson === 'object' && !Array.isArray(trace.payloadJson)
			? (trace.payloadJson as TracePayload)
			: null;
	const previousTrace = allTraces[index - 1];
	const deltaMs = previousTrace
		? new Date(trace.createdAt).getTime() - new Date(previousTrace.createdAt).getTime()
		: null;

	return {
		id: trace.id,
		time: formatTime(trace.createdAt),
		rawTime: new Date(trace.createdAt).getTime(),
		deltaMs,
		session: trace.sessionId,
		task: trace.taskId,
		step: trace.stepNumber,
		phase: payload?.to ?? payload?.phase ?? '-',
		transition:
			payload?.from || payload?.to ? `${payload?.from ?? '-'} → ${payload?.to ?? '-'}` : '-',
		action: trace.traceType,
		actor: trace.traceType.includes('llm')
			? 'LLM'
			: trace.traceType.includes('tool')
				? 'Tools'
				: 'Runtime',
		status: trace.traceType.includes('failed')
			? 'failed'
			: trace.traceType.includes('cancelled')
				? 'cancelled'
				: trace.traceType.includes('completed')
					? 'completed'
					: 'active',
		duration: payload?.durationMs ? `${payload.durationMs}ms` : '-',
		tokens: payload?.totalTokens
			? String(payload.totalTokens)
			: payload?.inputTokens
				? `${payload.inputTokens}/${payload.outputTokens ?? 0}`
				: '-',
		tool: Array.isArray(payload?.toolNames)
			? String(payload.toolNames[0] ?? '-')
			: (payload?.tool ?? '-'),
		model: payload?.model ?? snapshot?.lastModel ?? '-',
		summary: summarizeTrace(trace)
	};
}

export function mergeTrace(traces: RuntimeTrace[], nextTrace: RuntimeTrace): RuntimeTrace[] {
	if (traces.some((trace) => trace.id === nextTrace.id)) {
		return traces;
	}

	return [...traces, nextTrace].sort(
		(left, right) => new Date(left.createdAt).getTime() - new Date(right.createdAt).getTime()
	);
}

export function formatRelativeAge(timestamp: string | null, now: number): string {
	if (!timestamp) {
		return 'Waiting for events';
	}

	const elapsedMs = Math.max(0, now - new Date(timestamp).getTime());
	const seconds = Math.floor(elapsedMs / 1000);

	if (seconds < 1) return 'Just now';
	if (seconds < 60) return `${seconds}s ago`;

	const minutes = Math.floor(seconds / 60);
	if (minutes < 60) return `${minutes}m ago`;

	const hours = Math.floor(minutes / 60);
	return `${hours}h ago`;
}
