<script lang="ts">
    /**
     * SessionTabs Component
     * Horizontal tab bar for switching between multiple agent sessions.
     * Shows session status indicators and supports creating new sessions.
     */
    import {Plus} from 'lucide-svelte';
    import type {AgentSession} from '$lib/types/agent';

    interface Props {
		sessions: AgentSession[];
		activeSessionId: string | null;
		threadTitles?: Map<string, string>;
		onSelect: (sessionId: string) => void;
		onCreate: () => void;
	}

	let { sessions, activeSessionId, threadTitles, onSelect, onCreate }: Props = $props();

	function getStatusColor(session: AgentSession): string {
		switch (session.state.type) {
			case 'running':
			case 'launching':
				return 'var(--color-primary-400)';
			case 'user_input_required':
				return 'var(--color-warning)';
			case 'finished':
				return 'var(--color-success)';
			case 'error':
				return 'var(--color-error)';
			case 'cancelled':
				return 'var(--color-text-muted)';
			default:
				return 'var(--color-text-muted)';
		}
	}

	function getDisplayName(session: AgentSession): string {
		return threadTitles?.get(session.id) ?? session.agentName;
	}

	function getShortName(session: AgentSession): string {
		const name = getDisplayName(session);
		return name.length > 12 ? name.slice(0, 11) + '\u2026' : name;
	}
</script>

{#if sessions.length > 1 || sessions.length === 0}
	<div class="session-tabs" role="tablist" aria-label="Agent sessions">
		{#each sessions as session (session.id)}
			<button
				class="tab"
				class:active={session.id === activeSessionId}
				role="tab"
				aria-selected={session.id === activeSessionId}
				onclick={() => onSelect(session.id)}
				type="button"
				title={getDisplayName(session)}
			>
				<span
					class="tab-dot"
					class:pulse={session.state.type === 'running'}
					style:background={getStatusColor(session)}
				></span>
				<span class="tab-label">{getShortName(session)}</span>
			</button>
		{/each}
		<button
			class="tab add-tab"
			onclick={onCreate}
			type="button"
			title="New session"
			aria-label="Create new agent session"
		>
			<Plus size={14} />
		</button>
	</div>
{/if}

<style>
	.session-tabs {
		display: flex;
		align-items: stretch;
		gap: 1px;
		padding: 0 var(--space-2);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border-primary);
		overflow-x: auto;
		flex-shrink: 0;
	}

	.session-tabs::-webkit-scrollbar {
		height: 0;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
		padding: var(--space-2) var(--space-3);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		cursor: pointer;
		white-space: nowrap;
		transition:
			color var(--transition-fast),
			border-color var(--transition-fast),
			background var(--transition-fast);
	}

	.tab:hover {
		color: var(--color-text-secondary);
		background: var(--color-interactive-hover);
	}

	.tab.active {
		color: var(--color-text-primary);
		border-bottom-color: var(--color-primary-400);
	}

	.tab:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: -2px;
	}

	.tab-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.tab-dot.pulse {
		animation: pulse 1.5s ease-in-out infinite;
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

	.tab-label {
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.add-tab {
		color: var(--color-text-muted);
		padding: var(--space-2);
	}

	.add-tab:hover {
		color: var(--color-primary-400);
	}
</style>
