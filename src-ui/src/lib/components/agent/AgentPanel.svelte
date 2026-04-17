<script lang="ts">
	/**
	 * AgentPanel Component
	 * Right-side panel showing agent session state, tasks, files, and terminal output.
	 * Contains collapsible sections for session management.
	 */
	import { invoke } from '@tauri-apps/api/core';
	import SessionTabs from './SessionTabs.svelte';
	import SessionHistory from './SessionHistory.svelte';
	import AgentConfig from './AgentConfig.svelte';
	import AgentPanelHeader from './AgentPanelHeader.svelte';
	import AgentPanelActivitySection from './AgentPanelActivitySection.svelte';
	import AgentPanelRequestsSection from './AgentPanelRequestsSection.svelte';
	import AgentPanelFilesSection from './AgentPanelFilesSection.svelte';
	import type { AgentSession, PendingAction } from '$lib/types/agent';
	import TaskList from './TaskList.svelte';
	import RequestsList from './RequestsList.svelte';
	import FileTree from './FileTree.svelte';
	import ToolCallsView from './ToolCallsView.svelte';
	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('AgentPanel');

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

	let tasksExpanded = $state(true);
	let requestsExpanded = $state(true);
	let filesExpanded = $state(true);
	let toolCallsExpanded = $state(false);

	let currentView = $state<'active' | 'history'>('active');

	let showSessionHistory = $state(false);
	let showAgentConfig = $state(false);

	let totalTokensUsed = $derived(() => {
		if (!session.executionSteps || session.executionSteps.length === 0) return 0;
		const latestStep = session.executionSteps[session.executionSteps.length - 1];
		return latestStep.result?.tokensUsed ?? 0;
	});
	let harnessState = $derived(session.harnessState);
	let verificationSummary = $derived(harnessState?.startupVerification);
	let handoffSummary = $derived(harnessState?.handoff);
	let hasSessionSummary = $derived(!!verificationSummary || !!handoffSummary);

	async function handleFileClick(path: string) {
		if (!session.workspaceRoot) return;
		try {
			await invoke('open_file_diff_window', {
				filePath: path,
				workspaceRoot: session.workspaceRoot,
				sessionId: session.id
			});
		} catch (error) {
			log.warn('Failed to open file diff window', error);
		}
	}

	async function openTraceWindow() {
		if (!import.meta.env.DEV) return;
		try {
			await invoke('open_agent_trace_window', { sessionId: session.id });
		} catch (error) {
			log.warn('Failed to open trace window', error);
		}
	}
</script>

