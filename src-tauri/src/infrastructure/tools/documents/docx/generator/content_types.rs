//! Populate `[Content_Types].xml` with the Default and Override
//! entries the document needs.
//!
//! The crate's `ContentTypes::default()` only covers `.rels`, `.xml`,
//! and the document / styles / app / core parts. We add entries for
//! any media (png / jpeg), numbering, header, and footer parts the
//! plan actually produces.

use std::borrow::Cow;

use docx_rust::content_type::{DefaultContentType, OverrideContentType};
use docx_rust::Docx;

use super::plan::Plan;

const CT_PNG: &str = "image/png";
const CT_JPEG: &str = "image/jpeg";
const CT_NUMBERING: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml";
const CT_HEADER: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";
const CT_FOOTER: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml";

/// Add Default / Override entries to `[Content_Types].xml` for
/// everything the plan introduces.
pub fn populate(docx: &mut Docx<'_>, plan: &Plan) {
    add_defaults(docx, plan);
    add_overrides(docx, plan);
}

fn add_defaults(docx: &mut Docx<'_>, plan: &Plan) {
    let mut need_png = false;
    let mut need_jpeg = false;
    for image in &plan.images {
        if image.filename.ends_with(".png") {
            need_png = true;
        }
        if image.filename.ends_with(".jpeg") || image.filename.ends_with(".jpg") {
            need_jpeg = true;
        }
    }
    if need_png {
        ensure_default(docx, "png", CT_PNG);
    }
    if need_jpeg {
        ensure_default(docx, "jpeg", CT_JPEG);
    }
}

fn add_overrides(docx: &mut Docx<'_>, plan: &Plan) {
    if !plan.numbering.is_empty() {
        ensure_override(docx, "/word/numbering.xml", CT_NUMBERING);
    }
    if let Some(h) = plan.header.as_ref() {
        ensure_override(docx, &format!("/word/{}", h.filename), CT_HEADER);
    }
    if let Some(f) = plan.footer.as_ref() {
        ensure_override(docx, &format!("/word/{}", f.filename), CT_FOOTER);
    }
    if let Some(h) = plan.first_page_header.as_ref() {
        ensure_override(docx, &format!("/word/{}", h.filename), CT_HEADER);
    }
    if let Some(f) = plan.first_page_footer.as_ref() {
        ensure_override(docx, &format!("/word/{}", f.filename), CT_FOOTER);
    }
}

fn ensure_default(docx: &mut Docx<'_>, ext: &str, content_type: &str) {
    if docx.content_types.defaults.iter().any(|d| d.ext == ext) {
        return;
    }
    docx.content_types.defaults.push(DefaultContentType {
        ext: Cow::Owned(ext.to_string()),
        ty: Cow::Owned(content_type.to_string()),
    });
}

fn ensure_override(docx: &mut Docx<'_>, part: &str, content_type: &str) {
    if docx.content_types.overrides.iter().any(|o| o.part == part) {
        return;
    }
    docx.content_types.overrides.push(OverrideContentType {
        part: Cow::Owned(part.to_string()),
        ty: Cow::Owned(content_type.to_string()),
    });
}
