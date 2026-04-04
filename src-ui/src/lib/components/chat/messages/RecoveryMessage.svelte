<script lang="ts">
	/**
	 * RecoveryMessage Component
	 *
	 * Displays a recovery card in the chat when a session is paused after app restart.
	 * Shows Continue/Replan buttons for the user to choose how to proceed.
	 * Follows the same pattern as ApprovalMessage for consistency.
	 */
	import styles from './RecoveryMessage.module.css';
	import type { Message } from '$lib/types/message';
	import { Play } from 'lucide-svelte';
	import { fade } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';

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

<div class={styles.recoveryContainer} in:fade={{ duration: 200, easing: cubicOut }}>
	<div class={styles.contentRow}>
		<!-- Zone 1: Icon & Message -->
		<div class={styles.messageZone}>
			<div class={styles.iconWrapper}>
				<Play size={16} strokeWidth={2.5} />
			</div>
			<div class={styles.textWrapper}>
				<div class={styles.primaryText}>Your progress is saved</div>
				<div class={styles.secondaryText}>Great! Pick up where you left off, or start fresh.</div>
			</div>
		</div>

		<!-- Zone 2: Actions -->
		<div class={styles.actionZone}>
			{#if !isResolved}
				<div class={styles.buttonGroup}>
					<button
						type="button"
						class={[styles.btn, styles.btnSecondary].join(' ')}
						onclick={handleReplan}
						disabled={isLoading}
					>
						Replan
					</button>
					<button
						type="button"
						class={[styles.btn, styles.btnPrimary].join(' ')}
						onclick={handleContinue}
						disabled={isLoading}
						aria-busy={isLoading}
					>
						{#if isLoading}
							<div class={styles.spinner}></div>
						{/if}
						{isLoading ? 'Continuing...' : 'Continue'}
					</button>
				</div>
			{:else}
				<div class={styles.resolvedStatus} in:fade>
					{#if isContinued}
						<span>Resuming...</span>
					{:else if isReplanned}
						<span>Replanning...</span>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
