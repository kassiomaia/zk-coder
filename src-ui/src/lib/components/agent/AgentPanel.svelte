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
	import ToolCallsView from './ToolCallsView.svelte';
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
	let toolCallsExpanded = $state(false);

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
		selectedFile = null;
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
		<div class="header-left">
			<h3 class="panel-title">{session.agentName}</h3>
			<WorkspaceSelector
				workspacePath={session.workspaceRoot}
				onSelect={onWorkspaceChange}
				disabled={session.state.type === 'running' || session.state.type === 'user_input_required'}
			/>
		</div>
		<div class="header-actions">
			<button
				class="header-btn"
				onclick={() => (showAgentConfig = true)}
				type="button"
				aria-label="Agent configuration"
				title="Agent Configuration"
			>
				<Settings size={14} />
			</button>
			<button
				class="header-btn"
				onclick={() => (showSessionHistory = true)}
				type="button"
				aria-label="View session history"
				title="Session History"
			>
				<History size={14} />
			</button>
			<button class="header-btn close" onclick={onclose} type="button" aria-label="Close agent panel">
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

	<!-- Status bar -->
	<div
		class="session-status"
		class:waiting={session.state.type === 'user_input_required'}
		class:error={session.state.type === 'error'}
	>
		<span class="status-dot {getStatusDotClass(session)}"></span>
		<span class="status-label">{getStatusLabel(session)}</span>
	</div>

	<!-- Activity (Tasks + Progress) section -->
	<section class="panel-section activity-section">
		<button
			class="section-header"
			onclick={() => (tasksExpanded = !tasksExpanded)}
			type="button"
			aria-expanded={tasksExpanded}
			aria-controls="activity-section"
		>
			<span class="header-main">
				<span class="chevron-icon" class:is-expanded={tasksExpanded}>
					<ChevronRight size={14} strokeWidth={2.5} />
				</span>
				<span class="header-label">Activity</span>
			</span>
			
			<div class="header-meta">
				{#if session.state.type === 'running'}
					<span class="badge running">Step {session.state.currentStep}</span>
				{/if}
				{#if totalTokensUsed() > 0}
					<span class="badge tokens" title="Total tokens used">{formatTokenCount(totalTokensUsed())}</span>
				{/if}
				<Activity size={13} strokeWidth={1.5} class="section-icon" />
			</div>
		</button>
		{#if tasksExpanded}
			<div id="activity-section">
				<TaskList 
					tasks={session.tasks ?? []} 
					executionSteps={session.executionSteps ?? []}
					currentStep={session.state.type === 'running' ? session.state.currentStep : undefined}
				/>
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
			<span class="header-main">
				<span class="chevron-icon" class:is-expanded={filesExpanded}>
					<ChevronRight size={14} strokeWidth={2.5} />
				</span>
				<span class="header-label">Files</span>
			</span>

			<div class="header-meta">
				{#if (session.filesTouched?.length ?? 0) > 0}
					<span class="badge">{session.filesTouched?.length ?? 0}</span>
				{/if}
				<FileText size={13} strokeWidth={1.5} class="section-icon" />
			</div>
		</button>
		{#if filesExpanded}
			<div id="files-section">
				<FileTree 
					files={session.filesTouched ?? []} 
					onfileclick={handleFileClick} 
					activePath={selectedFile?.path}
				/>
			</div>
		{/if}
	</section>

	<!-- Tool Calls section -->
	<section class="panel-section tool-calls-section">
		<button
			class="section-header"
			onclick={() => (toolCallsExpanded = !toolCallsExpanded)}
			type="button"
			aria-expanded={toolCallsExpanded}
			aria-controls="tool-calls-section"
		>
			<span class="header-main">
				<span class="chevron-icon" class:is-expanded={toolCallsExpanded}>
					<ChevronRight size={14} strokeWidth={2.5} />
				</span>
				<span class="header-label">Tool Calls</span>
			</span>

			<div class="header-meta">
				<Activity size={13} strokeWidth={1.5} class="section-icon" />
			</div>
		</button>
		{#if toolCallsExpanded}
			<div id="tool-calls-section" class="tool-calls-content">
				<ToolCallsView toolCalls={session.toolCalls ?? []} />
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
		workspaceRoot={session.workspaceRoot ?? ''}
		sessionId={session.id}
		existingDiff={selectedFile.diff}
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
		align-items: flex-start;
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--color-border-primary);
		flex-shrink: 0;
		background: var(--color-bg-secondary);
	}

	.header-left {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.panel-title {
		font-size: var(--text-sm);
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
		letter-spacing: -0.01em;
		line-height: 1;
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		margin-top: -2px;
	}

	.header-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-1);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		opacity: 0.6;
		transition: all var(--transition-fast);
	}

	.header-btn:hover {
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
		opacity: 1;
	}

	.header-btn.close:hover {
		color: var(--color-error);
	}

	.header-btn:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: 2px;
	}

	/* Status bar */
	.session-status {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1-5) var(--space-4);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border-subtle);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		flex-shrink: 0;
	}

	.status-label {
		text-transform: uppercase;
		letter-spacing: var(--tracking-wider);
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
		justify-content: space-between;
		align-items: center;
		width: 100%;
		padding: var(--space-1-5) var(--space-4);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.header-main {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--color-text-primary);
	}

	.chevron-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		transition: transform var(--transition-fast);
		color: var(--color-text-muted);
	}

	.chevron-icon.is-expanded {
		transform: rotate(90deg);
	}

	.header-label {
		font-size: 12px;
		font-weight: 600;
		letter-spacing: -0.01em;
		color: var(--color-text-primary);
	}

	.badge {
		margin-left: auto;
		padding: 2px 6px;
		background: var(--color-neutral-800);
		border: 1px solid var(--color-border-subtle);
		border-radius: 4px;
		font-size: 9px;
		font-weight: 600;
		color: var(--color-text-secondary);
		line-height: 1;
		white-space: nowrap;
	}

	.badge.running {
		background: var(--color-primary-900/30);
		border-color: var(--color-primary-500/30);
		color: var(--color-primary-300);
	}

	.badge.tokens {
		background: var(--color-neutral-900);
		border-color: var(--color-border-subtle);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}


	.header-meta {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--color-text-muted);
	}

	:global(.section-icon) {
		opacity: 0.3;
		transition: opacity var(--transition-fast);
	}

	.section-header:hover :global(.section-icon) {
		opacity: 0.7;
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

	.tool-calls-section {
		flex: 1;
		min-height: 200px;
		display: flex;
		flex-direction: column;
		background: var(--color-neutral-950);
		margin-top: auto;
	}

	.tool-calls-content {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
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
