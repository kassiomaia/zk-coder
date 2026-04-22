<script lang="ts">
	/**
	 * SessionHistory Component
	 * Displays completed agent sessions for a conversation.
	 */
	import { slide } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { AlertCircle, CheckCircle, ChevronRight, Clock, History, X, XCircle } from 'lucide-svelte';
	import type { AgentSession } from '$lib/types/agent';
	import { agentStore } from '$lib/stores';
	import { createLogger } from '$lib/utils/logger';
	import { displayAgentName } from '$lib/utils/agentDisplay';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';
	import LoadingState from '$lib/components/shared/LoadingState.svelte';
	import StatusBadge from '$lib/components/shared/StatusBadge.svelte';

	const log = createLogger('SessionHistory');

	interface Props {
		/** Whether the history panel is open (if using modal) */
		open?: boolean;
		/** Conversation ID to fetch history for */
		conversationId: string | null;
		/** Close handler (if using modal) */
		onClose?: () => void;
		/** Session selection handler */
		onSelectSession?: (sessionId: string) => void;
		/** Whether to render as an inline panel instead of a modal */
		inline?: boolean;
	}

	let { open = false, conversationId, onClose, onSelectSession, inline = false }: Props = $props();

	let sessions = $state<AgentSession[]>([]);
	let loading = $state(false);
	let expandedSessionId = $state<string | null>(null);

	// The parent passes `session.conversationId` which is read through a
	// reactive proxy; any unrelated session mutation (streaming tokens, new
	// tool calls, etc.) would otherwise re-fire this effect and trigger an
	// infinite refetch. Cache the last-fetched key and skip when it is
	// unchanged.
	let lastFetchKey: string | null = null;
	$effect(() => {
		const cid = conversationId;
		const isActive = open || inline;
		const key = isActive && cid ? `${cid}` : null;

		if (key === lastFetchKey) return;
		lastFetchKey = key;

		if (!key || !cid) {
			sessions = [];
			expandedSessionId = null;
			return;
		}

		loading = true;
		agentStore
			.getSessionHistory(cid)
			.then((res) => {
				sessions = res;
				loading = false;
			})
			.catch((err) => {
				log.error('Failed to fetch session history:', err);
				loading = false;
			});
	});

	async function handleToggleExpand(sessionId: string) {
		if (expandedSessionId === sessionId) {
			expandedSessionId = null;
		} else {
			expandedSessionId = sessionId;
		}
	}

	function getSessionStatusIcon(session: AgentSession) {
		switch (session.state.type) {
			case 'finished':
				return CheckCircle;
			case 'error':
				return XCircle;
			case 'cancelled':
				return AlertCircle;
			default:
				return Clock;
		}
	}

	function getSessionStatusLabel(session: AgentSession): string {
		switch (session.state.type) {
			case 'finished':
				return 'Successful';
			case 'error':
				return 'Failed';
			case 'cancelled':
				return 'Interrupted';
			default:
				return 'Historical';
		}
	}

	function formatDate(isoString: string): string {
		const date = new Date(isoString);
		return date.toLocaleString();
	}

	function formatDuration(start: string, end: string): string {
		const startTime = new Date(start).getTime();
		const endTime = new Date(end).getTime();
		const diffMs = endTime - startTime;
		const diffSecs = Math.floor(diffMs / 1000);
		if (diffSecs < 60) return `${diffSecs}s`;
		const diffMins = Math.floor(diffSecs / 60);
		return `${diffMins}m ${diffSecs % 60}s`;
	}

	function getCompletedAt(session: AgentSession): string | null {
		if (session.state.type === 'finished') {
			return session.state.completedAt;
		}
		return null;
	}

	function formatSessionTitle(session: AgentSession): string {
		if (session.id) {
			const shortId = session.id.split('-')[0] || session.id.substring(0, 8);
			return `#${shortId}`;
		}
		return displayAgentName(session);
	}
</script>

