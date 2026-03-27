<script lang="ts">
/**
 * TerminalOutput Component
 *
 * Displays terminal command output as a transient message.
 * Monospace font with dark terminal styling.
 */

import type {Message} from '$lib/types/message';
import styles from './TerminalOutput.module.css';

interface Props {
	message: Message;
}

let { message }: Props = $props();

// Use $derived for reactive values from props
const streamType = $derived(message.agentData?.streamType ?? 'stdout');
const isCommand = $derived(streamType === 'command');
const isStderr = $derived(streamType === 'stderr');
const content = $derived(message.content ?? '');
const isTransient = $derived(message.isTransient ?? false);

// Auto-dismiss logic for transient messages
let dismissed = $state(false);

$effect(() => {
	if (message.isTransient) {
		// Auto-dismiss after 5 seconds for transient messages
		const timeout = setTimeout(() => {
			dismissed = true;
		}, 5000);

		return () => clearTimeout(timeout);
	}
});
</script>

{#if !dismissed}
	<div
		class={[
			styles.terminalOutput,
			isCommand && styles.command,
			isStderr && styles.stderr
		]
			.filter(Boolean)
			.join(' ')}
	>
		{#if isCommand}
			<span class={styles.prompt}>$ </span>
		{/if}
		<span class={styles.content}>{content}</span>
		{#if isTransient}
			<button
				class={styles.dismiss}
				onclick={() => (dismissed = true)}
				type="button"
				aria-label="Dismiss"
			>
				×
			</button>
		{/if}
	</div>
{/if}
