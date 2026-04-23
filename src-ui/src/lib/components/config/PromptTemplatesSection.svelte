<script lang="ts">
	/**
	 * PromptTemplatesSection
	 * Settings-panel section that lists every slash-command prompt template
	 * and lets the user create new ones, edit existing ones, reset builtins
	 * to their seeded defaults, or delete custom templates.
	 */
	import Button from '../shared/Button.svelte';
	import ConfirmDialog from '../shared/ConfirmDialog.svelte';
	import PromptTemplateEditorModal from './PromptTemplateEditorModal.svelte';
	import styles from './PromptTemplatesSection.module.css';
	import settingsStyles from './SettingsModal.module.css';
	import {
		promptTemplatesStore,
		type PromptTemplate
	} from '$lib/stores/promptTemplatesStore';
	import { CodeIcon, DeleteIcon, EditIcon, PlusIcon, RefreshIcon } from '$lib/icons';
	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('PromptTemplatesSection');

	let editorOpen = $state(false);
	let editing = $state<PromptTemplate | null>(null);
	let pendingDelete = $state<PromptTemplate | null>(null);

	let sortedTemplates = $derived(
		[...$promptTemplatesStore.templates].sort((a, b) => {
			if (a.sortOrder !== b.sortOrder) return a.sortOrder - b.sortOrder;
			return a.slug.localeCompare(b.slug);
		})
	);

	function openNewTemplate() {
		editing = null;
		editorOpen = true;
	}

	function openEdit(template: PromptTemplate) {
		editing = template;
		editorOpen = true;
	}

	function closeEditor() {
		editorOpen = false;
		editing = null;
	}

	async function confirmDelete() {
		if (!pendingDelete) return;
		try {
			await promptTemplatesStore.remove(pendingDelete.id);
		} catch (err) {
			log.error('Failed to delete template', err);
		} finally {
			pendingDelete = null;
		}
	}

	function cancelDelete() {
		pendingDelete = null;
	}

	async function handleReset(template: PromptTemplate) {
		try {
			await promptTemplatesStore.reset(template.slug);
		} catch (err) {
			log.error('Failed to reset template', err);
		}
	}

	async function toggleEnabled(template: PromptTemplate) {
		try {
			await promptTemplatesStore.save({
				id: template.id,
				slug: template.slug,
				title: template.title,
				description: template.description,
				icon: template.icon,
				category: template.category,
				content: template.content,
				isEnabled: !template.isEnabled
			});
		} catch (err) {
			log.error('Failed to toggle template', err);
		}
	}
</script>

<section class="modal-section {settingsStyles.section}">
	<header class={settingsStyles.sectionHeader}>
		<div class={settingsStyles.sectionTitleGroup}>
			<div class={settingsStyles.sectionIcon}>
				<CodeIcon size={16} />
			</div>
			<h3 class={settingsStyles.sectionTitle}>Prompt Templates</h3>
		</div>
		<Button variant="ghost" size="sm" onclick={openNewTemplate}>
			<PlusIcon size={14} />
			<span>New template</span>
		</Button>
	</header>

	<p class={styles.caption}>
		Type <code>/</code> in chat to open the command palette. Each command below expands into
		the prompt text it holds — <code>&#123;args&#125;</code> is replaced by whatever you type
		after the slash.
	</p>

	{#if $promptTemplatesStore.loading === 'loading' && sortedTemplates.length === 0}
		<div class={styles.empty}>Loading…</div>
	{:else if sortedTemplates.length === 0}
		<div class={styles.empty}>
			No templates yet. Create one to start building your command palette.
		</div>
	{:else}
		<ul class={styles.list}>
			{#each sortedTemplates as template (template.id)}
				<li class={[styles.item, !template.isEnabled && styles.isDisabled].filter(Boolean).join(' ')}>
					<div class={styles.info}>
						<div class={styles.titleRow}>
							<span class={styles.slug}>/{template.slug}</span>
							<span class={styles.title}>{template.title}</span>
							{#if template.isBuiltin}
								<span class={styles.badge}>Builtin</span>
							{/if}
							{#if !template.isEnabled}
								<span class={[styles.badge, styles.badgeMuted].join(' ')}>Disabled</span>
							{/if}
						</div>
						{#if template.description}
							<div class={styles.description}>{template.description}</div>
						{/if}
					</div>
					<div class={styles.actions}>
						<Button
							variant="ghost"
							size="sm"
							onclick={() => toggleEnabled(template)}
						>
							{template.isEnabled ? 'Disable' : 'Enable'}
						</Button>
						<Button
							variant="ghost"
							size="sm"
							onclick={() => openEdit(template)}
							icon={editIcon}
						>
							Edit
						</Button>
						{#if template.isBuiltin}
							<Button
								variant="ghost"
								size="sm"
								onclick={() => handleReset(template)}
								disabled={$promptTemplatesStore.loading === 'loading'}
								icon={resetIcon}
							>
								Reset
							</Button>
						{:else}
							<Button
								variant="ghost"
								size="sm"
								onclick={() => (pendingDelete = template)}
								icon={deleteIcon}
							>
								Delete
							</Button>
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	{/if}

	{#if $promptTemplatesStore.error}
		<div class={styles.error} role="alert">{$promptTemplatesStore.error}</div>
	{/if}
</section>

{#snippet editIcon()}
	<EditIcon size={14} />
{/snippet}

{#snippet resetIcon()}
	<RefreshIcon size={14} />
{/snippet}

{#snippet deleteIcon()}
	<DeleteIcon size={14} />
{/snippet}

<PromptTemplateEditorModal open={editorOpen} template={editing} onclose={closeEditor} />

<ConfirmDialog
	open={pendingDelete !== null}
	title="Delete template"
	message={pendingDelete
		? `Delete /${pendingDelete.slug}? This can't be undone.`
		: ''}
	confirmLabel="Delete"
	variant="danger"
	onconfirm={confirmDelete}
	oncancel={cancelDelete}
/>
