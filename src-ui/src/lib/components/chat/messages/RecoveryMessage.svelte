<script lang="ts">
	/**
	 * RecoveryMessage Component
	 *
	 * Displays a recovery card in the chat when a session is paused after app restart.
	 * Shows Continue/Replan buttons for the user to choose how to proceed.
	 * Follows the same pattern as ApprovalMessage for consistency.
	 */
	import ActionCard from '$lib/components/shared/ActionCard.svelte';
	import type { Message } from '$lib/types/message';
	import { Play, CheckCircle2, XCircle } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		message: Message;
		onContinue?: (sessionId: string) => void;
		onReplan?: (sessionId: string) => void;
	}

	let { message, onContinue, onReplan }: Props = $props();

	let isLoading = $state(false);

	const agentData = $derived(message.agentData);
	const sessionId = $derived(agentData?.sessionId ?? '');
	const recoveryResolved = $derived(agentData?.recoveryResolved ?? false);
	const recoveryDecision = $derived(agentData?.recoveryDecision);

	async function handleContinue() {
		if (!onContinue || recoveryResolved) return;
		isLoading = true;
		try {
			await onContinue(sessionId);
		} finally {
			isLoading = false;
		}
	}

	async function handleReplan() {
		if (!onReplan || recoveryResolved) return;
		isLoading = true;
		try {
			await onReplan(sessionId);
		} finally {
			isLoading = false;
		}
	}

	const isResolved = $derived(recoveryResolved);
	const isContinued = $derived(recoveryDecision === 'continued');
	const isReplanned = $derived(recoveryDecision === 'replanned');
</script>

{#snippet icon()}
	<Play size={16} strokeWidth={2.5} />
{/snippet}

{#snippet actions()}
	<div class="button-group">
		<button type="button" class="btn btn-secondary" onclick={handleReplan} disabled={isLoading}>
			Replan
		</button>
		<button
			type="button"
			class="btn btn-primary"
			onclick={handleContinue}
			disabled={isLoading}
			aria-busy={isLoading}
		>
			{#if isLoading}
				<div class="spinner"></div>
			{/if}
			{isLoading ? 'Continuing...' : 'Continue'}
		</button>
	</div>
{/snippet}

{#snippet resolvedContent()}
	{#if isContinued}
		<span>Resuming...</span>
	{:else if isReplanned}
		<span>Replanning...</span>
	{/if}
{/snippet}

<ActionCard
	{icon}
	title="Your progress is saved"
	description="Great! Pick up where you left off, or start fresh."
	{actions}
	resolved={isResolved}
	{resolvedContent}
/>

<style>
	.button-group {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-1-5);
		height: 32px;
		padding: 0 var(--space-3);
		border-radius: var(--radius-md);
		font-size: var(--text-xs);
		font-weight: var(--font-semibold);
		cursor: pointer;
		transition: all var(--transition-fast);
		border: 1px solid transparent;
	}

	.btn-secondary {
		background: transparent;
		border-color: var(--color-border-primary);
		color: var(--color-text-secondary);
	}

	.btn-secondary:hover:not(:disabled) {
		background: var(--color-surface-hover);
		color: var(--color-text-primary);
		border-color: var(--color-border-secondary);
	}

	.btn-primary {
		background: var(--color-primary-500);
		color: white;
		box-shadow: var(--shadow-xs);
	}

	.btn-primary:hover:not(:disabled) {
		background: var(--color-primary-600);
		transform: translateY(-1px);
		box-shadow: var(--shadow-sm);
	}

	.btn-primary:active:not(:disabled) {
		transform: translateY(0);
	}

	.btn:disabled {
		opacity: var(--opacity-disabled);
		cursor: not-allowed;
	}

	.spinner {
		width: 12px;
		height: 12px;
		border: 2px solid var(--color-border-secondary);
		border-top-color: var(--color-text-primary);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
