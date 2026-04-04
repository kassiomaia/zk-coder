<script lang="ts">
	/**
	 * Spinner Component
	 * A loading spinner animation for indicating pending operations.
	 */
	import { SpinnerIcon } from '$lib/icons';

	interface Props {
		size?: number;
		class?: string;
	}

	let { size = 16, class: className = '' }: Props = $props();
</script>

<SpinnerIcon {size} class={className} />
