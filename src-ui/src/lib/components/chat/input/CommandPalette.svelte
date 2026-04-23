<script lang="ts">
	/**
	 * CommandPalette Component
	 * Dropdown menu for selecting commands, triggered by "/" in the input.
	 */
	import styles from './CommandPalette.module.css';
	import type { Command } from '$lib/types/command';
	import { getCategoryLabel, groupCommandsByCategory } from '$lib/types/command';

	interface Props {
		/** Current search query */
		query: string;
		/** Available commands to display */
		commands: Command[];
		/** Handler when a command is selected */
		onSelect: (command: Command) => void;
		/** Handler to close the palette */
		onClose: () => void;
	}

	let { query, commands, onSelect, onClose }: Props = $props();

	let selectedIndex = $state(0);
	let listRef: HTMLUListElement | undefined = $state();

	let groupedCommands = $derived(groupCommandsByCategory(commands));
	let flatCommands = $derived(commands);

	$effect(() => {
		if (flatCommands.length > 0) {
			selectedIndex = 0;
		}
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, flatCommands.length - 1);
			scrollToSelected();
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
			scrollToSelected();
		} else if (e.key === 'Enter') {
			e.preventDefault();
			if (flatCommands[selectedIndex]) {
				onSelect(flatCommands[selectedIndex]);
			}
		} else if (e.key === 'Escape') {
			e.preventDefault();
			onClose();
		}
	}

	function scrollToSelected() {
		const selectedEl = listRef?.querySelector('[data-selected="true"]');
		selectedEl?.scrollIntoView({ block: 'nearest' });
	}

	function handleItemClick(command: Command, index: number) {
		selectedIndex = index;
		onSelect(command);
	}

	$effect(() => {
		document.addEventListener('keydown', handleKeydown);
		return () => document.removeEventListener('keydown', handleKeydown);
	});
</script>

{#if flatCommands.length > 0}
	<div
		class={styles.palette}
		role="listbox"
		aria-label="Available commands"
		aria-activedescendant={flatCommands.length > 0 ? `command-option-${selectedIndex}` : undefined}
		tabindex="-1"
	>
		<div class={styles.header}>
			<span class={styles.title}>Commands</span>
			{#if query}
				<span class={styles.query}>/{query}</span>
			{/if}
		</div>

		<ul bind:this={listRef} class={styles.list}>
			{#each Array.from(groupedCommands.entries()) as [category, cmds]}
				<li class={styles.category}>
					<span class={styles.categoryLabel}>{getCategoryLabel(category)}</span>
					<ul class={styles.categoryList}>
						{#each cmds as command}
							{@const globalIndex = flatCommands.indexOf(command)}
							<button
								id="command-option-{globalIndex}"
								class={[styles.item, globalIndex === selectedIndex && styles.selected]
									.filter(Boolean)
									.join(' ')}
								role="option"
								aria-selected={globalIndex === selectedIndex}
								data-selected={globalIndex === selectedIndex}
								onclick={() => handleItemClick(command, globalIndex)}
								onmouseenter={() => (selectedIndex = globalIndex)}
								type="button"
							>
								<span class={styles.itemName}>/{command.name}</span>
								<span class={styles.itemDesc}>{command.description}</span>
								{#if command.shortcut}
									<span class={styles.shortcut}>{command.shortcut}</span>
								{/if}
							</button>
						{/each}
					</ul>
				</li>
			{/each}
		</ul>

		<div class={styles.footer}>
			<span class={styles.hint}>Esc to close</span>
		</div>
	</div>
{:else}
	<div class={styles.empty}>
		<span class={styles.emptyText}>No commands found for "{query}"</span>
	</div>
{/if}
