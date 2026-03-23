<script lang="ts">
	/**
	 * ApprovalDialog Component
	 * Modal dialog for approving or denying agent actions.
	 * Shows action details with optional diff/text preview.
	 */
	import Modal from '$lib/components/shared/Modal.svelte';
	import Button from '$lib/components/shared/Button.svelte';
	import { AlertTriangle, FileEdit, Trash2, Terminal, Globe } from 'lucide-svelte';
	import type { PendingAction, ActionType } from '$lib/types/agent';

	interface Props {
		action: PendingAction;
		onapprove: () => void;
		ondeny: () => void;
	}

	let { action, onapprove, ondeny }: Props = $props();

	let isLoading = $state(false);

	async function handleApprove() {
		isLoading = true;
		try {
			onapprove();
		} finally {
			isLoading = false;
		}
	}

	function getActionIcon(actionType: ActionType) {
		switch (actionType.type) {
			case 'write_file':
				return FileEdit;
			case 'delete_file':
				return Trash2;
			case 'execute_command':
				return Terminal;
			case 'web_request':
				return Globe;
			default:
				return AlertTriangle;
		}
	}

	function getActionLabel(actionType: ActionType): string {
		switch (actionType.type) {
			case 'write_file':
				return 'Write File';
			case 'delete_file':
				return 'Delete File';
			case 'execute_command':
				return 'Execute Command';
			case 'web_request':
				return 'Web Request';
			case 'read_file':
				return 'Read File';
			default:
				return 'Action';
		}
	}

	function getIconClass(actionType: ActionType): string {
		switch (actionType.type) {
			case 'delete_file':
				return 'danger';
			case 'execute_command':
				return 'info';
			default:
				return 'warning';
		}
	}

	let ActionIcon = $derived(getActionIcon(action.actionType));
	let actionLabel = $derived(getActionLabel(action.actionType));
	let iconClass = $derived(getIconClass(action.actionType));
</script>

<Modal open={true} onclose={ondeny} title="Action Approval">
	<div class="approval-dialog">
		<div class="dialog-header">
			<div class="icon-wrapper {iconClass}">
				<ActionIcon size={20} />
			</div>
			<div class="header-text">
				<h3>Allow {actionLabel}?</h3>
				<p class="tool-name">Tool: {action.toolName}</p>
			</div>
		</div>

		<div class="dialog-content">
			<p class="description">{action.description}</p>

			{#if action.preview}
				<div class="preview-section">
					<div class="preview-header">
						<span>Preview</span>
						{#if action.preview.language}
							<span class="preview-lang">{action.preview.language}</span>
						{/if}
					</div>
					<pre class="preview-content"><code>{action.preview.content}</code></pre>
				</div>
			{/if}
		</div>

		<div class="dialog-actions">
			<Button variant="ghost" onclick={ondeny} disabled={isLoading}>
				Deny
			</Button>
			<Button variant="primary" onclick={handleApprove} loading={isLoading}>
				Allow
			</Button>
		</div>
	</div>
</Modal>

<style>
	.approval-dialog {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-4);
		min-width: 400px;
		max-width: 600px;
	}

	.dialog-header {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.icon-wrapper {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		border-radius: var(--radius-lg);
		flex-shrink: 0;
	}

	.icon-wrapper.warning {
		background: var(--color-warning-light);
		color: var(--color-warning);
	}

	.icon-wrapper.danger {
		background: var(--color-error-light);
		color: var(--color-error);
	}

	.icon-wrapper.info {
		background: var(--color-surface-hover);
		color: var(--color-text-secondary);
	}

	.header-text h3 {
		font-size: var(--text-lg);
		font-weight: var(--font-semibold);
		color: var(--color-text-primary);
		margin: 0;
	}

	.tool-name {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
		margin: var(--space-1) 0 0 0;
	}

	.description {
		color: var(--color-text-secondary);
		line-height: var(--leading-relaxed);
		margin: 0;
	}

	.preview-section {
		margin-top: var(--space-2);
		border: 1px solid var(--color-border-primary);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.preview-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) var(--space-3);
		background: var(--color-surface);
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.preview-lang {
		font-family: var(--font-mono);
		color: var(--color-text-muted);
	}

	.preview-content {
		padding: var(--space-3);
		margin: 0;
		max-height: 200px;
		overflow: auto;
		background: var(--color-neutral-950);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-text-primary);
		line-height: var(--leading-relaxed);
	}

	.dialog-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		padding-top: var(--space-2);
	}
</style>
