/**
 * ArtifactPanel Component Tests
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import ArtifactPanel from './ArtifactPanel.svelte';
import type { Artifact, ArtifactType } from './ArtifactCard.svelte';

describe('ArtifactPanel', () => {
	const mockArtifacts: Artifact[] = [
		{
			id: 'art_1',
			sessionId: 'session_1',
			artifactType: 'Docx',
			filename: 'report.docx',
			createdAt: '2024-01-01T12:00:00Z',
			sizeBytes: 24576,
			metadata: { title: 'Weekly Report' }
		},
		{
			id: 'art_2',
			sessionId: 'session_1',
			artifactType: 'Pptx',
			filename: 'deck.pptx',
			createdAt: '2024-01-02T14:30:00Z',
			sizeBytes: 1048576
		},
		{
			id: 'art_3',
			sessionId: 'session_1',
			artifactType: 'Pdf',
			filename: 'document.pdf',
			createdAt: '2024-01-03T09:15:00Z',
			sizeBytes: 524288
		}
	];

	describe('rendering', () => {
		it('should display all artifacts', () => {
			render(ArtifactPanel, { artifacts: mockArtifacts });

			expect(screen.getByText('report.docx')).toBeInTheDocument();
			expect(screen.getByText('deck.pptx')).toBeInTheDocument();
			expect(screen.getByText('document.pdf')).toBeInTheDocument();
		});

		it('should display the total count', () => {
			render(ArtifactPanel, { artifacts: mockArtifacts });

			expect(screen.getByText('3 documents')).toBeInTheDocument();
		});

		it('should show empty state when no artifacts', () => {
			render(ArtifactPanel, { artifacts: [] });

			expect(screen.getByText('No documents yet')).toBeInTheDocument();
		});

		it('should show filter buttons', () => {
			render(ArtifactPanel, { artifacts: mockArtifacts });

			expect(screen.getByRole('button', { name: /all documents/i })).toBeInTheDocument();
			expect(screen.getByRole('button', { name: /word \(/i })).toBeInTheDocument();
			expect(screen.getByRole('button', { name: /powerpoint \(/i })).toBeInTheDocument();
			expect(screen.getByRole('button', { name: /pdf \(/i })).toBeInTheDocument();
		});
	});

	describe('filtering', () => {
		it('should filter to show only Word documents', async () => {
			render(ArtifactPanel, { artifacts: mockArtifacts });

			const wordFilter = screen.getByRole('button', { name: /word \(/i });
			wordFilter.click();

			await tick();

			expect(screen.getByText('report.docx')).toBeInTheDocument();
			expect(screen.queryByText('deck.pptx')).not.toBeInTheDocument();
			expect(screen.queryByText('document.pdf')).not.toBeInTheDocument();
		});

		it('should filter to show only PowerPoint documents', async () => {
			render(ArtifactPanel, { artifacts: mockArtifacts });

			const pptxFilter = screen.getByRole('button', { name: /powerpoint \(/i });
			pptxFilter.click();

			await tick();

			expect(screen.queryByText('report.docx')).not.toBeInTheDocument();
			expect(screen.getByText('deck.pptx')).toBeInTheDocument();
			expect(screen.queryByText('document.pdf')).not.toBeInTheDocument();
		});

		it('should filter to show only PDF documents', async () => {
			render(ArtifactPanel, { artifacts: mockArtifacts });

			const pdfFilter = screen.getByRole('button', { name: /pdf \(/i });
			pdfFilter.click();

			await tick();

			expect(screen.queryByText('report.docx')).not.toBeInTheDocument();
			expect(screen.queryByText('deck.pptx')).not.toBeInTheDocument();
			expect(screen.getByText('document.pdf')).toBeInTheDocument();
		});

		it('should show empty state for filter with no results', async () => {
			const onlyWordArtifacts: Artifact[] = [
				{ ...mockArtifacts[0] }
			];

			render(ArtifactPanel, { artifacts: onlyWordArtifacts });

			const pdfFilter = screen.getByRole('button', { name: /pdf \(/i });
			pdfFilter.click();

			await tick();

			expect(screen.getByText(/no pdf documents/i)).toBeInTheDocument();
		});
	});

	describe('selection', () => {
		it('should highlight selected artifact', () => {
			render(ArtifactPanel, {
				artifacts: mockArtifacts,
				selectedArtifactId: 'art_1'
			});

			const selectedArtifact = screen.getByText('report.docx').closest('[aria-selected]');
			expect(selectedArtifact).toHaveAttribute('aria-selected', 'true');
		});

		it('should call onSelect when artifact is clicked', async () => {
			const onSelect = vi.fn();
			render(ArtifactPanel, {
				artifacts: mockArtifacts,
				onSelect
			});

			const artifactItem = screen.getByText('report.docx').closest('[role="button"]');
			artifactItem?.click();

			await tick();

			expect(onSelect).toHaveBeenCalledWith(mockArtifacts[0]);
		});
	});

	describe('file size formatting', () => {
		it('should display size in bytes for small files', () => {
			const smallArtifact: Artifact[] = [{
				id: 'art_small',
				sessionId: 'session_1',
				artifactType: 'Docx',
				filename: 'tiny.docx',
				createdAt: '2024-01-01T12:00:00Z',
				sizeBytes: 512
			}];

			render(ArtifactPanel, { artifacts: smallArtifact });

			expect(screen.getByText('512 B')).toBeInTheDocument();
		});

		it('should display size in KB for medium files', () => {
			render(ArtifactPanel, { artifacts: mockArtifacts });

			expect(screen.getByText('24 KB')).toBeInTheDocument();
		});

		it('should display size in MB for large files', () => {
			render(ArtifactPanel, { artifacts: mockArtifacts });

			expect(screen.getByText('1 MB')).toBeInTheDocument(); // 1048576 bytes = 1 MB
		});
	});

	describe('grouping', () => {
		it('should group artifacts by type when "all" filter is selected', () => {
			render(ArtifactPanel, {
				artifacts: mockArtifacts,
				filter: 'all'
			});

			expect(screen.getByText(/word documents/i)).toBeInTheDocument();
			expect(screen.getByText(/powerpoint presentations/i)).toBeInTheDocument();
			expect(screen.getByText(/pdf documents/i)).toBeInTheDocument();
		});
	});
});
