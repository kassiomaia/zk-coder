//! Header / footer decoration painter.
//!
//! Runs after layout — once total page count is known — and walks every
//! page in `LayoutEngine::layers()` painting the theme-supplied header
//! and footer templates. Token expansion lives here so the renderer
//! itself stays oblivious to the decoration vocabulary.

use printpdf::{IndirectFontRef, Mm, PdfLayerReference};

use crate::core::documents::pdf::theme::Decorations;

/// Resolved per-page values used to expand template tokens.
pub(super) struct PageContext<'a> {
    pub title: &'a str,
    pub page: usize,
    pub total_pages: usize,
    pub section: &'a str,
}

/// Expands `{title}` `{page}` `{total_pages}` `{section}` in `template`.
///
/// Unknown tokens are left literal so theme typos surface in the output
/// rather than being silently dropped.
pub(super) fn expand_template(template: &str, ctx: &PageContext<'_>) -> String {
    let total = ctx.total_pages.to_string();
    let page = ctx.page.to_string();
    template
        .replace("{title}", ctx.title)
        .replace("{total_pages}", &total)
        .replace("{page}", &page)
        .replace("{section}", ctx.section)
}

/// Returns the section title that owns `page` (1-based).
///
/// `section_starts` lists `(starting_page_one_based, title)` in
/// document order. The matching section is the last one whose start
/// page is `<= page`. Empty input yields an empty string.
pub(super) fn section_for_page<'a>(
    section_starts: &'a [(usize, String)],
    page: usize,
) -> &'a str {
    let mut current: &str = "";
    for (start, title) in section_starts {
        if *start <= page {
            current = title.as_str();
        } else {
            break;
        }
    }
    current
}

/// Paints `header` (top) and `footer` (bottom) on `layer` using `font`
/// at the styling specified by `decorations`.
///
/// `page_height_mm` is the actual page height — the function anchors to
/// the page edges, not to the body margins, so headers/footers sit in
/// the gutter rather than competing with content.
pub(super) fn paint_decoration(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    decorations: &Decorations,
    header: Option<&str>,
    footer: Option<&str>,
    x_mm: f32,
    page_height_mm: f32,
) {
    if let Some(text) = header {
        let y_mm = page_height_mm - decorations.margin_mm;
        layer.use_text(text, decorations.font_size_pt, Mm(x_mm), Mm(y_mm), font);
    }
    if let Some(text) = footer {
        let y_mm = decorations.margin_mm;
        layer.use_text(text, decorations.font_size_pt, Mm(x_mm), Mm(y_mm), font);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn ctx<'a>(title: &'a str, page: usize, total: usize, section: &'a str) -> PageContext<'a> {
        PageContext {
            title,
            page,
            total_pages: total,
            section,
        }
    }

    #[test]
    fn expand_replaces_every_known_token() {
        let out = expand_template(
            "{title} — Section {section} — Page {page} / {total_pages}",
            &ctx("Quarterly", 2, 5, "Intro"),
        );
        assert_eq!(out, "Quarterly — Section Intro — Page 2 / 5");
    }

    #[test]
    fn expand_total_pages_resolved_not_left_literal() {
        let out = expand_template("Page {page} of {total_pages}", &ctx("T", 3, 7, ""));
        assert_eq!(out, "Page 3 of 7");
        assert!(!out.contains('{'), "no token braces must survive");
    }

    #[test]
    fn expand_leaves_unknown_tokens_literal() {
        let out = expand_template("hi {title} {date}", &ctx("ok", 1, 1, ""));
        // {date} is not in the vocabulary — must survive as-is so the
        // theme author notices their typo.
        assert_eq!(out, "hi ok {date}");
    }

    #[test]
    fn expand_substitutes_page_before_total_pages() {
        // Regression guard: replacing `{page}` first would clobber the
        // `page` substring inside `{total_pages}` ⇒ result of "Page 12 of 1of 1".
        // The implementation must replace `{total_pages}` before `{page}`.
        let out = expand_template("{page}/{total_pages}", &ctx("", 12, 99, ""));
        assert_eq!(out, "12/99");
    }

    #[test]
    fn section_for_page_picks_most_recent_start_le_target() {
        let starts = vec![
            (1_usize, "Cover".to_string()),
            (2_usize, "Body".to_string()),
            (5_usize, "Appendix".to_string()),
        ];
        assert_eq!(section_for_page(&starts, 1), "Cover");
        assert_eq!(section_for_page(&starts, 2), "Body");
        assert_eq!(section_for_page(&starts, 4), "Body");
        assert_eq!(section_for_page(&starts, 5), "Appendix");
        assert_eq!(section_for_page(&starts, 99), "Appendix");
    }

    #[test]
    fn section_for_page_empty_starts_returns_empty() {
        let starts: Vec<(usize, String)> = vec![];
        assert_eq!(section_for_page(&starts, 1), "");
    }
}
