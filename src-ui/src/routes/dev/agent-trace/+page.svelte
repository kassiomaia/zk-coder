<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
	import {
		Activity,
		AlertCircle,
		ArrowDown,
		Filter,
		Info,
		Radio,
		RefreshCw,
		Search,
		X,
		Zap
	} from 'lucide-svelte';
	import MonacoEditor from '$lib/components/shared/MonacoEditor.svelte';
	import type { AgentTraceEvent, ExecutionSnapshot, RuntimeTrace } from '$lib/types/agent';
	import { defaultWidths, formatRelativeAge, mergeTrace, buildRow } from './traceTable';
	import TracePlanTable from './TracePlanTable.svelte';
	import WindowHeader from '$lib/components/shared/WindowHeader.svelte';
	import shell from './AgentTraceShell.module.css';
	import { createLogger } from '$lib/utils/logger';

	type DebugResponse = {
		snapshot: ExecutionSnapshot | null;
		traces: RuntimeTrace[];
	};

	type TransportState =
		| 'connecting'
		| 'live'
		| 'polling_fallback'
		| 'subscription_failed'
		| 'reconnecting';

	const LISTENER_GRACE_MS = 2000;
	const POLLING_INTERVAL_MS = 1000;

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
	let detailWidth = $state(
		browser ? Number(localStorage.getItem('agent-trace-detail-width')) || 460 : 460
	);
	let isResizing = $state(false);
	let columnWidths = $state<Record<string, number>>(
		browser
			? {
					...defaultWidths,
					...JSON.parse(localStorage.getItem('agent-trace-column-widths') || '{}')
				}
			: { ...defaultWidths }
	);

	let transportState = $state<TransportState>('connecting');
	let isListening = $state(false);
	let listenerRegistered = $state(false);
	let windowLabel = $state('unknown');
	let lastEventAt = $state<string | null>(null);
	let lastPollAt = $state<string | null>(null);
	let liveEventsCount = $state(0);
	let now = $state(Date.now());

	let activeResizeCol = $state<string | null>(null);
	let startX = 0;
	let startWidth = 0;

	const log = createLogger('AgentTrace');

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
		transportState === 'connecting'
			? 'Connecting trace stream'
			: transportState === 'reconnecting'
				? 'Resyncing execution stream'
				: transportState === 'polling_fallback'
					? 'Polling fallback active'
					: transportState === 'subscription_failed'
						? 'Live subscription failed'
						: 'Listening in background'
	);
	const lastEventLabel = $derived(formatRelativeAge(lastEventAt, now));
	const lastPollLabel = $derived(formatRelativeAge(lastPollAt, now));
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

	function logTraceDebug(message: string, payload?: unknown) {
		if (!isDev) return;
		log.debug(message, payload ?? '');
	}

	function mergeDebugData(response: DebugResponse, targetSessionId: string) {
		sessionId = targetSessionId;
		if (
			!snapshot ||
			(response.snapshot &&
				new Date(response.snapshot.updatedAt).getTime() >= new Date(snapshot.updatedAt).getTime())
		) {
			snapshot = response.snapshot;
		}

		let merged = traces;
		for (const trace of response.traces) {
			merged = mergeTrace(merged, trace);
		}
		traces = merged;

		if (!selectedTraceId && merged.length > 0) {
			selectedTraceId = merged[merged.length - 1].id;
		}
	}

	async function loadDebugData(targetSessionId: string, mode: TransportState = 'reconnecting') {
		if (!targetSessionId) return;
		transportState = mode;
		error = null;

		try {
			const response = await invoke<DebugResponse>('get_agent_execution_debug', {
				sessionId: targetSessionId
			});
			if (mode === 'polling_fallback') {
				lastPollAt = new Date().toISOString();
				mergeDebugData(response, targetSessionId);
			} else {
				replaceDebugData(response, targetSessionId);
			}
			if (mode !== 'polling_fallback' && listenerRegistered) {
				transportState = liveEventsCount > 0 ? 'live' : 'connecting';
			}
		} catch (loadError) {
			error = String(loadError);
			if (mode === 'polling_fallback') {
				transportState = 'subscription_failed';
			}
		}
	}

	function handleSnapshotUpdate(nextSnapshot: ExecutionSnapshot) {
		if (sessionId && nextSnapshot.sessionId !== sessionId) return;

		if (!sessionId) {
			sessionId = nextSnapshot.sessionId;
		}

		snapshot = nextSnapshot;
		rememberLiveEvent();
		transportState = 'live';
		logTraceDebug('snapshot event received', {
			sessionId: nextSnapshot.sessionId,
			phase: nextSnapshot.phase,
			step: nextSnapshot.stepNumber
		});
	}

	function handleTraceUpdate(nextTrace: RuntimeTrace) {
		if (sessionId && nextTrace.sessionId !== sessionId) return;

		if (!sessionId) {
			sessionId = nextTrace.sessionId;
		}

		traces = mergeTrace(traces, nextTrace);
		selectedTraceId = selectedTraceId ?? nextTrace.id;
		rememberLiveEvent();
		transportState = 'live';
		logTraceDebug('trace event received', {
			sessionId: nextTrace.sessionId,
			traceId: nextTrace.id,
			traceType: nextTrace.traceType
		});
	}

	function selectTrace(id: string) {
		selectedTraceId = id;
	}

	function jumpToLatest() {
		selectedTraceId = traces[traces.length - 1]?.id ?? null;
	}

	function dismissError() {
		error = null;
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
		// Using `number | undefined` rather than `ReturnType<typeof setInterval>` because
		// the latter can resolve to `NodeJS.Timeout` when `@types/node` is in scope, which
		// clashes with the `number` that `window.setInterval` / `window.setTimeout` return
		// in a browser environment (this page runs in the Tauri webview).
		let tickHandle: number | undefined;
		let pollingHandle: number | undefined;
		let graceHandle: number | undefined;

		const currentWindow = getCurrentWebviewWindow();
		windowLabel = currentWindow.label;

		const stopPolling = () => {
			if (pollingHandle) {
				window.clearInterval(pollingHandle);
				pollingHandle = undefined;
			}
		};

		const startPollingFallback = () => {
			if (!sessionId || pollingHandle || transportState === 'live') return;
			transportState = 'polling_fallback';
			logTraceDebug('starting polling fallback', { sessionId, windowLabel });
			pollingHandle = window.setInterval(() => {
				if (!sessionId) return;
				void loadDebugData(sessionId, 'polling_fallback');
			}, POLLING_INTERVAL_MS);
		};

		const resetGracePeriod = () => {
			if (graceHandle) {
				window.clearTimeout(graceHandle);
			}
			if (!sessionId) return;
			graceHandle = window.setTimeout(() => {
				if (liveEventsCount === 0 && transportState !== 'live') {
					startPollingFallback();
				}
			}, LISTENER_GRACE_MS);
		};

		const handleWindowFocus = () => {
			if (sessionId) {
				void loadDebugData(sessionId, 'reconnecting');
				resetGracePeriod();
			}
		};

		void (async () => {
			const initialSessionId = getInitialSessionId();
			if (initialSessionId) {
				await loadDebugData(initialSessionId, 'connecting');
			} else {
				transportState = 'connecting';
			}

			try {
				unlistenTrace = await currentWindow.listen<AgentTraceEvent>(traceEventName, (event) => {
					if (transportState === 'polling_fallback') {
						stopPolling();
					}

					if (event.payload.type === 'snapshot_updated') {
						handleSnapshotUpdate(event.payload.snapshot);
						return;
					}

					handleTraceUpdate(event.payload.trace);
				});

				unlistenFocus = await currentWindow.listen<string | null>(focusEventName, async (event) => {
					const nextSessionId = event.payload ?? '';
					logTraceDebug('focus event received', { nextSessionId, windowLabel });
					if (!nextSessionId) return;
					resetLiveCounters();
					await loadDebugData(nextSessionId, 'reconnecting');
					resetGracePeriod();
				});

				listenerRegistered = true;
				isListening = true;
				logTraceDebug('window listeners registered', { windowLabel });
				resetGracePeriod();
			} catch (listenerError) {
				listenerRegistered = false;
				isListening = false;
				transportState = 'subscription_failed';
				error = String(listenerError);
				logTraceDebug('window listener registration failed', listenerError);
				startPollingFallback();
			}

			tickHandle = window.setInterval(() => {
				now = Date.now();
			}, 1000);
			window.addEventListener('focus', handleWindowFocus);
		})();

		return () => {
			isListening = false;
			listenerRegistered = false;
			stopPolling();
			unlistenTrace?.();
			unlistenFocus?.();
			if (graceHandle) {
				window.clearTimeout(graceHandle);
			}
			if (tickHandle) {
				window.clearInterval(tickHandle);
			}
			window.removeEventListener('focus', handleWindowFocus);
		};
	});
