//! Integration tests for per-turn file-mutation verifier footers.

use edgecrab_tools::{
    MutationKind, MutationRecord, MutationTurnState, extract_file_mutation_targets,
    file_mutation_result_landed, render_success_footer,
};
use serde_json::json;

#[test]
fn read_only_turn_produces_no_success_footer() {
    assert!(render_success_footer(&[]).is_none());
}

#[test]
fn write_file_landed_detection() {
    assert!(file_mutation_result_landed(
        "write_file",
        r#"{"ok":true,"bytes":3,"lines":1}"#
    ));
    assert!(!file_mutation_result_landed(
        "write_file",
        r#"{"error":"collision"}"#
    ));
}

#[test]
fn failure_superseded_by_success_in_same_turn() {
    let turn = MutationTurnState::new();
    let args = json!({"path": "src/lib.rs", "old_string": "a", "new_string": "b"});
    turn.record_tool_outcome("patch", &args, r#"{"error":"not found"}"#, true);
    assert_eq!(turn.take_failed().len(), 1);
    turn.record_tool_outcome(
        "patch",
        &args,
        r#"{"ok":true,"before_lines":1,"after_lines":2}"#,
        false,
    );
    assert!(turn.take_failed().is_empty());
}

#[test]
fn turn_footer_combines_success_and_failure_sections() {
    let turn = MutationTurnState::new();
    turn.push_success(MutationRecord {
        path: "ok.rs".into(),
        kind: MutationKind::Add,
        lines_added: 5,
        lines_removed: 0,
    });
    let args = json!({"path": "bad.rs", "content": "x"});
    turn.record_tool_outcome("write_file", &args, r#"{"error":"denied"}"#, true);
    let footer = turn.render_turn_footer();
    assert!(footer.contains("files-mutated"));
    assert!(footer.contains("NOT modified"));
    assert!(footer.contains("ok.rs"));
    assert!(footer.contains("bad.rs"));
}

#[test]
fn patch_v4a_targets_match_hermes_shape() {
    let body = "*** Begin Patch\n*** Update File: a.rs\n*** Delete File: b.rs\n*** End Patch\n";
    let paths = extract_file_mutation_targets("patch", &json!({"mode": "patch", "patch": body}));
    assert_eq!(paths, vec!["a.rs", "b.rs"]);
}
