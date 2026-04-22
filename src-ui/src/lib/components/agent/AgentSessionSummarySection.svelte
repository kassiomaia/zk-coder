<script lang="ts">
	/**
	 * AgentSessionSummarySection
	 * Durable session summary for the agent panel. Surfaces harness mode,
	 * startup verification, and handoff context so paused/resumed sessions
	 * can be understood without scrolling the chat transcript.
	 */
	import { Play } from 'lucide-svelte';
	import type {
		AgentSessionHarnessState,
		SessionState,
		StartupVerificationState
	} from '$lib/types/agent';

	interface Props {
		sessionId: string;
		sessionState: SessionState;
		harnessState: AgentSessionHarnessState;
		onContinue?: (sessionId: string) => void;
		onReplan?: (sessionId: string) => void;
	}

	let { sessionId, sessionState, harnessState, onContinue, onReplan }: Props = $props();

	let verification = $derived(harnessState.startupVerification);
	let isPaused = $derived(sessionState.type === 'paused');
	let canResume = $derived(isPaused && harnessState.mode === 'resume_verifier');
	let isBlocked = $derived(verification?.state === 'failed');
	let busy = $state(false);

	function modeLabel(mode: AgentSessionHarnessState['mode']): string {
		return mode.replace('_', ' ');
	}

	function verificationPillClass(state: StartupVerificationState): string {
		switch (state) {
			case 'running':
				return 'pill pill-running';
			case 'passed':
				return 'pill pill-passed';
			case 'failed':
				return 'pill pill-failed';
			case 'idle':
				return 'pill pill-idle';
		}
	}

	function verificationPillLabel(state: StartupVerificationState): string {
		switch (state) {
			case 'running':
				return 'Verifying';
			case 'passed':
				return 'Verified';
			case 'failed':
				return 'Blocked';
			case 'idle':
				return 'Not checked';
		}
	}

	async function handleContinue() {
		if (!onContinue || busy) return;
		busy = true;
		try {
			await onContinue(sessionId);
		} finally {
			busy = false;
		}
	}

	async function handleReplan() {
		if (!onReplan || busy) return;
		busy = true;
		try {
			await onReplan(sessionId);
		} finally {
			busy = false;
		}
	}
</script>

<section class="session-summary-section" aria-label="Session summary">
	<div class="summary-header-row">
		<div>
			<h4 class="summary-title">Session Summary</h4>
			<p class="summary-subtitle">Verification context for this session.</p>
		</div>
		<span class="summary-mode">{modeLabel(harnessState.mode)}</span>
	</div>

	{#if verification}
		<div class="summary-grid">
			<div class="summary-item">
				<span class="summary-label">Verification</span>
				<span class="summary-value verification-value">
					<span class={verificationPillClass(verification.state)}>
						{verificationPillLabel(verification.state)}
					</span>
					<span class="verification-headline">{verification.headline}</span>
				</span>
				{#if verification.details && verification.details.length > 0}
					<ul class="verification-details">
						{#each verification.details as detail}
							<li>{detail}</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>
	{/if}

	{#if canResume}
		<div class="resume-actions" role="group" aria-label="Resume options">
			<p class="resume-hint">
				{isBlocked
					? 'Resume is blocked. Replan or investigate before continuing.'
					: 'Pick up where you left off.'}
			</p>
			<div class="resume-buttons">
				<button
					type="button"
					class="btn btn-secondary"
					onclick={handleReplan}
					disabled={busy || !onReplan}
					aria-label="Replan session"
				>
					Replan
				</button>
				<button
					type="button"
					class="btn btn-primary"
					onclick={handleContinue}
					disabled={busy || !onContinue || isBlocked}
					aria-busy={busy}
					aria-label="Continue session"
				>
					{#if busy}
						<span class="spinner" aria-hidden="true"></span>
					{:else}
						<Play size={12} strokeWidth={2.5} />
					{/if}
					{busy ? 'Resuming...' : 'Continue'}
				</button>
			</div>
		</div>
	{/if}
</section>

<style>
	.session-summary-section {
		padding: var(--space-3);
		background: var(--depth-bg-1);
		border-bottom: 1px solid var(--color-border-primary);
		animation: section-fade-in 0.4s var(--ease-out) backwards;
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

	.verification-value {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1-5);
	}

	.verification-headline {
		flex: 1;
		min-width: 0;
	}

	.verification-details {
		margin: var(--space-1) 0 0;
		padding-left: var(--space-4);
		font-size: var(--text-2xs);
		color: var(--color-text-secondary);
		line-height: 1.4;
	}

	.pill {
		display: inline-flex;
		align-items: center;
		height: 18px;
		padding: 0 var(--space-1-5);
		border-radius: var(--radius-full);
		font-size: var(--text-2xs);
		font-weight: var(--font-semibold);
		letter-spacing: 0.02em;
		text-transform: uppercase;
		white-space: nowrap;
	}

	.pill-running {
		background: var(--color-primary-subtle);
		color: var(--color-primary-400);
	}

	.pill-passed {
		background: var(--color-success-light, var(--color-success));
		color: var(--color-success);
	}

	.pill-failed {
		background: var(--color-warning-light, var(--color-warning));
		color: var(--color-warning);
	}

	.pill-idle {
		background: var(--color-surface-hover);
		color: var(--color-text-muted);
	}

	.resume-actions {
		margin-top: var(--space-3);
		padding-top: var(--space-3);
		border-top: 1px solid var(--color-border-subtle);
	}

	.resume-hint {
		margin: 0 0 var(--space-2);
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
	}

	.resume-buttons {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-1-5);
		height: 30px;
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
</style>
