<script lang="ts">
	/**
	 * SessionHistory Component
	 * Displays completed agent sessions for a conversation.
	 */
	import { slide } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { X, Clock, CheckCircle, XCircle, AlertCircle, ChevronRight, Terminal, ListChecks } from 'lucide-svelte';
	import type { AgentSession } from '$lib/types/agent';
	import { agentStore } from '$lib/stores';

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

	// Fetch history when panel opens or inline mode is active
	$effect(() => {
		const cid = conversationId;
		const isActive = open || inline;

		if (isActive && cid) {
			loading = true;
			agentStore.getSessionHistory(cid).then((res) => {
				sessions = res;
				loading = false;
			}).catch(err => {
				console.error(err);
				loading = false;
			});
		} else {
			sessions = [];
			expandedSessionId = null;
		}
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

	function getSessionStatusText(session: AgentSession): string {
		switch (session.state.type) {
			case 'finished':
				return 'Completed';
			case 'error':
				return 'Failed';
			case 'cancelled':
				return 'Cancelled';
			default:
				return 'Unknown';
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
		return session.agentName || 'Code Agent';
	}
</script>

{#if open || inline}
	{#if !inline}
		<div class="overlay" role="presentation" onclick={onClose} onkeydown={(e) => e.key === 'Escape' && onClose?.()}>
			<div class="panel modal-panel" role="dialog" aria-modal="true" aria-labelledby="session-history-title" tabindex="-1">
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
		<div class="loading-state">
			<p>Loading sessions...</p>
		</div>
	{:else if sessions.length === 0}
		<div class="empty-state">
			<p>No completed sessions found</p>
		</div>
	{:else}
		<div class="timeline-list">
			{#each sessions as session, i (session.id)}
				{@const completedAt = getCompletedAt(session)}
				{@const isExpanded = expandedSessionId === session.id}
				<div class="timeline-item" class:is-expanded={isExpanded}>
					<!-- The Connector Line -->
					{#if i < sessions.length - 1}
						<div class="timeline-connector"></div>
					{/if}

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
								<span class="session-duration">
									{formatDuration(session.createdAt, completedAt)}
								</span>
							{/if}
							<div class="chevron" class:is-expanded={isExpanded}>
								<ChevronRight size={14} />
							</div>
						</button>
					</div>

					{#if isExpanded}
						<div class="expanded-details" transition:slide={{ duration: 200, easing: cubicOut }}>
							<div class="details-inner">
								<div class="session-summary">
									<div class="summary-item">
										<span class="label">Status</span>
										<span class="value {session.state.type}">{getSessionStatusText(session)}</span>
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
										<button
											class="restore-btn-inline disabled"
											disabled
											type="button"
										>
											Restore Session (Soon)
										</button>
										<p class="helper-text">Full state restoration is coming in a future update.</p>
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
		padding: var(--space-3) var(--space-4);
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
		gap: var(--space-1);
		position: relative;
	}

	.timeline-item {
		position: relative;
		padding-bottom: var(--space-1);
	}

	.timeline-connector {
		position: absolute;
		left: 10px;
		top: 24px;
		bottom: -4px;
		width: 1px;
		background: var(--color-border-primary);
		opacity: 0.3;
		z-index: 0;
	}

	.timeline-header-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		position: relative;
		z-index: 1;
	}

	.status-indicator {
		flex-shrink: 0;
		width: 20px;
		display: flex;
		justify-content: center;
	}

	.status-dot {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-bg-tertiary);
		border: 1px solid var(--color-border-subtle);
		color: var(--color-text-muted);
	}

	.status-dot.finished {
		background: rgba(34, 197, 94, 0.1);
		border-color: var(--color-success);
		color: var(--color-success);
	}

	.status-dot.error {
		background: rgba(239, 68, 68, 0.1);
		border-color: var(--color-error);
		color: var(--color-error);
	}

	.session-card {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		cursor: pointer;
		text-align: left;
		transition: all var(--transition-fast);
	}

	.session-card:hover {
		background: var(--color-interactive-hover);
	}

	.is-expanded .session-card {
		background: var(--color-interactive-selected);
		border-color: var(--color-primary-500);
	}

	.session-main-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
	}

	.session-title {
		font-size: 13px;
		font-weight: 400;
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		letter-spacing: -0.01em;
	}

	.session-meta {
		font-size: 10px;
		color: var(--color-text-muted);
		opacity: 0.8;
	}

	.session-duration {
		font-size: 10px;
		font-weight: 600;
		color: var(--color-text-secondary);
		padding: 1px 6px;
		background: var(--color-bg-tertiary);
		border: 1px solid var(--color-border-subtle);
		border-radius: 4px;
	}

	.chevron {
		color: var(--color-text-muted);
		transition: transform var(--transition-fast);
		opacity: 0.5;
	}

	.chevron.is-expanded {
		transform: rotate(90deg);
		opacity: 1;
	}

	/* Expanded Details */
	.expanded-details {
		margin-left: 0;
		margin-top: 2px;
		margin-bottom: var(--space-3);
	}

	.details-inner {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-4);
		background: var(--color-bg-tertiary);
		border-radius: var(--radius-md);
		border: 1px solid var(--color-border-subtle);
	}

	.session-summary {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.summary-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-size: 11px;
	}

	.summary-item .label {
		color: var(--color-text-muted);
		font-weight: 500;
	}

	.summary-item .value {
		color: var(--color-text-secondary);
		font-weight: 600;
	}

	.summary-item .value.finished {
		color: #10b981;
	}

	.summary-item .value.error {
		color: #ef4444;
	}

	.action-footer {
		margin-top: var(--space-2);
		padding-top: var(--space-3);
		border-top: 1px solid var(--color-border-subtle);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.restore-btn-inline {
		padding: var(--space-2) var(--space-4);
		background: var(--color-primary-600);
		color: #fff;
		border: none;
		border-radius: var(--radius-md);
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
	}

	.restore-btn-inline.disabled {
		background: var(--color-bg-secondary);
		border: 1px solid var(--color-border-subtle);
		color: var(--color-text-muted);
		cursor: not-allowed;
		opacity: 0.6;
	}

	.soon-badge {
		font-size: 10px;
		opacity: 0.7;
		font-weight: 400;
	}

	.helper-text {
		display: none;
	}

	.loading-details {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		font-style: italic;
		text-align: center;
		padding: var(--space-2);
	}

	/* Modal specific overrides (to maintain backward compatibility if needed) */
	.modal-panel {
		background: var(--color-bg-primary);
		border-radius: var(--radius-lg);
		width: 90%;
		max-width: 800px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
	}

	.modal-content {
		padding: var(--space-4);
		overflow-y: auto;
	}

	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.loading-state,
	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-8);
		color: var(--color-text-muted);
		font-size: var(--text-sm);
		grid-column: 1 / -1;
	}
</style>
