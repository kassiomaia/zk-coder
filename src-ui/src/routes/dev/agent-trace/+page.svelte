<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { 
		Activity, 
		Clock, 
		Database, 
		Cpu, 
		Zap, 
		Filter, 
		ArrowDown, 
		Search,
		ChevronRight,
		Info,
		AlertCircle,
		CheckCircle2
	} from 'lucide-svelte';
	import type { AgentTraceEvent, ExecutionSnapshot, RuntimeTrace } from '$lib/types/agent';
	import MonacoEditor from '$lib/components/shared/MonacoEditor.svelte';
	import styles from './AgentTracePage.module.css';

	type DebugResponse = {
		snapshot: ExecutionSnapshot | null;
		traces: RuntimeTrace[];
	};

	type TableRow = {
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

	// State
	let snapshot = $state<ExecutionSnapshot | null>(null);
	let traces = $state<RuntimeTrace[]>([]);
	let selectedTraceId = $state<string | null>(null);
	let sessionId = $state('');
	let filterType = $state('');
	let filterStep = $state('');
	let filterPhase = $state('');
	let autoScroll = $state(true);
	let error = $state<string | null>(null);
	let traceContainer = $state<HTMLDivElement | undefined>(undefined);
	
	// Layout State
	let detailWidth = $state(browser ? Number(localStorage.getItem('agent-trace-detail-width')) || 450 : 450);
	let isResizing = $state(false);

	// Column Widths
	const defaultWidths: Record<string, number> = {
		time: 100,
		delta: 80,
		step: 60,
		phase: 100,
		action: 140,
		actor: 80,
		status: 100,
		duration: 90,
		tokens: 100,
		tool: 120,
		summary: 400
	};
	let columnWidths = $state<Record<string, number>>(
		browser ? JSON.parse(localStorage.getItem('agent-trace-column-widths') || '{}') : defaultWidths
	);

	// Ensure all defaults are present
	for (const key in defaultWidths) {
		if (!columnWidths[key]) columnWidths[key] = defaultWidths[key];
	}

	const isDev = import.meta.env.DEV;

	// Persistence
	$effect(() => {
		if (browser) {
			localStorage.setItem('agent-trace-detail-width', String(detailWidth));
			localStorage.setItem('agent-trace-column-widths', JSON.stringify(columnWidths));
		}
	});

	// Derived unique values for filters
	let uniqueTypes = $derived([...new Set(traces.map(t => t.traceType))].sort());
	let uniqueSteps = $derived([...new Set(traces.map(t => t.stepNumber))].sort((a, b) => a - b));
	let uniquePhases = $derived([...new Set(traces.map(t => {
		const p = t.payloadJson as any;
		return (p?.to) || (p?.phase) || '';
	}).filter(Boolean))].sort());

	function getInitialSessionId(): string {
		if (!browser) return '';
		return new URL(window.location.href).searchParams.get('sessionId') ?? '';
	}

	async function loadDebugData(targetSessionId: string) {
		if (!targetSessionId) return;
		try {
			const response = await invoke<DebugResponse>('get_agent_execution_debug', {
				sessionId: targetSessionId
			});
			sessionId = targetSessionId;
			snapshot = response.snapshot;
			traces = response.traces;
			if (!selectedTraceId && response.traces.length > 0) {
				selectedTraceId = response.traces[response.traces.length - 1].id;
			}
		} catch (loadError) {
			error = String(loadError);
		}
	}

	function formatTime(value: string): string {
		return new Date(value).toLocaleTimeString([], { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
	}

	function summarize(trace: RuntimeTrace): string {
		const payload = trace.payloadJson;
		if (typeof payload === 'string') return payload;
		if (!payload || typeof payload !== 'object' || Array.isArray(payload)) return trace.traceType;
		const p = payload as Record<string, unknown>;
		if (typeof p.error === 'string') return p.error;
		if (typeof p.reason === 'string') return p.reason;
		if (typeof p.message === 'string') return p.message;
		if (Array.isArray(p.toolNames)) return p.toolNames.join(', ');
		if (typeof p.model === 'string') return p.model;
		return trace.traceType;
	}

	function toRow(trace: RuntimeTrace, index: number, allTraces: RuntimeTrace[]): TableRow {
		const payload = trace.payloadJson as Record<string, any>;
		const prevTrace = allTraces[index - 1];
		const deltaMs = prevTrace ? new Date(trace.createdAt).getTime() - new Date(prevTrace.createdAt).getTime() : null;

		return {
			id: trace.id,
			time: formatTime(trace.createdAt),
			rawTime: new Date(trace.createdAt).getTime(),
			deltaMs,
			session: trace.sessionId,
			task: trace.taskId,
			step: trace.stepNumber,
			phase: payload?.to || payload?.phase || '-',
			transition: (payload?.from || payload?.to) ? `${payload?.from || '-'} → ${payload?.to || '-'}` : '-',
			action: trace.traceType,
			actor: trace.traceType.includes('llm') ? 'LLM' : trace.traceType.includes('tool') ? 'Tools' : 'Runtime',
			status: trace.traceType.includes('failed') ? 'failed' : 
					trace.traceType.includes('cancelled') ? 'cancelled' : 
					trace.traceType.includes('completed') ? 'completed' : 'active',
			duration: payload?.durationMs ? `${payload.durationMs}ms` : '-',
			tokens: payload?.totalTokens ? String(payload.totalTokens) : 
					payload?.inputTokens ? `${payload.inputTokens}/${payload.outputTokens || 0}` : '-',
			tool: Array.isArray(payload?.toolNames) ? String(payload.toolNames[0] || '-') : (payload?.tool || '-'),
			model: payload?.model || snapshot?.lastModel || '-',
			summary: summarize(trace)
		};
	}

	let rows = $derived(
		traces
			.map((t, i) => toRow(t, i, traces))
			.filter(r => !filterType || r.action === filterType)
			.filter(r => !filterStep || String(r.step) === filterStep)
			.filter(r => !filterPhase || r.phase === filterPhase)
	);

	let selectedTrace = $derived(traces.find((trace) => trace.id === selectedTraceId) ?? null);

	function selectTrace(id: string) {
		selectedTraceId = id;
	}

	function scrollToBottom() {
		if (autoScroll && traceContainer) {
			traceContainer.scrollTop = traceContainer.scrollHeight;
		}
	}

	$effect(() => {
		rows;
		queueMicrotask(scrollToBottom);
	});

	// Column Resizer Logic
	let activeResizeCol = $state<string | null>(null);
	let startX = 0;
	let startWidth = 0;

	function onColResizeStart(e: MouseEvent, col: string) {
		e.stopPropagation();
		activeResizeCol = col;
		startX = e.clientX;
		startWidth = columnWidths[col];
		document.addEventListener('mousemove', onColResizeMove);
		document.addEventListener('mouseup', onColResizeEnd);
	}

	function onColResizeMove(e: MouseEvent) {
		if (!activeResizeCol) return;
		const diff = e.clientX - startX;
		columnWidths[activeResizeCol] = Math.max(50, startWidth + diff);
	}

	function onColResizeEnd() {
		activeResizeCol = null;
		document.removeEventListener('mousemove', onColResizeMove);
		document.removeEventListener('mouseup', onColResizeEnd);
	}

	// Main resizer
	function handleMouseDown() {
		isResizing = true;
		document.body.style.cursor = 'col-resize';
		document.addEventListener('mousemove', handleMouseMove);
		document.addEventListener('mouseup', handleMouseUp);
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isResizing) return;
		detailWidth = Math.max(250, window.innerWidth - e.clientX);
	}

	function handleMouseUp() {
		isResizing = false;
		document.body.style.cursor = '';
		document.removeEventListener('mousemove', handleMouseMove);
		document.removeEventListener('mouseup', handleMouseUp);
	}

	onMount(() => {
		if (!isDev) {
			error = 'The trace window is only available in development mode.';
			return undefined;
		}

		let unlistenTrace: (() => void) | undefined;
		let unlistenFocus: (() => void) | undefined;

		void (async () => {
			await loadDebugData(getInitialSessionId());

			unlistenTrace = await listen<AgentTraceEvent>('agent:trace', (event) => {
				if (event.payload.type === 'snapshot_updated') {
					if (!sessionId || event.payload.snapshot.sessionId === sessionId) {
						snapshot = event.payload.snapshot;
					}
					return;
				}

				if (!sessionId || event.payload.trace.sessionId === sessionId) {
					traces = [...traces, event.payload.trace];
					if (!selectedTraceId) {
						selectedTraceId = event.payload.trace.id;
					}
				}
			});

			unlistenFocus = await listen<string | null>('agent:trace-focus', async (event) => {
				const nextSessionId = event.payload ?? '';
				if (!nextSessionId) return;
				await loadDebugData(nextSessionId);
			});
		})();

		return () => {
			unlistenTrace?.();
			unlistenFocus?.();
		};
	});
</script>

<svelte:head>
	<title>Agent Trace</title>
</svelte:head>

{#if !isDev}
	<div class={styles.warningSurface}>Agent trace is available only in development mode.</div>
{:else}
	<div class={styles.page}>
		<header class={`${styles.surface} ${styles.header}`}>
			<div class={styles.title}>
				<h1><Activity size={24} /> Agent Trace</h1>
				<p>Live execution insights for <code>{sessionId || 'unknown'}</code></p>
			</div>
			
			<div class={styles.snapshotGrid}>
				<div class={styles.snapshotCell}>
					<span class={styles.snapshotLabel}>Current Phase</span>
					<strong class={styles.snapshotValue}>{snapshot?.phase ?? '-'}</strong>
				</div>
				<div class={styles.snapshotCell}>
					<span class={styles.snapshotLabel}>Step</span>
					<strong class={styles.snapshotValue}>{snapshot?.stepNumber ?? 0}</strong>
				</div>
				<div class={styles.snapshotCell}>
					<span class={styles.snapshotLabel}>Total Tokens</span>
					<strong class={styles.snapshotValue}>
						{snapshot ? `${snapshot.totalInputTokens}/${snapshot.totalOutputTokens}` : '-'}
					</strong>
				</div>
				<div class={styles.snapshotCell}>
					<span class={styles.snapshotLabel}>Active Model</span>
					<strong class={styles.snapshotValue}>{snapshot?.lastModel ?? '-'}</strong>
				</div>
			</div>
		</header>

		{#if error}
			<div class={styles.warningSurface}><AlertCircle size={16} /> {error}</div>
		{/if}

		<section class={`${styles.surface} ${styles.filters}`}>
			<div class={styles.iconWrapper}><Search size={14} /></div>
			<input class={styles.field} bind:value={sessionId} placeholder="Session ID" />
			
			<div class={styles.iconWrapper}><Filter size={14} /></div>
			
			<select class={styles.select} bind:value={filterType}>
				<option value="">All Types</option>
				{#each uniqueTypes as type}
					<option value={type}>{type}</option>
				{/each}
			</select>

			<select class={styles.select} bind:value={filterPhase}>
				<option value="">All Phases</option>
				{#each uniquePhases as phase}
					<option value={phase}>{phase}</option>
				{/each}
			</select>

			<select class={`${styles.select} ${styles.fieldCompact}`} bind:value={filterStep}>
				<option value="">All Steps</option>
				{#each uniqueSteps as step}
					<option value={String(step)}>Step {step}</option>
				{/each}
			</select>

			<label class={styles.toggle}>
				<input bind:checked={autoScroll} type="checkbox" />
				<span>Auto-scroll</span>
			</label>
			
			<button
				class={styles.button}
				type="button"
				onclick={() => (selectedTraceId = traces[traces.length - 1]?.id ?? null)}
			>
				<Zap size={14} /> Jump to latest
			</button>
		</section>

		<div class={styles.layout}>
			<div class={`${styles.surface} ${styles.tableShell}`} bind:this={traceContainer}>
				<table class={styles.table}>
					<thead>
						<tr>
							<th style:width="{columnWidths.time}px">
								<div class={styles.iconWrapper}><Clock size={12} /> Time</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'time')}></div>
							</th>
							<th style:width="{columnWidths.delta}px">
								<div class={styles.iconWrapper}><Zap size={12} /> Δ</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'delta')}></div>
							</th>
							<th style:width="{columnWidths.step}px">
								<div class={styles.iconWrapper}><ChevronRight size={12} /> Step</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'step')}></div>
							</th>
							<th style:width="{columnWidths.phase}px">
								<div class={styles.iconWrapper}><Activity size={12} /> Phase</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'phase')}></div>
							</th>
							<th style:width="{columnWidths.action}px">
								<div class={styles.iconWrapper}><Zap size={12} /> Action</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'action')}></div>
							</th>
							<th style:width="{columnWidths.actor}px">
								<div class={styles.iconWrapper}><Cpu size={12} /> Actor</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'actor')}></div>
							</th>
							<th style:width="{columnWidths.status}px">
								<div class={styles.iconWrapper}><CheckCircle2 size={12} /> Status</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'status')}></div>
							</th>
							<th style:width="{columnWidths.duration}px">
								<div class={styles.iconWrapper}><Clock size={12} /> Dur.</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'duration')}></div>
							</th>
							<th style:width="{columnWidths.tokens}px">
								<div class={styles.iconWrapper}><Database size={12} /> Tokens</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'tokens')}></div>
							</th>
							<th style:width="{columnWidths.tool}px">
								<div class={styles.iconWrapper}><Cpu size={12} /> Tool</div>
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'tool')}></div>
							</th>
							<th style:width="{columnWidths.summary}px">
								Summary
								<div class={styles.colResizer} onmousedown={(e) => onColResizeStart(e, 'summary')}></div>
							</th>
						</tr>
					</thead>
					<tbody>
						{#each rows as row (row.id)}
							<tr
								class={`${styles.row} ${row.id === selectedTraceId ? styles.rowSelected : ''} ${row.status === 'failed' ? styles.rowFailed : ''} ${row.status === 'completed' ? styles.rowCompleted : ''}`}
								onclick={() => selectTrace(row.id)}
							>
								<td class={styles.rowCode}>{row.time}</td>
								<td class={`${styles.deltaCell} ${row.deltaMs && row.deltaMs > 1000 ? styles.deltaPositive : ''}`}>
									{row.deltaMs ? `+${row.deltaMs}ms` : '-'}
								</td>
								<td style="text-align: center;">{row.step}</td>
								<td>
									<span class={`${styles.pill} ${styles.pillNeutral}`}>{row.phase}</span>
								</td>
								<td title={row.action}>{row.action}</td>
								<td>
									<span class={`${styles.pill} ${row.actor === 'LLM' ? styles.pillPrimary : row.actor === 'Tools' ? styles.pillSuccess : styles.pillNeutral}`}>
										{row.actor}
									</span>
								</td>
								<td>
									<span class={`${styles.pill} ${
										row.status === 'completed' ? styles.pillSuccess : 
										row.status === 'failed' ? styles.pillError : 
										row.status === 'cancelled' ? styles.pillWarning : styles.pillPrimary
									}`}>
										{row.status}
									</span>
								</td>
								<td>{row.duration}</td>
								<td>{row.tokens}</td>
								<td title={row.tool}>{row.tool}</td>
								<td class={styles.summaryCell} title={row.summary}>{row.summary}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<div 
				class={`${styles.resizer} ${isResizing ? styles.resizerActive : ''}`} 
				onmousedown={handleMouseDown}
			></div>

			<aside class={`${styles.surface} ${styles.detail}`} style:width="{detailWidth}px">
				<h2><Info size={18} /> Trace Payload</h2>
				<div class={styles.payloadContainer}>
					{#if selectedTrace}
						<div class={styles.payload}>
							<MonacoEditor 
								value={JSON.stringify(selectedTrace, null, 2)} 
								language="json" 
								readOnly={true}
								theme="tokyo-night"
								minimap={true}
							/>
						</div>
					{:else}
						<div class={styles.placeholder}>
							<Search size={48} style="opacity: 0.2" />
							<span>Select a row to inspect payload</span>
						</div>
					{/if}
				</div>
			</aside>
		</div>
	</div>
{/if}

<style>
	:global(body) {
		background: var(--color-bg-primary);
		overflow: hidden;
	}
</style>