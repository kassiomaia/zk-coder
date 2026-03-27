<script lang="ts">
    /**
     * ManualModelList Component
     * Displays and manages manually configured models for a provider.
     */
    import styles from './ManualModelList.module.css';
    import {invoke} from '@tauri-apps/api/core';

    interface ModelCapabilities {
		analysis: boolean;
		code: boolean;
		vision: boolean;
		audio: boolean;
		tips: boolean;
		review: boolean;
	}

	interface ManualModelEntry {
		name: string;
		capabilities: ModelCapabilities;
	}

	interface Props {
		/** Provider ID */
		providerId: string;
	}

	let { providerId }: Props = $props();

	let models = $state<ManualModelEntry[]>([]);
	let isLoading = $state(false);
	let showAddDialog = $state(false);
	let newModelName = $state('');
	let newCapabilities = $state<ModelCapabilities>({
		analysis: true,
		code: true,
		vision: false,
		audio: false,
		tips: true,
		review: true
	});
	let errorMessage = $state('');
	let isSaving = $state(false);

	async function loadModels() {
		isLoading = true;
		errorMessage = '';
		try {
			const result = await invoke<ManualModelEntry[]>('get_manual_models', {
				providerId
			});
			models = result;
		} catch (error) {
			errorMessage = error as string;
		} finally {
			isLoading = false;
		}
	}

	async function addModel() {
		if (!newModelName.trim()) {
			errorMessage = 'Model name is required';
			return;
		}

		isSaving = true;
		errorMessage = '';
		try {
			await invoke('add_manual_model', {
				request: {
					providerId,
					name: newModelName.trim(),
					capabilities: newCapabilities
				}
			});
			newModelName = '';
			newCapabilities = {
				analysis: true,
				code: true,
				vision: false,
				audio: false,
				tips: true,
				review: true
			};
			showAddDialog = false;
			await loadModels();
		} catch (error) {
			errorMessage = error as string;
		} finally {
			isSaving = false;
		}
	}

	async function removeModel(modelName: string) {
		if (!confirm(`Remove model "${modelName}"?`)) {
			return;
		}

		errorMessage = '';
		try {
			await invoke('remove_manual_model', {
				providerId,
				modelName
			});
			await loadModels();
		} catch (error) {
			errorMessage = error as string;
		}
	}

	function openAddDialog() {
		// Infer capabilities from model name
		const name = newModelName.toLowerCase();
		newCapabilities = {
			analysis: true,
			code: true,
			vision: name.includes('vision') || name.includes('gpt-4o') || name.includes('claude-3'),
			audio: name.includes('whisper') || name.includes('tts') || name.includes('audio'),
			tips: true,
			review: true
		};
		showAddDialog = true;
	}

	// Load models on mount
	$effect(() => {
		loadModels();
	});
</script>

<div class={styles.container}>
	<header class={styles.header}>
		<h3 class={styles.title}>Manual Models</h3>
		<button class={styles.addButton} onclick={() => (showAddDialog = true)} type="button">
			<svg
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<path d="M12 5v14M5 12h14" />
			</svg>
			Add Model
		</button>
	</header>

	{#if errorMessage}
		<div class={styles.error}>{errorMessage}</div>
	{/if}

	{#if isLoading}
		<div class={styles.loading}>Loading models...</div>
	{:else if models.length === 0}
		<div class={styles.empty}>
			No manual models configured. Add models to use them with this provider.
		</div>
	{:else}
		<ul class={styles.list}>
			{#each models as model (model.name)}
				<li class={styles.item}>
					<div class={styles.modelInfo}>
						<span class={styles.modelName}>{model.name}</span>
						<span class={styles.capabilities}>
							{#if model.capabilities.vision}Vision{/if}
							{#if model.capabilities.audio}Audio{/if}
							{#if model.capabilities.code}Code{/if}
						</span>
					</div>
					<button
						class={styles.removeButton}
						onclick={() => removeModel(model.name)}
						aria-label="Remove model"
						type="button"
					>
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<polyline points="3 6 5 6 21 6" />
							<path
								d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
							/>
						</svg>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>

{#if showAddDialog}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class={styles.dialogOverlay} onclick={() => (showAddDialog = false)} role="presentation">
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class={styles.dialog} onclick={(e) => e.stopPropagation()} role="presentation">
			<header class={styles.dialogHeader}>
				<h3>Add Manual Model</h3>
				<button
					class={styles.closeButton}
					onclick={() => (showAddDialog = false)}
					aria-label="Close"
					type="button"
				>
					<svg
						width="20"
						height="20"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<path d="M18 6L6 18M6 6l12 12" />
					</svg>
				</button>
			</header>

			<div class={styles.dialogBody}>
				<label class={styles.label} for="model-name">
					Model Name
					<span class={styles.required}>*</span>
				</label>
				<input
					id="model-name"
					class={styles.input}
					bind:value={newModelName}
					placeholder="e.g., gpt-4-turbo, claude-3-opus-20240229"
					onkeydown={(e) => e.key === 'Enter' && addModel()}
				/>

				<fieldset class={styles.fieldset}>
					<legend class={styles.legend}>Capabilities</legend>
					<label class={styles.checkboxLabel}>
						<input
							type="checkbox"
							bind:checked={newCapabilities.analysis}
							class={styles.checkbox}
						/>
						<span>Analysis</span>
					</label>
					<label class={styles.checkboxLabel}>
						<input type="checkbox" bind:checked={newCapabilities.code} class={styles.checkbox} />
						<span>Code</span>
					</label>
					<label class={styles.checkboxLabel}>
						<input type="checkbox" bind:checked={newCapabilities.vision} class={styles.checkbox} />
						<span>Vision</span>
					</label>
					<label class={styles.checkboxLabel}>
						<input type="checkbox" bind:checked={newCapabilities.audio} class={styles.checkbox} />
						<span>Audio</span>
					</label>
					<label class={styles.checkboxLabel}>
						<input type="checkbox" bind:checked={newCapabilities.tips} class={styles.checkbox} />
						<span>Tips</span>
					</label>
					<label class={styles.checkboxLabel}>
						<input type="checkbox" bind:checked={newCapabilities.review} class={styles.checkbox} />
						<span>Review</span>
					</label>
				</fieldset>
			</div>

			<footer class={styles.dialogFooter}>
				{#if errorMessage}
					<span class={styles.error}>{errorMessage}</span>
				{/if}
				<button
					class={styles.cancelButton}
					onclick={() => (showAddDialog = false)}
					disabled={isSaving}
					type="button"
				>
					Cancel
				</button>
				<button
					class={styles.saveButton}
					onclick={addModel}
					disabled={isSaving || !newModelName.trim()}
					type="button"
				>
					{isSaving ? 'Adding...' : 'Add Model'}
				</button>
			</footer>
		</div>
	</div>
{/if}
