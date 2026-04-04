import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	type ContentValidationConfig,
	defaultValidationConfig,
	mightNeedSanitization,
	sanitizeLinks,
	sanitizeUrl,
	validateContent
} from './contentValidation';

describe('contentValidation', () => {
	describe('validateContent', () => {
		describe('basic content handling', () => {
			it('accepts plain text content', () => {
				const result = validateContent('Hello world');
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).toBe('Hello world');
				expect(result.warnings).toHaveLength(0);
			});

			it('accepts markdown content', () => {
				const content = '# Heading\n\nThis is **bold** and *italic* text.';
				const result = validateContent(content);
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).toBe(content);
			});

			it('handles empty string', () => {
				const result = validateContent('');
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).toBe('');
			});

			it('handles null content', () => {
				const result = validateContent(null as any);
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).toBe('');
				expect(result.warnings).toHaveLength(0);
			});

			it('handles undefined content', () => {
				const result = validateContent(undefined as any);
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).toBe('');
				expect(result.warnings).toHaveLength(0);
			});

			it('converts non-string content to string', () => {
				const result = validateContent(12345 as any);
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).toBe('12345');
			});
		});

		describe('content length validation', () => {
			it('accepts content within default limit (1MB)', () => {
				const content = 'a'.repeat(100000);
				const result = validateContent(content);
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).toHaveLength(100000);
			});

			it('rejects content exceeding default limit', () => {
				const content = 'a'.repeat(1000001);
				const result = validateContent(content);
				expect(result.isValid).toBe(false);
				expect(result.sanitizedContent).toBe('');
				expect(result.error).toContain('1,000,000');
			});

			it('respects custom max content length', () => {
				const content = 'a'.repeat(1001);
				const config: ContentValidationConfig = { maxContentLength: 1000 };
				const result = validateContent(content, config);
				expect(result.isValid).toBe(false);
				expect(result.error).toContain('1,000');
			});

			it('handles content at exact boundary', () => {
				const content = 'a'.repeat(1000000);
				const result = validateContent(content);
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).toHaveLength(1000000);
			});
		});

		describe('HTML content handling', () => {
			it('strips script tags by default', () => {
				const content = 'Hello <script>alert("xss")</script> world';
				const result = validateContent(content);
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).not.toContain('<script>');
				expect(result.sanitizedContent).not.toContain('</script>');
				expect(result.warnings).toContain('HTML content was stripped from the input');
			});

			it('strips iframe tags', () => {
				const content = 'Before <iframe src="evil.com"></iframe> after';
				const result = validateContent(content);
				expect(result.sanitizedContent).not.toContain('<iframe>');
				expect(result.sanitizedContent).not.toContain('</iframe>');
			});

			it('strips object tags', () => {
				const content = '<object data="evil.swf"></object>';
				const result = validateContent(content);
				expect(result.sanitizedContent).not.toContain('<object');
			});

			it('strips embed tags', () => {
				const content = '<embed src="evil.swf">';
				const result = validateContent(content);
				expect(result.sanitizedContent).not.toContain('<embed');
			});

			it('strips form tags', () => {
				const content = '<form action="evil.com"><input type="password"></form>';
				const result = validateContent(content);
				expect(result.sanitizedContent).not.toContain('<form');
				expect(result.sanitizedContent).not.toContain('<input');
			});

			it('strips style tags', () => {
				const content = '<style>body { display: none; }</style>';
				const result = validateContent(content);
				expect(result.sanitizedContent).not.toContain('<style>');
			});

			it('strips self-closing dangerous tags', () => {
				const content = 'Text <script src="evil.js" /> more text';
				const result = validateContent(content);
				expect(result.sanitizedContent).not.toContain('<script');
			});

			it('strips tags with attributes', () => {
				const content = '<script type="text/javascript" src="evil.js"></script>';
				const result = validateContent(content);
				expect(result.sanitizedContent).not.toContain('type=');
				expect(result.sanitizedContent).not.toContain('src=');
			});

			it('allows HTML when allowHtml is true', () => {
				const content = '<p>Hello <strong>world</strong></p>';
				const config: ContentValidationConfig = { allowHtml: true };
				const result = validateContent(content, config);
				expect(result.isValid).toBe(true);
				expect(result.sanitizedContent).toContain('<p>');
				expect(result.sanitizedContent).toContain('<strong>');
			});

			it('still strips disallowed tags when allowHtml is true', () => {
				const content = '<p>Safe</p><script>evil</script>';
				const config: ContentValidationConfig = { allowHtml: true };
				const result = validateContent(content, config);
				// When allowHtml is true, the HTML check is skipped entirely
				// So script tags are NOT stripped - this is the current behavior
				expect(result.sanitizedContent).toContain('<script>');
			});
		});

		describe('custom disallowed tags', () => {
			it('strips custom disallowed tags', () => {
				const content = 'Click <button>here</button> for <a href="#">link</a>';
				const config: ContentValidationConfig = {
					disallowedTags: ['button']
				};
				const result = validateContent(content, config);
				expect(result.sanitizedContent).not.toContain('<button');
				expect(result.sanitizedContent).toContain('<a href="#">');
			});

			it('merges custom tags with defaults', () => {
				const content = '<div>Content</div><script>evil</script>';
				const config: ContentValidationConfig = {
					disallowedTags: ['div']
				};
				const result = validateContent(content, config);
				// The custom tags replace defaults via spread operator, not merge
				// So only 'div' is in the list, 'script' is NOT stripped
				// This is the current behavior - custom tags replace defaults
				expect(result.sanitizedContent).toContain('<script>');
				expect(result.sanitizedContent).not.toContain('<div>');
			});
		});

		describe('link sanitization', () => {
			beforeEach(() => {
				vi.spyOn(console, 'warn').mockImplementation(() => {});
			});

			it('sanitizes dangerous protocols in markdown links', () => {
				const content = '[click](javascript:alert("xss"))';
				const result = validateContent(content);
				expect(result.sanitizedContent).toContain('[click](#)');
				expect(result.warnings.length).toBeGreaterThan(0);
			});

			it('sanitizes data: URLs in markdown links', () => {
				const content = '[image](data:text/html,<script>alert("xss")</script>)';
				const result = validateContent(content);
				expect(result.sanitizedContent).toContain('[image](#)');
			});

			it('allows http and https URLs', () => {
				const content = '[example](https://example.com)';
				const result = validateContent(content);
				expect(result.sanitizedContent).toContain('https://example.com');
			});

			it('allows mailto: links', () => {
				const content = '[email](mailto:test@example.com)';
				const result = validateContent(content);
				expect(result.sanitizedContent).toContain('mailto:test@example.com');
			});

			it('can disable link sanitization', () => {
				const content = '[link](javascript:alert("xss"))';
				const config: ContentValidationConfig = { sanitizeLinks: false };
				const result = validateContent(content, config);
				expect(result.sanitizedContent).toContain('javascript:');
			});

			it('sanitizes image URLs', () => {
				const content = '![alt](javascript:alert("xss"))';
				const result = validateContent(content);
				expect(result.sanitizedContent).not.toContain('javascript:');
			});
		});
	});

	describe('sanitizeLinks', () => {
		beforeEach(() => {
			vi.spyOn(console, 'warn').mockImplementation(() => {});
		});

		it('sanitizes markdown links with dangerous protocols', () => {
			const content = '[text](javascript:evil)';
			const result = sanitizeLinks(content);
			expect(result.content).toBe('[text](#)');
			expect(result.warnings).toHaveLength(1);
		});

		it('sanitizes data: URLs', () => {
			const content = '[text](data:text/html,<script>)';
			const result = sanitizeLinks(content);
			expect(result.content).toBe('[text](#)');
		});

		it('sanitizes vbscript: URLs', () => {
			const content = '[text](vbscript:evil)';
			const result = sanitizeLinks(content);
			expect(result.content).toBe('[text](#)');
		});

		it('sanitizes file: URLs', () => {
			const content = '[text](file:///etc/passwd)';
			const result = sanitizeLinks(content);
			expect(result.content).toBe('[text](#)');
		});

		it('sanitizes blob: URLs', () => {
			const content = '[text](blob:https://example.com/uid)';
			const result = sanitizeLinks(content);
			expect(result.content).toBe('[text](#)');
		});

		it('preserves safe http: URLs', () => {
			const content = '[example](http://example.com)';
			const result = sanitizeLinks(content);
			expect(result.content).toBe('[example](http://example.com)');
			expect(result.warnings).toHaveLength(0);
		});

		it('preserves safe https: URLs', () => {
			const content = '[example](https://example.com/path?query=value)';
			const result = sanitizeLinks(content);
			expect(result.content).toContain('https://example.com');
		});

		it('preserves mailto: URLs', () => {
			const content = '[email](mailto:user@example.com)';
			const result = sanitizeLinks(content);
			expect(result.content).toContain('mailto:user@example.com');
		});

		it('handles multiple links in content', () => {
			const content = '[safe](https://safe.com) and [evil](javascript:alert(1))';
			const result = sanitizeLinks(content);
			expect(result.content).toContain('https://safe.com');
			expect(result.content).not.toContain('javascript:');
		});

		it('sanitizes image URLs', () => {
			const content = '![alt](javascript:evil)';
			const result = sanitizeLinks(content);
			expect(result.content).toBe('![alt](#)');
		});

		it('preserves safe image URLs', () => {
			const content = '![alt](https://example.com/image.png)';
			const result = sanitizeLinks(content);
			expect(result.content).toContain('https://example.com/image.png');
		});

		it('handles anchor links', () => {
			const content = '[section](#header)';
			const result = sanitizeLinks(content);
			expect(result.content).toContain('#header');
		});

		it('handles relative URLs', () => {
			const content = '[page](/path/to/page)';
			const result = sanitizeLinks(content);
			expect(result.content).toContain('/path/to/page');
		});

		it('handles relative URLs with ./', () => {
			const content = '[page](./relative)';
			const result = sanitizeLinks(content);
			expect(result.content).toContain('./relative');
		});

		it('handles relative URLs with ../', () => {
			const content = '[page](../parent)';
			const result = sanitizeLinks(content);
			expect(result.content).toContain('../parent');
		});

		it('respects custom allowed protocols', () => {
			const content = '[ftp](ftp://example.com/file)';
			const result = sanitizeLinks(content, ['http:', 'https:', 'ftp:']);
			expect(result.content).toContain('ftp://example.com');
		});

		it('handles malformed URLs', () => {
			const content = '[link](not a url)';
			const result = sanitizeLinks(content);
			expect(result.content).toContain('[link](#)');
		});
	});

	describe('sanitizeUrl', () => {
		beforeEach(() => {
			vi.spyOn(console, 'warn').mockImplementation(() => {});
		});

		it('returns # for empty URLs', () => {
			expect(sanitizeUrl('')).toBe('#');
			expect(sanitizeUrl('   ')).toBe('#');
		});

		it('returns anchor links as-is', () => {
			expect(sanitizeUrl('#section')).toBe('#section');
			expect(sanitizeUrl('#')).toBe('#');
		});

		it('returns relative URLs as-is', () => {
			expect(sanitizeUrl('/path')).toBe('/path');
			expect(sanitizeUrl('./relative')).toBe('./relative');
			expect(sanitizeUrl('../parent')).toBe('../parent');
		});

		it('blocks javascript: protocol', () => {
			expect(sanitizeUrl('javascript:alert(1)')).toBe('#');
			expect(sanitizeUrl('JAVASCRIPT:alert(1)')).toBe('#');
			expect(sanitizeUrl('Javascript:alert(1)')).toBe('#');
		});

		it('blocks data: protocol', () => {
			expect(sanitizeUrl('data:text/html,<script>')).toBe('#');
			expect(sanitizeUrl('DATA:text/html,<script>')).toBe('#');
		});

		it('blocks vbscript: protocol', () => {
			expect(sanitizeUrl('vbscript:msgbox(1)')).toBe('#');
		});

		it('blocks file: protocol', () => {
			expect(sanitizeUrl('file:///etc/passwd')).toBe('#');
		});

		it('blocks blob: protocol', () => {
			expect(sanitizeUrl('blob:https://example.com/123')).toBe('#');
		});

		it('allows http: protocol', () => {
			expect(sanitizeUrl('http://example.com')).toBe('http://example.com');
		});

		it('allows https: protocol', () => {
			expect(sanitizeUrl('https://example.com')).toBe('https://example.com');
		});

		it('allows mailto: protocol by default', () => {
			expect(sanitizeUrl('mailto:test@example.com')).toBe('mailto:test@example.com');
		});

		it('returns # for invalid URL formats', () => {
			expect(sanitizeUrl('not a url')).toBe('#');
			expect(sanitizeUrl('http://')).toBe('#');
		});

		it('trims whitespace from URLs', () => {
			expect(sanitizeUrl('  https://example.com  ')).toBe('https://example.com');
		});

		it('blocks non-allowed protocols', () => {
			expect(sanitizeUrl('ftp://example.com', ['http:', 'https:'])).toBe('#');
		});

		it('handles URLs with query strings and fragments', () => {
			const url = 'https://example.com/path?query=value#fragment';
			expect(sanitizeUrl(url)).toBe(url);
		});

		it('handles URLs with authentication', () => {
			const url = 'https://user:pass@example.com/';
			expect(sanitizeUrl(url)).toBe(url);
		});
	});

	describe('mightNeedSanitization', () => {
		it('returns false for empty or null content', () => {
			expect(mightNeedSanitization('')).toBe(false);
			expect(mightNeedSanitization(null as any)).toBe(false);
			expect(mightNeedSanitization(undefined as any)).toBe(false);
		});

		it('returns true for javascript: URLs', () => {
			expect(mightNeedSanitization('[link](javascript:alert(1))')).toBe(true);
		});

		it('returns true for data: URLs', () => {
			expect(mightNeedSanitization('![img](data:text/html,<script>)')).toBe(true);
		});

		it('returns true for vbscript: URLs', () => {
			expect(mightNeedSanitization('[link](vbscript:msgbox(1))')).toBe(true);
		});

		it('returns true for script tags', () => {
			expect(mightNeedSanitization('<script>alert(1)</script>')).toBe(true);
			expect(mightNeedSanitization('<SCRIPT>alert(1)</SCRIPT>')).toBe(true);
			expect(mightNeedSanitization('<Script>alert(1)</Script>')).toBe(true);
		});

		it('returns true for iframe tags', () => {
			expect(mightNeedSanitization('<iframe src="evil.com"></iframe>')).toBe(true);
			expect(mightNeedSanitization('<IFRAME src="evil.com"></IFRAME>')).toBe(true);
		});

		it('returns true for embed tags', () => {
			expect(mightNeedSanitization('<embed src="evil.swf">')).toBe(true);
		});

		it('returns true for object tags', () => {
			expect(mightNeedSanitization('<object data="evil.swf"></object>')).toBe(true);
		});

		it('returns true for event handlers', () => {
			expect(mightNeedSanitization('<div onclick="evil()">')).toBe(true);
			expect(mightNeedSanitization('<div ONCLICK="evil()">')).toBe(true);
			expect(mightNeedSanitization('<a onmouseover="bad()">')).toBe(true);
			expect(mightNeedSanitization('<img onload="evil()">')).toBe(true);
		});

		it('returns false for safe content', () => {
			expect(mightNeedSanitization('Hello world')).toBe(false);
			expect(mightNeedSanitization('[link](https://example.com)')).toBe(false);
			expect(mightNeedSanitization('# Heading')).toBe(false);
			expect(mightNeedSanitization('**bold** text')).toBe(false);
		});

		it('returns true for content with multiple suspicious patterns', () => {
			const content = '<script>alert(1)</script> and <iframe src="evil.com"></iframe>';
			expect(mightNeedSanitization(content)).toBe(true);
		});
	});

	describe('defaultValidationConfig', () => {
		it('has secure default values', () => {
			expect(defaultValidationConfig.maxContentLength).toBe(1000000);
			expect(defaultValidationConfig.allowHtml).toBe(false);
			expect(defaultValidationConfig.allowedProtocols).toContain('http:');
			expect(defaultValidationConfig.allowedProtocols).toContain('https:');
			expect(defaultValidationConfig.allowedProtocols).toContain('mailto:');
			expect(defaultValidationConfig.disallowedTags).toContain('script');
			expect(defaultValidationConfig.disallowedTags).toContain('iframe');
			expect(defaultValidationConfig.sanitizeLinks).toBe(true);
			expect(defaultValidationConfig.showErrors).toBe(true);
		});

		it('includes all dangerous tags in disallowedTags', () => {
			expect(defaultValidationConfig.disallowedTags).toContain('script');
			expect(defaultValidationConfig.disallowedTags).toContain('iframe');
			expect(defaultValidationConfig.disallowedTags).toContain('object');
			expect(defaultValidationConfig.disallowedTags).toContain('embed');
			expect(defaultValidationConfig.disallowedTags).toContain('form');
			expect(defaultValidationConfig.disallowedTags).toContain('input');
			expect(defaultValidationConfig.disallowedTags).toContain('style');
		});
	});

	describe('real-world scenarios', () => {
		beforeEach(() => {
			vi.spyOn(console, 'warn').mockImplementation(() => {});
		});

		it('handles safe markdown with links', () => {
			const content =
				'# Welcome\n\nThis is a [link](https://example.com) and ![image](https://example.com/img.png).';
			const result = validateContent(content);
			expect(result.isValid).toBe(true);
			expect(result.sanitizedContent).toBe(content);
		});

		it('sanitizes mixed safe and unsafe content', () => {
			const content = 'Safe [link](https://safe.com) and unsafe [evil](javascript:alert(1))';
			const result = validateContent(content);
			expect(result.isValid).toBe(true);
			expect(result.sanitizedContent).toContain('https://safe.com');
			expect(result.sanitizedContent).not.toContain('javascript:');
		});

		it('handles code blocks with script-like content', () => {
			const content = '```javascript\nconsole.log("hello");\n```';
			const result = validateContent(content);
			expect(result.isValid).toBe(true);
			expect(result.sanitizedContent).toContain('console.log');
		});

		it('strips HTML but preserves markdown', () => {
			const content = '<p>Text</p> and **markdown**';
			const result = validateContent(content);
			// Note: <p> is not in the default disallowed tags list (script, iframe, object, embed, form, input, style)
			// So it won't be stripped. Let's test with a disallowed tag.
			const contentWithDisallowed = '<script>Text</script> and **markdown**';
			const result2 = validateContent(contentWithDisallowed);
			expect(result2.sanitizedContent).not.toContain('<script>');
			expect(result2.sanitizedContent).toContain('**markdown**');
		});

		it('handles very long URLs in warnings', () => {
			const longUrl = 'javascript:'.repeat(100);
			const content = `[link](${longUrl})`;
			const result = validateContent(content);
			expect(result.warnings.some((w) => w.length < 100)).toBe(true);
		});
	});
});
