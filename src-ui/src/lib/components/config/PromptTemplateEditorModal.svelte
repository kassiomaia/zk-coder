<script lang="ts">
	/**
	 * PromptTemplateEditorModal
	 * Create or edit a slash-command prompt template. On save, delegates to the
	 * `promptTemplatesStore`; the caller only decides which template to edit
	 * and when to close the modal.
	 */
	import Modal from '../shared/Modal.svelte';
	import Button from '../shared/Button.svelte';
	import Input from '../shared/Input.svelte';
	import Select from '../shared/Select.svelte';
	import styles from './PromptTemplateEditorModal.module.css';
	import {
		promptTemplatesStore,
		type PromptTemplate
	} from '$lib/stores/promptTemplatesStore';
	import type { SavePromptTemplateRequest } from '$lib/api/config';
	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('PromptTemplateEditor');

	interface Props {
		open: boolean;
		/** Template to edit. `null` means create-new. */
		template: PromptTemplate | null;
		onclose: () => void;
	}

	let { open, template, onclose }: Props = $props();

	let slug = $state('');
	let title = $state('');
	let description = $state('');
	let category = $state('code');
	let icon = $state('');
	let content = $state('');
	let isEnabled = $state(true);
	let errorMessage = $state<string | null>(null);
	let saving = $state(false);

	const iconOptions = [
		{ value: '', label: 'No icon' },
		{ value: 'code', label: 'Code' },
		{ value: 'book', label: 'Book' },
		{ value: 'refactor', label: 'Refactor' },
		{ value: 'bolt', label: 'Bolt' },
		{ value: 'beaker', label: 'Beaker' },
		{ value: 'document', label: 'Document' },
		{ value: 'settings', label: 'Settings' }
	];

	const categoryOptions = [
		{ value: 'code', label: 'Code' },
		{ value: 'custom', label: 'Custom' }
	];

	const SLUG_RE = /^[a-z0-9][a-z0-9-]*$/;

	$effect(() => {
		if (!open) return;
		if (template) {
			slug = template.slug;
			title = template.title;
			description = template.description;
			category = template.category || 'custom';
			icon = template.icon ?? '';
			content = template.content;
			isEnabled = template.isEnabled;
		} else {
			slug = '';
			title = '';
			description = '';
			category = 'custom';
			icon = '';
			content = '\n\n{args}';
			isEnabled = true;
		}
		errorMessage = null;
	});

	let isBuiltin = $derived(template?.isBuiltin ?? false);
	let preview = $derived(content.replace(/\{args\}/g, 'YOUR INPUT'));
	let canSave = $derived(
		!saving &&
			title.trim().length > 0 &&
			content.trim().length > 0 &&
			category.trim().length > 0 &&
			(isBuiltin || SLUG_RE.test(slug.trim()))
	);

	async function handleSave() {
		if (!canSave) return;
		errorMessage = null;
		saving = true;
		try {
			const request: SavePromptTemplateRequest = {
				id: template?.id,
				slug: slug.trim(),
				title: title.trim(),
				description: description.trim(),
				icon: icon ? icon : null,
				category: category.trim(),
				content,
				isEnabled
			};
			await promptTemplatesStore.save(request);
			onclose();
		} catch (err) {
			log.error('Failed to save prompt template', err);
			errorMessage = err instanceof Error ? err.message : String(err);
		} finally {
			saving = false;
		}
	}

	function handleClose() {
		if (saving) return;
		onclose();
	}
</script>

<Modal
	title={template ? `Edit /${template.slug}` : 'New prompt template'}
	{open}
	size="lg"
	stacked
	onclose={handleClose}
>
	{#snippet children()}
		<div class={styles.form}>
			<div class={styles.row}>
				<Input
					name="template-slug"
					label="Slug"
					value={slug}
					onchange={(v) => (slug = v)}
					placeholder="refactor"
					helper={isBuiltin
						? 'Builtin slug cannot be changed.'
						: 'Lowercase letters, digits, and dashes. Must start with a letter or digit.'}
					disabled={isBuiltin}
					required
				/>
				<Input
					name="template-title"
					label="Title"
					value={title}
					onchange={(v) => (title = v)}
					placeholder="Refactor"
					required
				/>
			</div>

			<Input
				name="template-description"
				label="Description"
				value={description}
				onchange={(v) => (description = v)}
				placeholder="Short subtitle shown in the command palette"
			/>

			<div class={styles.row}>
				<div class={styles.field}>
					<label class={styles.label} for="template-category">Category</label>
					<Select
						value={category}
						options={categoryOptions}
						onchange={(v) => (category = v)}
					/>
				</div>
				<div class={styles.field}>
					<label class={styles.label} for="template-icon">Icon</label>
					<Select value={icon} options={iconOptions} onchange={(v) => (icon = v)} />
				</div>
			</div>

			<div class={styles.field}>
				<label class={styles.label} for="template-content">Template</label>
				<textarea
					id="template-content"
					class={styles.textarea}
					rows="8"
					value={content}
					oninput={(e) => (content = (e.currentTarget as HTMLTextAreaElement).value)}
				></textarea>
				<p class={styles.helper}>
					Use <code>&#123;args&#125;</code> where the user's input should be inserted.
				</p>
			</div>

			<label class={styles.toggle}>
				<input
					type="checkbox"
					checked={isEnabled}
					onchange={(e) => (isEnabled = (e.currentTarget as HTMLInputElement).checked)}
				/>
				<span>Enabled (appears in the palette)</span>
			</label>

			<div class={styles.preview}>
				<div class={styles.previewLabel}>Preview (with YOUR INPUT substituted for &#123;args&#125;):</div>
				<pre class={styles.previewBody}>{preview}</pre>
			</div>

			{#if errorMessage}
				<div class={styles.error} role="alert">{errorMessage}</div>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={handleClose} disabled={saving}>Cancel</Button>
		<Button variant="primary" onclick={handleSave} disabled={!canSave}>
			{saving ? 'Saving…' : 'Save'}
		</Button>
	{/snippet}
</Modal>
