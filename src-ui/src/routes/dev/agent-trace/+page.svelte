<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import {
		Activity,
		AlertCircle,
		ArrowDown,
		Filter,
		Info,
		Radio,
		RefreshCw,
		Search,
		Zap
	} from 'lucide-svelte';
	import MonacoEditor from '$lib/components/shared/MonacoEditor.svelte';
	import type { AgentTraceEvent, ExecutionSnapshot, RuntimeTrace } from '$lib/types/agent';
	import {
		defaultWidths,
		formatRelativeAge,
		mergeTrace,
		buildRow
	} from './traceTable';
	import TracePlanTable from './TracePlanTable.svelte';
	import shell from './AgentTraceShell.module.css';

	type DebugResponse = {
		snapshot: ExecutionSnapshot | null;
		traces: RuntimeTrace[];
	};

	type LiveState = 'connecting' | 'listening' | 'reconnecting';

	const isDev = import.meta.env.DEV;
	const focusEventName = 'agent:trace-focus';
	const traceEventName = 'agent:trace';

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
	let detailWidth = $state(browser ? Number(localStorage.getItem('agent-trace-detail-width')) || 460 : 460);
	let isResizing = $state(false);
	let columnWidths = $state<Record<string, number>>(
		browser
			? {
					...defaultWidths,
					...JSON.parse(localStorage.getItem('agent-trace-column-widths') || '{}')
				}
			: { ...defaultWidths }
	);

	let liveState = $state<LiveState>('connecting');
	let isListening = $state(false);
	let lastEventAt = $state<string | null>(null);
	let liveEventsCount = $state(0);
	let now = $state(Date.now());

	let activeResizeCol = $state<string | null>(null);
	let startX = 0;
	let startWidth = 0;

	const uniqueTypes = $derived([...new Set(traces.map((trace) => trace.traceType))].sort());
	const uniqueSteps = $derived(
		[...new Set(traces.map((trace) => trace.stepNumber))].sort((left, right) => left - right)
	);
	const uniquePhases = $derived(
		[
			...new Set(
				traces
					.map((trace) => {
						const payload = trace.payloadJson as Record<string, unknown> | null;
						return typeof payload === 'object' && payload
							? String(payload.to ?? payload.phase ?? '')
							: '';
					})
					.filter(Boolean)
			)
		].sort()
	);

	const rows = $derived(
		traces
			.map((trace, index, allTraces) => buildRow(trace, index, allTraces, snapshot))
			.filter((row) => !filterType || row.action === filterType)
			.filter((row) => !filterStep || String(row.step) === filterStep)
			.filter((row) => !filterPhase || row.phase === filterPhase)
	);

	const selectedTrace = $derived(traces.find((trace) => trace.id === selectedTraceId) ?? null);
	const listeningLabel = $derived(
		liveState === 'connecting'
			? 'Connecting trace stream'
			: liveState === 'reconnecting'
				? 'Resyncing execution stream'
				: 'Listening in background'
	);
	const lastEventLabel = $derived(formatRelativeAge(lastEventAt, now));
	const headerTone = $derived(snapshot?.phase ?? 'idle');

	$effect(() => {
		if (!browser) return;
		localStorage.setItem('agent-trace-detail-width', String(detailWidth));
		localStorage.setItem('agent-trace-column-widths', JSON.stringify(columnWidths));
	});

	$effect(() => {
		rows;
		queueMicrotask(() => {
			if (autoScroll && traceContainer) {
				traceContainer.scrollTop = traceContainer.scrollHeight;
			}
		});
	});

	function getInitialSessionId(): string {
		if (!browser) return '';
		return new URL(window.location.href).searchParams.get('sessionId') ?? '';
	}

	function rememberLiveEvent() {
		lastEventAt = new Date().toISOString();
		liveEventsCount += 1;
	}

	function replaceDebugData(response: DebugResponse, targetSessionId: string) {
		sessionId = targetSessionId;
		snapshot = response.snapshot;
		traces = [...response.traces].sort(
			(left, right) => new Date(left.createdAt).getTime() - new Date(right.createdAt).getTime()
		);
		selectedTraceId = traces.length > 0 ? traces[traces.length - 1].id : null;
	}

	async function loadDebugData(targetSessionId: string, mode: LiveState = 'reconnecting') {
		if (!targetSessionId) return;
		liveState = mode;
		error = null;

		try {
			const response = await invoke<DebugResponse>('get_agent_execution_debug', {
				sessionId: targetSessionId
			});
			replaceDebugData(response, targetSessionId);
			liveState = 'listening';
		} catch (loadError) {
			error = String(loadError);
		}
	}

	function handleSnapshotUpdate(nextSnapshot: ExecutionSnapshot) {
		if (sessionId && nextSnapshot.sessionId !== sessionId) return;

		if (!sessionId) {
			sessionId = nextSnapshot.sessionId;
		}

		snapshot = nextSnapshot;
		rememberLiveEvent();
		liveState = 'listening';
	}

	function handleTraceUpdate(nextTrace: RuntimeTrace) {
		if (sessionId && nextTrace.sessionId !== sessionId) return;

		if (!sessionId) {
			sessionId = nextTrace.sessionId;
		}

		traces = mergeTrace(traces, nextTrace);
		selectedTraceId = selectedTraceId ?? nextTrace.id;
		rememberLiveEvent();
		liveState = 'listening';
	}

	function selectTrace(id: string) {
		selectedTraceId = id;
	}

	function jumpToLatest() {
		selectedTraceId = traces[traces.length - 1]?.id ?? null;
	}

	function resetLiveCounters() {
		liveEventsCount = 0;
		lastEventAt = null;
	}

	async function reconnect() {
		if (!sessionId) return;
		resetLiveCounters();
		await loadDebugData(sessionId, 'reconnecting');
	}

	function onColResizeStart(event: MouseEvent, column: string) {
		event.stopPropagation();
		activeResizeCol = column;
		startX = event.clientX;
		startWidth = columnWidths[column];
		document.addEventListener('mousemove', onColResizeMove);
		document.addEventListener('mouseup', onColResizeEnd);
	}

	function onColResizeMove(event: MouseEvent) {
		if (!activeResizeCol) return;
		const difference = event.clientX - startX;
		columnWidths[activeResizeCol] = Math.max(50, startWidth + difference);
	}

	function onColResizeEnd() {
		activeResizeCol = null;
		document.removeEventListener('mousemove', onColResizeMove);
		document.removeEventListener('mouseup', onColResizeEnd);
	}

	function handleMouseDown() {
		isResizing = true;
		document.body.style.cursor = 'col-resize';
		document.addEventListener('mousemove', handleMouseMove);
		document.addEventListener('mouseup', handleMouseUp);
	}

	function handleMouseMove(event: MouseEvent) {
		if (!isResizing) return;
		detailWidth = Math.max(280, window.innerWidth - event.clientX);
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
		let tickHandle: ReturnType<typeof window.setInterval> | undefined;

		const handleWindowFocus = () => {
			if (sessionId) {
				void loadDebugData(sessionId, 'reconnecting');
			}
		};

		void (async () => {
			const initialSessionId = getInitialSessionId();
			if (initialSessionId) {
				await loadDebugData(initialSessionId, 'connecting');
			} else {
				liveState = 'listening';
			}

			unlistenTrace = await listen<AgentTraceEvent>(traceEventName, (event) => {
				if (event.payload.type === 'snapshot_updated') {
					handleSnapshotUpdate(event.payload.snapshot);
					return;
				}

				handleTraceUpdate(event.payload.trace);
			});

			unlistenFocus = await listen<string | null>(focusEventName, async (event) => {
				const nextSessionId = event.payload ?? '';
				if (!nextSessionId) return;
				resetLiveCounters();
				await loadDebugData(nextSessionId, 'reconnecting');
			});

			isListening = true;
			liveState = 'listening';
			tickHandle = window.setInterval(() => {
				now = Date.now();
			}, 1000);
			window.addEventListener('focus', handleWindowFocus);
		})();

		return () => {
			isListening = false;
			unlistenTrace?.();
			unlistenFocus?.();
			if (tickHandle) {
				window.clearInterval(tickHandle);
			}
			window.removeEventListener('focus', handleWindowFocus);
		};
	});
