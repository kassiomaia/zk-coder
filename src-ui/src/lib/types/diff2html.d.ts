declare module 'diff2html/lib/ui/js/diff2html-ui' {
	export class Diff2HtmlUI {
		constructor(targetElement: HTMLElement, diff: string, configuration: any);
		draw(): void;
		highlightCode(): void;
	}
}

declare module 'diff2html/bundles/css/diff2html.min.css' {
	const content: Record<string, string>;
	export default content;
}

declare module 'highlight.js/lib/core' {
	import type { HLJSApi } from 'highlight.js';
	const hljs: HLJSApi;
	export default hljs;
}

declare module 'highlight.js/lib/languages/typescript' {
	import type { LanguageFn } from 'highlight.js';
	const typescript: LanguageFn;
	export default typescript;
}

declare module 'highlight.js/lib/languages/javascript' {
	import type { LanguageFn } from 'highlight.js';
	const javascript: LanguageFn;
	export default javascript;
}

declare module 'highlight.js/lib/languages/rust' {
	import type { LanguageFn } from 'highlight.js';
	const rust: LanguageFn;
	export default rust;
}

declare module 'highlight.js/lib/languages/python' {
	import type { LanguageFn } from 'highlight.js';
	const python: LanguageFn;
	export default python;
}

declare module 'highlight.js/lib/languages/css' {
	import type { LanguageFn } from 'highlight.js';
	const css: LanguageFn;
	export default css;
}

declare module 'highlight.js/lib/languages/scss' {
	import type { LanguageFn } from 'highlight.js';
	const scss: LanguageFn;
	export default scss;
}

declare module 'highlight.js/lib/languages/json' {
	import type { LanguageFn } from 'highlight.js';
	const json: LanguageFn;
	export default json;
}

declare module 'highlight.js/lib/languages/yaml' {
	import type { LanguageFn } from 'highlight.js';
	const yaml: LanguageFn;
	export default yaml;
}

declare module 'highlight.js/lib/languages/markdown' {
	import type { LanguageFn } from 'highlight.js';
	const markdown: LanguageFn;
	export default markdown;
}

declare module 'highlight.js/lib/languages/xml' {
	import type { LanguageFn } from 'highlight.js';
	const xml: LanguageFn;
	export default xml;
}

declare module 'highlight.js/lib/languages/bash' {
	import type { LanguageFn } from 'highlight.js';
	const bash: LanguageFn;
	export default bash;
}

declare module 'highlight.js/lib/languages/sql' {
	import type { LanguageFn } from 'highlight.js';
	const sql: LanguageFn;
	export default sql;
}

declare module 'highlight.js/lib/languages/plaintext' {
	import type { LanguageFn } from 'highlight.js';
	const plaintext: LanguageFn;
	export default plaintext;
}
