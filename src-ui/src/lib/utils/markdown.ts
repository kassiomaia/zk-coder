/**
 * Markdown rendering utility using `marked` + the shared `hljs` instance.
 *
 * Supports: syntax highlighting, tables, task lists, base64 images, and
 * sentinel-based code-block extraction so consumers can render code blocks
 * with their own component without parsing the rendered HTML back through
 * the DOM.
 *
 * ## Performance contract
 *
 * - Language tag present and registered → `hljs.highlight(code, {language})`.
 *   Size-capped at `MAX_HIGHLIGHT_BYTES` to keep a single large buffer from
 *   blocking the main thread.
 * - Language tag absent → a cheap first-line heuristic picks `json` or `bash`
 *   when obvious, otherwise the code is escaped and rendered plain. We never
 *   call `hljs.highlightAuto`, which is catastrophically slow because it
 *   probes every registered grammar.
 */
import { marked } from 'marked';
import hljs from './hljs';
import { createLogger } from './logger';

const log = createLogger('Markdown');

// Types for marked tokens - based on marked library structure
interface MarkedToken {
	text?: string;
	href?: string;
	title?: string | null;
	lang?: string;
}

/**
 * Upper bound on code size that will be passed to `hljs.highlight`. Above this
 * we return escaped plain text. Highlight time scales non-linearly with input
 * size; past a few tens of kilobytes the cost outweighs any readability gain.
 */
const MAX_HIGHLIGHT_BYTES = 10 * 1024;

/**
 * A code block extracted from rendered markdown. Parallel to the sentinels in
 * `RenderedMarkdown.html`: sentinel `<!--CODEBLOCK:N-->` corresponds to index N.
 */
export interface CodeBlockData {
	language: string;
	code: string;
	highlightedHtml: string;
}

export interface RenderedMarkdown {
	html: string;
	codeBlocks: CodeBlockData[];
}

/**
 * Escape HTML special characters comprehensively.
 * Protects against XSS by escaping all potentially dangerous characters.
 */
export function escapeHtml(str: string): string {
	return str
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#039;')
		.replace(/`/g, '&#x60;')
		.replace(/\//g, '&#x2F;');
}

/**
 * Cheap first-line heuristic for untagged code blocks. Returns a language name
 * hljs has registered, or `null` when nothing obvious matches.
 *
 * Intentionally conservative: we would rather render unhighlighted than
 * mis-tag the grammar and produce spurious colors.
 */
function detectLanguageFromHeuristic(code: string): string | null {
	const trimmed = code.trim();
	if (trimmed.length === 0) return null;

	// First non-empty line, up to the first newline
	const firstLineEnd = trimmed.indexOf('\n');
	const firstLine = firstLineEnd === -1 ? trimmed : trimmed.slice(0, firstLineEnd);

	// JSON/JSON5 objects and arrays
	if (firstLine.startsWith('{') || firstLine.startsWith('[')) {
		return 'json';
	}

	// Shell: prompts, shebangs, and common commands at line start
	if (firstLine.startsWith('$ ') || firstLine.startsWith('#!/')) {
		return 'bash';
	}
	const shellPrefixes = [
		'cd ',
		'ls ',
		'pwd',
		'git ',
		'cargo ',
		'npm ',
		'pnpm ',
		'yarn ',
		'make ',
		'mkdir ',
		'rm ',
		'cp ',
		'mv ',
		'echo ',
		'cat ',
		'sudo '
	];
	for (const prefix of shellPrefixes) {
		if (firstLine === prefix.trimEnd() || firstLine.startsWith(prefix)) return 'bash';
	}

	return null;
}

function highlightCode(code: string, lang: string): string {
	if (code.length > MAX_HIGHLIGHT_BYTES) {
		return escapeHtml(code);
	}
	try {
		return hljs.highlight(code, { language: lang }).value;
	} catch (error) {
		log.warn('[Markdown] Code highlighting failed for language:', lang, {
			error: error instanceof Error ? error.message : String(error),
			codePreview: code.slice(0, 100) + (code.length > 100 ? '...' : '')
		});
		return escapeHtml(code);
	}
}

// Per-call state for the code renderer to accumulate extracted blocks.
// Populated by `renderMarkdownWithBlocks` and consumed in the custom `code`
// renderer below via the closure on `currentBlocks`.
let currentBlocks: CodeBlockData[] | null = null;

// Custom renderer for images, links, and code blocks.
const renderer = {
	image(token: MarkedToken): string | false {
		const src = token.href || '';
		const alt = token.text || '';
		const title = token.title;
		const titleAttr = title ? ` title="${escapeHtml(title)}"` : '';
		const lazyAttr = src.startsWith('data:image') ? ' loading="lazy"' : '';
		return `<img src="${escapeHtml(src)}" alt="${escapeHtml(alt)}"${titleAttr}${lazyAttr} class="markdown-image" data-zoomable="true" />`;
	},

	link(token: MarkedToken): string | false {
		const url = token.href || '#';
		const text = token.text || '';
		const title = token.title;
		const titleAttr = title ? ` title="${escapeHtml(title)}"` : '';
		return `<a href="${escapeHtml(url)}"${titleAttr} target="_blank" rel="noopener noreferrer" class="markdown-link">${text}</a>`;
	},

	code(token: MarkedToken): string | false {
		const code = token.text || '';
		const rawLang = (token.lang || '').trim();
		// Some markdown uses ```lang info-string; we only care about the first word.
		const lang = rawLang.split(/\s+/)[0] ?? '';

		let language: string;
		let highlightedHtml: string;

		if (lang && hljs.getLanguage(lang)) {
			language = lang;
			highlightedHtml = highlightCode(code, lang);
		} else {
			const detected = detectLanguageFromHeuristic(code);
			if (detected) {
				language = detected;
				highlightedHtml = highlightCode(code, detected);
			} else {
				language = lang || 'plaintext';
				highlightedHtml = escapeHtml(code);
			}
		}

		// If we're running inside `renderMarkdownWithBlocks`, emit a sentinel
		// and push the block into the side-array. Otherwise fall back to the
		// inline `<pre class="code-block">` form so `renderMarkdown` callers
		// that just want a single HTML string still work.
		if (currentBlocks) {
			const index = currentBlocks.length;
			currentBlocks.push({ language, code, highlightedHtml });
			return `<!--CODEBLOCK:${index}-->`;
		}

		const langAttr = language ? ` data-language="${escapeHtml(language)}"` : '';
		return `<pre class="code-block"${langAttr}><code>${highlightedHtml}</code></pre>`;
	}
};

