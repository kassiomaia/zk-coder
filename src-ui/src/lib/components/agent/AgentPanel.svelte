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
				return 'Preparing session';
			case 'running':
				return `Executing step ${session.state.currentStep}`;
			case 'wait_for_request':
				return 'Ready for instructions';
			case 'user_input_required':
				return 'Awaiting your approval';
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
			{:else}
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
							<ChevronRight size={14} strokeWidth={2} />
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
					<div id="activity-section" class="section-content-wrapper">
						<TaskList
							tasks={session.tasks ?? []}
						/>
					</div>
				{/if}
			</section>

			<!-- Requests section -->
			<section class="panel-section requests-section" style="animation-delay: 0.1s">
				<button
					class="section-header"
					onclick={() => (requestsExpanded = !requestsExpanded)}
					type="button"
					aria-expanded={requestsExpanded}
					aria-controls="requests-section"
				>
					<span class="header-main">
						<span class="chevron-icon" class:is-expanded={requestsExpanded}>
							<ChevronRight size={14} strokeWidth={2} />
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
					<div id="requests-section" class="section-content-wrapper">
						<RequestsList
							steps={session.executionSteps ?? []}
							currentStep={session.state.type === 'running' ? session.state.currentStep : undefined}
						/>
					</div>
				{/if}
			</section>

			<!-- Files section -->
			<section class="panel-section" style="animation-delay: 0.2s">
				<button
					class="section-header"
					onclick={() => (filesExpanded = !filesExpanded)}
					type="button"
					aria-expanded={filesExpanded}
					aria-controls="files-section"
				>
					<span class="header-main">
						<span class="chevron-icon" class:is-expanded={filesExpanded}>
							<ChevronRight size={14} strokeWidth={2} />
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
					<div id="files-section" class="section-content-wrapper">
						<FileTree 
							files={session.filesTouched ?? []} 
							onfileclick={handleFileClick} 
							activePath={selectedFile?.path}
						/>
					</div>
				{/if}
			</section>

				<!-- Tool Calls section -->
				<section class="panel-section tool-calls-section" style="animation-delay: 0.3s">
					<button
						class="section-header"
						onclick={() => (toolCallsExpanded = !toolCallsExpanded)}
						type="button"
						aria-expanded={toolCallsExpanded}
						aria-controls="tool-calls-section"
					>
						<span class="header-main">
							<span class="chevron-icon" class:is-expanded={toolCallsExpanded}>
								<ChevronRight size={14} strokeWidth={2} />
							</span>
							<span class="header-label">Tool Calls</span>
						</span>

						<div class="header-meta">
							<Activity size={13} strokeWidth={1.5} class="section-icon" />
						</div>
					</button>
					{#if toolCallsExpanded}
						<div id="tool-calls-section" class="tool-calls-content section-content-wrapper">
							<ToolCallsView toolCalls={session.toolCalls ?? []} />
						</div>
					{/if}
				</section>
			{/if}
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
		background: var(--depth-bg-2);
		font-size: var(--text-sm);
		overflow: hidden;
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) var(--component-padding);
		border-bottom: 1px solid var(--color-border-subtle);
		flex-shrink: 0;
		background: var(--depth-bg-2);
	}

	.header-left {
		display: flex;
		flex-direction: column;
		gap: var(--space-px);
		min-width: 0;
		flex: 1;
	}

	.panel-title {
		font-size: var(--text-2xs);
		font-weight: var(--font-semibold);
		color: var(--text-metadata-color);
		margin: 0;
		letter-spacing: 0.05em;
		line-height: var(--leading-tight);
		text-transform: uppercase;
		opacity: 0.8;
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}

	.header-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--space-6);
		height: var(--space-6);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		opacity: 0.6;
		transition: all var(--transition-fast);
	}

	.header-btn:hover {
		opacity: 1;
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
	}

	.header-btn.active {
		color: var(--color-primary-400);
		opacity: 1;
		background: var(--color-primary-subtle);
	}

	.header-divider {
		width: 1px;
		height: var(--space-3-5);
		background: var(--color-border-primary);
		margin: 0 var(--space-0-5);
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
		padding: var(--space-2) var(--space-4);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border-primary);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		flex-shrink: 0;
	}

	.status-label {
		text-transform: none;
		letter-spacing: normal;
		color: var(--color-text-primary);
	}

	.status-dot {
		width: var(--space-1-5);
		height: var(--space-1-5);
		border-radius: var(--radius-full);
		background: var(--color-text-muted);
		position: relative;
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
		0% { transform: scale(1); opacity: 0.8; }
		100% { transform: scale(2.5); opacity: 0; }
	}

	.status-dot.complete { background: var(--color-success); }
	.status-dot.error { background: var(--color-error); }
	.status-dot.waiting { background: var(--color-warning); }

	.retry-btn {
		margin-left: auto;
		padding: var(--space-0-5) var(--space-2);
		border: 1px solid var(--color-warning-border);
		border-radius: var(--radius-sm);
		background: var(--color-warning-subtle);
		color: var(--color-warning);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.retry-btn:hover {
		background: var(--color-warning-light);
		border-color: var(--color-warning);
	}

	/* Sections */
	.panel-section {
		border-bottom: 1px solid var(--color-border-primary);
		transition: background var(--transition-fast);
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		width: 100%;
		padding: var(--space-2) var(--space-3);
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
		transition: transform var(--transition-smooth);
		color: var(--color-text-muted);
	}

	.chevron-icon.is-expanded {
		transform: rotate(90deg);
	}

	.header-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		letter-spacing: 0.05em;
		color: var(--color-text-tertiary);
		text-transform: uppercase;
	}

	.badge {
		padding: var(--space-px) var(--space-1-5);
		background: var(--color-neutral-800);
		border: 1px solid var(--color-border-primary);
		border-radius: var(--radius-sm);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		line-height: var(--leading-tight);
		white-space: nowrap;
	}

	.badge.running {
		background: var(--color-primary-subtle);
		border-color: var(--color-primary-border);
		color: var(--color-primary-400);
	}

	.badge.tokens {
		background: var(--color-neutral-900);
		border-color: var(--color-border-primary);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}

	.header-meta {
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
		color: var(--color-text-muted);
	}

	:global(.section-icon) {
		opacity: 0.4;
		transition: all var(--transition-fast);
	}

	.section-header:hover :global(.section-icon) {
		opacity: 0.8;
		color: var(--color-text-primary);
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
		padding: var(--space-px) var(--space-1-5);
		background: var(--color-neutral-800);
		border: 1px solid var(--color-border-primary);
		border-radius: var(--radius-sm);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		line-height: var(--leading-tight);
		white-space: nowrap;
	}

	.badge.tokens {
		background: var(--color-neutral-900);
		border-color: var(--color-border-primary);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
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
		background: var(--color-primary-subtle);
		color: var(--color-primary-400);
	}

	.step-number {
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--space-4-5);
		height: var(--space-4-5);
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
		scroll-behavior: smooth;
	}

	.panel-section {
		border-bottom: 1px solid var(--color-border-primary);
		transition: background var(--transition-fast);
		animation: section-fade-in 0.4s var(--ease-out) backwards;
	}

	@keyframes section-fade-in {
		from { opacity: 0; transform: translateY(4px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.section-content-wrapper {
		overflow: hidden;
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
		width: var(--space-1-5);
	}

	.panel-content::-webkit-scrollbar-track {
		background: transparent;
	}

	.panel-content::-webkit-scrollbar-thumb {
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
	}

	/* Empty state */
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
		transition: transform var(--transition-fast), background var(--transition-fast);
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
