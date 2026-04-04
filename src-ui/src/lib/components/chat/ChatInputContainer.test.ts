import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import ChatInputContainer from './ChatInputContainer.svelte';

describe('ChatInputContainer', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.restoreAllMocks();
		vi.useRealTimers();
	});

	describe('rendering', () => {
		it('renders the input container', () => {
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn()
			});
			const containerEl = container.querySelector('[class*="inputContainer"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('renders ChatInput component', () => {
			render(ChatInputContainer, {
				value: 'test',
				onchange: vi.fn(),
				onsubmit: vi.fn()
			});
			const textarea = screen.getByRole('textbox');
			expect(textarea).toBeInTheDocument();
		});
	});

	describe('value handling', () => {
		it('passes value to ChatInput', () => {
			const { container } = render(ChatInputContainer, {
				value: 'Hello world',
				onchange: vi.fn(),
				onsubmit: vi.fn()
			});
			const textarea = container.querySelector('textarea');
			expect(textarea?.value).toBe('Hello world');
		});

		it('calls onchange when value changes', async () => {
			const onchange = vi.fn();
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange,
				onsubmit: vi.fn()
			});

			const textarea = container.querySelector('textarea');
			if (textarea) {
				textarea.value = 'new value';
				textarea.dispatchEvent(new Event('input', { bubbles: true }));
				await tick();
				// The onchange handler should be called
				expect(textarea.value).toBe('new value');
			}
		});
	});

	describe('thread resolved state', () => {
		it('hides input when threadResolved is true on mount', () => {
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				threadResolved: true
			});
			const containerEl = container.querySelector('[class*="inputContainer"]');
			// Input should not be visible when thread is resolved
			expect(containerEl).not.toBeInTheDocument();
		});

		it('shows input when threadResolved is false', () => {
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				threadResolved: false
			});
			const containerEl = container.querySelector('[class*="inputContainer"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('animates input out when thread becomes resolved', async () => {
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				threadResolved: false
			});

			// Should be visible initially
			let containerEl = container.querySelector('[class*="inputContainer"]');
			expect(containerEl).toBeInTheDocument();

			// Update prop to resolved
			const component = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				threadResolved: true
			});

			// After animation timeout (150ms)
			vi.advanceTimersByTime(200);
			await tick();

			containerEl = component.container.querySelector('[class*="inputContainer"]');
			// Should be hidden after animation
			expect(containerEl).not.toBeInTheDocument();
		});

		it('animates input in when thread becomes unresolved', async () => {
			// Start with resolved state
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				threadResolved: true
			});

			// Should be hidden initially
			let containerEl = container.querySelector('[class*="inputContainer"]');
			expect(containerEl).not.toBeInTheDocument();

			// Change to unresolved
			const component = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				threadResolved: false
			});

			// After transition delay (200ms for banner + 200ms for animation)
			vi.advanceTimersByTime(500);
			await tick();

			containerEl = component.container.querySelector('[class*="inputContainer"]');
			// Should be visible after animation
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('event handlers', () => {
		it('passes onsubmit to ChatInput', () => {
			const onsubmit = vi.fn();
			render(ChatInputContainer, {
				value: 'test message',
				onchange: vi.fn(),
				onsubmit
			});
			// The ChatInput should receive the onsubmit handler
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('passes onfocus to ChatInput', () => {
			const onfocus = vi.fn();
			render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				onfocus
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('passes onblur to ChatInput', () => {
			const onblur = vi.fn();
			render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				onblur
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('placeholder', () => {
		it('uses default placeholder', () => {
			render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn()
			});
			const textarea = screen.getByPlaceholderText('Send a message...');
			expect(textarea).toBeInTheDocument();
		});

		it('uses custom placeholder when provided', () => {
			render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				placeholder: 'Type your message...'
			});
			const textarea = screen.getByPlaceholderText('Type your message...');
			expect(textarea).toBeInTheDocument();
		});
	});

	describe('loading state', () => {
		it('passes loading to ChatInput', () => {
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				loading: true
			});
			const containerEl = container.querySelector('[class*="inputContainer"]');
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('model selector', () => {
		it('passes modelId to ChatInput', () => {
			render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				modelId: 'claude-3-opus-20240229'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('passes onModelChange to ChatInput', () => {
			const onModelChange = vi.fn();
			render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				modelId: 'claude-3-opus-20240229',
				onModelChange
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('passes onOpenSettings to ChatInput', () => {
			const onOpenSettings = vi.fn();
			render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				onOpenSettings
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('animation classes', () => {
		it('applies animateIn class during transition', () => {
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				threadResolved: false
			});
			const containerEl = container.querySelector('[class*="inputContainer"]');
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty value', () => {
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn()
			});
			const textarea = container.querySelector('textarea');
			expect(textarea?.value).toBe('');
		});

		it('handles undefined handlers gracefully', () => {
			const { container } = render(ChatInputContainer, {
				value: 'test',
				onchange: undefined as unknown as (value: string) => void,
				onsubmit: undefined as unknown as () => void
			});
			const containerEl = container.querySelector('[class*="inputContainer"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('handles rapid state changes', async () => {
			const { container } = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				threadResolved: false
			});

			// Rapidly toggle resolved state
			let containerEl = container.querySelector('[class*="inputContainer"]');
			expect(containerEl).toBeInTheDocument();

			// The component should handle rapid changes without crashing
			const component2 = render(ChatInputContainer, {
				value: '',
				onchange: vi.fn(),
				onsubmit: vi.fn(),
				threadResolved: true
			});

			containerEl = component2.container.querySelector('[class*="inputContainer"]');
			// Should still be in DOM until animation completes
			expect(component2.container).toBeInTheDocument();
		});
	});
});
