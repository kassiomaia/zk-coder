<script lang="ts">
	/**
	 * AgentPanelEmpty Component
	 * Empty state displayed when no agent session is active.
	 */
	import { Bot, Layout } from 'lucide-svelte';
	import WorkspaceSelector from './WorkspaceSelector.svelte';

	interface Props {
		workspacePath?: string;
		onSelectWorkspace: (path: string) => void;
		onReadyToWork: () => void;
	}

	let { workspacePath, onSelectWorkspace, onReadyToWork }: Props = $props();
</script>

<div class="agent-panel-empty">
	<div class="empty-header">
		<div class="empty-icon-container">
			<Bot size={20} strokeWidth={2} />
		</div>
		<div class="empty-content">
			{#if workspacePath}
				<p class="empty-title">Ready to Assist</p>
				<p class="empty-hint">Workspace active. Start a thread to begin.</p>
			{:else}
				<p class="empty-title">Workspace Required</p>
				<p class="empty-hint">Select a workspace to enable agents.</p>
			{/if}
		</div>
	</div>

	<div class="empty-state-group">
		<div class="workspace-section">
			<div class="workspace-header">
				<Layout size={14} />
				<span>Workspace Status</span>
			</div>
			<div class="workspace-selector-container">
				<WorkspaceSelector {workspacePath} onSelect={onSelectWorkspace} />
			</div>
		</div>

		<button
			class="empty-button"
			onclick={onReadyToWork}
			type="button"
			disabled={!workspacePath}
			title={!workspacePath ? 'Select a workspace first' : 'Create a new thread'}
		>
			Ready to Work
		</button>
	</div>
</div>

<style>
	.agent-panel-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: flex-start;
		padding: var(--space-8) var(--space-4);
		height: 100%;
		text-align: left;
		animation: emptyStateFadeIn var(--transition-slow) forwards;
	}

	@keyframes emptyStateFadeIn {
		from {
			opacity: 0;
			transform: translateY(var(--space-1));
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.empty-header {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		width: 100%;
		margin-bottom: var(--space-6);
		padding: 0 var(--space-2);
	}

	.empty-icon-container {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		width: 40px;
		height: 40px;
		background: var(--color-interactive-hover);
		color: var(--color-text-secondary);
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border-subtle);
	}

	.empty-content {
		flex: 1;
		min-width: 0;
	}

	.empty-title {
		font-size: var(--text-base);
		font-weight: var(--font-semibold);
		color: var(--color-text-primary);
		margin: 0 0 var(--space-0-5);
		letter-spacing: var(--tracking-tight);
	}

	.empty-hint {
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
		margin: 0;
		line-height: var(--leading-tight);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.empty-state-group {
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-4);
		background: var(--color-interactive-hover);
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border-subtle);
	}

	.workspace-section {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		width: 100%;
		gap: var(--space-2);
	}

	.workspace-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wider);
	}

	.workspace-selector-container {
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: var(--depth-bg-3);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		transition: border-color var(--transition-fast);
	}

	.workspace-selector-container:hover {
		border-color: var(--color-border-primary);
	}

	.empty-button {
		width: 100%;
		padding: var(--space-2-5) var(--space-4);
		font-size: var(--text-sm);
		font-weight: var(--font-semibold);
		color: var(--color-neutral-0);
		background: var(--color-primary-600);
		border: 1px solid var(--color-primary-500);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition: all var(--transition-fast);
		box-shadow: var(--shadow-sm);
	}

	.empty-button:hover:not(:disabled) {
		background: var(--color-primary-500);
		border-color: var(--color-primary-400);
		transform: translateY(-1px);
		box-shadow: var(--shadow-md);
	}

	.empty-button:active:not(:disabled) {
		transform: translateY(0);
	}

	.empty-button:disabled {
		opacity: var(--opacity-disabled);
		cursor: not-allowed;
		background: var(--color-bg-secondary);
		border-color: var(--color-border-secondary);
		color: var(--color-text-muted);
	}

	@media (prefers-reduced-motion: reduce) {
		.agent-panel-empty,
		.empty-button {
			animation: none;
			transition: none;
		}

		.empty-button:active:not(:disabled) {
			transform: none;
		}
	}
</style>
