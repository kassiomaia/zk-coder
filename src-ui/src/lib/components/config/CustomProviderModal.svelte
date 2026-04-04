<script lang="ts">
	/**
	 * CustomProviderModal Component
	 * Modal for adding a custom LLM provider.
	 */
	import styles from './CustomProviderModal.module.css';
	import Modal from '../shared/Modal.svelte';
	import Button from '../shared/Button.svelte';
	import Input from '../shared/Input.svelte';
	import Spinner from '../shared/Spinner.svelte';
	import { configStore } from '$lib/stores/configStore';
	import type { ApiType } from '$lib/types/config';
	import { get } from 'svelte/store';

	interface Props {
		open: boolean;
		onclose: () => void;
		onsaved?: (providerId: string) => void;
	}

	let { open, onclose, onsaved }: Props = $props();

	// Form state
	let providerId = $state('');
	let providerName = $state('');
	let baseUrl = $state('');
	let apiType: ApiType = $state('openai');
	let isIdManuallyEdited = $state(false);

	// Validation state
	let error = $state('');
	let isSubmitting = $state(false);

	/**
	 * Generate a unique ID from a provider name.
	 * Uses crypto.randomUUID() to ensure uniqueness.
	 * @param name - The provider name to generate an ID from
	 * @returns A unique, sanitized ID string
	 */
	function generateId(name: string): string {
		// Enhanced sanitization: allow hyphens and underscores, collapse multiple hyphens
		const sanitized = name
			.toLowerCase()
			.replace(/[^a-z0-9-_]/g, '-') // Replace non-alphanumeric (except -_) with hyphen
			.replace(/-+/g, '-') // Collapse multiple hyphens into one
			.replace(/^-|-$/g, ''); // Remove leading/trailing hyphens

		// Use crypto.randomUUID() for guaranteed uniqueness
		const randomSuffix = crypto.randomUUID().substring(0, 8);

		return `${sanitized}-${randomSuffix}`;
	}

	function handleNameChange(value: string) {
		providerName = value;
		// Auto-generate ID from name only if user hasn't manually edited it
		if (!isIdManuallyEdited) {
			providerId = generateId(value);
		}
	}

	function handleIdChange(value: string) {
		providerId = value;
		// Mark ID as manually edited when user changes it
		isIdManuallyEdited = true;
	}

	function resetForm() {
		providerId = '';
		providerName = '';
		baseUrl = '';
		apiType = 'openai';
		error = '';
		isSubmitting = false;
		isIdManuallyEdited = false;
	}

	function handleClose() {
		resetForm();
		onclose();
	}

	async function handleSubmit() {
		// Validation
		if (!providerName.trim()) {
			error = 'Provider name is required';
			return;
		}

		if (!providerId.trim()) {
			error = 'Provider ID is required';
			return;
		}

		if (!baseUrl.trim()) {
			error = 'Base URL is required';
			return;
		}

		// Validate URL format
		try {
			new URL(baseUrl);
		} catch {
			error = 'Invalid URL format';
			return;
		}

		// Check for duplicate ID
		const state = get(configStore);
		const existing = state.providers.find((p) => p.id === providerId);
		if (existing) {
			error = 'A provider with this ID already exists';
			return;
		}

		isSubmitting = true;
		error = '';

		try {
			await configStore.saveProvider({
				id: providerId,
				name: providerName,
				icon: '🔌',
				isCustom: true,
				apiType,
				baseUrl,
				isConfigured: false
			});

			// Call onsaved with the provider ID before closing
			onsaved?.(providerId);
			handleClose();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to add provider';
		} finally {
			isSubmitting = false;
		}
	}

	$effect(() => {
		if (open) {
			resetForm();
		}
	});
</script>

<Modal title="Add Custom Provider" {open} onclose={handleClose} stacked={true}>
	<div class={styles.form}>
		<Input
			name="providerName"
			type="text"
			value={providerName}
			onchange={handleNameChange}
			label="Provider Name"
			placeholder="My Custom Provider"
			helper="Display name for this provider"
		/>

		<Input
			name="providerId"
			type="text"
			value={providerId}
			onchange={handleIdChange}
			label="Provider ID"
			placeholder="my-custom-provider"
			helper="Unique identifier (auto-generated from name)"
		/>

		<div class={styles.field}>
			<span class={styles.label}>API Type</span>
			<div class={styles.radioGroup}>
				<label class={styles.radio}>
					<input
						type="radio"
						name="apiType"
						value="openai"
						checked={apiType === 'openai'}
						onchange={() => (apiType = 'openai')}
					/>
					<span class={styles.radioLabel}>
						<span class={styles.radioTitle}>OpenAI Compatible</span>
						<span class={styles.radioDesc}>Standard OpenAI API format</span>
					</span>
				</label>
				<label class={styles.radio}>
					<input
						type="radio"
						name="apiType"
						value="anthropic"
						checked={apiType === 'anthropic'}
						onchange={() => (apiType = 'anthropic')}
					/>
					<span class={styles.radioLabel}>
						<span class={styles.radioTitle}>Anthropic Compatible</span>
						<span class={styles.radioDesc}>Claude API format</span>
					</span>
				</label>
				<label class={styles.radio}>
					<input
						type="radio"
						name="apiType"
						value="google"
						checked={apiType === 'google'}
						onchange={() => (apiType = 'google')}
					/>
					<span class={styles.radioLabel}>
						<span class={styles.radioTitle}>Google (Gemini) Compatible</span>
						<span class={styles.radioDesc}>Google's OpenAI-compatible API</span>
					</span>
				</label>
			</div>
		</div>

		<Input
			name="baseUrl"
			type="text"
			value={baseUrl}
			onchange={(v) => (baseUrl = v)}
			label="Base URL"
			placeholder="https://api.example.com/v1"
			helper="API endpoint URL (e.g., https://api.example.com/v1)"
		/>
	</div>

	{#if error}
		<div class={styles.error}>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<circle cx="12" cy="12" r="10" />
				<line x1="12" y1="8" x2="12" y2="12" />
				<line x1="12" y1="16" x2="12.01" y2="16" />
			</svg>
			<span>{error}</span>
		</div>
	{/if}

	<div class={styles.info}>
		<svg
			xmlns="http://www.w3.org/2000/svg"
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
		>
			<circle cx="12" cy="12" r="10" />
			<line x1="12" y1="16" x2="12" y2="12" />
			<line x1="12" y1="8" x2="12.01" y2="8" />
		</svg>
		<span>After adding, you'll need to configure the API key for this provider.</span>
	</div>

	{#snippet footer()}
		<div class={styles.footer}>
			<Button variant="secondary" onclick={handleClose}>Cancel</Button>
			<Button
				variant="primary"
				onclick={handleSubmit}
				disabled={isSubmitting || !providerName.trim() || !baseUrl.trim()}
			>
				{#if isSubmitting}
					<Spinner size={16} />
				{/if}
				Add Provider
			</Button>
		</div>
	{/snippet}
</Modal>