{#if open || inline}
	{#if !inline}
		<div
			class="overlay"
			role="presentation"
			onclick={onClose}
			onkeydown={(e) => e.key === 'Escape' && onClose?.()}
		>
			<div
				class="panel modal-panel"
				role="dialog"
				aria-modal="true"
				aria-labelledby="session-history-title"
				tabindex="-1"
			>
				<div class="header">
					<h2 id="session-history-title">Session History</h2>
					<button class="close-btn" onclick={onClose} type="button" aria-label="Close">
						<X size={20} />
					</button>
				</div>
				<div class="content modal-content">
					{@render historyContent()}
				</div>
			</div>
		</div>
	{:else}
		<div class="history-container">
			{@render historyContent()}
		</div>
	{/if}
{/if}

{#snippet historyContent()}
	{#if loading}
		<LoadingState class="loading-state" message="Loading sessions..." />
	{:else if sessions.length === 0}
		<EmptyState icon={History} message="No sessions yet" />
	{:else}
		<div class="timeline-list">
			{#each sessions as session, i (session.id)}
				{@const completedAt = getCompletedAt(session)}
				{@const isExpanded = expandedSessionId === session.id}
				{@const StatusIcon = getSessionStatusIcon(session)}
				<div class="timeline-item" class:is-expanded={isExpanded}>
					<div class="timeline-header-row">
						<button
							class="session-card"
							onclick={() => handleToggleExpand(session.id)}
							type="button"
						>
							<div class="session-main-info">
								<span class="session-title">{formatSessionTitle(session)}</span>
							</div>
							{#if completedAt && session.createdAt}
								<StatusBadge
									variant="mono"
									size="sm"
									text={formatDuration(session.createdAt, completedAt)}
								/>
							{/if}
							<div class="chevron" class:is-expanded={isExpanded}>
								<ChevronRight size={14} strokeWidth={2.5} />
							</div>
						</button>
					</div>

					{#if isExpanded}
						<div class="expanded-details" transition:slide={{ duration: 240, easing: cubicOut }}>
							<div class="details-inner">
								<div class="session-summary">
									<div class="summary-item">
										<span class="label">Status</span>
										<span class="value {session.state.type}">
											<StatusIcon size={12} strokeWidth={2.5} class="status-icon" />
											{getSessionStatusLabel(session)}
										</span>
									</div>
									{#if session.createdAt}
										<div class="summary-item">
											<span class="label">Started</span>
											<span class="value">{formatDate(session.createdAt)}</span>
										</div>
									{/if}
									{#if completedAt && session.createdAt}
										<div class="summary-item">
											<span class="label">Duration</span>
											<span class="value">{formatDuration(session.createdAt, completedAt)}</span>
										</div>
									{/if}
								</div>

								{#if onSelectSession}
									<div class="action-footer">
										<p class="helper-text-visible">
											Full session restoration is currently in development and will be available in
											a future update.
										</p>
									</div>
								{/if}
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
{/snippet}

<style>
	.history-container {
		padding: var(--space-2) var(--space-3);
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		overflow-y: auto;
		background: var(--color-bg-secondary);
	}

	.timeline-list {
		display: flex;
		flex-direction: column;
		gap: 1px;
		position: relative;
	}

	.timeline-item {
		position: relative;
		border-radius: var(--radius-sm);
		transition: all var(--transition-fast);
	}

	.timeline-item.is-expanded {
		background: var(--color-interactive-selected);
		margin: var(--space-1) 0;
	}

	.timeline-header-row {
		display: flex;
		align-items: center;
		position: relative;
		z-index: 1;
	}

	.session-card {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-1-5) var(--space-3);
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		cursor: pointer;
		text-align: left;
		transition: all var(--transition-fast);
	}

	.session-card:hover {
		background: var(--color-interactive-hover);
	}

	.is-expanded .session-card {
		background: transparent;
	}

	.session-main-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
	}

	.session-title {
		font-size: 11px;
		font-weight: 500;
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-family: var(--font-mono);
	}

	.session-duration-badge {
		font-size: 10px;
		font-weight: 600;
		color: var(--color-text-tertiary);
		padding: 1px 6px;
		background: var(--color-neutral-850);
		border: 1px solid var(--color-border-primary);
		border-radius: 4px;
		font-family: var(--font-mono);
	}

	.chevron {
		color: var(--color-text-muted);
		transition: transform var(--transition-smooth);
		opacity: 0.4;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.chevron.is-expanded {
		transform: rotate(90deg);
		opacity: 0.8;
	}

	/* Expanded Details */
	.expanded-details {
		padding: 0 var(--space-3) var(--space-3) var(--space-3);
	}

	.details-inner {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-3);
		background: var(--color-bg-secondary);
		border-radius: var(--radius-sm);
		border: 1px solid var(--color-border-primary);
	}

	.session-summary {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}

	.summary-item {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		font-size: 11px;
	}

	.summary-item .label {
		color: var(--color-text-muted);
		font-weight: 400;
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.summary-item .value {
		color: var(--color-text-secondary);
		font-weight: 500;
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
	}

	.summary-item .value.finished {
		color: var(--color-success);
	}

	.summary-item .value.error {
		color: var(--color-error);
	}

	.summary-item .value.cancelled {
		color: var(--color-warning);
	}

	.status-icon {
		opacity: 0.8;
	}

	.action-footer {
		padding-top: var(--space-3);
		border-top: 1px solid var(--color-border-primary);
	}

	.helper-text-visible {
		font-size: 10px;
		color: var(--color-text-muted);
		line-height: 1.4;
		margin: 0;
		opacity: 0.7;
		font-style: italic;
	}
</style>
