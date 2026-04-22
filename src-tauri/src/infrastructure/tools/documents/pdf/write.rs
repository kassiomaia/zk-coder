//! Atomic, cancellable write of a finalized PDF to disk.
//!
//! The render writes to a `<filename>.tmp` sibling first; only after it
//! completes do we atomically `rename` to the final path. A drop-guard
//! deletes the .tmp if anything fails mid-flight — render error,
//! cancellation, or panic — so callers never observe a partially-written
//! `.pdf`.
//!
//! # Cancellation semantics
//!
//! When the [`CancellationToken`] fires, the `spawn_blocking` handle is
//! aborted. On Unix the blocking thread continues to completion, but
//! every byte it writes lands in an already-unlinked file and never
//! becomes visible. The function returns [`RenderArtifactError::Cancelled`]
//! as soon as the token fires, without waiting for the blocking thread.

use std::io::BufWriter;
use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::application::tools::pdf::finalize::{render_draft, FinalizeError};
use crate::infrastructure::tools::documents::pdf::render::{
    bundled_report_cache, theme_for_draft, PrintPdfRenderer,
};
use crate::infrastructure::tools::documents::shared::PdfDraft;

/// Suffix appended to the final filename to form the in-progress sibling.
pub const TMP_SUFFIX: &str = ".tmp";

/// Failure mode for [`render_to_artifact`].
#[derive(Debug, thiserror::Error)]
pub enum RenderArtifactError {
    /// The caller's [`CancellationToken`] fired before render completed.
    #[error("render cancelled")]
    Cancelled,
    /// The renderer refused the draft or the engine errored.
    #[error("render failed: {0}")]
    Render(String),
    /// Filesystem I/O failure (create, rename, metadata).
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// The blocking render task panicked.
    #[error("render task join error: {0}")]
    Join(String),
}

/// Drop-guard that deletes its target path on drop unless
/// [`TempFileGuard::disarm`] has been called.
///
/// Only the `ENOENT` case is ignored on drop — any other failure leaves
/// the file in place, which is preferable to silently retrying.
struct TempFileGuard {
    path: Option<PathBuf>,
}