marked.use({ renderer });
marked.setOptions({
	gfm: true,
	breaks: false
});

function normalizeContent(content: string): string {
	return content
		.replace(/\r\n/g, '\n')
		.replace(/\r/g, '\n')
		.replace(/\n{3,}/g, '\n\n')
		.trim();
}

/**
 * Render markdown, returning both the HTML (with `<!--CODEBLOCK:N-->`
 * sentinels in place of each fenced code block) and the parallel list of
 * extracted code blocks.
 *
 * Callers that want to render code blocks with a dedicated component should
 * use this API and split `html` on the sentinel regex. Keeping the split as
 * a string operation avoids reparsing the rendered HTML through the DOM.
 */
export function renderMarkdownWithBlocks(content: string): RenderedMarkdown {
	if (!content) return { html: '', codeBlocks: [] };

	const blocks: CodeBlockData[] = [];
	const previous = currentBlocks;
	currentBlocks = blocks;
	try {
		const html = marked.parse(normalizeContent(content)) as string;
		return { html, codeBlocks: blocks };
	} finally {
		currentBlocks = previous;
	}
}

/**
 * Regex used to split a sentinel-bearing HTML string into alternating
 * html/code parts. Exported for tests and for `splitOnSentinels` consumers
 * that want the canonical boundary.
 */
export const CODE_BLOCK_SENTINEL_RE = /<!--CODEBLOCK:(\d+)-->/g;

export type RenderedPart =
	| { type: 'html'; html: string }
	| { type: 'code'; data: CodeBlockData };

/**
 * Split sentinel-bearing HTML into parts suitable for direct rendering.
 *
 * Empty html parts between adjacent sentinels are elided, but leading and
 * trailing html parts are always preserved (even when empty) so consumers
 * can bind stable keys to positions.
 */
export function splitOnSentinels(html: string, codeBlocks: CodeBlockData[]): RenderedPart[] {
	if (codeBlocks.length === 0) {
		return [{ type: 'html', html }];
	}

	const parts: RenderedPart[] = [];
	let lastIndex = 0;

	// Reset global regex state before each walk
	CODE_BLOCK_SENTINEL_RE.lastIndex = 0;

	for (const match of html.matchAll(CODE_BLOCK_SENTINEL_RE)) {
		const start = match.index ?? 0;
		const htmlSlice = html.slice(lastIndex, start);
		if (htmlSlice.length > 0) {
			parts.push({ type: 'html', html: htmlSlice });
		}

		const idx = Number(match[1]);
		const data = codeBlocks[idx];
		if (data) {
			parts.push({ type: 'code', data });
		}

		lastIndex = start + match[0].length;
	}

	const tail = html.slice(lastIndex);
	if (tail.length > 0) {
		parts.push({ type: 'html', html: tail });
	}

	return parts;
}

/**
 * Normalize line endings and render markdown content to an HTML string.
 *
 * Code blocks are inlined into the returned HTML (using `<pre class="code-block">`)
 * rather than replaced with sentinels, which preserves the existing contract
 * for callers that don't need separately-rendered code blocks.
 */
export function renderMarkdown(content: string): string {
	if (!content) return '';
	// Calling `marked.parse` without setting `currentBlocks` makes the custom
	// `code` renderer emit inline `<pre>` tags — see the branch there.
	return marked.parse(normalizeContent(content)) as string;
}

/**
 * Render inline markdown content to HTML (no paragraph wrapper).
 */
export function renderMarkdownInline(content: string): string {
	if (!content) return '';
	return marked.parseInline(content) as string;
}

/**
 * Strip markdown formatting and return plain text.
 */
export function stripMarkdown(content: string): string {
	if (!content) return '';
	const html = marked.parse(content) as string;
	return html
		.replace(/<[^>]*>/g, '')
		.replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>')
		.replace(/&amp;/g, '&')
		.replace(/&quot;/g, '"')
		.replace(/&#039;/g, "'")
		.trim();
}

export { marked };
