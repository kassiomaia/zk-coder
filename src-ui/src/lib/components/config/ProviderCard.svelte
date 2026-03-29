<script lang="ts">
    /**
     * ProviderCard Component
     * Standardized list item for providers.
     */
    import styles from './ProviderCard.module.css';
    import ProviderIcon from './ProviderIcon.svelte';
    import type {Provider} from '$lib/types/config';
    import {getProviderDefaultModels} from '$lib/types/config';

    interface Props {
    provider: Provider;
    onclick: () => void;
  }

  let { provider, onclick }: Props = $props();

  let cardClasses = $derived(
    [styles.card, provider.isConfigured && styles.configured].filter(Boolean).join(' ')
  );
</script>

<button class={cardClasses} {onclick} type="button" aria-label="Configure {provider.name}">
  <div class={styles.iconWrapper}>
    <ProviderIcon {provider} size={20} />
  </div>
  <div class={styles.details}>
    <div class={styles.name}>{provider.name}</div>
    <div class={styles.models}>
      {#if provider.isCustom}
        {provider.baseUrl ?? 'Custom Provider'}
      {:else}
        {getProviderDefaultModels(provider.id)}
      {/if}
    </div>
  </div>
  <div class={styles.status}>
    {#if provider.isConfigured}
      <span class={styles.configuredText}>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
        >
          <polyline points="20 6 9 17 4 12" />
        </svg>
        Configured
      </span>
    {:else}
      <span class={styles.actionLabel}>Click to configure</span>
    {/if}
  </div>
</button>
