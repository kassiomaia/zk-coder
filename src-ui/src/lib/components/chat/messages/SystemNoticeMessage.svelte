<script lang="ts">
	import { Minimize2, Scan } from 'lucide-svelte';
	import type { Message } from '$lib/types/message';
	import { getContextNoticeCopy } from '$lib/utils/systemNotices';
	import styles from './SystemNoticeMessage.module.css';

	interface Props {
		message: Message;
	}

	let { message }: Props = $props();

	const notice = $derived(message.metadata?.systemNotice);
	const copy = $derived(
		notice
			? getContextNoticeCopy(notice.reason)
			: { title: 'Context updated', body: message.content }
	);
</script>

{#if notice}
	<div class={styles.noticeCard} aria-label={copy.title}>
		<div class={styles.iconWrap} aria-hidden="true">
			{#if notice.reason === 'token_limit'}
				<Minimize2 size={16} strokeWidth={2} />
			{:else}
				<Scan size={16} strokeWidth={2} />
			{/if}
		</div>

		<div class={styles.content}>
			<div class={styles.title}>{copy.title}</div>
			<p class={styles.body}>{copy.body}</p>
		</div>

		{#if notice.beforeTokens !== notice.afterTokens}
			<div class={styles.metrics}>
				<span class={styles.metricValue}>
					{notice.beforeTokens.toLocaleString()} -> {notice.afterTokens.toLocaleString()} tokens
				</span>
			</div>
		{/if}
	</div>
{:else}
	<div class={styles.noticeFallback}>
		{message.content}
	</div>
{/if}
