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

	// Group commands by category
	let groupedCommands = $derived(groupCommandsByCategory(commands));
	let flatCommands = $derived(commands);

	// Reset selection when commands change
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

	// Attach keyboard listener
	$effect(() => {
		document.addEventListener('keydown', handleKeydown);
		return () => document.removeEventListener('keydown', handleKeydown);
	});

	// Get icon SVG by name
	function getIconSvg(icon: string | undefined): string {
		return icon || 'terminal';
	}
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
			{#each Array.from(groupedCommands.entries()) as [category, cmds], categoryIndex}
				<li class={styles.category}>
					<span class={styles.categoryLabel}>{getCategoryLabel(category)}</span>
					<ul class={styles.categoryList}>
						{#each cmds as command, cmdIndex}
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
								<span class={styles.itemIcon}>
									<svg
										width="16"
										height="16"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
										stroke-linecap="round"
										stroke-linejoin="round"
									>
										{#if command.icon === 'code'}
											<polyline points="16 18 22 12 16 6" />
											<polyline points="8 6 2 12 8 18" />
										{:else if command.icon === 'book'}
											<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
											<path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
										{:else if command.icon === 'refresh'}
											<polyline points="23 4 23 10 17 10" />
											<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
										{:else if command.icon === 'zap'}
											<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
										{:else if command.icon === 'check-circle'}
											<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
											<polyline points="22 4 12 14.01 9 11.01" />
										{:else if command.icon === 'file-text'}
											<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
											<polyline points="14 2 14 8 20 8" />
											<line x1="16" y1="13" x2="8" y2="13" />
											<line x1="16" y1="17" x2="8" y2="17" />
										{:else if command.icon === 'message-square'}
											<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
										{:else if command.icon === 'trash'}
											<polyline points="3 6 5 6 21 6" />
											<path
												d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
											/>
										{:else if command.icon === 'download'}
											<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
											<polyline points="7 10 12 15 17 10" />
											<line x1="12" y1="15" x2="12" y2="3" />
										{:else if command.icon === 'file'}
											<path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
											<polyline points="13 2 13 9 20 9" />
										{:else if command.icon === 'hash'}
											<line x1="4" y1="9" x2="20" y2="9" />
											<line x1="4" y1="15" x2="20" y2="15" />
											<line x1="10" y1="3" x2="8" y2="21" />
											<line x1="16" y1="3" x2="14" y2="21" />
										{:else if command.icon === 'git-branch'}
											<line x1="6" y1="3" x2="6" y2="15" />
											<circle cx="18" cy="6" r="3" />
											<circle cx="6" cy="18" r="3" />
											<path d="M18 9a9 9 0 0 1-9 9" />
										{:else if command.icon === 'git-commit'}
											<circle cx="12" cy="12" r="4" />
											<line x1="1.05" y1="12" x2="7" y2="12" />
											<line x1="17.01" y1="12" x2="22.96" y2="12" />
										{:else if command.icon === 'git-compare'}
											<circle cx="18" cy="18" r="3" />
											<circle cx="6" cy="6" r="3" />
											<path d="M13 6h3a2 2 0 0 1 2 2v7" />
											<line x1="6" y1="9" x2="6" y2="21" />
										{:else if command.icon === 'cpu'}
											<rect x="4" y="4" width="16" height="16" rx="2" ry="2" />
											<rect x="9" y="9" width="6" height="6" />
											<line x1="9" y1="1" x2="9" y2="4" />
											<line x1="15" y1="1" x2="15" y2="4" />
											<line x1="9" y1="20" x2="9" y2="23" />
											<line x1="15" y1="20" x2="15" y2="23" />
											<line x1="20" y1="9" x2="23" y2="9" />
											<line x1="20" y1="14" x2="23" y2="14" />
											<line x1="1" y1="9" x2="4" y2="9" />
											<line x1="1" y1="14" x2="4" y2="14" />
										{:else if command.icon === 'x-circle'}
											<circle cx="12" cy="12" r="10" />
											<line x1="15" y1="9" x2="9" y2="15" />
											<line x1="9" y1="9" x2="15" y2="15" />
										{:else if command.icon === 'settings'}
											<circle cx="12" cy="12" r="3" />
											<path
												d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
											/>
										{:else if command.icon === 'moon'}
											<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
										{:else}
											<polyline points="4 17 10 11 4 5" />
											<line x1="12" y1="19" x2="20" y2="19" />
										{/if}
									</svg>
								</span>
								<span class={styles.itemContent}>
									<span class={styles.itemName}>/{command.name}</span>
									<span class={styles.itemDesc}>{command.description}</span>
								</span>
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
			<span class={styles.hint}>
				<kbd>↑↓</kbd> to navigate
				<kbd>Enter</kbd> to select
				<kbd>Esc</kbd> to close
			</span>
		</div>
	</div>
{:else}
	<div class={styles.empty}>
		<span class={styles.emptyText}>No commands found for "{query}"</span>
	</div>
{/if}
