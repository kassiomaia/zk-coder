/**
 * Icons Module
 *
 * Centralized icon system for the application.
 * All icons are based on Feather Icons (24x24 default viewbox).
 *
 * Usage:
 * ```svelte
 * <script>
 *   import { CopyIcon, EditIcon, DeleteIcon } from '$lib/icons';
 * </script>
 *
 * <CopyIcon size={16} class="my-icon" />
 * <EditIcon size={20} strokeWidth={2.5} />
 * ```
 */

// ============================================================================
// Base Icon Component
// ============================================================================

export { default as Icon } from './Icon.svelte';

export type { IconProps } from './icons.d';

// ============================================================================
// Action Icons
// ============================================================================

export { default as CopyIcon } from './CopyIcon.svelte';
export { default as EditIcon } from './EditIcon.svelte';
export { default as DeleteIcon } from './DeleteIcon.svelte';
export { default as CheckIcon } from './CheckIcon.svelte';
export { default as RefreshIcon } from './RefreshIcon.svelte';
export { default as RefreshCcwIcon } from './RefreshCcwIcon.svelte';
export { default as RenameIcon } from './RenameIcon.svelte';
export { default as ArchiveIcon } from './ArchiveIcon.svelte';
export { default as ThumbsUpIcon } from './ThumbsUpIcon.svelte';
export { default as ThumbsDownIcon } from './ThumbsDownIcon.svelte';

// ============================================================================
// Navigation Icons
// ============================================================================

export { default as ChevronDownIcon } from './ChevronDownIcon.svelte';
export { default as ArrowDownIcon } from './ArrowDownIcon.svelte';
export { default as ArrowUpIcon } from './ArrowUpIcon.svelte';

// ============================================================================
// Action Icons (additional)
// ============================================================================

export { default as PlusIcon } from './PlusIcon.svelte';
export { default as CodeIcon } from './CodeIcon.svelte';
export { default as SendIcon } from './SendIcon.svelte';

// ============================================================================
// UI Icons
// ============================================================================

export { default as CloseIcon } from './CloseIcon.svelte';
export { default as SearchIcon } from './SearchIcon.svelte';
export { default as MoreIcon } from './MoreIcon.svelte';
export { default as SettingsIcon } from './SettingsIcon.svelte';

// ============================================================================
// Communication Icons
// ============================================================================

export { default as MessageIcon } from './MessageIcon.svelte';
export { default as MessageSquareIcon } from './MessageSquareIcon.svelte';

// ============================================================================
// Status Icons
// ============================================================================

export { default as SpinnerIcon } from './SpinnerIcon.svelte';
export { default as CheckCircleIcon } from './CheckCircleIcon.svelte';
export { default as AlertCircleIcon } from './AlertCircleIcon.svelte';

// ============================================================================
// File Icons
// ============================================================================

export { default as FileIcon } from './FileIcon.svelte';
export { default as PaperclipIcon } from './PaperclipIcon.svelte';

// ============================================================================
// Misc Icons
// ============================================================================

export { default as UserIcon } from './UserIcon.svelte';
export { default as BoxIcon } from './BoxIcon.svelte';
