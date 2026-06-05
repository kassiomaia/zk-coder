//! Build the `word/numbering.xml` part from the planned list set.

use std::borrow::Cow;

use docx_rust::document::{
    AbstractNum, AbstractNumId, Level, LevelStart, LevelText, MultiLevelType, Num, NumFmt,
    Numbering, PPr,
};
use docx_rust::formatting::Indent;
use docx_rust::Docx;

use crate::core::documents::docx::theme::DocxTheme;

use super::plan::PlannedNumbering;
use super::units::pt_to_twips;

/// Attach the numbering part to the Docx. Callers gate on
/// `!numberings.is_empty()` so the crate skips writing the part
/// entirely when no lists appear in the document.
pub fn install_numbering(docx: &mut Docx<'_>, numberings: &[PlannedNumbering], theme: &DocxTheme) {
    let mut numbering = Numbering::default();
    for planned in numberings {
        numbering
            .abstract_numberings
            .push(abstract_def(planned, theme));
        numbering.numberings.push(num_def(planned));
    }
    docx.numbering = Some(numbering);
}

fn abstract_def(planned: &PlannedNumbering, theme: &DocxTheme) -> AbstractNum<'static> {
    let (fmt, text) = if planned.bullet {
        ("bullet", "\u{2022}")
    } else {
        ("decimal", "%1.")
    };
    let levels = (0..9)
        .map(|lvl| level_def(planned, lvl, fmt, text, theme))
        .collect();
    AbstractNum {
        abstract_num_id: Some(planned.abstract_id as isize),
        nsid: None,
        multi_level_type: Some(MultiLevelType {
            value: Cow::Borrowed("hybridMultilevel"),
        }),
        levels,
    }
}

fn level_def(
    planned: &PlannedNumbering,
    level: i32,
    fmt: &'static str,
    text_template: &'static str,
    theme: &DocxTheme,
) -> Level<'static> {
    let start = if planned.bullet {
        1
    } else {
        i32::try_from(planned.start_at).unwrap_or(1)
    };
    let text = if planned.bullet {
        text_template.to_string()
    } else {
        text_template.replace("%1", &format!("%{}", level + 1))
    };
    Level {
        i_level: Some(level as isize),
        start: Some(LevelStart {
            value: Some(start as isize),
        }),
        number_format: Some(NumFmt {
            value: Cow::Borrowed(fmt),
        }),
        level_text: Some(LevelText {
            value: Some(Cow::Owned(text)),
        }),
        justification: None,
        p_pr: Some(level_indent(level, theme)),
        r_pr: Vec::new(),
    }
}

/// Hanging-indent setup: `left` grows with depth so each level is
/// visibly nested, and `hanging` equals one indent unit so the
/// marker sits to the left of wrapped text.
fn level_indent(level: i32, theme: &DocxTheme) -> PPr {
    let indent_twips = pt_to_twips(theme.spacing.list_indent_pt) as isize;
    let left = indent_twips * ((level + 1) as isize);
    PPr {
        indent: Some(Indent {
            left: Some(left),
            hanging: Some(indent_twips),
            ..Indent::default()
        }),
    }
}

fn num_def(planned: &PlannedNumbering) -> Num {
    Num {
        num_id: Some(planned.num_id as isize),
        abstract_num_id: Some(AbstractNumId {
            value: Some(planned.abstract_id as isize),
        }),
        level_overrides: Vec::new(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn each_level_carries_indent_scaled_by_depth() {
        let theme = DocxTheme::report_default();
        let planned = PlannedNumbering {
            num_id: 1,
            abstract_id: 0,
            bullet: true,
            start_at: 1,
        };
        let abs = abstract_def(&planned, &theme);
        let step = pt_to_twips(theme.spacing.list_indent_pt) as isize;
        for (i, level) in abs.levels.iter().enumerate() {
            let p_pr = level.p_pr.as_ref().expect("level must carry p_pr");
            let indent = p_pr.indent.as_ref().expect("p_pr must carry indent");
            assert_eq!(
                indent.left,
                Some(step * (i as isize + 1)),
                "level {i} left indent does not scale with list_indent_pt",
            );
            assert_eq!(indent.hanging, Some(step));
        }
    }

    #[test]
    fn overridden_list_indent_flows_through() {
        let mut theme = DocxTheme::report_default();
        theme.spacing.list_indent_pt = 36;
        let planned = PlannedNumbering {
            num_id: 1,
            abstract_id: 0,
            bullet: false,
            start_at: 1,
        };
        let abs = abstract_def(&planned, &theme);
        let first = abs
            .levels
            .first()
            .unwrap()
            .p_pr
            .as_ref()
            .unwrap()
            .indent
            .as_ref()
            .unwrap();
        assert_eq!(first.left, Some(pt_to_twips(36) as isize));
    }
}
