<script lang="ts">
	/**
	 * AgentPanel Component
	 * Right-side panel showing agent session state, tasks, files, and terminal output.
	 * Contains collapsible sections and approval dialog trigger.
	 */
	import {
		ChevronDown,
		ChevronRight,
		Terminal,
		FileText,
		ListChecks,
		X
	} from 'lucide-svelte';
	import TaskList from './TaskList.svelte';
	import FileTree from './FileTree.svelte';
	import TerminalView from './TerminalView.svelte';
	import ApprovalDialog from './ApprovalDialog.svelte';
	import type { AgentSession, PendingAction } from '$lib/types/agent';

	interface Props {
		session: AgentSession;
		pendingActions: PendingAction[];
		onApprove: (actionId: string) => void;
		onDeny: (actionId: string) => void;
		onclose: () => void;
	}

	let { session, pendingActions, onApprove, onDeny, onclose }: Props = $props();

	/** Collapsible section state */
	let tasksExpanded = $state(true);
	let filesExpanded = $state(true);
	let terminalExpanded = $state(false);

	/** Approval dialog derived from pending actions */
	let showApprovalDialog = $derived(pendingActions.length > 0);
	let currentApproval = $derived(pendingActions[0] ?? null);

	function getStatusLabel(session: AgentSession): string {
		switch (session.state.type) {
			case 'launching':
				return 'Launching...';
			case 'running':
				return `Step ${session.state.currentStep}/${session.state.maxSteps}`;
			case 'wait_for_request':
				return 'Idle';
			case 'user_input_required':
				return 'Waiting for approval';
			case 'finished':
				return 'Completed';
			case 'cancelled':
				return 'Cancelled';
			case 'error':
				return 'Error';
			default:
				return 'Unknown';
		}
	}

	function getStatusDotClass(session: AgentSession): string {
		switch (session.state.type) {
			case 'running':
			case 'launching':
				return 'running';
			case 'user_input_required':
				return 'waiting';
			case 'finished':
				return 'complete';
			case 'error':
				return 'error';
			default:
				return '';
		}
	}
</script>

<nav class="agent-panel" aria-label="Agent panel">
	<!-- Header -->
	<header class="panel-header">
		<h3 class="panel-title">{session.agentName}</h3>
		<button
			class="close-btn"
			onclick={onclose}
			type="button"
			aria-label="Close agent panel"
		>
			<X size={16} />
		</button>
	</header>

	<!-- Status bar -->
	<div
		class="session-status"
		class:waiting={session.state.type === 'user_input_required'}
		class:error={session.state.type === 'error'}
	>
		<span class="status-dot {getStatusDotClass(session)}"></span>
		<span>{getStatusLabel(session)}</span>
	</div>

	<!-- Tasks section -->
	<section class="panel-section">
		<button
			class="section-header"
			onclick={() => (tasksExpanded = !tasksExpanded)}
			type="button"
			aria-expanded={tasksExpanded}
			aria-controls="tasks-section"
		>
			{#if tasksExpanded}
				<ChevronDown size={14} />
			{:else}
				<ChevronRight size={14} />
			{/if}
			<ListChecks size={14} />
			<span>Tasks</span>
			{#if session.tasks.length > 0}
				<span class="badge">{session.tasks.length}</span>
			{/if}
		</button>
		{#if tasksExpanded}
			<div id="tasks-section">
				<TaskList tasks={session.tasks} />
			</div>
		{/if}
	</section>

	<!-- Files section -->
	<section class="panel-section">
		<button
			class="section-header"
			onclick={() => (filesExpanded = !filesExpanded)}
			type="button"
			aria-expanded={filesExpanded}
			aria-controls="files-section"
		>
			{#if filesExpanded}
				<ChevronDown size={14} />
			{:else}
				<ChevronRight size={14} />
			{/if}
			<FileText size={14} />
			<span>Files</span>
			{#if session.filesTouched.length > 0}
				<span class="badge">{session.filesTouched.length}</span>
			{/if}
		</button>
		{#if filesExpanded}
			<div id="files-section">
				<FileTree files={session.filesTouched} />
			</div>
		{/if}
	</section>

	<!-- Terminal section -->
	<section class="panel-section terminal-section">
		<button
			class="section-header"
			onclick={() => (terminalExpanded = !terminalExpanded)}
			type="button"
			aria-expanded={terminalExpanded}
			aria-controls="terminal-section"
		>
			{#if terminalExpanded}
				<ChevronDown size={14} />
			{:else}
				<ChevronRight size={14} />
			{/if}
			<Terminal size={14} />
			<span>Terminal</span>
		</button>
		{#if terminalExpanded}
			<div id="terminal-section" class="terminal-content">
				<TerminalView output={session.terminalOutput} />
			</div>
		{/if}
	</section>
</nav>

<!-- Approval Dialog (modal overlay) -->
{#if showApprovalDialog && currentApproval}
	<ApprovalDialog
		action={currentApproval}
		onapprove={() => onApprove(currentApproval.id)}
		ondeny={() => onDeny(currentApproval.id)}
	/>
{/if}

<style>
	.agent-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: var(--agent-panel-width, 320px);
		background: var(--color-bg-secondary);
		font-size: var(--text-sm);
		overflow-y: auto;
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--color-border-primary);
		flex-shrink: 0;
	}

	.panel-title {
		font-size: var(--text-sm);
		font-weight: var(--font-medium);
		color: var(--color-text-primary);
		margin: 0;
	}

	.close-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-1);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		transition:
			background var(--transition-fast),
			color var(--transition-fast);
	}

	.close-btn:hover {
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
	}

	.close-btn:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: 2px;
	}

	/* Status bar */
	.session-status {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-4);
		background: var(--color-surface);
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
		flex-shrink: 0;
	}

	.session-status.waiting {
		background: var(--color-warning-light);
		color: var(--color-warning-dark);
	}

	.session-status.error {
		background: var(--color-error-light);
		color: var(--color-error);
	}

	.status-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-text-muted);
		flex-shrink: 0;
	}

	.status-dot.running {
		background: var(--color-primary-400);
		animation: pulse 1.5s ease-in-out infinite;
	}

	.status-dot.waiting {
		background: var(--color-warning);
	}

	.status-dot.complete {
		background: var(--color-success);
	}

	.status-dot.error {
		background: var(--color-error);
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}

	/* Sections */
	.panel-section {
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-2) var(--space-4);
		text-align: left;
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wide);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.section-header:hover {
		background: var(--color-interactive-hover);
	}

	.section-header:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: -2px;
	}

	.badge {
		margin-left: auto;
		padding: 0 var(--space-1-5);
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		line-height: 1.6;
	}

	.terminal-section {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}

	.terminal-content {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}

	/* Scrollbar styling */
	.agent-panel::-webkit-scrollbar {
		width: 6px;
	}

	.agent-panel::-webkit-scrollbar-track {
		background: transparent;
	}

	.agent-panel::-webkit-scrollbar-thumb {
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
	}
</style>
