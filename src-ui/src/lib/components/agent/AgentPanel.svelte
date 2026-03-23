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
		X,
		History,
		Settings,
		Activity
	} from 'lucide-svelte';
	import TaskList from './TaskList.svelte';
	import FileTree from './FileTree.svelte';
	import TerminalView from './TerminalView.svelte';
	import ApprovalDialog from './ApprovalDialog.svelte';
	import WorkspaceSelector from './WorkspaceSelector.svelte';
	import SessionTabs from './SessionTabs.svelte';
	import SessionHistory from './SessionHistory.svelte';
	import AgentConfig from './AgentConfig.svelte';
	import FileDiffModal from './FileDiffModal.svelte';
	import type { AgentSession, PendingAction } from '$lib/types/agent';

	interface Props {
		session: AgentSession;
		allSessions?: AgentSession[];
		pendingActions: PendingAction[];
		onApprove: (actionId: string) => void;
		onDeny: (actionId: string) => void;
		onWorkspaceChange: (path: string) => void;
		onSelectSession?: (sessionId: string) => void;
		onCreateSession?: () => void;
		onChangeAgent?: (agentId: string) => void;
		onclose: () => void;
	}

	let {
		session,
		allSessions = [],
		pendingActions,
		onApprove,
		onDeny,
		onWorkspaceChange,
		onSelectSession,
		onCreateSession,
		onChangeAgent,
		onclose
	}: Props = $props();

	/** Collapsible section state */
	let tasksExpanded = $state(true);
	let stepsExpanded = $state(true);
	let filesExpanded = $state(true);
	let terminalExpanded = $state(false);

	/** Modal states */
	let showSessionHistory = $state(false);
	let showAgentConfig = $state(false);
	let showFileDiff = $state(false);
	let selectedFile = $state<{ path: string; diff?: string } | null>(null);

	/** Approval dialog derived from pending actions */
	let showApprovalDialog = $derived(pendingActions.length > 0);
	let currentApproval = $derived(pendingActions[0] ?? null);

	function getStatusLabel(session: AgentSession): string {
		switch (session.state.type) {
			case 'launching':
				return 'Launching...';
			case 'running':
				return `Step ${session.state.currentStep}`;
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

	function getStepActionLabel(step: import('$lib/types/agent').ExecutionStep): string {
		if (step.action.type === 'llm_request') return `LLM: ${step.action.model}`;
		if (step.action.type === 'tool_call') return `Tool: ${step.action.tool}`;
		return 'Unknown';
	}

	function handleFileClick(path: string) {
		const file = session.filesTouched?.find((f) => f.path === path);
		if (file) {
			selectedFile = { path: file.path, diff: file.diff };
			showFileDiff = true;
		}
	}

	function closeFileDiff() {
		showFileDiff = false;
	}

	/** Calculate total tokens used - latest step's total (API reports cumulative) */
	let totalTokensUsed = $derived(() => {
		// tokens_used is cumulative (total_tokens), so take latest, not sum
		if (!session.executionSteps || session.executionSteps.length === 0) return 0;
		const latestStep = session.executionSteps[session.executionSteps.length - 1];
		return latestStep.result?.tokensUsed ?? 0;
	});

	/** Format token count for display (e.g., 12.5k) */
	function formatTokenCount(tokens: number): string {
		if (tokens >= 1000) {
			return `${(tokens / 1000).toFixed(1)}k`;
		}
		return tokens.toString();
	}
</script>

<nav class="agent-panel" aria-label="Agent panel">
	<!-- Header -->
	<header class="panel-header">
		<h3 class="panel-title">{session.agentName}</h3>
		<div class="header-actions">
			<button
				class="config-btn"
				onclick={() => (showAgentConfig = true)}
				type="button"
				aria-label="Agent configuration"
				title="Agent Configuration"
			>
				<Settings size={14} />
			</button>
			<button
				class="history-btn"
				onclick={() => (showSessionHistory = true)}
				type="button"
				aria-label="View session history"
				title="Session History"
			>
				<History size={14} />
			</button>
			<button class="close-btn" onclick={onclose} type="button" aria-label="Close agent panel">
				<X size={16} />
			</button>
		</div>
	</header>

	<!-- Session tabs (shown when multiple sessions exist) -->
	{#if allSessions.length > 1 || allSessions.length === 0}
		<SessionTabs
			sessions={allSessions}
			activeSessionId={session.id}
			onSelect={(id) => onSelectSession?.(id)}
			onCreate={() => onCreateSession?.()}
		/>
	{/if}

	<!-- Workspace selector -->
	<WorkspaceSelector
		workspacePath={session.workspaceRoot}
		onSelect={onWorkspaceChange}
		disabled={session.state.type === 'running' || session.state.type === 'launching'}
	/>

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
			{#if (session.tasks?.length ?? 0) > 0}
				<span class="badge">{session.tasks?.length ?? 0}</span>
			{/if}
		</button>
		{#if tasksExpanded}
			<div id="tasks-section">
				<TaskList tasks={session.tasks ?? []} />
			</div>
		{/if}
	</section>

	<!-- Execution Progress section -->
	<section class="panel-section">
		<button
			class="section-header"
			onclick={() => (stepsExpanded = !stepsExpanded)}
			type="button"
			aria-expanded={stepsExpanded}
			aria-controls="steps-section"
		>
			{#if stepsExpanded}
				<ChevronDown size={14} />
			{:else}
				<ChevronRight size={14} />
			{/if}
			<Activity size={14} />
			<span>Progress</span>
			{#if session.state.type === 'running'}
				<span class="badge">{session.state.currentStep}</span>
			{/if}
			{#if totalTokensUsed() > 0}
				<span class="badge tokens" title="Total tokens used">{formatTokenCount(totalTokensUsed())} tokens</span>
			{/if}
		</button>
		{#if stepsExpanded && (session.executionSteps?.length ?? 0) > 0}
			<div id="steps-section" class="steps-list">
				{#each session.executionSteps ?? [] as step}
					<div
						class="step-item"
						class:current={session.state.type === 'running' &&
							step.stepNumber === session.state.currentStep}
					>
						<span class="step-number">{step.stepNumber}</span>
						<span class="step-action">{getStepActionLabel(step)}</span>
						{#if step.result}
							<span class="step-duration">{step.result.durationMs}ms</span>
						{/if}
					</div>
				{/each}
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
			{#if (session.filesTouched?.length ?? 0) > 0}
				<span class="badge">{session.filesTouched?.length ?? 0}</span>
			{/if}
		</button>
		{#if filesExpanded}
			<div id="files-section">
				<FileTree files={session.filesTouched ?? []} onfileclick={handleFileClick} />
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
				<TerminalView output={session.terminalOutput ?? []} />
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

<!-- Session History (modal overlay) -->
<SessionHistory
	open={showSessionHistory}
	conversationId={session.conversationId}
	onClose={() => (showSessionHistory = false)}
	onSelectSession={(id) => onSelectSession?.(id)}
/>

<!-- Agent Config (modal overlay) -->
<AgentConfig
	open={showAgentConfig}
	onClose={() => (showAgentConfig = false)}
	onSelectAgent={(id) => onChangeAgent?.(id)}
	selectedAgentId={session.agentId}
/>

<!-- File Diff Modal (modal overlay) -->
{#if showFileDiff && selectedFile}
	<FileDiffModal
		open={showFileDiff}
		onclose={closeFileDiff}
		filePath={selectedFile.path}
		diff={selectedFile.diff ?? ''}
	/>
{/if}

<style>
	.agent-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: var(--agent-panel-width);
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

	.header-actions {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}

	.history-btn {
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

	.history-btn:hover {
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
	}

	.history-btn:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: 2px;
	}

	.config-btn {
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

	.config-btn:hover {
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
	}

	.config-btn:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: 2px;
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

	.badge.tokens {
		background: var(--color-primary-900);
		color: var(--color-primary-300);
		font-weight: 500;
	}

	/* Execution Steps */
	.steps-list {
		padding: var(--space-2) var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.step-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1-5) var(--space-2);
		border-radius: var(--radius-sm);
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
		transition: background var(--transition-fast);
	}

	.step-item.current {
		background: var(--color-primary-500/10);
		color: var(--color-primary-400);
	}

	.step-number {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		border-radius: var(--radius-full);
		background: var(--color-neutral-700);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		flex-shrink: 0;
	}

	.step-item.current .step-number {
		background: var(--color-primary-500);
		color: var(--color-bg-primary);
	}

	.step-action {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.step-duration {
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		flex-shrink: 0;
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
