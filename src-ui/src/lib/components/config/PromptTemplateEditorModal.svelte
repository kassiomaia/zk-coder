<script lang="ts">
	/**
	 * PromptTemplateEditorModal
	 * Create or edit a slash-command prompt template. On save, delegates to the
	 * `promptTemplatesStore`; the caller only decides which template to edit
	 * and when to close the modal.
	 */
	import { derived } from 'svelte/store';
	import Modal from '../shared/Modal.svelte';
	import Button from '../shared/Button.svelte';
	import Input from '../shared/Input.svelte';
	import styles from './PromptTemplateEditorModal.module.css';
	import {
		promptTemplatesStore,
		type PromptTemplate
	} from '$lib/stores/promptTemplatesStore';
	import type { SavePromptTemplateRequest } from '$lib/api/config';
	import {
		BoxIcon,
		ChevronDownIcon,
		CodeIcon,
		FileIcon,
		UserIcon
	} from '$lib/icons';
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
	let category = $state('custom');
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

	const SLUG_RE = /^[a-z0-9][a-z0-9-]*$/;

	const categorySuggestions = derived(promptTemplatesStore, ($s) => {
		const seen = new Set<string>();
		for (const t of $s.templates) {
			const c = t.category?.trim();
			if (c) seen.add(c);
		}
		return Array.from(seen).sort();
	});

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
			content = '';
			isEnabled = true;
		}
		errorMessage = null;
	});

	let isBuiltin = $derived(template?.isBuiltin ?? false);
	let canSave = $derived(
		!saving &&
			title.trim().length > 0 &&
			content.trim().length > 0 &&
			category.trim().length > 0 &&
			(isBuiltin || SLUG_RE.test(slug.trim()))
	);
	let modalTitle = $derived(template ? `Edit /${template.slug}` : 'New prompt template');

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
	title={modalTitle}
	{open}
	size="lg"
	stacked
	onclose={handleClose}
>
	{#snippet children()}
		<div class={styles.form}>
			<!-- IDENTITY -->
			<section class="modal-section {styles.section}">
				<header class={styles.sectionHeader}>
					<div class={styles.sectionTitleGroup}>
						<span class={styles.sectionIcon} aria-hidden="true">
							<UserIcon size={16} />
						</span>
						<h3 class={styles.sectionTitle}>Identity</h3>
					</div>
					<label class={styles.enabledToggle}>
						<input
							type="checkbox"
							checked={isEnabled}
							onchange={(e) => (isEnabled = (e.currentTarget as HTMLInputElement).checked)}
						/>
						<span>Enabled</span>
					</label>
				</header>

				{#if !isBuiltin}
					<div class={styles.field}>
						<Input
							name="template-slug"
							label="Slug"
							value={slug}
							onchange={(v) => (slug = v)}
							placeholder="commit-message"
							helper="Lowercase letters, digits, and dashes. Must start with a letter or digit. Appears in the palette as /slug."
							required
						/>
					</div>
				{/if}

				<div class={styles.row}>
					<div class={styles.field}>
						<Input
							name="template-title"
							label="Title"
							value={title}
							onchange={(v) => (title = v)}
							placeholder="Commit message"
							required
						/>
					</div>
					<div class={styles.field}>
						<Input
							name="template-description"
							label="Description"
							value={description}
							onchange={(v) => (description = v)}
							placeholder="Short subtitle shown in the palette"
						/>
					</div>
				</div>
			</section>

			<!-- APPEARANCE -->
			<section class="modal-section {styles.section}">
				<header class={styles.sectionHeader}>
					<div class={styles.sectionTitleGroup}>
						<span class={styles.sectionIcon} aria-hidden="true">
							<BoxIcon size={16} />
						</span>
						<h3 class={styles.sectionTitle}>Appearance</h3>
					</div>
				</header>

				<div class={styles.row}>
					<div class={styles.field}>
						<label class={styles.nativeLabel} for="template-category">Category</label>
						<input
							id="template-category"
							class={styles.nativeInput}
							list="template-category-options"
							value={category}
							oninput={(e) => (category = (e.currentTarget as HTMLInputElement).value)}
							placeholder="custom"
						/>
						<datalist id="template-category-options">
							{#each $categorySuggestions as option (option)}
								<option value={option}></option>
							{/each}
						</datalist>
						<p class={styles.helperText}>
							Free text. Templates with the same category are grouped in the palette.
						</p>
					</div>

					<div class={styles.field}>
						<label class={styles.nativeLabel} for="template-icon">Icon</label>
						<div class={styles.iconRow}>
							<div class={styles.iconGlyph} aria-hidden="true">
								{#if icon === 'code'}
									<CodeIcon size={16} />
								{:else if icon === 'book' || icon === 'document'}
									<FileIcon size={16} />
								{:else if icon === 'settings'}
									<BoxIcon size={16} />
								{:else}
									<span class={styles.iconGlyphEmpty}>—</span>
								{/if}
							</div>
							<div class={styles.selectWrapper}>
								<select
									id="template-icon"
									class={styles.nativeSelect}
									value={icon}
									onchange={(e) => (icon = (e.currentTarget as HTMLSelectElement).value)}
								>
									{#each iconOptions as option (option.value)}
										<option value={option.value} selected={option.value === icon}>
											{option.label}
										</option>
									{/each}
								</select>
								<span class={styles.selectArrow} aria-hidden="true">
									<ChevronDownIcon size={16} />
								</span>
							</div>
						</div>
					</div>
				</div>
			</section>

			<!-- TEMPLATE -->
			<section class="modal-section {styles.section}">
				<header class={styles.sectionHeader}>
					<div class={styles.sectionTitleGroup}>
						<span class={styles.sectionIcon} aria-hidden="true">
							<FileIcon size={16} />
						</span>
						<h3 class={styles.sectionTitle}>Template</h3>
					</div>
				</header>

				<div class={styles.field}>
					<label class={styles.nativeLabel} for="template-content">Body</label>
					<textarea
						id="template-content"
						class={styles.textarea}
						rows="10"
						placeholder="Write instructions for the model. Use &#123;args&#125; where the user's input should be inserted."
						value={content}
						oninput={(e) => (content = (e.currentTarget as HTMLTextAreaElement).value)}
					></textarea>
					<p class={styles.helperText}>
						Use <code>&#123;args&#125;</code> where the user's input should be inserted.
						Multi-line arguments are preserved verbatim.
					</p>
				</div>

			</section>

			{#if errorMessage}
				<div class={styles.error} role="alert">{errorMessage}</div>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={handleClose} disabled={saving}>Cancel</Button>
		<Button variant="primary" onclick={handleSave} disabled={!canSave}>
			{saving ? 'Saving…' : template ? 'Save changes' : 'Save'}
		</Button>
	{/snippet}
</Modal>
