/**
 * File-system tool renderers: fs_read_file / fs_write_file / fs_glob / fs_grep
 * plus their unprefixed aliases (read/write/glob/grep and *_file variants).
 */
import { FilePenLine, FileSearch, FileText, Search } from 'lucide-svelte';
import type { AnyToolRenderer } from './types';
import FsReadBody from '../FsReadBody.svelte';
import FsWriteBody from '../FsWriteBody.svelte';

interface FsPathArgs {
	path?: string;
	filePath?: string;
}

interface FsPatternArgs {
	pattern?: string;
}

interface FsReadResult {
	content?: string;
}

interface FsGlobResult {
	paths?: string[];
}

const pendingOrErrorOutcome = (status: string): string | null => {
	if (status === 'pending') return 'Executing...';
	if (status === 'error') return 'Action failed';
	return null;
};

export const fsReadRenderer: AnyToolRenderer = {
	icon: FileText,
	getSummary: (args) => {
		const a = args as FsPathArgs | undefined;
		return { title: 'Read file', subtitle: a?.filePath ?? a?.path };
	},
	getOutcome: (result, status) => {
		const short = pendingOrErrorOutcome(status);
		if (short) return short;
		const res = result as FsReadResult | undefined;
		return `Read ${res?.content?.split('\n').length ?? 0} lines`;
	},
	BodyComponent: FsReadBody
};

export const fsWriteRenderer: AnyToolRenderer = {
	icon: FilePenLine,
	getSummary: (args) => {
		const a = args as FsPathArgs | undefined;
		return { title: 'Modified file', subtitle: a?.filePath ?? a?.path };
	},
	getOutcome: (_result, status) => {
		const short = pendingOrErrorOutcome(status);
		if (short) return short;
		return 'Changes applied successfully';
	},
	BodyComponent: FsWriteBody
};

export const fsGlobRenderer: AnyToolRenderer = {
	icon: FileSearch,
	getSummary: (args) => {
		const a = args as FsPatternArgs | undefined;
		return { title: 'Searched files', subtitle: a?.pattern };
	},
	getOutcome: (result, status) => {
		const short = pendingOrErrorOutcome(status);
		if (short) return short;
		const count = Array.isArray(result)
			? result.length
			: ((result as FsGlobResult | undefined)?.paths?.length ?? 0);
		return `Found ${count} matching files`;
	}
};

export const fsGrepRenderer: AnyToolRenderer = {
	icon: Search,
	getSummary: (args) => {
		const a = args as FsPatternArgs | undefined;
		return { title: 'Searched content', subtitle: a?.pattern };
	},
	getOutcome: (result, status) => {
		const short = pendingOrErrorOutcome(status);
		if (short) return short;
		const matchCount = Array.isArray(result)
			? result.length
			: Object.keys((result as object) ?? {}).length;
		return `Found matches in ${matchCount} files`;
	}
};