</script>

<svelte:head>
	<title>Agent Trace</title>
	<style>
		html,
		body {
			background: transparent !important;
		}
	</style>
</svelte:head>

{#if !isDev}
	<div class={shell.warningSurface}>Agent trace is available only in development mode.</div>
{:else}
	<div class={shell.page} data-tone={headerTone}>
		<WindowHeader title="Agent Trace" subtitle={sessionId || 'waiting...'}>
			{#snippet icon()}
				<Activity size={18} />
			{/snippet}
		</WindowHeader>

		<div class={shell.systemStrip} data-state={transportState}>
			<div class={shell.liveBadge}>
				<div class={shell.liveIndicator}></div>
				<span>{listeningLabel}</span>
			</div>

			<div class={shell.snapshotGrid}>
				<div class={shell.snapshotCell}>
					<span class={shell.snapshotLabel}>Phase</span>
					<strong class={shell.snapshotValue}>{snapshot?.phase ?? '-'}</strong>
				</div>
				<div class={shell.snapshotCell}>
					<span class={shell.snapshotLabel}>Step</span>
					<strong class={shell.snapshotValue}>{snapshot?.stepNumber ?? 0}</strong>
				</div>
				<div class={shell.snapshotCell}>
					<span class={shell.snapshotLabel}>Tokens</span>
					<strong class={shell.snapshotValue}>
						{snapshot ? `${snapshot.totalInputTokens}/${snapshot.totalOutputTokens}` : '-'}
					</strong>
				</div>
				<div class={shell.snapshotCell}>
					<span class={shell.snapshotLabel}>Model</span>
					<strong class={shell.snapshotValue}>{snapshot?.lastModel ?? '-'}</strong>
				</div>
			</div>
		</div>

		{#if error}
			<div class={shell.warningSurface}>
				<AlertCircle size={14} />
				<span class="flex-1">{error}</span>
				<button class={shell.dismissButton} onclick={dismissError} aria-label="Dismiss error">
					<X size={14} />
				</button>
			</div>
		{/if}

		<section class={shell.toolbar}>
			<div class={shell.toolbarSearch}>
				<Search size={14} />
				<input class={shell.field} bind:value={sessionId} placeholder="Session ID" />
			</div>

			<div class={shell.toolbarFilters}>
				<select class={shell.select} bind:value={filterType}>
					<option value="">Actions: All</option>
					{#each uniqueTypes as type}
						<option value={type}>{type}</option>
					{/each}
				</select>

				<select class={shell.select} bind:value={filterPhase}>
					<option value="">Phases: All</option>
					{#each uniquePhases as phase}
						<option value={phase}>{phase}</option>
					{/each}
				</select>

				<select class={shell.select} bind:value={filterStep}>
					<option value="">Steps: All</option>
					{#each uniqueSteps as step}
						<option value={String(step)}>Step {step}</option>
					{/each}
				</select>
			</div>

			<div class={shell.toolbarActions}>
				<label class={shell.toggle}>
					<input bind:checked={autoScroll} type="checkbox" />
					<span>Auto-scroll</span>
				</label>

				<button class={shell.buttonGhost} type="button" onclick={reconnect} disabled={!sessionId}>
					<RefreshCw size={13} />
					Resync
				</button>

				<button class={shell.buttonPrimary} type="button" onclick={jumpToLatest}>
					<ArrowDown size={13} />
					Latest
				</button>
			</div>
		</section>

		<div class={shell.layout}>
			<div class={shell.tableShell} bind:this={traceContainer}>
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

			<aside class={shell.detail} style:width="{detailWidth}px">
				<div class={shell.detailHeader}>
					<h2 class={shell.detailTitle}>
						<Info size={14} />
						Trace payload
					</h2>
					<div class={shell.detailMeta}>
						<span>{selectedTrace?.traceType ?? 'None'}</span>
					</div>
				</div>

				<div class={shell.payloadContainer}>
					{#if selectedTrace}
						<MonacoEditor
							value={JSON.stringify(selectedTrace, null, 2)}
							language="json"
							readOnly={true}
							minimap={true}
							class="flex-1"
						/>
					{:else}
						<div class={shell.placeholder}>
							<Search size={44} />
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
