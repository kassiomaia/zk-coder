<script lang="ts">
	/**
	 * WebSearchModal — dedicated modal for the Tools > Web Search configuration.
	 *
	 * Opened from SettingsModal. Renders as a stacked modal so the parent
	 * Settings view stays behind it.
	 */
	import Modal from '../../shared/Modal.svelte';
	import WebSearchSection from './WebSearchSection.svelte';
	import styles from './WebSearchModal.module.css';

	interface Props {
		open: boolean;
		onclose: () => void;
		/** Canonicalized workspace path; '' means no workspace active. */
		workspacePath?: string;
		/** Tilde-compressed display path for policy header. */
		workspaceDisplayPath?: string;
	}

	let { open, onclose, workspacePath = '', workspaceDisplayPath }: Props = $props();
</script>

<Modal title="Tools — Web Search" {open} {onclose} size="lg" stacked>
	{#snippet children()}
		<div class={styles.body}>
			<WebSearchSection {workspacePath} {workspaceDisplayPath} />
		</div>
	{/snippet}
</Modal>
