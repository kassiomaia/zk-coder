<script lang="ts">
	import { Activity, Bug, History, Settings, X, FileText } from 'lucide-svelte';
	import type { AgentSession } from '$lib/types/agent';
	import WorkspaceSelector from './WorkspaceSelector.svelte';

	interface Props {
		session: AgentSession;
		currentView: 'active' | 'history';
		onRetryLastRequest?: (sessionId: string) => void;
		onWorkspaceChange: (path: string) => void;
		onSelectHistoryView: () => void;
		onSelectActiveView: () => void;
		onOpenTraceWindow: () => void;
		onOpenConfig: () => void;
		onclose: () => void;
	}

	let {
		session,
		currentView,
		onRetryLastRequest,
		onWorkspaceChange,
		onSelectHistoryView,
		onSelectActiveView,
		onOpenTraceWindow,
		onOpenConfig,
		onclose
	}: Props = $props();

	let canRetryRecovered = $derived(session.state.type === 'error' && session.state.recoverable);
	let verificationHeadline = $derived(session.harnessState?.startupVerification?.headline);

	function getStatusLabel(session: AgentSession): string {
		if (session.harnessState?.startupVerification?.state === 'running') {
			return verificationHeadline || 'Verifying workspace';
		}

		if (
			session.state.type === 'paused' &&
			session.harnessState?.mode === 'resume_verifier' &&
			verificationHeadline
		) {
			return verificationHeadline;
		}

		if (session.state.type === 'finished' && session.harnessState?.cleanExitRecorded) {
			return 'Handoff ready';
		}

		switch (session.state.type) {
			case 'launching':
				return 'Preparing session';
			case 'running':
				return `Executing step ${session.state.currentStep}`;
			case 'wait_for_request':
				return 'Ready for instructions';
			case 'user_input_required':
				return 'Awaiting your approval';
			case 'paused':
				return 'Progress saved';
			case 'finished':
				return 'Task completed successfully';
			case 'cancelled':
				return 'Operation cancelled';
			case 'error':
				return 'Execution interrupted';
			default:
				return 'System idle';
		}
	}

	function getStatusDotClass(session: AgentSession): string {
		switch (session.state.type) {
			case 'running':
			case 'launching':
				return 'running';
			case 'user_input_required':
				return 'waiting';
			case 'paused':
				return 'paused';
			case 'finished':
				return 'complete';
			case 'error':
				return 'error';
			default:
				return '';
		}
	}
</script>

