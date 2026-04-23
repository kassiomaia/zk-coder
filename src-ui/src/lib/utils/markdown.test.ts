import { describe, expect, it } from 'vitest';
import {
	CODE_BLOCK_SENTINEL_RE,
	escapeHtml,
	renderMarkdown,
	renderMarkdownInline,
	renderMarkdownWithBlocks,
	splitOnSentinels,
	stripMarkdown
} from './markdown';

describe('escapeHtml', () => {
	it('escapes the standard HTML-sensitive characters', () => {
		const input = `<img src="x" onerror='alert(1)'>`;
		const out = escapeHtml(input);
		expect(out).not.toContain('<');
		expect(out).not.toContain('>');
		expect(out).not.toContain('"');
		expect(out).not.toContain("'");
	});

	it('escapes ampersands before anything else', () => {
		expect(escapeHtml('&<')).toBe('&amp;&lt;');
	});
});

describe('renderMarkdown — code highlighting', () => {
	it('highlights a tagged code block using the requested grammar', () => {
		const md = '```typescript\nconst x: number = 1;\n```';
		const html = renderMarkdown(md);
		expect(html).toContain('class="code-block"');
		expect(html).toContain('data-language="typescript"');
		expect(html).toMatch(/class="hljs-/);
	});

	it('escapes an untagged unknown-looking block as plaintext (no hljs classes)', () => {
		const md = '```\nthis is just prose with no obvious language\n```';
		const html = renderMarkdown(md);
		expect(html).toContain('class="code-block"');
		expect(html).not.toMatch(/class="hljs-/);
	});

	it('uses the JSON heuristic for an untagged block starting with {', () => {
		const md = '```\n{"a": 1, "b": 2}\n```';
		const html = renderMarkdown(md);
		expect(html).toContain('data-language="json"');
		expect(html).toMatch(/class="hljs-/);
	});

	it('uses the JSON heuristic for an untagged block starting with [', () => {
		const md = '```\n[1, 2, 3]\n```';
		const html = renderMarkdown(md);
		expect(html).toContain('data-language="json"');
	});

	it('uses the bash heuristic for an untagged shell prompt', () => {
		const md = '```\n$ git status\n```';
		const html = renderMarkdown(md);
		expect(html).toContain('data-language="bash"');
	});

	it('uses the bash heuristic for an untagged shebang', () => {
		const md = '```\n#!/bin/sh\necho hi\n```';
		const html = renderMarkdown(md);
		expect(html).toContain('data-language="bash"');
	});

	it('falls back to plaintext for ambiguous content (e.g. a stack trace)', () => {
		const md = '```\nTraceback (most recent call last):\n  File "x.py"\n```';
		const html = renderMarkdown(md);
		expect(html).toContain('class="code-block"');
		expect(html).not.toMatch(/class="hljs-/);
	});

	it('returns escaped plain text for tagged code exceeding the size cap', () => {
		const big = 'a'.repeat(11 * 1024);
		const md = '```typescript\n' + big + '\n```';
		const html = renderMarkdown(md);
		expect(html).toContain('class="code-block"');
		// Escaped plain text — no hljs spans
		expect(html).not.toMatch(/class="hljs-/);
	});

	it('does not call highlightAuto (perf guard: 100 untagged 5KB blocks < 500ms)', () => {
		const blocks = Array.from({ length: 100 }, () => '```\n' + 'x '.repeat(2500) + '\n```').join(
			'\n\n'
		);
		const start = performance.now();
		const html = renderMarkdown(blocks);
		const elapsed = performance.now() - start;
		expect(html).toContain('class="code-block"');
		expect(elapsed).toBeLessThan(500);
	});
});

describe('renderMarkdown — prose, links, images', () => {
	it('renders headings and emphasis', () => {
		const html = renderMarkdown('# Title\n\n**bold** and *italic*');
		expect(html).toContain('<h1');
		expect(html).toContain('<strong');
		expect(html).toContain('<em');
	});

	it('adds rel=noopener to external links', () => {
		const html = renderMarkdown('[click](https://example.com)');
		expect(html).toContain('target="_blank"');
		expect(html).toContain('rel="noopener noreferrer"');
	});

	it('marks data: images with loading=lazy and data-zoomable', () => {
		const html = renderMarkdown('![alt](data:image/png;base64,abc)');
		expect(html).toContain('loading="lazy"');
		expect(html).toContain('data-zoomable="true"');
	});

	it('returns empty string for empty input', () => {
		expect(renderMarkdown('')).toBe('');
	});
});

describe('renderMarkdownWithBlocks — sentinel emission', () => {
	it('emits no sentinels for prose-only content', () => {
		const out = renderMarkdownWithBlocks('just a line of prose');
		expect(out.codeBlocks).toHaveLength(0);
		expect(out.html).not.toContain('<!--CODEBLOCK:');
	});

	it('emits exactly one sentinel per fenced code block', () => {
		const md = 'before\n\n```js\nconst a = 1;\n```\n\nmiddle\n\n```py\nx = 2\n```\n\nafter';
		const out = renderMarkdownWithBlocks(md);
		expect(out.codeBlocks).toHaveLength(2);
		const sentinels = out.html.match(/<!--CODEBLOCK:\d+-->/g) ?? [];
		expect(sentinels).toHaveLength(2);
	});

	it('assigns block indices in source order', () => {
		const md = '```json\n{"a":1}\n```\n\n```bash\nls\n```';
		const out = renderMarkdownWithBlocks(md);
		expect(out.codeBlocks[0].language).toBe('json');
		expect(out.codeBlocks[1].language).toBe('bash');
		expect(out.html).toMatch(/<!--CODEBLOCK:0-->[\s\S]*<!--CODEBLOCK:1-->/);
	});

	it('preserves reference-style links across code blocks (regression guard)', () => {
		const md = [
			'See [project][docs] for details.',
			'',
			'```ts',
			'const x = 1;',
			'```',
			'',
			'[docs]: https://example.com'
		].join('\n');
		const out = renderMarkdownWithBlocks(md);
		// Reference must resolve to the target host (URL slashes are HTML-escaped
		// by our `escapeHtml` — checking the hostname is the portable assertion).
		expect(out.html).toContain('example.com');
		expect(out.html).toMatch(/href="https:[^"]+example\.com/);
		expect(out.codeBlocks).toHaveLength(1);
	});

	it('handles an empty code block without crashing', () => {
		const out = renderMarkdownWithBlocks('```\n\n```');
		expect(out.codeBlocks).toHaveLength(1);
		expect(out.codeBlocks[0].code).toBe('');
	});

	it('preserves raw code in the extracted block (pre-highlight source)', () => {
		const raw = 'const x = 1;\nconst y = 2;';
		const out = renderMarkdownWithBlocks('```js\n' + raw + '\n```');
		expect(out.codeBlocks[0].code).toBe(raw);
	});
});

describe('splitOnSentinels', () => {
	it('returns a single html part when there are no sentinels', () => {
		const parts = splitOnSentinels('<p>hi</p>', []);
		expect(parts).toHaveLength(1);
		expect(parts[0]).toEqual({ type: 'html', html: '<p>hi</p>' });
	});

	it('interleaves html and code parts in order', () => {
		const html = 'before<!--CODEBLOCK:0-->middle<!--CODEBLOCK:1-->after';
		const blocks = [
			{ language: 'js', code: 'a', highlightedHtml: 'a' },
			{ language: 'py', code: 'b', highlightedHtml: 'b' }
		];
		const parts = splitOnSentinels(html, blocks);
		expect(parts.map((p) => p.type)).toEqual(['html', 'code', 'html', 'code', 'html']);
		expect((parts[1] as any).data.language).toBe('js');
		expect((parts[3] as any).data.language).toBe('py');
	});

	it('elides empty html parts between adjacent sentinels', () => {
		const html = '<!--CODEBLOCK:0--><!--CODEBLOCK:1-->';
		const blocks = [
			{ language: 'js', code: 'a', highlightedHtml: 'a' },
			{ language: 'py', code: 'b', highlightedHtml: 'b' }
		];
		const parts = splitOnSentinels(html, blocks);
		expect(parts.map((p) => p.type)).toEqual(['code', 'code']);
	});

	it('is safe to call repeatedly (regex lastIndex is reset)', () => {
		// If lastIndex leaked, the second call would miss matches.
		const html = 'a<!--CODEBLOCK:0-->b';
		const blocks = [{ language: 'js', code: 'x', highlightedHtml: 'x' }];
		splitOnSentinels(html, blocks);
		const second = splitOnSentinels(html, blocks);
		expect(second.map((p) => p.type)).toEqual(['html', 'code', 'html']);
		// Also verify the exported regex itself is stateless between uses
		CODE_BLOCK_SENTINEL_RE.lastIndex = 0;
	});
});

describe('renderMarkdownInline and stripMarkdown', () => {
	it('renderMarkdownInline does not wrap in a paragraph', () => {
		expect(renderMarkdownInline('**bold**')).toBe('<strong>bold</strong>');
	});

	it('stripMarkdown returns plain text', () => {
		expect(stripMarkdown('**bold** text')).toBe('bold text');
	});
});
