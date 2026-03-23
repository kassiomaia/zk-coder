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
	const hljs: {
		registerLanguage(name: string, module: any): void;
		highlight(code: string, language: string): { value: string };
	};
	export default hljs;
}

declare module 'highlight.js/lib/languages/typescript' {
	const typescript: any;
	export default typescript;
}

declare module 'highlight.js/lib/languages/javascript' {
	const javascript: any;
	export default javascript;
}

declare module 'highlight.js/lib/languages/rust' {
	const rust: any;
	export default rust;
}

declare module 'highlight.js/lib/languages/python' {
	const python: any;
	export default python;
}

declare module 'highlight.js/lib/languages/css' {
	const css: any;
	export default css;
}

declare module 'highlight.js/lib/languages/scss' {
	const scss: any;
	export default scss;
}

declare module 'highlight.js/lib/languages/json' {
	const json: any;
	export default json;
}

declare module 'highlight.js/lib/languages/yaml' {
	const yaml: any;
	export default yaml;
}

declare module 'highlight.js/lib/languages/markdown' {
	const markdown: any;
	export default markdown;
}

declare module 'highlight.js/lib/languages/xml' {
	const xml: any;
	export default xml;
}