<header class="panel-header">
	<div class="header-left">
		<div class="context-info">
			<h3 class="panel-title">{session.agentName}</h3>
			<span class="system-label">Agent Session</span>
		</div>

		<div class="status-indicator">
			<span class="status-dot {getStatusDotClass(session)}"></span>
			<span class="status-label">{getStatusLabel(session)}</span>
			{#if canRetryRecovered}
				<button
					class="retry-btn-mini"
					type="button"
					onclick={() => onRetryLastRequest?.(session.id)}
					aria-label="Retry last request"
					title="Retry last request"
				>
					<Activity size={10} />
					Retry
				</button>
			{/if}
		</div>
	</div>

	<div class="header-actions">
		<button class="header-btn close" onclick={onclose} type="button" aria-label="Close agent panel">
			<X size={14} />
		</button>
	</div>
</header>

<div class="panel-toolbar">
	<div class="toolbar-group">
		<button
			class="toolbar-btn"
			class:active={currentView === 'active'}
			onclick={onSelectActiveView}
			type="button"
			aria-label="Active session"
		>
			<Activity size={14} />
			<span>Active</span>
		</button>
		<button
			class="toolbar-btn"
			class:active={currentView === 'history'}
			onclick={onSelectHistoryView}
			type="button"
			aria-label="View session history"
		>
			<History size={14} />
			<span>History</span>
		</button>
	</div>

	<div class="toolbar-group actions">
		{#if import.meta.env.DEV}
			<button
				class="toolbar-btn-icon"
				onclick={onOpenTraceWindow}
				type="button"
				title="Agent Trace"
			>
				<Bug size={14} />
			</button>
		{/if}
		<button
			class="toolbar-btn-icon"
			onclick={onOpenConfig}
			type="button"
			title="Agent Configuration"
		>
			<Settings size={14} />
		</button>
	</div>
</div>

<div class="workspace-bar">
	<div class="workspace-info">
		<span class="workspace-microcopy">Targeting project</span>
		<WorkspaceSelector
			workspacePath={session.workspaceRoot}
			onSelect={onWorkspaceChange}
			disabled={session.state.type === 'running' || session.state.type === 'user_input_required'}
		/>
	</div>
</div>

{#if !session.workspaceRoot}
	<div class="empty-workspace-state">
		<div class="empty-icon-wrapper">
			<FileText size={48} strokeWidth={1} />
		</div>
		<h4 class="empty-title">No workspace selected</h4>
		<p class="empty-description">
			Select a project folder to enable the agent to analyze and edit your code.
		</p>
		<div class="empty-actions">
			<WorkspaceSelector
				workspacePath={session.workspaceRoot}
				onSelect={onWorkspaceChange}
				disabled={session.state.type === 'running' || session.state.type === 'user_input_required'}
			/>
		</div>
	</div>
{/if}

<style>
	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3) var(--component-padding);
		border-bottom: 1px solid var(--color-border-subtle);
		flex-shrink: 0;
		background: var(--depth-bg-2);
		animation: header-entrance var(--transition-normal) var(--ease-out);
	}

	.panel-toolbar {
		display: flex;
		align-items: center;
		padding: var(--space-1-5) var(--component-padding);
		background: var(--depth-bg-1);
		border-bottom: 1px solid var(--color-border-subtle);
		flex-shrink: 0;
	}

	.workspace-bar {
		display: flex;
		align-items: center;
		padding: var(--space-1-5) var(--component-padding);
		background: var(--depth-bg-2);
		border-bottom: 1px solid var(--color-border-subtle);
		flex-shrink: 0;
	}

	.workspace-info {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
		flex: 1;
	}

	.workspace-microcopy {
		font-size: 10px;
		font-weight: var(--font-medium);
		color: var(--color-text-muted);
		letter-spacing: normal;
		opacity: 0.6;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.toolbar-group {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}

	.toolbar-group.actions {
		margin-left: auto;
	}

	.toolbar-divider {
		display: none;
	}

	.toolbar-btn {
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-md);
		border: 1px solid transparent;
		color: var(--color-text-tertiary);
		background: transparent;
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.toolbar-btn:hover {
		color: var(--color-text-primary);
		background: var(--color-interactive-hover);
	}

	.toolbar-btn.active {
		color: var(--color-primary-400);
		background: var(--color-primary-subtle);
		border-color: var(--color-primary-border);
	}

	.toolbar-btn-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--space-6);
		height: var(--space-6);
		border-radius: var(--radius-sm);
		color: var(--color-text-tertiary);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.toolbar-btn-icon:hover {
		color: var(--color-text-primary);
		background: var(--color-interactive-hover);
	}

	@keyframes header-entrance {
		from {
			opacity: 0;
			transform: translateY(-4px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.header-left {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
		flex: 1;
	}

	.context-info {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
		min-width: 0;
	}

	.panel-title {
		font-size: var(--text-xs);
		font-weight: var(--font-semibold);
		color: var(--color-text-primary);
		margin: 0;
		line-height: var(--leading-tight);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.system-label {
		font-size: 10px;
		font-weight: var(--font-medium);
		color: var(--color-text-muted);
		letter-spacing: normal;
		opacity: 0.5;
		white-space: nowrap;
	}

	.status-indicator {
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
	}

	.status-label {
		font-size: 11px;
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		line-height: 1;
	}

	.status-dot {
		width: 6px;
		height: 6px;
		border-radius: var(--radius-full);
		background: var(--color-text-muted);
		position: relative;
		flex-shrink: 0;
		transition: background var(--transition-normal) var(--state-ease);
	}

	.status-dot.running {
		background: var(--color-primary-400);
		box-shadow: 0 0 0 2px var(--color-primary-subtle);
	}

	.status-dot.running::after {
		content: '';
		position: absolute;
		inset: -2px;
		border-radius: 50%;
		border: 1px solid var(--color-primary-400);
		animation: status-pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
	}

	@keyframes status-pulse {
		0% {
			transform: scale(1);
			opacity: 0.8;
		}
		100% {
			transform: scale(2.5);
			opacity: 0;
		}
	}

	.status-dot.complete {
		background: var(--color-success);
	}
	.status-dot.error {
		background: var(--color-error);
	}
	.status-dot.waiting {
		background: var(--color-warning);
	}
	.status-dot.paused {
		background: var(--color-info);
	}

	.workspace-context {
		display: none;
	}

	.header-actions {
		display: flex;
		align-items: center;
		margin-left: var(--space-2);
	}

	.action-cluster {
		display: none;
	}

	.header-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--space-7);
		height: var(--space-7);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: all var(--transition-fast) var(--state-ease);
	}

	.header-btn:hover {
		color: var(--color-text-primary);
		background: var(--color-interactive-active);
	}

	.header-btn.close:hover {
		color: var(--color-error);
		background: var(--color-error-light);
	}

	.retry-btn-mini {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		margin-left: var(--space-2);
		padding: var(--space-0-5) var(--space-1-5);
		border: 1px solid var(--color-warning-border);
		border-radius: var(--radius-sm);
		background: var(--color-warning-light);
		color: var(--color-warning);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		cursor: pointer;
		transition: all var(--transition-fast);
		line-height: 1;
	}

	.retry-btn-mini:hover {
		background: var(--color-warning-light);
		border-color: var(--color-warning);
		transform: translateY(-0.5px);
	}

	.empty-workspace-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		flex: 1;
		padding: var(--space-8) var(--space-6);
		text-align: center;
		color: var(--color-text-muted);
	}

	.empty-icon-wrapper {
		width: var(--space-16);
		height: var(--space-16);
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-bg-secondary);
		border-radius: var(--radius-2xl);
		margin-bottom: var(--space-4);
		color: var(--color-text-muted);
		opacity: 0.5;
	}

	.empty-title {
		font-size: var(--text-base);
		font-weight: var(--font-semibold);
		color: var(--color-text-primary);
		margin-bottom: var(--space-2);
	}

	.empty-description {
		font-size: var(--text-sm);
		line-height: var(--leading-relaxed);
		margin-bottom: var(--space-6);
		max-width: 240px;
	}

	.empty-actions {
		display: flex;
		justify-content: center;
		width: 100%;
	}

	.empty-actions :global(.workspace-selector) {
		background: var(--color-primary-500);
		color: white;
		padding: var(--space-2) var(--space-4);
		border-radius: var(--radius-md);
		gap: var(--space-2);
		font-size: var(--text-sm);
		font-weight: var(--font-medium);
		box-shadow: var(--shadow-sm);
		transition:
			transform var(--transition-fast),
			background var(--transition-fast);
	}

	.empty-actions :global(.workspace-selector:hover) {
		background: var(--color-primary-600);
		transform: translateY(-1px);
	}

	.empty-actions :global(.workspace-selector .workspace-label) {
		display: none;
	}

	.empty-actions :global(.workspace-selector .workspace-path) {
		color: inherit;
		border-bottom: none;
	}
</style>
