<script lang="ts">
	/**
	 * AgentClarificationPrompt
	 *
	 * Surfaces the pending clarification question emitted by
	 * `agent_narrate` with `intent = "question"` (Workstream E.2/E.4).
	 * The harness is paused until the user replies or the clarification
	 * timeout fires on the backend. Reply submission re-uses the
	 * `sendRequest` path in `agentStore`, which is the same IPC the
	 * main chat input uses — no duplicate send logic.
	 *
	 * Accessibility notes:
	 * - The container is a `<section>` labelled by the question text
	 *   so screen readers announce the block before the composer.
	 * - The textarea is a native `<textarea>` with an explicit `<label>`.
	 * - Enter submits (matching the main chat input's default shortcut);
	 *   Shift+Enter adds a newline.
	 * - The primary submit uses the native `<button type="submit">` so
	 *   the form is keyboard-navigable by default.
	 */
	import { MessageCircleQuestion, Send } from 'lucide-svelte';
	import type { AwaitingClarificationDetails } from '$lib/types/agent';

	interface Props {
		sessionId: string;
		details: AwaitingClarificationDetails;
		onReply: (sessionId: string, content: string) => Promise<void> | void;
	}

	let { sessionId, details, onReply }: Props = $props();

	let reply = $state('');
	let busy = $state(false);
	let errorMessage = $state<string | null>(null);

	let trimmedReply = $derived(reply.trim());
	let canSubmit = $derived(!busy && trimmedReply.length > 0);

	function relativeTimestamp(iso: string): string {
		try {
			const asked = new Date(iso).getTime();
			const now = Date.now();
			const seconds = Math.max(0, Math.round((now - asked) / 1000));
			if (seconds < 60) return `${seconds}s ago`;
			const minutes = Math.round(seconds / 60);
			if (minutes < 60) return `${minutes}m ago`;
			const hours = Math.round(minutes / 60);
			return `${hours}h ago`;
		} catch {
			return '';
		}
	}

	async function submit(event?: Event) {
		event?.preventDefault();
		if (!canSubmit) return;
		busy = true;
		errorMessage = null;
		try {
			await onReply(sessionId, trimmedReply);
			reply = '';
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : String(err);
		} finally {
			busy = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			void submit();
		}
	}
</script>

<section
	class="clarification"
	aria-labelledby="clarification-question-{sessionId}"
	style="animation-delay: 0.15s"
>
	<header class="clarification-header">
		<span class="clarification-icon" aria-hidden="true">
			<MessageCircleQuestion size={16} />
		</span>
		<span class="clarification-label">Question for you</span>
		<span class="clarification-timestamp" title={details.askedAt}>
			{relativeTimestamp(details.askedAt)}
		</span>
	</header>

	<p id="clarification-question-{sessionId}" class="clarification-question">
		{details.prompt}
	</p>

	<form class="clarification-form" onsubmit={submit}>
		<label class="visually-hidden" for="clarification-reply-{sessionId}">
			Reply to the agent's question
		</label>
		<textarea
			id="clarification-reply-{sessionId}"
			class="clarification-textarea"
			bind:value={reply}
			onkeydown={handleKeydown}
			placeholder="Answer the question to resume execution…"
			rows="3"
			disabled={busy}
			aria-describedby={errorMessage
				? `clarification-error-${sessionId}`
				: `clarification-hint-${sessionId}`}
		></textarea>

		{#if errorMessage}
			<p
				id="clarification-error-{sessionId}"
				class="clarification-error"
				role="alert"
			>
				{errorMessage}
			</p>
		{:else}
			<p id="clarification-hint-{sessionId}" class="clarification-hint">
				Enter to send · Shift+Enter for a new line
			</p>
		{/if}

		<div class="clarification-actions">
			<button type="submit" class="reply-button" disabled={!canSubmit}>
				<Send size={14} aria-hidden="true" />
				<span>{busy ? 'Sending…' : 'Send reply'}</span>
			</button>
		</div>
	</form>
</section>

<style>
	.clarification {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3);
		margin: var(--space-2) var(--space-3);
		background: var(--color-warning-light);
		border: 1px solid var(--color-warning-border);
		border-left: 3px solid var(--color-warning);
		border-radius: var(--radius-md);
		animation: clarification-fade-in 0.3s var(--ease-out) backwards;
	}

	@keyframes clarification-fade-in {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.clarification-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.clarification-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: var(--color-warning);
	}

	.clarification-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-semibold);
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--color-warning-dark);
	}

	.clarification-timestamp {
		margin-left: auto;
		font-size: var(--text-2xs);
		color: var(--color-text-tertiary);
	}

	.clarification-question {
		margin: 0;
		font-size: var(--text-sm);
		line-height: 1.5;
		color: var(--color-text-primary);
	}

	.clarification-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.clarification-textarea {
		resize: vertical;
		min-height: calc(var(--space-6) * 2);
		padding: var(--space-2);
		font-family: inherit;
		font-size: var(--text-sm);
		line-height: 1.5;
		color: var(--color-text-primary);
		background: var(--depth-bg-1);
		border: 1px solid var(--color-border-primary);
		border-radius: var(--radius-sm);
		transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
	}

	.clarification-textarea:focus-visible {
		outline: none;
		border-color: var(--color-warning);
		box-shadow: 0 0 0 2px var(--color-warning-border);
	}

	.clarification-textarea:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.clarification-hint,
	.clarification-error {
		margin: 0;
		font-size: var(--text-2xs);
	}

	.clarification-hint {
		color: var(--color-text-tertiary);
	}

	.clarification-error {
		color: var(--color-danger, var(--color-warning-dark));
	}

	.clarification-actions {
		display: flex;
		justify-content: flex-end;
	}

	.reply-button {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1-5);
		padding: var(--space-1) var(--space-3);
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-on-accent, #fff);
		background: var(--color-warning);
		border: 1px solid var(--color-warning-dark);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition:
			background var(--transition-fast),
			transform var(--transition-fast),
			opacity var(--transition-fast);
	}

	.reply-button:hover:not(:disabled) {
		background: var(--color-warning-dark);
	}

	.reply-button:focus-visible {
		outline: 2px solid var(--color-warning-border);
		outline-offset: 2px;
	}

	.reply-button:active:not(:disabled) {
		transform: translateY(1px);
	}

	.reply-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}

	@media (prefers-reduced-motion: reduce) {
		.clarification {
			animation: none;
		}
		.reply-button {
			transition: none;
		}
	}
</style>
