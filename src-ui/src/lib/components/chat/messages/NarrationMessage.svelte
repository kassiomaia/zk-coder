<script lang="ts">
/**
 * NarrationMessage Component
 *
 * Displays a persistent natural-language update from the agent.
 * These are lighter than full responses but provide context during execution.
 */
import type {Message} from '$lib/types/message';
import styles from './NarrationMessage.module.css';
import {Info} from 'lucide-svelte';

interface Props {
	message: Message;
}

let {message}: Props = $props();
</script>

<div class={styles.narration}>
	<span class={styles.icon}>
		<Info size={14} />
	</span>
	<span class={styles.content}>{message.content}</span>
</div>
