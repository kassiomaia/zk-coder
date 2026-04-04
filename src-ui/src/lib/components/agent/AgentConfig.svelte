<script lang="ts">
	/**
	 * AgentConfig Component
	 * Displays available agents and allows viewing their configuration.
	 */
	import { Check, ChevronDown, ChevronRight, Cpu, Settings, Shield, Wrench } from 'lucide-svelte';
	import type { Agent, TrustMode } from '$lib/types/agent';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';
	import LoadingState from '$lib/components/shared/LoadingState.svelte';
	import StatusBadge from '$lib/components/shared/StatusBadge.svelte';

	interface Props {
		open: boolean;
		onClose: () => void;
		onSelectAgent?: (agentId: string) => void;
		selectedAgentId?: string | null;
	}

	let { open, onClose, onSelectAgent, selectedAgentId }: Props = $props();

	let agents = $state<Agent[]>([]);
	let loading = $state(false);
	let expandedAgentId = $state<string | null>(null);

	// Fetch agents when panel opens
	$effect(() => {
		const fetchAgents = async () => {
			if (open) {
				loading = true;
				try {
					const { invoke } = await import('@tauri-apps/api/core');
					const agentsData = await import('$lib/stores/agentStore').then((m) =>
						m.agentStore.listAgents()
					);
					agents = agentsData;
				} catch {
					// Fallback to mock data for development
					agents = [
						{
							id: 'code-agent',
							name: 'Agent',
							description: 'Assists with code reading, writing, and execution',
							systemPrompt:
								'You are a code assistant agent. You can read files, write code, search codebases, and execute commands.',
							allowedTools: [
								'fs_read_file',
								'fs_write_file',
								'fs_list_files',
								'search_grep',
								'system_execute'
							],
							defaultModelId: 'code',
							trustMode: 'untrusted'
						}
					];
				}
				loading = false;
			}
		};
		fetchAgents();
	});

	function getTrustModeText(trustMode: TrustMode): string {
		switch (trustMode) {
			case 'trusted':
				return 'Trusted - Auto-runs all tools';
			case 'restricted':
				return 'Restricted - Approval for all tools';
			case 'untrusted':
				return 'Untrusted - Approval for destructive ops';
			default:
				return 'Unknown';
		}
	}

	function getTrustModeClass(trustMode: TrustMode): string {
		switch (trustMode) {
			case 'trusted':
				return 'trusted';
			case 'restricted':
				return 'restricted';
			case 'untrusted':
				return 'untrusted';
			default:
				return '';
		}
	}

	function toggleExpand(agentId: string) {
		expandedAgentId = expandedAgentId === agentId ? null : agentId;
	}

	function handleSelectAgent(agentId: string) {
		onSelectAgent?.(agentId);
		onClose();
	}
</script>

