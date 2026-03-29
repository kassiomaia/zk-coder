<script lang="ts">
  /**
   * ArtifactCard Component
   * Displays a document artifact in the chat area with download capability.
   * Shows icon, filename, size, and download link.
   */
  import styles from './ArtifactCard.module.css';

  /** Artifact types supported */
  export type ArtifactType = 'Docx' | 'Pptx' | 'Pdf';

  /** Artifact reference from the backend */
  export interface ArtifactRef {
    /** Unique artifact ID */
    id: string;
    /** Session ID that created this artifact */
    sessionId: string;
    /** Type of document */
    artifactType: ArtifactType;
    /** Filename with extension */
    filename: string;
    /** When the artifact was created */
    createdAt: string;
    /** File size in bytes */
    sizeBytes: number;
    /** Optional metadata */
    metadata?: ArtifactMetadata;
  }

  /** Additional metadata about the artifact */
  export interface ArtifactMetadata {
    title?: string;
    description?: string;
    generatedBy?: string;
    sourcePrompt?: string;
    template?: string;
    tags?: string[];
  }

  interface Props {
    /** The artifact to display */
    artifact: ArtifactRef;
    /** Optional handler for download click */
    onDownload?: (artifact: ArtifactRef) => void | Promise<void>;
  }

  let { artifact, onDownload }: Props = $props();

  // File type icons (emoji for simplicity, could be SVG icons)
  const icons: Record<ArtifactType, string> = {
    Docx: '📄',
    Pptx: '📊',
    Pdf: '📕'
  };

  // File type labels for display
  const typeLabels: Record<ArtifactType, string> = {
    Docx: 'Word',
    Pptx: 'PowerPoint',
    Pdf: 'PDF'
  };

  // MIME types for download
  const mimeTypes: Record<ArtifactType, string> = {
    Docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    Pptx: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    Pdf: 'application/pdf'
  };

  let isDownloading = $state(false);
  let downloadError = $state<string | null>(null);

  async function handleDownload() {
    if (isDownloading) return;

    isDownloading = true;
    downloadError = null;

    try {
      // Call the download handler if provided
      if (onDownload) {
        await onDownload(artifact);
      } else {
        // Default: invoke Tauri command to download artifact
        const { invoke } = await import('@tauri-apps/api/core');
        const [filename, content] = await invoke('documents_download_artifact', {
          artifactId: artifact.id
        }) as [string, number[]];

        // Create blob and trigger download
        const uint8Array = new Uint8Array(content);
        const blob = new Blob([uint8Array], { type: mimeTypes[artifact.artifactType] });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        link.click();
        URL.revokeObjectURL(url);
      }
    } catch (error) {
      console.error('Download failed:', error);
      downloadError = error instanceof Error ? error.message : 'Download failed';
    } finally {
      isDownloading = false;
    }
  }

  // Format file size for display
  function formatSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${Math.round((bytes / Math.pow(k, i)) * 100) / 100} ${sizes[i]}`;
  }

  // Get icon for artifact type
  function getIcon(type: ArtifactType): string {
    return icons[type] || '📄';
  }

  // Get type label for display
  function getTypeLabel(type: ArtifactType): string {
    return typeLabels[type] || 'Document';
  }

  // Get CSS class for artifact type
  function getTypeClass(type: ArtifactType): string {
    return styles[type.toLowerCase()] || styles.docx;
  }
</script>

<div class={`${styles.artifactCard} ${isDownloading ? styles.downloading : ''}`}>
  <div class={styles.iconWrapper}>
    <span class={styles.icon}>{getIcon(artifact.artifactType)}</span>
  </div>

  <div class={styles.content}>
    <div class={styles.header}>
      <span class={styles.filename}>{artifact.filename}</span>
      <span class={`${styles.badge} ${getTypeClass(artifact.artifactType)}`}>
        {getTypeLabel(artifact.artifactType)}
      </span>
    </div>

    <div class={styles.meta}>
      <span class={styles.size}>{formatSize(artifact.sizeBytes)}</span>
      {#if artifact.metadata?.title}
        <span class={styles.title}>— {artifact.metadata.title}</span>
      {/if}
    </div>

    {#if downloadError}
      <div class={styles.error}>
        {downloadError}
      </div>
    {/if}
  </div>

  <button
    class={styles.downloadButton}
    onclick={handleDownload}
    disabled={isDownloading}
    type="button"
    title="Download {artifact.filename}"
  >
    {#if isDownloading}
      <span class={styles.spinner}></span>
      <span>Downloading...</span>
    {:else}
      <span class={styles.downloadIcon}>↓</span>
      <span>Download</span>
    {/if}
  </button>
</div>
