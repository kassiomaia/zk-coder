<script lang="ts">
/**
 * ApprovalMessage Component
 *
 * Displays an inline approval card in the chat for agent actions requiring user permission.
 * Shows action details, optional preview, and Allow/Deny buttons.
 */
import styles from './ApprovalMessage.module.css';
import DiffViewer from '$lib/components/agent/DiffViewer.svelte';
import type {Message} from '$lib/types/message';
import type {ActionType} from '$lib/types/agent';
import {AlertTriangle, CheckCircle2, ChevronDown, ChevronUp, FileEdit, Globe, ShieldCheck, Terminal, Trash2, XCircle} from 'lucide-svelte';
import { slide, fade } from 'svelte/transition';
import { cubicOut } from 'svelte/easing';

interface Props {
	message: Message;
	onApprove?: (sessionId: string, actionId: string) => void;
	onDeny?: (sessionId: string, actionId: string) => void;
}

let {message, onApprove, onDeny}: Props = $props();

let isLoading = $state(false);
let isExpanded = $state(false);

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

function getActionLabel(actionType: ActionType | undefined): string {
	if (!actionType) return 'action';
	switch (actionType.type) {
		case 'write_file':
			return 'modifying a file';
		case 'delete_file':
			return 'removing a file';
		case 'execute_command':
			return 'running a command';
		case 'web_request':
			return 'accessing the internet';
		case 'read_file':
			return 'reading a file';
		default:
			return 'performing an action';
	}
}

function getActionImpact(actionType: ActionType | undefined): string {
	if (!actionType) return 'This action requires your confirmation to proceed safely.';
	switch (actionType.type) {
		case 'write_file':
			return 'The system will update the contents of a local file.';
		case 'delete_file':
			return 'This will permanently remove the specified file from your system.';
		case 'execute_command':
			return 'A terminal command will be executed in your current environment.';
		case 'web_request':
			return 'The system will send or receive data from an external web service.';
		case 'read_file':
			return 'The system will access and read the contents of a local file.';
		default:
			return 'This action requires your confirmation to proceed safely.';
	}
}

let actionLabel = $derived(getActionLabel(pendingAction?.actionType));
let actionImpact = $derived(getActionImpact(pendingAction?.actionType));
let isResolved = $derived(approvalResolved);
let isApproved = $derived(approvalDecision === 'approved');
let isDenied = $derived(approvalDecision === 'denied');

function toggleExpand() {
	isExpanded = !isExpanded;
}
</script>

<div 
	class={styles.approvalContainer} 
	in:fade={{ duration: 200, easing: cubicOut }}
>
	<div class={styles.contentRow}>
		<!-- Zone 1: Intent & Context -->
		<div class={styles.intentZone}>
			<div class={styles.iconWrapper}>
				<ShieldCheck size={16} strokeWidth={2.5} />
			</div>
			<div class={styles.textWrapper}>
				<div class={styles.primaryText}>
					Assistant wants permission for <span class={styles.actionHighlight}>{actionLabel}</span>
				</div>
				<div class={styles.secondaryText}>
					{pendingAction?.description || actionImpact}
				</div>
			</div>
		</div>

		<!-- Zone 2: Actions -->
		<div class={styles.actionZone}>
			{#if !isResolved}
				<div class={styles.buttonGroup}>
					<button
						type="button"
						class={[styles.btn, styles.btnSecondary].join(' ')}
						onclick={handleDeny}
						disabled={isLoading}
					>
						Deny
					</button>
					<button
						type="button"
						class={[styles.btn, styles.btnPrimary].join(' ')}
						onclick={handleApprove}
						disabled={isLoading}
						aria-busy={isLoading}
					>
						{#if isLoading}
							<div class={styles.spinner}></div>
						{/if}
						{isLoading ? 'Allowing...' : 'Allow'}
					</button>
				</div>
			{:else}
				<div class={styles.resolvedStatus} in:fade>
					{#if isApproved}
						<CheckCircle2 size={14} class={styles.successIcon} />
						<span>Permission Granted</span>
					{:else if isDenied}
						<XCircle size={14} class={styles.errorIcon} />
						<span>Action Denied</span>
					{/if}
				</div>
			{/if}
		</div>
	</div>

	{#if pendingAction?.preview && !isResolved}
		<div class={styles.disclosureZone}>
			<button 
				type="button"
				class={styles.disclosureTrigger}
				onclick={toggleExpand}
				aria-expanded={isExpanded}
			>
				<span>{isExpanded ? 'Hide' : 'Inspect'} technical details</span>
				{#if isExpanded}
					<ChevronUp size={12} />
				{:else}
					<ChevronDown size={12} />
				{/if}
			</button>

			{#if isExpanded}
				<div 
					class={styles.previewWrapper}
					transition:slide={{ duration: 200, easing: cubicOut }}
				>
					<div class={styles.previewHeader}>
						<span>{pendingAction.preview.contentType === 'diff' ? 'Proposed Changes' : 'Action Details'}</span>
						{#if pendingAction.preview.language}
							<code>{pendingAction.preview.language}</code>
						{/if}
					</div>
					<div class={styles.previewScroll}>
						{#if pendingAction.preview.contentType === 'diff'}
							<DiffViewer diff={pendingAction.preview.content} maxHeight={200} />
						{:else}
							<pre class={styles.previewContent}><code>{pendingAction.preview.content}</code></pre>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
