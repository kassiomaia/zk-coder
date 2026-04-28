<script lang="ts">
	/**
	 * AppearanceSection
	 *
	 * Theme picker for the Settings modal. Three options — Light / Dark / System
	 * — backed by the shared themeStore which persists the choice via Tauri to
	 * coder_config.db and reflects it onto `data-theme` on <html>.
	 *
	 * The previews are composed from the actual design tokens of each theme so
	 * they stay in sync with the rest of the app without a separate snapshot.
	 */

	import styles from './AppearanceSection.module.css';
	import { themeStore, themePreference, type ThemePreference } from '$lib/stores/themeStore';
	import { CheckIcon, MoonIcon, MonitorIcon, SunIcon } from '$lib/icons';
	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('AppearanceSection');

	type ThemeOption = {
		value: ThemePreference;
		label: string;
		description: string;
		previewClass: string;
		Icon: typeof SunIcon;
	};

	const options: ThemeOption[] = [
		{
			value: 'light',
			label: 'Light',
			description: 'Pale, warm surfaces tuned for daylight work.',
			previewClass: styles.previewLight,
			Icon: SunIcon
		},
		{
			value: 'dark',
			label: 'Dark',
			description: 'Warm, low-contrast neutrals for low-light sessions.',
			previewClass: styles.previewDark,
			Icon: MoonIcon
		},
		{
			value: 'system',
			label: 'System',
			description: "Match the operating system's color scheme.",
			previewClass: styles.previewSystem,
			Icon: MonitorIcon
		}
	];

	let selected = $state<ThemePreference>('system');

	$effect(() => {
		const unsubscribe = themePreference.subscribe((value) => {
			selected = value;
		});
		return unsubscribe;
	});

	async function pick(next: ThemePreference): Promise<void> {
		if (next === selected) return;
		try {
			await themeStore.setPreference(next);
		} catch (err) {
			log.error('Failed to update theme preference', err);
		}
	}

	function handleKeydown(event: KeyboardEvent, value: ThemePreference): void {
		const order: ThemePreference[] = ['light', 'dark', 'system'];
		const currentIndex = order.indexOf(value);
		if (currentIndex === -1) return;

		let nextIndex = currentIndex;
		if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
			nextIndex = (currentIndex + 1) % order.length;
		} else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
			nextIndex = (currentIndex - 1 + order.length) % order.length;
		} else {
			return;
		}
		event.preventDefault();
		const target = document.querySelector<HTMLButtonElement>(
			`[data-theme-option="${order[nextIndex]}"]`
		);
		target?.focus();
		void pick(order[nextIndex]);
	}
</script>

<div
	class={styles.themeGrid}
	role="radiogroup"
	aria-label="Theme"
>
	{#each options as option (option.value)}
		{@const isChecked = selected === option.value}
		<button
			type="button"
			role="radio"
			aria-checked={isChecked}
			class={styles.themeOption}
			data-theme-option={option.value}
			onclick={() => pick(option.value)}
			onkeydown={(event) => handleKeydown(event, option.value)}
		>
			<div class={`${styles.preview} ${option.previewClass}`} aria-hidden="true">
				<div class={styles.previewHeader}>
					<span class={styles.previewDot}></span>
					<span class={styles.previewDot}></span>
					<span class={styles.previewDot}></span>
				</div>
				<div class={styles.previewSidebar}>
					<div class={`${styles.previewSidebarRow} ${styles.previewSidebarRowActive}`}></div>
					<div class={styles.previewSidebarRow}></div>
					<div class={styles.previewSidebarRow}></div>
					<div class={styles.previewSidebarRow}></div>
				</div>
				<div class={styles.previewContent}>
					<div class={`${styles.previewContentRow} ${styles.previewContentRowMedium}`}></div>
					<div class={styles.previewContentRow}></div>
					<div class={`${styles.previewContentRow} ${styles.previewContentRowShort}`}></div>
				</div>
			</div>

			<div class={styles.meta}>
				<span class={styles.label}>
					<span class={styles.labelText}>
						<span class={styles.labelIcon} aria-hidden="true">
							<option.Icon size={14} />
						</span>
						{option.label}
					</span>
					<span class={styles.checkMark} aria-hidden="true">
						<CheckIcon size={10} strokeWidth={3} />
					</span>
				</span>
				<span class={styles.description}>{option.description}</span>
			</div>
		</button>
	{/each}
</div>
