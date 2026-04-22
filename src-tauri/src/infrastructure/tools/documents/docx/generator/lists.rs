//! Build the `word/numbering.xml` part from the planned list set.

use std::borrow::Cow;

use docx_rust::document::{
    AbstractNum, AbstractNumId, Level, LevelStart, LevelText, MultiLevelType, Num, NumFmt,
    Numbering,
};
use docx_rust::Docx;

use super::plan::PlannedNumbering;

/// Attach the numbering part to the Docx. Callers gate on
/// `!numberings.is_empty()` so the crate skips writing the part
/// entirely when no lists appear in the document.
pub fn install_numbering(docx: &mut Docx<'_>, numberings: &[PlannedNumbering]) {
    let mut numbering = Numbering::default();
    for planned in numberings {
        numbering.abstract_numberings.push(abstract_def(planned));
        numbering.numberings.push(num_def(planned));
    }
    docx.numbering = Some(numbering);
}

fn abstract_def(planned: &PlannedNumbering) -> AbstractNum<'static> {
    let (fmt, text) = if planned.bullet {
        ("bullet", "\u{2022}")
    } else {
        ("decimal", "%1.")
    };
    let levels = (0..9)
        .map(|lvl| level_def(planned, lvl, fmt, text))
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
        p_pr: None,
        r_pr: Vec::new(),
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
