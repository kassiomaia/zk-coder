<script lang="ts">
    /**
     * AgentPanel Component
     * Right-side panel showing agent session state, tasks, files, and terminal output.
     * Contains collapsible sections and approval dialog trigger.
     */
    import {Activity, Bug, ChevronRight, Cpu, FileText, History, Settings, X} from 'lucide-svelte';
	import { invoke } from '@tauri-apps/api/core';
    import TaskList from './TaskList.svelte';
    import RequestsList from './RequestsList.svelte';
    import FileTree from './FileTree.svelte';
    import ToolCallsView from './ToolCallsView.svelte';
    import ApprovalDialog from './ApprovalDialog.svelte';
    import WorkspaceSelector from './WorkspaceSelector.svelte';
    import SessionTabs from './SessionTabs.svelte';
    import SessionHistory from './SessionHistory.svelte';
    import AgentConfig from './AgentConfig.svelte';
    import FileDiffModal from './FileDiffModal.svelte';
    import type {AgentSession, PendingAction} from '$lib/types/agent';

    interface Props {
		session: AgentSession;
		allSessions?: AgentSession[];
		sessionTitles?: Map<string, string>;
		pendingActions: PendingAction[];
		onApprove: (actionId: string) => void;
		onDeny: (actionId: string) => void;
		onWorkspaceChange: (path: string) => void;
		onSelectSession?: (sessionId: string) => void;
		onCreateSession?: () => void;
		onChangeAgent?: (agentId: string) => void;
		onRetryLastRequest?: (sessionId: string) => void;
		onclose: () => void;
	}

	let {
		session,
		allSessions = [],
		sessionTitles = new Map(),
		pendingActions,
		onApprove,
		onDeny,
		onWorkspaceChange,
		onSelectSession,
		onCreateSession,
		onChangeAgent,
		onRetryLastRequest,
		onclose
	}: Props = $props();

	/** Collapsible section state */
	let tasksExpanded = $state(true);
	let requestsExpanded = $state(true);
	let filesExpanded = $state(true);
	let toolCallsExpanded = $state(false);

	/** View state: 'active' (current session) or 'history' (all sessions) */
	let currentView = $state<'active' | 'history'>('active');

	/** Modal states */
	let showSessionHistory = $state(false);
	let showAgentConfig = $state(false);
	let showFileDiff = $state(false);
	let selectedFile = $state<{ path: string; diff?: string } | null>(null);

	/** Approval dialog derived from pending actions */
	let showApprovalDialog = $derived(pendingActions.length > 0);
	let currentApproval = $derived(pendingActions[0] ?? null);
	let canRetryRecovered = $derived(
		session.state.type === 'error' && session.state.recoverable
	);

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

	async function openTraceWindow() {
		if (!import.meta.env.DEV) return;
		try {
			await invoke('open_agent_trace_window', { sessionId: session.id });
		} catch (error) {
			console.warn('[AgentPanel] Failed to open trace window', error);
		}
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
				class:active={currentView === 'active'}
				onclick={() => (currentView = 'active')}
				type="button"
				aria-label="Active session"
				title="Active Session"
			>
				<Activity size={14} />
			</button>
			<button
				class="header-btn"
				class:active={currentView === 'history'}
				onclick={() => (currentView = 'history')}
				type="button"
				aria-label="View session history"
				title="Session History"
			>
				<History size={14} />
			</button>
			<div class="header-divider"></div>
			{#if import.meta.env.DEV}
				<button
					class="header-btn"
					onclick={openTraceWindow}
					type="button"
					aria-label="Open agent trace window"
					title="Agent Trace"
				>
					<Bug size={14} />
				</button>
			{/if}
			<button
				class="header-btn"
				onclick={() => (showAgentConfig = true)}
				type="button"
				aria-label="Agent configuration"
				title="Agent Configuration"
			>
				<Settings size={14} />
			</button>
			<button class="header-btn close" onclick={onclose} type="button" aria-label="Close agent panel">
				<X size={16} />
			</button>
		</div>
	</header>

	<!-- Content Area -->
	<div class="panel-content">
		{#if currentView === 'active'}
			<!-- Session tabs (shown when multiple sessions exist) -->
			{#if allSessions.length > 1 || allSessions.length === 0}
				<SessionTabs
					sessions={allSessions}
					activeSessionId={session.id}
					threadTitles={sessionTitles}
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
				{#if canRetryRecovered}
					<button
						class="retry-btn"
						type="button"
						onclick={() => onRetryLastRequest?.(session.id)}
						aria-label="Retry last request"
					>
						Retry last request
					</button>
				{/if}
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
						/>
					</div>
				{/if}
			</section>

			<!-- Requests section -->
			<section class="panel-section requests-section">
				<button
					class="section-header"
					onclick={() => (requestsExpanded = !requestsExpanded)}
					type="button"
					aria-expanded={requestsExpanded}
					aria-controls="requests-section"
				>
					<span class="header-main">
						<span class="chevron-icon" class:is-expanded={requestsExpanded}>
							<ChevronRight size={14} strokeWidth={2.5} />
						</span>
						<span class="header-label">Requests</span>
					</span>

					<div class="header-meta">
						{#if (session.executionSteps?.length ?? 0) > 0}
							<span class="badge">{session.executionSteps?.length ?? 0}</span>
						{/if}
						<Cpu size={13} strokeWidth={1.5} class="section-icon" />
					</div>
				</button>
				{#if requestsExpanded}
					<div id="requests-section">
						<RequestsList
							steps={session.executionSteps ?? []}
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
		{:else if currentView === 'history'}
			<SessionHistory
				conversationId={session.conversationId}
				onSelectSession={(id) => {
					onSelectSession?.(id);
					currentView = 'active';
				}}
				inline={true}
			/>
		{/if}
	</div>
</nav>

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
		width: 100%;
		min-height: 0;
		min-width: 0;
		background: var(--color-bg-secondary);
		font-size: var(--text-sm);
		overflow: hidden;
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		padding: var(--space-2) var(--space-3);
		border-bottom: 1px solid var(--color-border-primary);
		flex-shrink: 0;
		background: var(--color-bg-secondary);
	}

	.header-left {
		display: flex;
		flex-direction: column;
		gap: 0;
		min-width: 0;
		flex: 1;
	}

	.panel-title {
		font-size: 11px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
		letter-spacing: -0.01em;
		line-height: 1.2;
		text-transform: uppercase;
		opacity: 0.8;
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 2px;
		margin-top: -4px;
	}

	.header-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 4px;
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		opacity: 0.5;
		transition: all var(--transition-fast);
	}

	.header-btn:hover {
		opacity: 1;
		background: var(--color-interactive-hover);
	}

	.header-btn.active {
		color: var(--color-primary-400);
		opacity: 1;
		background: var(--color-primary-500/10);
	}

	.header-divider {
		width: 1px;
		height: 12px;
		background: var(--color-border-subtle);
		margin: 0 4px;
	}

	.header-btn.close:hover {
		color: var(--color-error);
		background: var(--color-error-light);
	}

	/* Status bar */
	.session-status {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-3);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border-subtle);
		font-size: 10px;
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		flex-shrink: 0;
	}

	.status-label {
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.retry-btn {
		margin-left: auto;
		padding: 2px 8px;
		border: 1px solid var(--color-warning);
		border-radius: var(--radius-sm);
		background: var(--color-warning-light);
		color: var(--color-warning);
		font-size: 10px;
		font-weight: 600;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.retry-btn:hover {
		background: var(--color-warning-light);
		filter: brightness(0.95);
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
		padding: var(--space-1) var(--space-3);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.header-main {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--color-text-primary);
	}

	.chevron-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		transition: transform var(--transition-fast);
		color: var(--color-text-muted);
		opacity: 0.5;
	}

	.chevron-icon.is-expanded {
		transform: rotate(90deg);
	}

	.header-label {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: -0.01em;
		color: var(--color-text-secondary);
		text-transform: uppercase;
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

	.panel-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		overflow-y: auto;
		scrollbar-gutter: stable;
	}

	.history-view-placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-muted);
		font-style: italic;
	}

	/* Scrollbar styling */
	.panel-content::-webkit-scrollbar {
		width: 6px;
	}

	.panel-content::-webkit-scrollbar-track {
		background: transparent;
	}

	.panel-content::-webkit-scrollbar-thumb {
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
	}
</style>
