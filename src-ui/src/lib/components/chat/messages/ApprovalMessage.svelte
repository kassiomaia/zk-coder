<script lang="ts">
/**
 * ApprovalMessage Component
 *
 * Displays an inline approval card in the chat for agent actions requiring user permission.
 * Shows action details, optional preview, and Allow/Deny buttons.
 */
import Button from '$lib/components/shared/Button.svelte';
import DiffViewer from '$lib/components/agent/DiffViewer.svelte';
import type {Message} from '$lib/types/message';
import type {ActionType} from '$lib/types/agent';
import styles from './ApprovalMessage.module.css';
import {AlertTriangle, CheckCircle2, FileEdit, Globe, Terminal, Trash2, XCircle} from 'lucide-svelte';

interface Props {
	message: Message;
	onApprove?: (sessionId: string, actionId: string) => void;
	onDeny?: (sessionId: string, actionId: string) => void;
}

let {message, onApprove, onDeny}: Props = $props();

let isLoading = $state(false);

const agentData = $derived(message.agentData);
const pendingAction = $derived(agentData?.pendingAction);
const approvalResolved = $derived(agentData?.approvalResolved ?? false);
const approvalDecision = $derived(agentData?.approvalDecision);
const sessionId = $derived(agentData?.sessionId ?? '');
const actionId = $derived(pendingAction?.id ?? '');

async function handleApprove() {
	if (!onApprove || approvalResolved) return;
	isLoading = true;
	try {
		await onApprove(sessionId, actionId);
	} finally {
		isLoading = false;
	}
}

async function handleDeny() {
	if (!onDeny || approvalResolved) return;
	isLoading = true;
	try {
		await onDeny(sessionId, actionId);
	} finally {
		isLoading = false;
	}
}

function getActionIcon(actionType: ActionType | undefined) {
	if (!actionType) return AlertTriangle;
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

function getActionLabel(actionType: ActionType | undefined): string {
	if (!actionType) return 'Action';
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

function getIconClass(actionType: ActionType | undefined): string {
	if (!actionType) return 'warning';
	switch (actionType.type) {
		case 'delete_file':
			return 'danger';
		case 'execute_command':
			return 'info';
		default:
			return 'warning';
	}
}

let ActionIcon = $derived(getActionIcon(pendingAction?.actionType));
let actionLabel = $derived(getActionLabel(pendingAction?.actionType));
let iconClass = $derived(getIconClass(pendingAction?.actionType));
let isResolved = $derived(approvalResolved);
let isApproved = $derived(approvalDecision === 'approved');
let isDenied = $derived(approvalDecision === 'denied');
</script>

<div class={[styles.approvalCard, iconClass].filter(Boolean).join(' ')}>
	<div class={styles.header}>
		<div class={styles.iconWrapper}>
			<ActionIcon size={16} />
		</div>
		<div class={styles.headerText}>
			<span class={styles.title}>Allow {actionLabel}?</span>
		</div>
	</div>

	{#if pendingAction?.description}
		<p class={styles.description}>{pendingAction.description}</p>
	{/if}

	{#if pendingAction?.preview && !isResolved}
		<div class={styles.previewSection}>
			<div class={styles.previewHeader}>
				<span>{pendingAction.preview.contentType === 'diff' ? 'Changes' : 'Preview'}</span>
				{#if pendingAction.preview.language}
					<span class={styles.previewLang}>{pendingAction.preview.language}</span>
				{/if}
			</div>
			{#if pendingAction.preview.contentType === 'diff'}
				<DiffViewer diff={pendingAction.preview.content} maxHeight={200} />
			{:else}
				<pre class={styles.previewContent}><code>{pendingAction.preview.content}</code></pre>
			{/if}
		</div>
	{/if}

	<div class={styles.actions}>
		{#if !isResolved}
			<Button variant="ghost" onclick={handleDeny} disabled={isLoading} size="sm">Deny</Button>
			<Button variant="primary" onclick={handleApprove} loading={isLoading} size="sm">Allow</Button>
		{:else if isApproved}
			<span class={styles.resolvedBadge} class:approved={true}>
				<CheckCircle2 size={14} />
				Approved
			</span>
		{:else if isDenied}
			<span class={styles.resolvedBadge} class:denied={true}>
				<XCircle size={14} />
				Denied
			</span>
		{/if}
	</div>
</div>
