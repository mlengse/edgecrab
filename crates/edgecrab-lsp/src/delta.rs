//! Delta baseline filtering for post-write LSP diagnostics (Hermes parity).

use std::collections::HashSet;

use lsp_types::Diagnostic;

use crate::range_shift::{LineShift, shift_baseline};

/// Content-equality key for cross-edit delta filtering (mirrors Hermes `_diag_key`).
pub fn diagnostic_key(d: &Diagnostic) -> String {
    let severity = d
        .severity
        .map(|s| match s {
            lsp_types::DiagnosticSeverity::ERROR => 1,
            lsp_types::DiagnosticSeverity::WARNING => 2,
            lsp_types::DiagnosticSeverity::INFORMATION => 3,
            lsp_types::DiagnosticSeverity::HINT => 4,
            _ => 1,
        })
        .unwrap_or(1);
    let code = d
        .code
        .as_ref()
        .map(|c| match c {
            lsp_types::NumberOrString::Number(n) => n.to_string(),
            lsp_types::NumberOrString::String(s) => s.clone(),
        })
        .unwrap_or_default();
    let source = d.source.clone().unwrap_or_default();
    let message = d.message.trim();
    format!(
        "{severity}\0{code}\0{source}\0{message}\0{}:{}-{}:{}",
        d.range.start.line,
        d.range.start.character,
        d.range.end.line,
        d.range.end.character
    )
}

/// Return diagnostics present in `current` but not in `baseline` (after optional line shift).
pub fn filter_introduced_diagnostics(
    current: Vec<Diagnostic>,
    baseline: &[Diagnostic],
    line_shift: Option<&LineShift>,
) -> Vec<Diagnostic> {
    if baseline.is_empty() {
        return current;
    }
    let shifted = if let Some(shift) = line_shift {
        shift_baseline(baseline, shift)
    } else {
        baseline.to_vec()
    };
    let seen: HashSet<String> = shifted.iter().map(diagnostic_key).collect();
    current
        .into_iter()
        .filter(|d| !seen.contains(&diagnostic_key(d)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{DiagnosticSeverity, NumberOrString, Position, Range};

    fn diag(line: u32, msg: &str) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position {
                    line,
                    character: 0,
                },
                end: Position {
                    line,
                    character: 1,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("E".into())),
            message: msg.into(),
            ..Default::default()
        }
    }

    #[test]
    fn filters_unchanged_diagnostics() {
        let baseline = vec![diag(0, "existing")];
        let current = vec![diag(0, "existing"), diag(1, "new")];
        let out = filter_introduced_diagnostics(current, &baseline, None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].message, "new");
    }
}
