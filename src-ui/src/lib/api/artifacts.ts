/**
 * Artifact / document API client.
 *
 * Typed wrappers for Tauri IPC commands that manage session artifacts
 * (generated Docx / Pptx / Pdf documents). Components should use these
 * instead of calling `invoke()` directly.
 */

import { invokeCommand } from './core';

export { ArtifactApiError } from './errors';

/**
 * Download an artifact as an in-memory byte buffer.
 *
 * The backend command returns a `(filename, bytes)` tuple. The bytes come
 * back as a plain `number[]` over IPC; the caller constructs the
 * `Uint8Array` / `Blob` locally to keep the buffer type well-defined for
 * `Blob`'s `BlobPart` parameter.
 */
export async function downloadArtifact(
	artifactId: string
): Promise<{ filename: string; content: number[] }> {
	const [filename, content] = await invokeCommand<[string, number[]]>(
		'artifact',
		'documents_download_artifact',
		{ artifactId }
	);
	return { filename, content };
}

/**
 * Save an artifact directly to a user-chosen filesystem path.
 */
export async function saveArtifactToPath(artifactId: string, destination: string): Promise<void> {
	await invokeCommand<void>('artifact', 'save_artifact_to_path', {
		artifactId,
		destination
	});
}
