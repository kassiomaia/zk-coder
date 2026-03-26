<script lang="ts">
	interface Props {
		onclick?: () => void;
		disabled?: boolean;
	}

	let { onclick, disabled = false }: Props = $props();
</script>

<button
	class="mock-new-thread-button"
	data-has-click={!!onclick}
	{disabled}
	onclick={onclick}
>
	New Thread
</button>
