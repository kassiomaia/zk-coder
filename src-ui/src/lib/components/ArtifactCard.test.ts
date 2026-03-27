/**
 * ArtifactCard Component Tests
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
	import ArtifactCard, { type ArtifactRef } from './ArtifactCard.svelte';

describe('ArtifactCard', () => {
	const mockArtifact: ArtifactRef = {
		id: 'art_123',
		sessionId: 'session_456',
		artifactType: 'Docx',
		filename: 'test-document.docx',
		createdAt: '2024-01-01T00:00:00Z',
		sizeBytes: 24576,
		metadata: {
			title: 'Test Document',
			description: 'A test document',
			tags: ['test', 'example']
		}
	};

	describe('rendering', () => {
		it('should display the filename', () => {
			render(ArtifactCard, { artifact: mockArtifact });
			expect(screen.getByText('test-document.docx')).toBeInTheDocument();
		});

		it('should display the file size formatted', () => {
			render(ArtifactCard, { artifact: mockArtifact });
			expect(screen.getByText('24 KB')).toBeInTheDocument();
		});

		it('should display the Word badge for DOCX files', () => {
			render(ArtifactCard, { artifact: mockArtifact });
			expect(screen.getByText('Word')).toBeInTheDocument();
		});

		it('should display the PowerPoint badge for PPTX files', () => {
			const pptxArtifact: ArtifactRef = { ...mockArtifact, artifactType: 'Pptx', filename: 'deck.pptx' };
			render(ArtifactCard, { artifact: pptxArtifact });
			expect(screen.getByText('PowerPoint')).toBeInTheDocument();
		});

		it('should display the PDF badge for PDF files', () => {
			const pdfArtifact: ArtifactRef = { ...mockArtifact, artifactType: 'Pdf', filename: 'doc.pdf' };
			render(ArtifactCard, { artifact: pdfArtifact });
			expect(screen.getByText('PDF')).toBeInTheDocument();
		});

		it('should display the download button', () => {
			render(ArtifactCard, { artifact: mockArtifact });
			expect(screen.getByRole('button', { name: /download/i })).toBeInTheDocument();
		});

		it('should display metadata title if present', () => {
			render(ArtifactCard, { artifact: mockArtifact });
			expect(screen.getByText('— Test Document')).toBeInTheDocument();
		});

		it('should not display metadata title if absent', () => {
			const artifactNoTitle: ArtifactRef = {
				...mockArtifact,
				metadata: undefined
			};
			render(ArtifactCard, { artifact: artifactNoTitle });
			expect(screen.queryByText(/—/)).not.toBeInTheDocument();
		});
	});

	describe('file size formatting', () => {
		it('should format bytes correctly', () => {
			const smallArtifact: ArtifactRef = { ...mockArtifact, sizeBytes: 0 };
			render(ArtifactCard, { artifact: smallArtifact });
			expect(screen.getByText('0 B')).toBeInTheDocument();
		});

		it('should format kilobytes correctly', () => {
			render(ArtifactCard, { artifact: mockArtifact });
			expect(screen.getByText('24 KB')).toBeInTheDocument();
		});

		it('should format megabytes correctly', () => {
			const largeArtifact: ArtifactRef = { ...mockArtifact, sizeBytes: 2_457_600 };
			render(ArtifactCard, { artifact: largeArtifact });
			expect(screen.getByText('2.34 MB')).toBeInTheDocument();
		});
	});

	describe('download handling', () => {
		it('should call onDownload when button is clicked', async () => {
			const onDownload = vi.fn().mockResolvedValue(undefined);
			const { getByRole } = render(ArtifactCard, {
				artifact: mockArtifact,
				onDownload
			});

			const button = getByRole('button', { name: /download/i });
			button.click();

			await new Promise(resolve => setTimeout(resolve, 0));
			expect(onDownload).toHaveBeenCalledWith(mockArtifact);
		});

		it('should show downloading state during download', async () => {
			let resolveDownload: () => void;
			const onDownload = vi.fn().mockImplementation(
				() =>
					new Promise<void>((resolve) => {
						resolveDownload = resolve;
					})
			);

			render(ArtifactCard, { artifact: mockArtifact, onDownload });

			const button = screen.getByRole('button', { name: /download/i });
			button.click();

			await new Promise(resolve => setTimeout(resolve, 0));
			expect(screen.getByText('Downloading...')).toBeInTheDocument();

			resolveDownload!();
			await new Promise(resolve => setTimeout(resolve, 100));
		});
	});
});