{#if open}
	<div
		class="overlay"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) onClose();
		}}
		onkeydown={(e) => e.key === 'Escape' && onClose()}
	>
		<div
			class="panel"
			role="dialog"
			aria-modal="true"
			aria-labelledby="agent-config-title"
			tabindex="-1"
		>
			<div class="header">
				<div class="header-title">
					<Settings size={20} />
					<h2 id="agent-config-title">Agent Configuration</h2>
				</div>
				<button class="close-btn" onclick={onClose} type="button" aria-label="Close"> × </button>
			</div>

			<div class="content">
				{#if loading}
					<LoadingState class="loading-state" message="Loading agents..." />
				{:else if agents.length === 0}
					<EmptyState icon={Cpu} message="No agents available" />
				{:else}
					<div class="agents-list">
						{#each agents as agent (agent.id)}
							{@const isExpanded = expandedAgentId === agent.id}
							{@const isSelected = selectedAgentId === agent.id}
							<div class="agent-card" class:selected={isSelected}>
								<div class="agent-header">
									<button
										class="agent-main"
										onclick={() => toggleExpand(agent.id)}
										type="button"
										aria-expanded={isExpanded}
									>
										<div class="agent-info">
											<h3 class="agent-name">
												{agent.name}
												{#if isSelected}
													<Check size={14} class="selected-icon" />
												{/if}
											</h3>
											<p class="agent-description">{agent.description}</p>
										</div>
										{#if isExpanded}
											<ChevronDown size={16} class="expand-icon" />
										{:else}
											<ChevronRight size={16} class="expand-icon" />
										{/if}
									</button>
									{#if onSelectAgent && !isSelected}
										<button
											class="select-btn"
											onclick={() => handleSelectAgent(agent.id)}
											type="button"
										>
											Select
										</button>
									{/if}
								</div>

								{#if isExpanded}
									<div class="agent-details">
										<!-- Trust Mode -->
										<div class="detail-row">
											<div class="detail-icon">
												{#if agent.trustMode === 'trusted'}
													<Check size={14} />
												{:else}
													<Shield size={14} />
												{/if}
											</div>
											<div class="detail-content">
												<span class="detail-label">Trust Mode</span>
												<span class="detail-value {getTrustModeClass(agent.trustMode)}">
													{getTrustModeText(agent.trustMode)}
												</span>
											</div>
										</div>

										<!-- Default Model -->
										<div class="detail-row">
											<div class="detail-icon">
												<Cpu size={14} />
											</div>
											<div class="detail-content">
												<span class="detail-label">Default Model</span>
												<span class="detail-value">{agent.defaultModelId}</span>
											</div>
										</div>

										<!-- Allowed Tools -->
										<div class="detail-row tools-row">
											<div class="detail-icon">
												<Wrench size={14} />
											</div>
											<div class="detail-content">
												<span class="detail-label">Allowed Tools</span>
												<div class="tools-list">
													{#each agent.allowedTools as tool}
														<StatusBadge variant="tag" size="sm" text={tool} />
													{/each}
												</div>
											</div>
										</div>

										<!-- System Prompt -->
										<div class="detail-row">
											<div class="detail-content full-width">
												<span class="detail-label">System Prompt</span>
												<pre class="system-prompt">{agent.systemPrompt}</pre>
											</div>
										</div>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<div class="footer">
				<p class="footer-text">
					Agents determine how the AI assistant interacts with your workspace and files.
				</p>
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
		max-width: 600px;
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

	.header-title {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.header-title h2 {
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0;
	}

	.close-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: var(--text-xl);
		transition: color var(--transition-fast);
	}

	.close-btn:hover {
		color: var(--color-text-primary);
		background: var(--color-interactive-hover);
	}

	.content {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-4);
	}

	.agents-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.agent-card {
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background: var(--color-bg-secondary);
		transition:
			border-color var(--transition-fast),
			box-shadow var(--transition-fast);
	}

	.agent-card:hover {
		border-color: var(--color-border-default);
	}

	.agent-card.selected {
		border-color: var(--color-primary-500);
		background: var(--color-interactive-selected);
	}

	.agent-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.agent-main {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3);
		background: transparent;
		border: none;
		cursor: pointer;
		text-align: left;
	}

	.agent-info {
		flex: 1;
		min-width: 0;
	}

	.agent-name {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--text-base);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0;
	}

	.selected-icon {
		color: var(--color-primary-500);
	}

	.agent-description {
		font-size: var(--text-sm);
		color: var(--color-text-secondary);
		margin: var(--space-0-5) 0 0 0;
	}

	.expand-icon {
		color: var(--color-text-muted);
		flex-shrink: 0;
		transition: transform var(--transition-fast);
	}

	.select-btn {
		padding: var(--space-1) var(--space-3);
		background: var(--color-primary-500);
		color: white;
		border: none;
		border-radius: var(--radius-sm);
		font-size: var(--text-sm);
		font-weight: 500;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.select-btn:hover {
		background: var(--color-primary-600);
	}

	.agent-details {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: 0 var(--space-3) var(--space-3) var(--space-3);
		border-top: 1px solid var(--color-border-subtle);
		margin-top: var(--space-1);
	}

	.detail-row {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
	}

	.detail-row.tools-row {
		align-items: flex-start;
	}

	.detail-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 20px;
		height: 20px;
		color: var(--color-text-muted);
		flex-shrink: 0;
	}

	.detail-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: var(--space-0-5);
	}

	.detail-content.full-width {
		width: 100%;
	}

	.detail-label {
		font-size: var(--text-xs);
		font-weight: 500;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.detail-value {
		font-size: var(--text-sm);
		color: var(--color-text-primary);
	}

	.detail-value.trusted {
		color: var(--color-success);
	}

	.detail-value.restricted {
		color: var(--color-warning);
	}

	.detail-value.untrusted {
		color: var(--color-primary-400);
	}

	.tools-list {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}

	.system-prompt {
		margin: var(--space-1) 0 0 0;
		padding: var(--space-2);
		background: var(--color-bg-tertiary);
		border-radius: var(--radius-sm);
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
		white-space: pre-wrap;
		word-break: break-word;
		max-height: 150px;
		overflow-y: auto;
	}

	.footer {
		padding: var(--space-3) var(--space-4);
		border-top: 1px solid var(--color-border-subtle);
		background: var(--color-bg-secondary);
		border-radius: 0 0 var(--radius-lg) var(--radius-lg);
	}

	.footer-text {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		text-align: center;
		margin: 0;
	}
</style>
