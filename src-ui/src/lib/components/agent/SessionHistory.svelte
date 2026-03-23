<script lang="ts">
	/**
	 * SessionHistory Component
	 * Displays completed agent sessions for a conversation.
	 */
	import { X, Clock, CheckCircle, XCircle, AlertCircle } from 'lucide-svelte';
	import type { AgentSession } from '$lib/types/agent';
	import { agentStore } from '$lib/stores';

	interface Props {
		/** Whether the history panel is open */
		open: boolean;
		/** Conversation ID to fetch history for */
		conversationId: string | null;
		/** Close handler */
		onClose: () => void;
		/** Session selection handler */
		onSelectSession?: (sessionId: string) => void;
	}

	let { open, conversationId, onClose, onSelectSession }: Props = $props();

	let sessions = $state<AgentSession[]>([]);
	let loading = $state(false);
	let selectedSessionId = $state<string | null>(null);
	let selectedSessionDetails = $state<AgentSession | null>(null);

	// Fetch history when panel opens
	$effect(() => {
		const fetchHistory = async () => {
			if (open && conversationId) {
				loading = true;
				sessions = await agentStore.getSessionHistory(conversationId);
				loading = false;
			} else {
				sessions = [];
				selectedSessionId = null;
				selectedSessionDetails = null;
			}
		};
		fetchHistory();
	});

	async function handleSelectSession(sessionId: string) {
		selectedSessionId = sessionId;
		const details = await agentStore.getSessionDetails(sessionId);
		selectedSessionDetails = details;
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
</script>

{#if open}
	<div class="overlay" onclick={onClose} onkeydown={(e) => e.key === 'Escape' && onClose()}>
		<div class="panel" onclick={(e) => e.stopPropagation()}>
			<div class="header">
				<h2>Session History</h2>
				<button class="close-btn" onclick={onClose} type="button" aria-label="Close">
					<X size={20} />
				</button>
			</div>

			<div class="content">
				{#if loading}
					<div class="loading-state">
						<p>Loading sessions...</p>
					</div>
				{:else if sessions.length === 0}
					<div class="empty-state">
						<p>No completed sessions found</p>
					</div>
				{:else}
					<div class="sessions-list">
						{#each sessions as session (session.id)}
							{@const completedAt = getCompletedAt(session)}
							<button
								class="session-item"
								class:selected={selectedSessionId === session.id}
								onclick={() => handleSelectSession(session.id)}
								type="button"
							>
								<div class="session-header">
									<svelte:component
										this={getSessionStatusIcon(session)}
										size={16}
										class="status-icon"
									/>
									<div class="session-info">
										<span class="session-title">{session.agentName}</span>
										<span class="session-meta"
											>{getSessionStatusText(session)} &bull; {formatDate(session.createdAt)}</span
										>
									</div>
								</div>
								{#if completedAt && session.createdAt}
									<span class="session-duration"
										>{formatDuration(session.createdAt, completedAt)}</span
									>
								{/if}
							</button>
						{/each}
					</div>

					{#if selectedSessionDetails}
						<div class="session-details">
							<h3>Session Details</h3>
							<div class="detail-section">
								<h4>Terminal Output</h4>
								{#if (selectedSessionDetails.terminalOutput?.length ?? 0) > 0}
									<div class="terminal-output">
										{#each selectedSessionDetails.terminalOutput ?? [] as line}
											<div class="terminal-line {line.type}">
												{line.content}
											</div>
										{/each}
									</div>
								{:else}
									<p class="empty-detail">No terminal output</p>
								{/if}
							</div>

							<div class="detail-section">
								<h4>Tasks</h4>
								{#if selectedSessionDetails.tasks && selectedSessionDetails.tasks.length > 0}
									<div class="tasks-list">
										{#each selectedSessionDetails.tasks as task}
											<div
												class="task-summary"
												class:completed={task.status === 'completed'}
												class:failed={task.status === 'failed'}
												class:running={task.status === 'in_progress'}
											>
												<span class="task-title">{task.title}</span>
												<span class="task-status">{task.status}</span>
											</div>
										{/each}
									</div>
								{:else}
									<p class="empty-detail">No tasks recorded</p>
								{/if}
							</div>

							{#if onSelectSession && selectedSessionDetails}
								{@const sessionId = selectedSessionDetails.id}
								<button
									class="restore-btn"
									onclick={() => {
										onSelectSession(sessionId);
										onClose();
									}}
									type="button"
								>
									View This Session
								</button>
							{/if}
						</div>
					{/if}
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.panel {
		background: var(--color-bg-primary);
		border-radius: var(--radius-lg);
		width: 90%;
		max-width: 800px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
		box-shadow:
			0 20px 25px -5px rgb(0 0 0 / 0.1),
			0 8px 10px -6px rgb(0 0 0 / 0.1);
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-4);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.header h2 {
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0;
	}

	.close-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-1);
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: color var(--transition-fast);
	}

	.close-btn:hover {
		color: var(--color-text-primary);
	}

	.content {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-4);
		padding: var(--space-4);
		overflow: hidden;
		max-height: calc(80vh - 70px);
	}

	.sessions-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		overflow-y: auto;
		padding-right: var(--space-2);
	}

	.session-item {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-3);
		border-radius: var(--radius-md);
		background: transparent;
		border: 1px solid var(--color-border-subtle);
		cursor: pointer;
		transition: all var(--transition-fast);
		text-align: left;
	}

	.session-item:hover {
		background: var(--color-interactive-hover);
		border-color: var(--color-border-default);
	}

	.session-item.selected {
		background: var(--color-interactive-selected);
		border-color: var(--color-primary-500);
	}

	.session-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.status-icon {
		flex-shrink: 0;
		color: var(--color-success);
	}

	.session-item:has(.status-icon:where(.XCircle, .AlertCircle)) .status-icon {
		color: var(--color-error);
	}

	.session-info {
		display: flex;
		flex-direction: column;
		gap: var(--space-0-5);
		min-width: 0;
		flex: 1;
	}

	.session-title {
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-text-primary);
	}

	.session-meta {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
	}

	.session-duration {
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
		align-self: flex-end;
	}

	.session-details {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		overflow-y: auto;
		padding-left: var(--space-2);
	}

	.session-details h3 {
		font-size: var(--text-base);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0;
	}

	.detail-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.detail-section h4 {
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-text-secondary);
		margin: 0;
	}

	.terminal-output {
		background: var(--color-bg-tertiary);
		border-radius: var(--radius-md);
		padding: var(--space-2);
		max-height: 200px;
		overflow-y: auto;
		font-family: monospace;
		font-size: var(--text-xs);
	}

	.terminal-line {
		white-space: pre-wrap;
		word-break: break-all;
	}

	.terminal-line.stdout {
		color: var(--color-text-primary);
	}

	.terminal-line.stderr {
		color: var(--color-error);
	}

	.terminal-line.info {
		color: var(--color-text-muted);
	}

	.tasks-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.task-summary {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-2);
		border-radius: var(--radius-sm);
		background: var(--color-bg-secondary);
	}

	.task-summary.status-completed {
		border-left: 2px solid var(--color-success);
	}

	.task-summary.status-failed {
		border-left: 2px solid var(--color-error);
	}

	.task-summary.status-in_progress {
		border-left: 2px solid var(--color-primary-400);
	}

	.task-title {
		font-size: var(--text-sm);
		color: var(--color-text-primary);
	}

	.task-status {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		text-transform: capitalize;
	}

	.empty-detail {
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		font-style: italic;
	}

	.restore-btn {
		padding: var(--space-2) var(--space-4);
		background: var(--color-primary-500);
		color: white;
		border: none;
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
		font-weight: 500;
		cursor: pointer;
		transition: background var(--transition-fast);
		align-self: flex-start;
	}

	.restore-btn:hover {
		background: var(--color-primary-600);
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
