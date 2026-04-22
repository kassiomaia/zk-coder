//! Build the `<w:drawing>` block for an embedded image and register
//! the picture bytes with the Docx's media map.

use std::borrow::Cow;

use docx_rust::document::{
    Blip, BlipFill, CNvPicPr, CNvPr, DocPr, Drawing, Extent, Graphic, GraphicData, Inline, NvPicPr,
    ParagraphContent, Picture, PrstGeom, Run, RunContent, SpPr, Stretch, Text, TextSpace,
};
use docx_rust::formatting::{CharacterProperty, Italics, Size};
use docx_rust::media::MediaType;
use docx_rust::Docx;

use crate::core::documents::docx::theme::DocxTheme;

use super::plan::PlannedImage;
use super::units::{mm_to_emu, pt_to_half_pt};

/// Install every planned image's bytes on the Docx media map.
///
/// `docx_rust::Docx::media` expects a `&Vec<u8>` with a lifetime
/// matching the Docx type parameter. The crate has no way to hand
/// ownership of the bytes to the Docx, so we leak a boxed clone to
/// obtain a `&'static Vec<u8>`. For a short-lived tool invocation
/// that generates a handful of documents per request, the leaked
/// bytes are reclaimed when the process exits; for the interactive
/// agent this is a bounded trickle rather than unbounded growth.
pub fn install_media(docx: &mut Docx<'_>, images: &[PlannedImage]) {
    for image in images {
        let leaked: &'static Vec<u8> = Box::leak(Box::new(image.bytes.clone()));
        docx.media
            .insert(image.filename.clone(), (MediaType::Image, leaked));
    }
}

/// Build the paragraph that carries the image and its caption.
#[must_use]
pub fn paragraph(
    planned: &PlannedImage,
    caption: Option<&str>,
    index: i32,
    theme: &DocxTheme,
) -> Vec<docx_rust::document::Paragraph<'static>> {
    let (width_emu, height_emu) = sized_emu(planned);
    let drawing = build_drawing(planned, index, width_emu, height_emu);
    let run = Run {
        property: None,
        content: vec![RunContent::Drawing(drawing)],
        ..Run::default()
    };
    let image_paragraph = docx_rust::document::Paragraph {
        property: None,
        content: vec![ParagraphContent::Run(run)],
        ..docx_rust::document::Paragraph::default()
    };
    let mut out = vec![image_paragraph];
    if let Some(text) = caption {
        out.push(caption_paragraph(text, theme));
    }
    out
}

fn sized_emu(planned: &PlannedImage) -> (u64, u64) {
    let width_emu = mm_to_emu(planned.width_mm);
    let aspect = f64::from(planned.height_px) / f64::from(planned.width_px.max(1));
    let height_emu = (width_emu as f64 * aspect).round() as u64;
    (width_emu.max(1), height_emu.max(1))
}

fn build_drawing(
    planned: &PlannedImage,
    index: i32,
    width_emu: u64,
    height_emu: u64,
) -> Drawing<'static> {
    Drawing {
        anchor: None,
        inline: Some(Inline {
            dist_t: None,
            dist_b: None,
            dist_l: None,
            dist_r: None,
            simple_pos_attr: None,
            relative_height: None,
            behind_doc: None,
            locked: None,
            layout_in_cell: None,
            allow_overlap: None,
            simple_pos: None,
            position_horizontal: None,
            position_vertical: None,
            extent: Some(Extent {
                cx: width_emu,
                cy: height_emu,
            }),
            doc_property: DocPr {
                id: Some((index + 1) as isize),
                name: Some(Cow::Owned(format!("Picture {}", index + 1))),
                descr: None,
            },
            graphic: Some(Graphic {
                a: Cow::Borrowed(
                    "http://schemas.openxmlformats.org/drawingml/2006/main",
                ),
                data: GraphicData {
                    uri: Cow::Borrowed(
                        "http://schemas.openxmlformats.org/drawingml/2006/picture",
                    ),
                    children: vec![Picture {
                        a: Cow::Borrowed(
                            "http://schemas.openxmlformats.org/drawingml/2006/picture",
                        ),
                        nv_pic_pr: NvPicPr {
                            c_nv_pr: Some(CNvPr {
                                id: Some(0),
                                name: Some(Cow::Owned(format!("Picture {}", index + 1))),
                                descr: None,
                            }),
                            c_nv_pic_pr: Some(CNvPicPr {}),
                        },
                        fill: BlipFill {
                            blip: Blip {
                                embed: Cow::Owned(planned.rel_id.clone()),
                                cstate: None,
                            },
                            stretch: Some(Stretch {
                                fill_rect: None,
                            }),
                        },
                        sp_pr: SpPr {
                            xfrm: None,
                            prst_geom: Some(PrstGeom {
                                prst: Some(Cow::Borrowed("rect")),
                                av_lst: None,
                            }),
                        },
                    }],
                },
            }),
        }),
    }
}

fn caption_paragraph(text: &str, theme: &DocxTheme) -> docx_rust::document::Paragraph<'static> {
    let mut props = CharacterProperty::default();
    props.italics = Some(Italics { value: Some(true) });
    props.size = Some(Size::from(
        pt_to_half_pt(theme.typography.body_font_size_pt.saturating_sub(1).max(6)) as isize,
    ));
    let run = Run {
        property: Some(props),
        content: vec![RunContent::Text(Text::from((
            format!("Figure: {text}"),
            TextSpace::Preserve,
        )))],
        ..Run::default()
    };
    docx_rust::document::Paragraph {
        property: None,
        content: vec![ParagraphContent::Run(run)],
        ..docx_rust::document::Paragraph::default()
    }
}