<nav class="agent-panel" aria-label="Agent panel">
	<AgentPanelHeader
		{session}
		{currentView}
		{onRetryLastRequest}
		{onWorkspaceChange}
		onSelectHistoryView={() => (currentView = 'history')}
		onSelectActiveView={() => (currentView = 'active')}
		onOpenTraceWindow={openTraceWindow}
		onOpenConfig={() => (showAgentConfig = true)}
		{onclose}
	/>

	<div class="panel-content">
		{#if currentView === 'active'}
			{#if !session.workspaceRoot}
				<!-- Empty state shown in AgentPanelHeader -->
			{:else}
				{#if allSessions.length > 1 || allSessions.length === 0}
					<SessionTabs
						sessions={allSessions}
						activeSessionId={session.id}
						threadTitles={sessionTitles}
						onSelect={(id) => onSelectSession?.(id)}
						onCreate={() => onCreateSession?.()}
					/>
				{/if}

				{#if hasSessionSummary}
					<section class="panel-section session-summary-section" aria-label="Session summary">
						<div class="summary-header-row">
							<div>
								<h4 class="summary-title">Session Summary</h4>
								<p class="summary-subtitle">
									Durable handoff and verification context for this session.
								</p>
							</div>
							{#if harnessState}
								<span class="summary-mode">{harnessState.mode.replace('_', ' ')}</span>
							{/if}
						</div>

						<div class="summary-grid">
							{#if verificationSummary}
								<div class="summary-item">
									<span class="summary-label">Verification</span>
									<span class="summary-value">{verificationSummary.headline}</span>
								</div>
							{/if}

							{#if handoffSummary}
								<div class="summary-item">
									<span class="summary-label">Current objective</span>
									<span class="summary-value">{handoffSummary.currentObjective}</span>
								</div>
								<div class="summary-item">
									<span class="summary-label">Next action</span>
									<span class="summary-value">{handoffSummary.nextRecommendedAction}</span>
								</div>
								<div class="summary-item">
									<span class="summary-label">Last increment</span>
									<span class="summary-value">{handoffSummary.lastCompletedIncrement}</span>
								</div>
								<div class="summary-item">
									<span class="summary-label">Environment</span>
									<span class="summary-value">
										{handoffSummary.environmentHealth.replace('_', ' ')}
									</span>
								</div>
								{#if handoffSummary.openIssues.length > 0}
									<div class="summary-item summary-item-wide">
										<span class="summary-label">Open issues</span>
										<span class="summary-value">{handoffSummary.openIssues.join(' • ')}</span>
									</div>
								{/if}
							{/if}
						</div>
					</section>
				{/if}

				<AgentPanelActivitySection
					expanded={tasksExpanded}
					onToggle={() => (tasksExpanded = !tasksExpanded)}
					currentStep={session.state.type === 'running' ? session.state.currentStep : undefined}
					totalTokensUsed={totalTokensUsed()}
				>
					<TaskList tasks={session.tasks ?? []} />
				</AgentPanelActivitySection>

				<AgentPanelRequestsSection
					expanded={requestsExpanded}
					onToggle={() => (requestsExpanded = !requestsExpanded)}
					steps={session.executionSteps ?? []}
					currentStep={session.state.type === 'running' ? session.state.currentStep : undefined}
				>
					<RequestsList
						steps={session.executionSteps ?? []}
						currentStep={session.state.type === 'running' ? session.state.currentStep : undefined}
					/>
				</AgentPanelRequestsSection>

				<AgentPanelFilesSection
					expanded={filesExpanded}
					onToggle={() => (filesExpanded = !filesExpanded)}
					fileCount={session.filesTouched?.length ?? 0}
				>
					<FileTree files={session.filesTouched ?? []} onfileclick={handleFileClick} />
				</AgentPanelFilesSection>

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
								<span class="chevron-rotate">›</span>
							</span>
							<span class="header-label">Tool Calls</span>
						</span>
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

	<AgentConfig
		open={showAgentConfig}
		onClose={() => (showAgentConfig = false)}
		onSelectAgent={(id) => onChangeAgent?.(id)}
		selectedAgentId={session.agentId}
	/>

</nav>

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

	.session-summary-section {
		padding: var(--space-3);
		background: var(--depth-bg-1);
	}

	.summary-header-row {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-3);
		margin-bottom: var(--space-3);
	}

	.summary-title {
		margin: 0;
		font-size: var(--text-sm);
		font-weight: var(--font-semibold);
		color: var(--color-text-primary);
	}

	.summary-subtitle {
		margin: var(--space-1) 0 0;
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
	}

	.summary-mode {
		display: inline-flex;
		align-items: center;
		padding: 0 var(--space-2);
		height: 22px;
		border-radius: var(--radius-full);
		background: var(--color-surface-hover);
		color: var(--color-text-secondary);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		text-transform: capitalize;
		white-space: nowrap;
	}

	.summary-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-2) var(--space-3);
	}

	.summary-item {
		display: flex;
		flex-direction: column;
		gap: var(--space-0-5);
		min-width: 0;
	}

	.summary-item-wide {
		grid-column: 1 / -1;
	}

	.summary-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.summary-value {
		font-size: var(--text-xs);
		color: var(--color-text-primary);
		line-height: 1.4;
		word-break: break-word;
	}

	@keyframes section-fade-in {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
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

	.chevron-rotate {
		font-size: 16px;
		font-weight: bold;
		line-height: 1;
	}

	.header-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		letter-spacing: 0.05em;
		color: var(--color-text-tertiary);
		text-transform: uppercase;
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

	.section-content-wrapper {
		overflow: hidden;
	}

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
</style>
