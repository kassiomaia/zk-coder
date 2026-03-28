import * as monaco from 'monaco-editor';

/**
 * Registers custom themes for the Monaco Editor.
 * This ensures that themes like 'tokyo-night' are available to all instances.
 */
export function registerCustomThemes() {
	// Tokyo Night Theme Definition
	monaco.editor.defineTheme('tokyo-night', {
		base: 'vs-dark',
		inherit: true,
		rules: [
			{ token: '', foreground: 'a9b1d6', background: '1a1b26' },
			{ token: 'variable', foreground: 'c0caf5' },
			{ token: 'variable.predefined', foreground: '7dcfff' },
			{ token: 'variable.parameter', foreground: 'e0af68' },
			{ token: 'constant', foreground: 'ff9e64' },
			{ token: 'comment', foreground: '565f89' },
			{ token: 'number', foreground: 'ff9e64' },
			{ token: 'number.hex', foreground: 'ff9e64' },
			{ token: 'regexp', foreground: 'b4f9f8' },
			{ token: 'annotation', foreground: 'ff9e64' },
			{ token: 'type', foreground: '2ac3de' },
			{ token: 'delimiter', foreground: '89ddff' },
			{ token: 'delimiter.parenthesis', foreground: 'a9b1d6' },
			{ token: 'delimiter.bracket', foreground: 'a9b1d6' },
			{ token: 'delimiter.square', foreground: 'a9b1d6' },
			{ token: 'tag', foreground: 'f7768e' },
			{ token: 'meta.selector.css', foreground: 'f7768e' },
			{ token: 'entity.other.attribute-name.css', foreground: 'e0af68' },
			{ token: 'entity.name.tag.css', foreground: 'f7768e' },
			{ token: 'keyword', foreground: 'bb9af7' },
			{ token: 'keyword.control', foreground: 'bb9af7' },
			{ token: 'keyword.operator', foreground: 'bb9af7' },
			{ token: 'keyword.other', foreground: 'bb9af7' },
			{ token: 'operator', foreground: '89ddff' },
			{ token: 'string', foreground: '9ece6a' },
			{ token: 'string.escape', foreground: '9ece6a' },
			{ token: 'string.regexp', foreground: 'b4f9f8' },
			{ token: 'attribute.name', foreground: 'bb9af7' },
			{ token: 'attribute.value', foreground: '9ece6a' },
			{ token: 'function', foreground: '7aa2f7' },
			{ token: 'function.method', foreground: '7aa2f7' },
			{ token: 'function.call', foreground: '7aa2f7' },
			{ token: 'function.definition', foreground: '7aa2f7' },
		],
		colors: {
			'editor.background': '#09090b',
			'editor.foreground': '#d4d4d8',
			'editor.lineHighlightBackground': '#18181b',
			'editor.selectionBackground': '#27272a',
			'editor.selectionHighlightBackground': '#27272a',
			'editorCursor.foreground': '#6366f1',
			'editorWhitespace.foreground': '#27272a',
			'editorIndentGuide.background': '#1f1f23',
			'editorIndentGuide.activeBackground': '#3f3f46',
			'editorLineNumber.foreground': '#3f3f46',
			'editorLineNumber.activeForeground': '#a1a1aa',
			'scrollbarSlider.background': '#1f1f23',
			'scrollbarSlider.hoverBackground': '#27272a',
			'scrollbarSlider.activeBackground': '#3f3f46',
			'minimap.background': '#09090b',
			'minimap.selectionHighlight': '#6366f140',
			'minimap.errorHighlight': '#ef4444',
			'minimap.warningHighlight': '#f59e0b',
			'minimapSlider.background': '#1f1f2360',
			'minimapSlider.hoverBackground': '#27272a80',
			'minimapSlider.activeBackground': '#3f3f46a0'
		}
	});
}
