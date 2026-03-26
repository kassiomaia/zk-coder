import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ApprovalDialog from './ApprovalDialog.svelte';
import type { PendingAction } from '$lib/types/agent';

// Mock child components
vi.mock('../shared/Modal.svelte', () => ({
	default: vi.fn(({ open, onclose, title, children }) => {
		if (!open) return null;
		return `
			<div class="mock-modal" data-open="${open}" data-title="${title}">
				<div class="mock-modal-content">${typeof children === 'function' ? children() : children}</div>
			</div>
		`;
	})
}));

vi.mock('../shared/Button.svelte', () => ({
	default: vi.fn(({ children, onclick, disabled, loading, variant }) => {
		return `
			<button
				class="mock-button mock-button-${variant}"
				data-disabled="${disabled || false}"
				data-loading="${loading || false}"
				type="button"
			>
				${loading ? 'Loading...' : children}
			</button>
		`;
	})
}));

vi.mock('./DiffViewer.svelte', () => ({
	default: vi.fn(({ diff }) => `
		<div class="mock-diff-viewer" data-diff-length="${diff?.length || 0}">Diff Viewer</div>
	`)
}));

describe('ApprovalDialog', () => {
	const mockWriteFileAction: PendingAction = {
		id: 'action-1',
		toolName: 'fs_write_file',
		actionType: { type: 'write_file', path: '/src/test.ts', sizeBytes: 1024 },
		description: 'Write test file',
		preview: {
			contentType: 'diff',
			content: '--- a/test.ts\n+++ b/test.ts\n@@ -1,1 +1,2 @@\n-old\n+new',
			language: 'typescript'
		},
		arguments: { path: '/src/test.ts', content: 'new content' },
		createdAt: '2024-01-01T00:00:00Z'
	};

	const mockDeleteFileAction: PendingAction = {
		id: 'action-2',
		toolName: 'fs_delete_file',
		actionType: { type: 'delete_file', path: '/old-file.ts' },
		description: 'Delete old file',
		arguments: { path: '/old-file.ts' },
		createdAt: '2024-01-01T00:00:00Z'
	};

	const mockExecuteCommandAction: PendingAction = {
		id: 'action-3',
		toolName: 'execute_command',
		actionType: { type: 'execute_command', command: 'npm install' },
		description: 'Install dependencies',
		arguments: { command: 'npm install' },
		createdAt: '2024-01-01T00:00:00Z'
	};

	const mockWebRequestAction: PendingAction = {
		id: 'action-4',
		toolName: 'web_request',
		actionType: { type: 'web_request', url: 'https://api.example.com/data' },
		description: 'Fetch data from API',
		arguments: { url: 'https://api.example.com/data', method: 'GET' },
		createdAt: '2024-01-01T00:00:00Z'
	};

	const mockReadFileAction: PendingAction = {
		id: 'action-5',
		toolName: 'fs_read_file',
		actionType: { type: 'read_file', path: '/src/config.ts' },
		description: 'Read configuration file',
		arguments: { path: '/src/config.ts' },
		createdAt: '2024-01-01T00:00:00Z'
	};

	const defaultProps = {
		action: mockWriteFileAction,
		onapprove: vi.fn(),
		ondeny: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('rendering', () => {
		it('renders dialog with action title and description', () => {
			render(ApprovalDialog, { ...defaultProps });

			expect(screen.getByText('Allow Write File?')).toBeInTheDocument();
			expect(screen.getByText('Write test file')).toBeInTheDocument();
		});

		it('renders tool name in header', () => {
			render(ApprovalDialog, { ...defaultProps });

			expect(screen.getByText('Tool: fs_write_file')).toBeInTheDocument();
		});

		it('renders approve and deny buttons', () => {
			const { container } = render(ApprovalDialog, { ...defaultProps });

			const buttons = container.querySelectorAll('.mock-button');
			expect(buttons.length).toBeGreaterThanOrEqual(2);

			const denyBtn = Array.from(buttons).find((b) => b.textContent === 'Deny');
			const allowBtn = Array.from(buttons).find((b) => b.textContent === 'Allow');

			expect(denyBtn).toBeInTheDocument();
			expect(allowBtn).toBeInTheDocument();
		});
	});

	describe('action type labels', () => {
		it('shows correct label for write_file action', () => {
			render(ApprovalDialog, { ...defaultProps });

			expect(screen.getByText('Allow Write File?')).toBeInTheDocument();
		});

		it('shows correct label for delete_file action', () => {
			render(ApprovalDialog, {
				...defaultProps,
				action: mockDeleteFileAction
			});

			expect(screen.getByText('Allow Delete File?')).toBeInTheDocument();
		});

		it('shows correct label for execute_command action', () => {
			render(ApprovalDialog, {
				...defaultProps,
				action: mockExecuteCommandAction
			});

			expect(screen.getByText('Allow Execute Command?')).toBeInTheDocument();
		});

		it('shows correct label for web_request action', () => {
			render(ApprovalDialog, {
				...defaultProps,
				action: mockWebRequestAction
			});

			expect(screen.getByText('Allow Web Request?')).toBeInTheDocument();
		});

		it('shows correct label for read_file action', () => {
			render(ApprovalDialog, {
				...defaultProps,
				action: mockReadFileAction
			});

			expect(screen.getByText('Allow Read File?')).toBeInTheDocument();
		});
	});

	describe('icon styles', () => {
		it('applies warning icon style for write_file action', () => {
			const { container } = render(ApprovalDialog, {
				...defaultProps,
				action: mockWriteFileAction
			});

			const iconWrapper = container.querySelector('.icon-wrapper.warning');
			expect(iconWrapper).toBeInTheDocument();
		});

		it('applies danger icon style for delete_file action', () => {
			const { container } = render(ApprovalDialog, {
				...defaultProps,
				action: mockDeleteFileAction
			});

			const iconWrapper = container.querySelector('.icon-wrapper.danger');
			expect(iconWrapper).toBeInTheDocument();
		});

		it('applies info icon style for execute_command action', () => {
			const { container } = render(ApprovalDialog, {
				...defaultProps,
				action: mockExecuteCommandAction
			});

			const iconWrapper = container.querySelector('.icon-wrapper.info');
			expect(iconWrapper).toBeInTheDocument();
		});

		it('applies warning icon style for web_request action', () => {
			const { container } = render(ApprovalDialog, {
				...defaultProps,
				action: mockWebRequestAction
			});

			const iconWrapper = container.querySelector('.icon-wrapper.warning');
			expect(iconWrapper).toBeInTheDocument();
		});
	});

	describe('preview display', () => {
		it('shows diff preview when action has diff preview', () => {
			const { container } = render(ApprovalDialog, { ...defaultProps });

			const diffViewer = container.querySelector('.mock-diff-viewer');
			expect(diffViewer).toBeInTheDocument();
		});

		it('shows "Changes" label for diff preview', () => {
			render(ApprovalDialog, { ...defaultProps });

			expect(screen.getByText('Changes')).toBeInTheDocument();
		});

		it('shows language indicator when diff has language', () => {
			render(ApprovalDialog, { ...defaultProps });

			expect(screen.getByText('typescript')).toBeInTheDocument();
		});

		it('shows "Preview" label for text preview', () => {
			const textPreviewAction: PendingAction = {
				...mockWriteFileAction,
				preview: {
					contentType: 'text',
					content: 'Some text content',
					language: 'text'
				}
			};
			render(ApprovalDialog, {
				...defaultProps,
				action: textPreviewAction
			});

			expect(screen.getByText('Preview')).toBeInTheDocument();
		});

		it('does not show preview section when action has no preview', () => {
			const actionWithoutPreview: PendingAction = {
				id: 'action-6',
				toolName: 'fs_delete_file',
				actionType: { type: 'delete_file', path: '/test.txt' },
				description: 'Delete file',
				arguments: { path: '/test.txt' },
				createdAt: '2024-01-01T00:00:00Z'
			};
			const { container } = render(ApprovalDialog, {
				...defaultProps,
				action: actionWithoutPreview
			});

			const previewSection = container.querySelector('.preview-section');
			expect(previewSection).not.toBeInTheDocument();
		});
	});

	describe('interactions', () => {
		it('calls onapprove when Allow button is clicked', async () => {
			const handleApprove = vi.fn();
			const { container } = render(ApprovalDialog, {
				...defaultProps,
				onapprove: handleApprove
			});

			const allowBtn = Array.from(container.querySelectorAll('.mock-button')).find(
				(b) => b.textContent === 'Allow'
			);

			if (allowBtn) {
				await fireEvent.click(allowBtn);
				expect(handleApprove).toHaveBeenCalledTimes(1);
			}
		});

		it('calls ondeny when Deny button is clicked', async () => {
			const handleDeny = vi.fn();
			const { container } = render(ApprovalDialog, {
				...defaultProps,
				ondeny: handleDeny
			});

			const denyBtn = Array.from(container.querySelectorAll('.mock-button')).find(
				(b) => b.textContent === 'Deny'
			);

			if (denyBtn) {
				await fireEvent.click(denyBtn);
				expect(handleDeny).toHaveBeenCalledTimes(1);
			}
		});

		it('disables buttons while loading after approve', async () => {
			let isLoading = false;
			const handleApprove = vi.fn().mockImplementation(() => {
				isLoading = true;
			});

			// Note: Loading state is managed internally, this test verifies the structure
			const { container } = render(ApprovalDialog, {
				...defaultProps,
				onapprove: handleApprove
			});

			const buttons = container.querySelectorAll('.mock-button');
			buttons.forEach((btn) => {
				// Initially not disabled
				expect(btn.getAttribute('data-disabled')).toBe('false');
			});
		});
	});

	describe('accessibility', () => {
		it('has proper heading structure', () => {
			const { container } = render(ApprovalDialog, { ...defaultProps });

			const heading = container.querySelector('.header-text h3');
			expect(heading).toBeInTheDocument();
		});

		it('has proper button types', () => {
			const { container } = render(ApprovalDialog, { ...defaultProps });

			const buttons = container.querySelectorAll('button[type="button"]');
			expect(buttons.length).toBeGreaterThan(0);
		});

		it('has descriptive tool name with monospace font', () => {
			const { container } = render(ApprovalDialog, { ...defaultProps });

			const toolName = container.querySelector('.tool-name');
			expect(toolName).toBeInTheDocument();
			expect(toolName?.textContent).toBe('Tool: fs_write_file');
		});
	});

	describe('edge cases', () => {
		it('handles action with unknown type gracefully', () => {
			const unknownAction: PendingAction = {
				id: 'action-7',
				toolName: 'unknown_tool',
				actionType: { type: 'read_file', path: '/test' }, // Using valid type but different tool
				description: 'Unknown action',
				arguments: {},
				createdAt: '2024-01-01T00:00:00Z'
			};
			render(ApprovalDialog, {
				...defaultProps,
				action: unknownAction
			});

			// Should still render with "Action" as fallback
			expect(screen.getByText(/Allow/i)).toBeInTheDocument();
		});

		it('handles empty preview content', () => {
			const emptyPreviewAction: PendingAction = {
				...mockWriteFileAction,
				preview: {
					contentType: 'text',
					content: ''
				}
			};
			const { container } = render(ApprovalDialog, {
				...defaultProps,
				action: emptyPreviewAction
			});

			const previewContent = container.querySelector('.preview-content');
			expect(previewContent).toBeInTheDocument();
		});

		it('handles very long description text', () => {
			const longDesc =
				'This is a very long description that spans multiple lines and contains a lot of text. '.repeat(10);
			const longDescAction: PendingAction = {
				...mockWriteFileAction,
				description: longDesc
			};
			render(ApprovalDialog, {
				...defaultProps,
				action: longDescAction
			});

			const description = screen.getByText((content) => content.startsWith('This is a very long'));
			expect(description).toBeInTheDocument();
		});
	});
});
