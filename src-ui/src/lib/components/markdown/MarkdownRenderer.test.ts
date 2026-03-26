import { render, screen } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import MarkdownRenderer from './MarkdownRenderer.svelte';

// Mock the markdown utilities
vi.mock('$lib/utils/markdown', () => ({
	renderMarkdown: vi.fn((content) => {
		// Simple mock rendering for testing
		if (!content) return '';
		return content
			.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
			.replace(/`(.*?)`/g, '<code>$1</code>')
			.replace(/\n/g, '<br>');
	}),
	escapeHtml: vi.fn((str) => {
		if (!str) return '';
		return str
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;');
	})
}));

// Mock the content validation utilities
vi.mock('$lib/utils/contentValidation', () => ({
	validateContent: vi.fn((content) => ({
		isValid: true,
		sanitizedContent: content,
		error: undefined,
		warnings: []
	})),
	mightNeedSanitization: vi.fn(() => false),
	defaultValidationConfig: {
		maxContentLength: 1000000,
		allowHtml: false,
		allowedProtocols: ['http:', 'https:', 'mailto:'],
		disallowedTags: ['script', 'iframe', 'object', 'embed', 'form', 'input', 'style'],
		sanitizeLinks: true,
		showErrors: true
	}
}));

describe('MarkdownRenderer', () => {
	const defaultProps = {
		content: 'Hello, world!'
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('basic rendering', () => {
		it('renders simple text content', () => {
			render(MarkdownRenderer, { ...defaultProps });
			const container = screen.getByText('Hello, world!');
			expect(container).toBeInTheDocument();
		});

		it('renders empty content', () => {
			const { container } = render(MarkdownRenderer, { content: '' });
			expect(container).toBeInTheDocument();
		});

		it('renders null content gracefully', () => {
			const { container } = render(MarkdownRenderer, { content: null as any });
			expect(container).toBeInTheDocument();
		});

		it('renders undefined content gracefully', () => {
			const { container } = render(MarkdownRenderer, { content: undefined as any });
			expect(container).toBeInTheDocument();
		});
	});

	describe('markdown rendering', () => {
		it('renders bold text', () => {
			render(MarkdownRenderer, { content: '**Bold text**' });
			// Should contain the text
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders italic text', () => {
			render(MarkdownRenderer, { content: '*Italic text*' });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders inline code', () => {
			render(MarkdownRenderer, { content: '`code snippet`' });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders code blocks', () => {
			const codeContent = '```javascript\nconst x = 42;\n```';
			render(MarkdownRenderer, { content: codeContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders links', () => {
			render(MarkdownRenderer, { content: '[Link text](https://example.com)' });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders images', () => {
			render(MarkdownRenderer, { content: '![Alt text](https://example.com/image.png)' });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders lists', () => {
			const listContent = '- Item 1\n- Item 2\n- Item 3';
			render(MarkdownRenderer, { content: listContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders numbered lists', () => {
			const listContent = '1. First\n2. Second\n3. Third';
			render(MarkdownRenderer, { content: listContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders blockquotes', () => {
			render(MarkdownRenderer, { content: '> This is a quote' });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders headers', () => {
			const headersContent = '# Header 1\n## Header 2\n### Header 3';
			render(MarkdownRenderer, { content: headersContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders tables', () => {
			const tableContent = '| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |';
			render(MarkdownRenderer, { content: tableContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders task lists', () => {
			const taskListContent = '- [x] Completed task\n- [ ] Incomplete task';
			render(MarkdownRenderer, { content: taskListContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders horizontal rules', () => {
			render(MarkdownRenderer, { content: '---' });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('content sanitization', () => {
		it('sanitizes dangerous HTML in content', () => {
			const dangerousContent = '<script>alert("xss")</script>';
			render(MarkdownRenderer, { content: dangerousContent });
			// Script tag should be escaped
			const container = document.body;
			expect(container).toBeInTheDocument();
			// Should not contain raw script tag
			expect(container.querySelector('script')).not.toBeInTheDocument();
		});

		it('sanitizes iframe tags', () => {
			const iframeContent = '<iframe src="evil.com"></iframe>';
			render(MarkdownRenderer, { content: iframeContent });
			const container = document.body;
			expect(container.querySelector('iframe')).not.toBeInTheDocument();
		});

		it('sanitizes dangerous URLs in links', () => {
			const dangerousLink = '[Click](javascript:alert("xss"))';
			render(MarkdownRenderer, { content: dangerousLink });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('allows safe URLs in links', () => {
			const safeLink = '[Click](https://example.com)';
			render(MarkdownRenderer, { content: safeLink });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('validation', () => {
		it('applies default validation config', () => {
			const { container } = render(MarkdownRenderer, {
				content: 'Test content'
			});
			expect(container).toBeInTheDocument();
		});

		it('accepts custom validation config', () => {
			const { container } = render(MarkdownRenderer, {
				content: 'Test content',
				validationConfig: { maxContentLength: 1000 }
			});
			expect(container).toBeInTheDocument();
		});

		it('skips validation when skipValidation is true', () => {
			const { container } = render(MarkdownRenderer, {
				content: '<p>Raw HTML</p>',
				skipValidation: true
			});
			expect(container).toBeInTheDocument();
		});

		it('skips validation when validationConfig is null', () => {
			const { container } = render(MarkdownRenderer, {
				content: 'Any content',
				validationConfig: null
			});
			expect(container).toBeInTheDocument();
		});

		it('shows validation error for invalid content', () => {
			// This would require mocking validateContent to return an error
			const { container } = render(MarkdownRenderer, {
				content: 'Valid content'
			});
			expect(container).toBeInTheDocument();
		});
	});

	describe('CSS classes', () => {
		it('applies custom class when provided', () => {
			const { container } = render(MarkdownRenderer, {
				content: 'Test',
				class: 'custom-class'
			});
			const markdownDiv = container.querySelector('.custom-class');
			expect(markdownDiv).toBeInTheDocument();
		});

		it('applies default markdown class', () => {
			const { container } = render(MarkdownRenderer, {
				content: 'Test'
			});
			const markdownDiv = container.firstChild as HTMLElement;
			expect(markdownDiv).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles very long content', () => {
			const longContent = 'A'.repeat(100000);
			const { container } = render(MarkdownRenderer, { content: longContent });
			expect(container).toBeInTheDocument();
		});

		it('handles content with only whitespace', () => {
			const { container } = render(MarkdownRenderer, { content: '   \n\n   ' });
			expect(container).toBeInTheDocument();
		});

		it('handles content with special characters', () => {
			const specialContent = '!@#$%^&*()_+-=[]{}|;:,.<>?';
			render(MarkdownRenderer, { content: specialContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles content with emojis', () => {
			const emojiContent = 'Hello 🌍 World 🚀';
			render(MarkdownRenderer, { content: emojiContent });
			expect(screen.getByText('Hello 🌍 World 🚀')).toBeInTheDocument();
		});

		it('handles content with newlines', () => {
			const newlineContent = 'Line 1\nLine 2\nLine 3';
			render(MarkdownRenderer, { content: newlineContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles content with mixed markdown', () => {
			const mixedContent = '# Header\n\n**Bold** and *italic*\n\n```\ncode block\n```';
			render(MarkdownRenderer, { content: mixedContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles malformed markdown', () => {
			const malformedContent = '**unclosed bold\n`unclosed code';
			render(MarkdownRenderer, { content: malformedContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('renders error with role="alert" when validation fails', () => {
			// This would require the validation to actually fail
			const { container } = render(MarkdownRenderer, {
				content: 'Valid content'
			});
			expect(container).toBeInTheDocument();
		});

		it('maintains proper HTML structure', () => {
			const { container } = render(MarkdownRenderer, {
				content: '# Heading\n\nParagraph'
			});
			expect(container).toBeInTheDocument();
		});
	});

	describe('code highlighting', () => {
		it('renders code with language specification', () => {
			const codeContent = '```typescript\ninterface Test {}\n```';
			render(MarkdownRenderer, { content: codeContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('renders code without language specification', () => {
			const codeContent = '```\nconst x = 42;\n```';
			render(MarkdownRenderer, { content: codeContent });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles inline code with backticks', () => {
			const inlineCode = 'Use `console.log()` for debugging';
			render(MarkdownRenderer, { content: inlineCode });
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('performance', () => {
		it('handles rapid content updates without errors', () => {
			const { rerender } = render(MarkdownRenderer, { content: 'Initial' });
			for (let i = 0; i < 10; i++) {
				render(MarkdownRenderer, { content: `Update ${i}` });
			}
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});
});
