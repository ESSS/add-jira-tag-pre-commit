use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper function to run the hook with a mocked branch name
fn run_hook(commit_msg: &str, branch_name: &str) -> (String, String, String, i32) {
    let temp_dir = TempDir::new().unwrap();
    let msg_file = temp_dir.path().join("COMMIT_EDITMSG");
    fs::write(&msg_file, commit_msg).unwrap();

    // Build the binary path
    let mut binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    binary_path.push("target");
    binary_path.push(if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    });
    binary_path.push(if cfg!(windows) {
        "add-jira-tag.exe"
    } else {
        "add-jira-tag"
    });

    let output = Command::new(&binary_path)
        .arg(&msg_file)
        .env("TEST_BRANCH_NAME", branch_name)
        .output()
        .expect("Failed to execute binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let result_msg = fs::read_to_string(&msg_file).unwrap();
    let code = output.status.code().unwrap_or(-1);

    (stdout, stderr, result_msg, code)
}

#[test]
fn test_fresh_commit_adds_tag_to_title() {
    let (stdout, stderr, result_msg, code) = run_hook("Add new feature\n", "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, "[PROJ-123] Add new feature\n");
    assert!(stdout.contains("Adding tag"));
}

#[test]
fn test_fresh_commit_with_body_adds_tag_to_title() {
    let input_msg = "Add new feature\n\nThis is the description.\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(
        result_msg,
        "[PROJ-123] Add new feature\n\nThis is the description.\n"
    );
}

#[test]
fn test_old_format_converts_to_new() {
    let input_msg = "Add new feature\n\nSome description\n\nPROJ-123\n";
    let (stdout, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert!(result_msg.starts_with("[PROJ-123] Add new feature"));
    let parts: Vec<&str> = result_msg.splitn(2, '\n').collect();
    if parts.len() > 1 {
        assert!(!parts[1].contains("PROJ-123\n"));
    }
    assert!(stdout.contains("Converting old format tag"));
}

#[test]
fn test_old_format_with_trailing_blank_lines() {
    let input_msg = "Add new feature\n\nDescription\n\n\nPROJ-123\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert!(result_msg.starts_with("[PROJ-123] Add new feature"));
    assert!(!result_msg.ends_with("\n\n\n"));
}

#[test]
fn test_already_has_bracket_tag_skips() {
    let input_msg = "[PROJ-123] Add new feature\n";
    let (stdout, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
    assert!(stdout.contains("already in title"));
}

#[test]
fn test_already_has_bracket_tag_with_body_skips() {
    let input_msg = "[PROJ-123] Add new feature\n\nDescription here.\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
}

#[test]
fn test_amend_with_branch_in_comments_adds_tag() {
    let input_msg = "Add feature\n\n# On branch fb-PROJ-123-feature\n# Changes to be committed:\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert!(result_msg.starts_with("[PROJ-123] Add feature"));
}

#[test]
fn test_non_matching_branch_skips() {
    let input_msg = "Add feature\n";
    let (stdout, stderr, result_msg, code) = run_hook(input_msg, "main");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
    assert!(stdout.contains("Skipping") || stdout.contains("not in format"));
}

#[test]
fn test_release_branch_skips() {
    let input_msg = "Add feature\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "rb-PROJ-v1.0.0");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
}

#[test]
fn test_different_jira_project() {
    let input_msg = "Add feature\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-EDEN-2869-some-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, "[EDEN-2869] Add feature\n");
}

#[test]
fn test_idempotent_double_run() {
    let input_msg = "Add new feature\n";

    // First run
    let (_, stderr1, result1, code1) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code1, 0, "Exit code {}, stderr: {}", code1, stderr1);

    // Second run on result
    let (_, stderr2, result2, code2) = run_hook(&result1, "fb-PROJ-123-feature");
    assert_eq!(code2, 0, "Exit code {}, stderr: {}", code2, stderr2);
    assert_eq!(result2, result1);
}

#[test]
fn test_both_formats_keeps_only_title_tag() {
    let input_msg = "[PROJ-123] Add new feature\n\nDescription\n\nPROJ-123\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, "[PROJ-123] Add new feature\n\nDescription\n");
    let parts: Vec<&str> = result_msg.splitn(2, '\n').collect();
    if parts.len() > 1 {
        assert!(!parts[1].contains("PROJ-123\n"));
    }
}

#[test]
fn test_fixup_commit_not_modified() {
    let input_msg = "fixup! Add new feature\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
}

#[test]
fn test_squash_commit_not_modified() {
    let input_msg = "squash! Add new feature\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
}

#[test]
fn test_amend_commit_not_modified() {
    let input_msg = "amend! Add new feature\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
}

#[test]
fn test_detached_head_skips() {
    let input_msg = "Add feature\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
}

#[test]
fn test_merge_commit_not_modified() {
    let input_msg = "Merge branch 'feature' into main\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
}

#[test]
fn test_tag_in_body_not_removed() {
    let input_msg = "Add feature\n\nRelated to PROJ-123 issue.\nPROJ-123\nMore text.\n";
    let (_, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    // The tag at the end is detected as old format
    assert!(result_msg.starts_with("[PROJ-123] Add feature"));
}

#[test]
fn test_different_project_tag_not_modified() {
    let input_msg = "[OTHER-456] Add feature from other project\n";
    let (stdout, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
    assert!(stdout.contains("already in title"));
}

#[test]
fn test_same_project_different_id_not_modified() {
    let input_msg = "[PROJ-456] Add feature from different task\n";
    let (stdout, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert_eq!(result_msg, input_msg);
    assert!(stdout.contains("already in title"));
}

#[test]
fn test_old_format_different_tag_preserved() {
    let input_msg = "Add feature\n\nSome description\n\nOTHER-456\n";
    let (stdout, stderr, result_msg, code) = run_hook(input_msg, "fb-PROJ-123-feature");
    assert_eq!(code, 0, "Exit code {}, stderr: {}", code, stderr);
    assert!(result_msg.starts_with("[OTHER-456] Add feature"));
    let parts: Vec<&str> = result_msg.splitn(2, '\n').collect();
    if parts.len() > 1 {
        assert!(!parts[1].contains("OTHER-456\n"));
    }
    assert!(stdout.contains("Converting old format tag"));
}
