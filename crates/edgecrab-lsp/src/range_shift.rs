//! Diff-aware line-shift map for cross-edit LSP delta filtering (Hermes parity).
//!
//! Port of `hermes-agent/agent/lsp/range_shift.py`.

use lsp_types::{Diagnostic, Position, Range};

/// Maps pre-edit 0-indexed line numbers to post-edit lines, or `None` if deleted.
pub type LineShift = Box<dyn Fn(u32) -> Option<u32> + Send + Sync>;

fn identity_line_shift(line: u32) -> Option<u32> {
    Some(line)
}

/// Build a line-shift closure from pre/post file text using `similar` opcodes.
pub fn build_line_shift(pre_text: &str, post_text: &str) -> LineShift {
    if pre_text == post_text {
        return Box::new(identity_line_shift);
    }

    let diff = similar::TextDiff::from_lines(pre_text, post_text);
    let opcodes: Vec<(similar::DiffTag, usize, usize, usize, usize)> = diff
        .ops()
        .iter()
        .map(|op| match *op {
            similar::DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => (
                similar::DiffTag::Equal,
                old_index,
                old_index + len,
                new_index,
                new_index + len,
            ),
            similar::DiffOp::Delete {
                old_index,
                old_len,
                new_index,
            } => (
                similar::DiffTag::Delete,
                old_index,
                old_index + old_len,
                new_index,
                new_index,
            ),
            similar::DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => (
                similar::DiffTag::Insert,
                old_index,
                old_index,
                new_index,
                new_index + new_len,
            ),
            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => (
                similar::DiffTag::Replace,
                old_index,
                old_index + old_len,
                new_index,
                new_index + new_len,
            ),
        })
        .collect();

    let post_line_count = post_text.lines().count();

    Box::new(move |line| {
        let line = line as usize;
        for (tag, i1, i2, j1, _j2) in &opcodes {
            if *i1 <= line && line < *i2 {
                return match tag {
                    similar::DiffTag::Equal => Some((line - i1 + j1) as u32),
                    similar::DiffTag::Delete | similar::DiffTag::Replace => None,
                    similar::DiffTag::Insert => None,
                };
            }
            if line < *i1 {
                break;
            }
        }
        if post_line_count == 0 {
            None
        } else {
            Some(post_line_count.saturating_sub(1) as u32)
        }
    })
}

fn shift_diagnostic_range(diag: &Diagnostic, shift: &LineShift) -> Option<Diagnostic> {
    let pre_start = diag.range.start.line;
    let pre_end = diag.range.end.line;

    let new_start = shift(pre_start)?;
    let mut new_end = shift(pre_end).unwrap_or(new_start);

    if new_end < new_start {
        new_end = new_start;
    }

    Some(Diagnostic {
        range: Range {
            start: Position {
                line: new_start,
                character: diag.range.start.character,
            },
            end: Position {
                line: new_end,
                character: diag.range.end.character,
            },
        },
        ..diag.clone()
    })
}

/// Remap baseline diagnostics into post-edit coordinates; drop deleted lines.
pub fn shift_baseline(baseline: &[Diagnostic], shift: &LineShift) -> Vec<Diagnostic> {
    baseline
        .iter()
        .filter_map(|d| shift_diagnostic_range(d, shift))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{DiagnosticSeverity, NumberOrString};

    fn diag(line: u32, message: &str) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position { line, character: 0 },
                end: Position { line, character: 1 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("E1".into())),
            source: Some("test".into()),
            message: message.into(),
            ..Default::default()
        }
    }

    #[test]
    fn identical_content_is_identity_shift() {
        let shift = build_line_shift("a\nb\n", "a\nb\n");
        assert_eq!(shift(1), Some(1));
    }

    #[test]
    fn deleted_line_maps_to_none() {
        let pre = "line0\nline1\nline2\n";
        let post = "line0\nline2\n";
        let shift = build_line_shift(pre, post);
        assert_eq!(shift(0), Some(0));
        assert_eq!(shift(1), None);
        assert_eq!(shift(2), Some(1));
    }

    #[test]
    fn shift_baseline_drops_deleted_diagnostics() {
        let pre = "a\nb\nc\n";
        let post = "a\nc\n";
        let shift = build_line_shift(pre, post);
        let baseline = vec![diag(1, "on deleted line")];
        assert!(shift_baseline(&baseline, &shift).is_empty());
    }
}
