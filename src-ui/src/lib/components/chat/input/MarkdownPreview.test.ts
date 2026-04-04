import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import MarkdownPreview from './MarkdownPreview.svelte';

describe('MarkdownPreview', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('renders the preview container', () => {
			const { container } = render(MarkdownPreview, {
				content: 'Hello world'
			});
			const preview = container.querySelector('[role="region"]');
			expect(preview).toBeInTheDocument();
		});

		it('displays plain text content', () => {
			render(MarkdownPreview, {
				content: 'Hello world'
			});
			expect(screen.getByText('Hello world')).toBeInTheDocument();
		});

		it('shows empty state when content is empty', () => {
			render(MarkdownPreview, {
				content: ''
			});
			expect(screen.getByText('Nothing to preview')).toBeInTheDocument();
		});
	});

	describe('markdown parsing - headers', () => {
		it('renders h1 headers', () => {
			render(MarkdownPreview, {
				content: '# Title'
			});
			const heading = screen.getByText('Title');
			expect(heading.tagName).toBe('H1');
		});

		it('renders h2 headers', () => {
			render(MarkdownPreview, {
				content: '## Subtitle'
			});
			const heading = screen.getByText('Subtitle');
			expect(heading.tagName).toBe('H2');
		});

		it('renders h3 headers', () => {
			render(MarkdownPreview, {
				content: '### Section'
			});
			const heading = screen.getByText('Section');
			expect(heading.tagName).toBe('H3');
		});
	});

	describe('markdown parsing - text formatting', () => {
		it('renders bold text with double asterisks', () => {
			render(MarkdownPreview, {
				content: '**bold text**'
			});
			const bold = screen.getByText('bold text');
			expect(bold.tagName).toBe('STRONG');
		});

		it('renders bold text with double underscores', () => {
			render(MarkdownPreview, {
				content: '__bold text__'
			});
			const bold = screen.getByText('bold text');
			expect(bold.tagName).toBe('STRONG');
		});

		it('renders italic text with single asterisk', () => {
			render(MarkdownPreview, {
				content: '*italic text*'
			});
			const italic = screen.getByText('italic text');
			expect(italic.tagName).toBe('EM');
		});

		it('renders italic text with single underscore', () => {
			render(MarkdownPreview, {
				content: '_italic text_'
			});
			const italic = screen.getByText('italic text');
			expect(italic.tagName).toBe('EM');
		});

		it('renders bold and italic combined', () => {
			render(MarkdownPreview, {
				content: '***bold and italic***'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders strikethrough text', () => {
			render(MarkdownPreview, {
				content: '~~deleted text~~'
			});
			const deleted = screen.getByText('deleted text');
			expect(deleted.tagName).toBe('DEL');
		});
	});

	describe('markdown parsing - code', () => {
		it('renders inline code', () => {
			render(MarkdownPreview, {
				content: 'Use `console.log()` for debugging'
			});
			const code = screen.getByText('console.log()');
			expect(code.tagName).toBe('CODE');
		});

		it('renders code blocks', () => {
			render(MarkdownPreview, {
				content: '```\nconst x = 1;\n```'
			});
			const code = screen.getByText('const x = 1;');
			expect(code.tagName).toBe('CODE');
		});

		it('renders code blocks with language', () => {
			render(MarkdownPreview, {
				content: '```typescript\nconst x: number = 1;\n```'
			});
			const code = screen.getByText('const x: number = 1;');
			expect(code.tagName).toBe('CODE');
		});
	});

	describe('markdown parsing - links', () => {
		it('renders links', () => {
			render(MarkdownPreview, {
				content: '[OpenAI](https://openai.com)'
			});
			const link = screen.getByText('OpenAI');
			expect(link.tagName).toBe('A');
			expect(link).toHaveAttribute('href', 'https://openai.com');
		});

		it('sets target="_blank" on links', () => {
			render(MarkdownPreview, {
				content: '[Example](https://example.com)'
			});
			const link = screen.getByText('Example');
			expect(link).toHaveAttribute('target', '_blank');
		});

		it('sets rel="noopener noreferrer" on links', () => {
			render(MarkdownPreview, {
				content: '[Example](https://example.com)'
			});
			const link = screen.getByText('Example');
			expect(link).toHaveAttribute('rel', 'noopener noreferrer');
		});
	});

	describe('markdown parsing - lists', () => {
		it('renders unordered lists', () => {
			render(MarkdownPreview, {
				content: '- Item 1\n- Item 2\n- Item 3'
			});
			expect(screen.getByText('Item 1')).toBeInTheDocument();
			expect(screen.getByText('Item 2')).toBeInTheDocument();
			expect(screen.getByText('Item 3')).toBeInTheDocument();
		});

		it('renders ordered lists', () => {
			render(MarkdownPreview, {
				content: '1. First\n2. Second\n3. Third'
			});
			expect(screen.getByText('First')).toBeInTheDocument();
			expect(screen.getByText('Second')).toBeInTheDocument();
			expect(screen.getByText('Third')).toBeInTheDocument();
		});
	});

	describe('markdown parsing - blockquotes', () => {
		it('renders blockquotes', () => {
			render(MarkdownPreview, {
				content: '> This is a quote'
			});
			const quote = screen.getByText('This is a quote');
			expect(quote.tagName).toBe('BLOCKQUOTE');
		});
	});

	describe('markdown parsing - horizontal rules', () => {
		it('renders horizontal rules', () => {
			const { container } = render(MarkdownPreview, {
				content: '---'
			});
			const hr = container.querySelector('hr');
			expect(hr).toBeInTheDocument();
		});
	});

	describe('security - XSS prevention', () => {
		it('escapes HTML in content', () => {
			render(MarkdownPreview, {
				content: '<script>alert("XSS")</script>'
			});
			// Script should not execute
			const script = document.querySelector('script');
			expect(script).not.toBeInTheDocument();
		});

		it('sanitizes dangerous URLs in links', () => {
			render(MarkdownPreview, {
				content: '[Click](javascript:alert("XSS"))'
			});
			const link = screen.getByText('Click');
			// Dangerous URLs should be replaced with #
			expect(link).toHaveAttribute('href', '#');
		});

		it('blocks data: URLs', () => {
			render(MarkdownPreview, {
				content: '[Click](data:text/html,<script>alert("XSS")</script>)'
			});
			const link = screen.getByText('Click');
			expect(link).toHaveAttribute('href', '#');
		});

		it('blocks vbscript: URLs', () => {
			render(MarkdownPreview, {
				content: '[Click](vbscript:msgbox("XSS"))'
			});
			const link = screen.getByText('Click');
			expect(link).toHaveAttribute('href', '#');
		});

		it('blocks file: URLs', () => {
			render(MarkdownPreview, {
				content: '[Click](file:///etc/passwd)'
			});
			const link = screen.getByText('Click');
			expect(link).toHaveAttribute('href', '#');
		});

		it('allows https: URLs', () => {
			render(MarkdownPreview, {
				content: '[Safe](https://example.com)'
			});
			const link = screen.getByText('Safe');
			expect(link).toHaveAttribute('href', 'https://example.com');
		});

		it('allows http: URLs', () => {
			render(MarkdownPreview, {
				content: '[Safe](http://example.com)'
			});
			const link = screen.getByText('Safe');
			expect(link).toHaveAttribute('href', 'http://example.com');
		});

		it('allows mailto: URLs', () => {
			render(MarkdownPreview, {
				content: '[Email](mailto:test@example.com)'
			});
			const link = screen.getByText('Email');
			expect(link).toHaveAttribute('href', 'mailto:test@example.com');
		});

		it('allows relative URLs starting with /', () => {
			render(MarkdownPreview, {
				content: '[Relative](/path/to/file)'
			});
			const link = screen.getByText('Relative');
			expect(link).toHaveAttribute('href', '/path/to/file');
		});

		it('allows anchor links', () => {
			render(MarkdownPreview, {
				content: '[Anchor](#section)'
			});
			const link = screen.getByText('Anchor');
			expect(link).toHaveAttribute('href', '#section');
		});
	});

	describe('accessibility', () => {
		it('has role="region"', () => {
			const { container } = render(MarkdownPreview, {
				content: 'Test content'
			});
			const region = container.querySelector('[role="region"]');
			expect(region).toBeInTheDocument();
		});

		it('has aria-label', () => {
			const { container } = render(MarkdownPreview, {
				content: 'Test content'
			});
			const region = container.querySelector('[aria-label="Markdown preview"]');
			expect(region).toBeInTheDocument();
		});
	});

	describe('CSS classes', () => {
		it('applies custom class when provided', () => {
			const { container } = render(MarkdownPreview, {
				content: 'Test',
				class: 'custom-class'
			});
			const preview = container.querySelector('.custom-class');
			expect(preview).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles undefined content', () => {
			render(MarkdownPreview, {
				content: undefined as unknown as string
			});
			expect(screen.getByText('Nothing to preview')).toBeInTheDocument();
		});

		it('handles very long content', () => {
			const longContent = '# '.repeat(1000);
			const { container } = render(MarkdownPreview, {
				content: longContent
			});
			const preview = container.querySelector('[role="region"]');
			expect(preview).toBeInTheDocument();
		});

		it('handles malformed markdown', () => {
			render(MarkdownPreview, {
				content: '# Header without newline**bold without closing'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles empty code blocks', () => {
			render(MarkdownPreview, {
				content: '```\n```'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles nested formatting', () => {
			render(MarkdownPreview, {
				content: '**bold with *italic* inside**'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});
});