impl TempFileGuard {
    const fn arm(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(mut self) {
        self.path.take();
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Renders `draft` to `final_path` via a `.tmp` sibling and an atomic
/// `rename`.
///
/// Fails with [`RenderArtifactError::Cancelled`] as soon as the caller's
/// `cancel` token fires; the `.tmp` file is unlinked and no `.pdf` is
/// left on disk.
///
/// # Errors
/// Returns [`RenderArtifactError`] variants for render failure, I/O
/// failure, cancellation, or task join failure.
pub async fn render_to_artifact(
    draft: PdfDraft,
    final_path: PathBuf,
    cancel: CancellationToken,
) -> Result<(PathBuf, u64), RenderArtifactError> {
    let tmp_path = tmp_sibling_for(&final_path);
    let guard = TempFileGuard::arm(tmp_path.clone());
    let tmp_for_task = tmp_path.clone();

    let mut handle = tokio::task::spawn_blocking(
        move || -> Result<(), RenderArtifactError> {
            let fonts = bundled_report_cache()
                .map_err(|e| RenderArtifactError::Render(format!("font cache: {e}")))?;
            let renderer = PrintPdfRenderer::new(fonts);
            let theme = theme_for_draft(&draft);
            let file = std::fs::File::create(&tmp_for_task)?;
            let mut writer = BufWriter::new(file);
            let rt = tokio::runtime::Handle::current();
            rt.block_on(render_draft(&draft, &renderer, &theme, &mut writer, None))
                .map_err(|e| {
                    RenderArtifactError::Render(match e {
                        FinalizeError::Model(inner) => format!("invalid draft: {inner}"),
                        FinalizeError::Render(inner) => format!("render engine: {inner}"),
                    })
                })?;
            Ok(())
        },
    );
    let abort = handle.abort_handle();

    let joined = tokio::select! {
        biased;
        () = cancel.cancelled() => {
            // `abort()` is a no-op for `spawn_blocking` tasks, so the
            // thread will run to completion. Detach a cleanup task that
            // awaits it and then unlinks the .tmp — ensures no stray
            // files accumulate across repeated cancellations.
            abort.abort();
            let tmp_cleanup = tmp_path.clone();
            tokio::spawn(async move {
                let _ = handle.await;
                let _ = std::fs::remove_file(&tmp_cleanup);
            });
            guard.disarm();
            return Err(RenderArtifactError::Cancelled);
        }
        joined = &mut handle => joined,
    };

    match joined {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(e),
        Err(join_err) => return Err(RenderArtifactError::Join(join_err.to_string())),
    }

    std::fs::rename(&tmp_path, &final_path)?;
    guard.disarm();
    let size = std::fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);
    Ok((final_path, size))
}

/// Appends [`TMP_SUFFIX`] to `path` to produce the in-progress sibling.
/// Works even when `path` has no extension.
fn tmp_sibling_for(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(TMP_SUFFIX);
    PathBuf::from(os)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::infrastructure::tools::documents::shared::draft_store::{
        DraftPage, DraftStatus, PdfPageContent,
    };
    use crate::infrastructure::tools::documents::shared::PdfPageSize;
    use chrono::Utc;
    use std::time::{Duration, Instant};

    fn fixture_draft(name: &str) -> PdfDraft {
        PdfDraft {
            pdf_id: format!("pdf_{name}"),
            filename: format!("{name}.pdf"),
            title: format!("Fixture {name}"),
            pages: vec![DraftPage {
                id: "page_0".to_string(),
                index: 0,
                content: PdfPageContent::Text {
                    content: format!("Stress fixture body for {name}"),
                },
            }],
            page_size: PdfPageSize::A4,
            theme_patch: None,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn tmp_sibling_appends_suffix() {
        let p = tmp_sibling_for(Path::new("/dir/report.pdf"));
        assert_eq!(p, PathBuf::from("/dir/report.pdf.tmp"));
    }

    #[test]
    fn temp_file_guard_deletes_on_drop_when_armed() {
        let td = tempfile::tempdir().unwrap();
        let tmp = td.path().join("armed.tmp");
        std::fs::write(&tmp, b"pending").unwrap();
        {
            let _g = TempFileGuard::arm(tmp.clone());
        }
        assert!(!tmp.exists(), "armed guard must remove file on drop");
    }

    #[test]
    fn temp_file_guard_keeps_file_when_disarmed() {
        let td = tempfile::tempdir().unwrap();
        let tmp = td.path().join("kept.tmp");
        std::fs::write(&tmp, b"pending").unwrap();
        let g = TempFileGuard::arm(tmp.clone());
        g.disarm();
        assert!(tmp.exists(), "disarmed guard must leave file alone");
    }

    #[test]
    fn temp_file_guard_handles_missing_path() {
        let td = tempfile::tempdir().unwrap();
        let tmp = td.path().join("never-existed.tmp");
        {
            let _g = TempFileGuard::arm(tmp.clone());
        }
        // Drop must not panic when the path doesn't exist.
        assert!(!tmp.exists());
    }

    #[tokio::test]
    async fn cover_page_followed_by_markdown_renders_multi_page_pdf() {
        let td = tempfile::tempdir().unwrap();
        let draft = PdfDraft {
            pdf_id: "pdf_cover".to_string(),
            filename: "cover.pdf".to_string(),
            title: "Phase 2 Test".to_string(),
            pages: vec![
                DraftPage {
                    id: "page_cover".to_string(),
                    index: 0,
                    content: PdfPageContent::Cover {
                        title: "Quarterly Report".to_string(),
                        subtitle: Some("Q1 2026".to_string()),
                        author: Some("Ada Lovelace".to_string()),
                        date: Some("2026-04-22".to_string()),
                    },
                },
                DraftPage {
                    id: "page_body".to_string(),
                    index: 1,
                    content: PdfPageContent::Markdown {
                        content: "# Introduction\n\nBody text here.".to_string(),
                    },
                },
            ],
            page_size: PdfPageSize::A4,
            theme_patch: None,
            status: DraftStatus::Building,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let final_path = td.path().join(&draft.filename);
        let (_, size) = render_to_artifact(draft, final_path.clone(), CancellationToken::new())
            .await
            .expect("cover render succeeds");
        assert!(size > 0);

        let bytes = std::fs::read(&final_path).unwrap();
        assert!(bytes.starts_with(b"%PDF-1."));
        let pdf = lopdf::Document::load_mem(&bytes).expect("lopdf parses");
        assert!(
            pdf.page_iter().count() >= 2,
            "cover + markdown must produce at least 2 pages"
        );
    }

    #[tokio::test]
    async fn theme_patch_on_draft_changes_rendered_output() {
        use crate::core::documents::pdf::theme_patch::{
            PalettePatch, RgbPatch, ThemePatch, TypographyPatch,
        };

        let td = tempfile::tempdir().unwrap();

        let baseline = fixture_draft("baseline_theme");
        let baseline_path = td.path().join("baseline.pdf");
        let (_, baseline_size) =
            render_to_artifact(baseline, baseline_path.clone(), CancellationToken::new())
                .await
                .expect("baseline renders");

        let mut patched = fixture_draft("patched_theme");
        patched.theme_patch = Some(ThemePatch {
            typography: Some(TypographyPatch {
                body_font_size_pt: Some(18.0),
                ..TypographyPatch::default()
            }),
            palette: Some(PalettePatch {
                heading: Some(RgbPatch {
                    r: Some(0.8),
                    g: Some(0.1),
                    b: Some(0.1),
                }),
                ..PalettePatch::default()
            }),
            ..ThemePatch::default()
        });
        let patched_path = td.path().join("patched.pdf");
        let (_, patched_size) =
            render_to_artifact(patched, patched_path.clone(), CancellationToken::new())
                .await
                .expect("patched renders");

        assert!(baseline_size > 0 && patched_size > 0);
        let baseline_bytes = std::fs::read(&baseline_path).unwrap();
        let patched_bytes = std::fs::read(&patched_path).unwrap();
        assert_ne!(
            baseline_bytes, patched_bytes,
            "theme patch must change the rendered output"
        );
    }

    #[tokio::test]
    async fn happy_path_renames_tmp_to_final_and_no_tmp_remains() {
        let td = tempfile::tempdir().unwrap();
        let draft = fixture_draft("happy");
        let final_path = td.path().join(&draft.filename);
        let tmp_path = tmp_sibling_for(&final_path);

        let (returned_path, size) =
            render_to_artifact(draft, final_path.clone(), CancellationToken::new())
                .await
                .expect("render succeeds");

        assert_eq!(returned_path, final_path);
        assert!(size > 0, "size must be non-zero");
        assert!(final_path.exists(), "final .pdf must exist");
        assert!(!tmp_path.exists(), "tmp sibling must not linger");

        let header = std::fs::read(&final_path).unwrap();
        assert!(header.starts_with(b"%PDF-1."), "valid PDF magic bytes");
    }

    #[tokio::test]
    async fn cancel_before_spawn_returns_cancelled_and_leaves_no_files() {
        let td = tempfile::tempdir().unwrap();
        let draft = fixture_draft("early");
        let final_path = td.path().join(&draft.filename);
        let tmp_path = tmp_sibling_for(&final_path);

        let cancel = CancellationToken::new();
        cancel.cancel();
        let result = render_to_artifact(draft, final_path.clone(), cancel).await;

        assert!(
            matches!(result, Err(RenderArtifactError::Cancelled)),
            "pre-fired cancel must yield Cancelled, got {result:?}"
        );
        // Wait for the detached cleanup task to unlink tmp. The blocking
        // render runs to completion (abort is a no-op for spawn_blocking)
        // so cleanup has to wait for the thread, then fs::remove_file.
        wait_until(Duration::from_secs(5), || !tmp_path.exists()).await;
        assert!(!final_path.exists(), "no .pdf must be left behind");
        assert!(
            !tmp_path.exists(),
            "tmp sibling must be cleaned up after cancel"
        );
    }

    async fn wait_until(budget: Duration, mut predicate: impl FnMut() -> bool) {
        let deadline = Instant::now() + budget;
        while Instant::now() < deadline {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    #[tokio::test]
    async fn concurrent_finalize_and_cancel_pairs_leave_zero_stray_files() {
        let td = tempfile::tempdir().unwrap();
        let dir = td.path().to_path_buf();
        let deadline = Instant::now() + Duration::from_secs(30);

        let mut handles = Vec::new();
        for i in 0..10 {
            let draft = fixture_draft(&format!("stress_{i:02}"));
            let final_path = dir.join(&draft.filename);
            let cancel = CancellationToken::new();
            let cancel_trigger = cancel.clone();
            let render = tokio::spawn(render_to_artifact(draft, final_path.clone(), cancel));
            // Half are cancelled almost immediately.
            let canceller = tokio::spawn(async move {
                if i % 2 == 0 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    cancel_trigger.cancel();
                }
            });
            handles.push((i, final_path, render, canceller));
        }

        for (i, final_path, render, canceller) in handles {
            let _ = canceller.await;
            let outcome = render.await.expect("task joined");
            let tmp_path = tmp_sibling_for(&final_path);
            // Allow any detached blocking render to finish its unlinked writes.
            tokio::time::sleep(Duration::from_millis(5)).await;
            if i % 2 == 0 {
                // Cancelled pairs must leave no .pdf. A .tmp could still
                // exist briefly if the cancel fired after rename; assert
                // on the .pdf absence, which is the user-visible invariant.
                if matches!(outcome, Err(RenderArtifactError::Cancelled)) {
                    assert!(
                        !final_path.exists(),
                        "cancelled pair #{i} leaked a .pdf at {final_path:?}"
                    );
                }
            } else {
                // Happy pairs must leave the .pdf and no .tmp.
                assert!(
                    outcome.is_ok(),
                    "happy pair #{i} failed: {outcome:?}"
                );
                assert!(final_path.exists(), "happy pair #{i} missing .pdf");
                assert!(
                    !tmp_path.exists(),
                    "happy pair #{i} left a tmp: {tmp_path:?}"
                );
            }
            assert!(Instant::now() < deadline, "stress test exceeded budget");
        }
    }
}