</script>

<svelte:head>
	<title>Agent Trace</title>
</svelte:head>

{#if !isDev}
	<div class={shell.warningSurface}>Agent trace is available only in development mode.</div>
{:else}
	<div class={shell.page} data-tone={headerTone}>
		<header class={`${shell.surface} ${shell.header}`}>
			<div class={shell.headerMain}>
				<div class={shell.titleBlock}>
					<div class={shell.eyebrow}>Development diagnostics</div>
					<h1 class={shell.title}>
						<Activity size={22} />
						Agent Trace
					</h1>
					<p class={shell.subtitle}>
						Live execution plan for <code>{sessionId || 'waiting for session'}</code>
					</p>
				</div>

				<div class={shell.liveBanner} data-state={liveState}>
					<div class={shell.liveBadge}>
						<Radio size={14} />
						<span>{listeningLabel}</span>
					</div>
					<div class={shell.liveMeta}>
						<span>{isListening ? 'stream active' : 'stream offline'}</span>
						<span>last event {lastEventLabel}</span>
						<span>{liveEventsCount} live updates</span>
					</div>
				</div>
			</div>

			<div class={shell.snapshotGrid}>
				<div class={shell.snapshotCell}>
					<span class={shell.snapshotLabel}>Current phase</span>
					<strong class={shell.snapshotValue}>{snapshot?.phase ?? '-'}</strong>
				</div>
				<div class={shell.snapshotCell}>
					<span class={shell.snapshotLabel}>Step</span>
					<strong class={shell.snapshotValue}>{snapshot?.stepNumber ?? 0}</strong>
				</div>
				<div class={shell.snapshotCell}>
					<span class={shell.snapshotLabel}>Total tokens</span>
					<strong class={shell.snapshotValue}>
						{snapshot ? `${snapshot.totalInputTokens}/${snapshot.totalOutputTokens}` : '-'}
					</strong>
				</div>
				<div class={shell.snapshotCell}>
					<span class={shell.snapshotLabel}>Active model</span>
					<strong class={shell.snapshotValue}>{snapshot?.lastModel ?? '-'}</strong>
				</div>
			</div>
		</header>

		{#if error}
			<div class={shell.warningSurface}>
				<AlertCircle size={16} />
				{error}
			</div>
		{/if}

		<section class={`${shell.surface} ${shell.toolbar}`}>
			<div class={shell.toolbarSearch}>
				<div class={shell.iconWrapper}><Search size={14} /></div>
				<input class={shell.field} bind:value={sessionId} placeholder="Session ID" />
			</div>

			<div class={shell.toolbarFilters}>
				<div class={shell.filterGroup}>
					<div class={shell.iconWrapper}><Filter size={14} /></div>
					<select class={shell.select} bind:value={filterType}>
						<option value="">All types</option>
						{#each uniqueTypes as type}
							<option value={type}>{type}</option>
						{/each}
					</select>
				</div>

				<select class={shell.select} bind:value={filterPhase}>
					<option value="">All phases</option>
					{#each uniquePhases as phase}
						<option value={phase}>{phase}</option>
					{/each}
				</select>

				<select class={`${shell.select} ${shell.selectCompact}`} bind:value={filterStep}>
					<option value="">All steps</option>
					{#each uniqueSteps as step}
						<option value={String(step)}>Step {step}</option>
					{/each}
				</select>
			</div>

			<div class={shell.toolbarActions}>
				<label class={shell.toggle}>
					<input bind:checked={autoScroll} type="checkbox" />
					<span>Auto-follow</span>
				</label>

				<button class={shell.buttonGhost} type="button" onclick={reconnect} disabled={!sessionId}>
					<RefreshCw size={14} />
					Resync
				</button>

				<button class={shell.buttonPrimary} type="button" onclick={jumpToLatest}>
					<ArrowDown size={14} />
					Latest event
				</button>
			</div>
		</section>

		<div class={shell.layout}>
			<div class={`${shell.surface} ${shell.tableShell}`} bind:this={traceContainer}>
				<TracePlanTable
					{rows}
					{columnWidths}
					{selectedTraceId}
					onSelect={selectTrace}
					onResizeStart={onColResizeStart}
				/>
			</div>

			<button
				class={`${shell.resizer} ${isResizing ? shell.resizerActive : ''}`}
				type="button"
				aria-label="Resize payload panel"
				onmousedown={handleMouseDown}
			></button>

			<aside class={`${shell.surface} ${shell.detail}`} style:width="{detailWidth}px">
				<div class={shell.detailHeader}>
					<h2 class={shell.detailTitle}>
						<Info size={18} />
						Trace payload
					</h2>
					<div class={shell.detailMeta}>
						<span>{selectedTrace?.traceType ?? 'No trace selected'}</span>
						<span>{selectedTrace ? buildRow(selectedTrace, 0, [], snapshot).model : ''}</span>
					</div>
				</div>

				<div class={shell.payloadContainer}>
					{#if selectedTrace}
						<MonacoEditor
							value={JSON.stringify(selectedTrace, null, 2)}
							language="json"
							readOnly={true}
							theme="tokyo-night"
							minimap={true}
							class="flex-1"
						/>
					{:else}
						<div class={shell.placeholder}>
							<Search size={44} />
							<span>Select a row to inspect the live payload</span>
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
		height: 100vh;
	}

	.layout {
		display: flex;
		flex: 1;
		min-height: 0;
		height: 100%;
		overflow: hidden;
	}
</style>
