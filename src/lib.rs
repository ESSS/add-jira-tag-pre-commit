use regex::Regex;
use std::process::Command;

/// Extract JIRA tag from branch name following pattern: fb-{TAG}-{suffix}
pub fn extract_jira_from_branch(branch: &str) -> Option<String> {
    let re = Regex::new(r"^fb-([A-Z]+-[0-9]+)(-.*)?$").unwrap();
    re.captures(branch)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Reasons why a commit message should be skipped
#[derive(Debug, PartialEq)]
pub enum SkipReason {
    Fixup,
    Squash,
    Amend,
    Merge,
}

/// Check if the commit title should skip modification
pub fn should_skip_title(title: &str) -> Option<SkipReason> {
    if title.starts_with("fixup!") {
        Some(SkipReason::Fixup)
    } else if title.starts_with("squash!") {
        Some(SkipReason::Squash)
    } else if title.starts_with("amend!") {
        Some(SkipReason::Amend)
    } else if title.starts_with("Merge ") {
        Some(SkipReason::Merge)
    } else {
        None
    }
}

/// Check if title already has a JIRA tag in bracket format
pub fn has_bracket_tag(title: &str) -> bool {
    let re = Regex::new(r"^\[[A-Z]+-[0-9]+\]").unwrap();
    re.is_match(title)
}

/// Find old format tag (JIRA tag alone as last non-blank line)
pub fn find_old_format_tag(content: &str) -> Option<String> {
    let re = Regex::new(r"^[A-Z]+-[0-9]+$").unwrap();

    // Find the last non-blank line
    content
        .lines()
        .rfind(|line| !line.trim().is_empty())
        .and_then(|line| {
            if re.is_match(line.trim()) {
                Some(line.trim().to_string())
            } else {
                None
            }
        })
}

/// Remove trailing tag and any trailing blank lines
pub fn remove_trailing_tag(content: &str, tag: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines = Vec::new();
    let mut found_tag = false;

    // Process lines from end to beginning
    for line in lines.iter().rev() {
        if !found_tag && line.trim() == tag {
            found_tag = true;
            continue; // Skip the tag line
        }
        if !found_tag && line.trim().is_empty() {
            continue; // Skip trailing blank lines before the tag
        }
        result_lines.push(*line);
    }

    // Reverse back to original order
    result_lines.reverse();

    // Remove any remaining trailing blank lines
    while result_lines.last().is_some_and(|l| l.trim().is_empty()) {
        result_lines.pop();
    }

    let mut result = result_lines.join("\n");
    if !result.is_empty() && content.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Result of processing a commit message
#[derive(Debug, PartialEq)]
pub enum ProcessResult {
    Added(String),     // Tag was added (branch tag)
    Converted(String), // Old format converted (original tag)
    AlreadyTagged,     // Already has bracket tag
    Skipped(String),   // Skipped with reason
}

/// Main processing logic for commit messages
pub fn process_commit_message(content: &str, branch: &str) -> (String, ProcessResult) {
    // Extract JIRA tag from branch
    let branch_tag = match extract_jira_from_branch(branch) {
        Some(tag) => tag,
        None => {
            return (
                content.to_string(),
                ProcessResult::Skipped("Branch is not in format 'fb-{JIRA_TAG}'.".to_string()),
            );
        }
    };

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return (
            content.to_string(),
            ProcessResult::Skipped("Empty commit message.".to_string()),
        );
    }

    let title = lines[0];

    // Check for special commit types that should not be modified
    if let Some(reason) = should_skip_title(title) {
        let reason_str = match reason {
            SkipReason::Fixup => "fixup",
            SkipReason::Squash => "squash",
            SkipReason::Amend => "amend",
            SkipReason::Merge => "merge",
        };
        return (
            content.to_string(),
            ProcessResult::Skipped(format!(
                "Skipping {}/squash/amend/merge commit...",
                reason_str
            )),
        );
    }

    // Check for old format tag
    if let Some(old_tag) = find_old_format_tag(content) {
        // Remove the old format tag from the end
        let cleaned = remove_trailing_tag(content, &old_tag);

        // Check if title already has a bracket tag after cleaning
        let new_lines: Vec<&str> = cleaned.lines().collect();
        let new_title = if !new_lines.is_empty() {
            new_lines[0]
        } else {
            ""
        };

        if has_bracket_tag(new_title) {
            // Already has bracket tag, just return cleaned content
            return (cleaned, ProcessResult::Converted(old_tag));
        }

        // Add the OLD tag (not branch tag) to the title
        let tagged = format!("[{}] {}", old_tag, cleaned);
        return (tagged, ProcessResult::Converted(old_tag));
    }

    // Check if title already has bracket tag
    if has_bracket_tag(title) {
        return (content.to_string(), ProcessResult::AlreadyTagged);
    }

    // Add branch tag to the beginning of the title
    let tagged = format!("[{}] {}", branch_tag, content);
    (tagged, ProcessResult::Added(branch_tag))
}

/// Get current git branch name
pub fn get_current_branch() -> Option<String> {
    // Check for TEST_BRANCH_NAME environment variable (for testing)
    if let Ok(test_branch) = std::env::var("TEST_BRANCH_NAME") {
        return if test_branch.is_empty() {
            None
        } else {
            Some(test_branch)
        };
    }

    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = String::from_utf8(output.stdout).ok()?;
    let branch = branch.trim();

    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_jira_from_branch() {
        assert_eq!(
            extract_jira_from_branch("fb-PROJ-123-feature"),
            Some("PROJ-123".to_string())
        );
        assert_eq!(
            extract_jira_from_branch("fb-EDEN-2869-some-feature"),
            Some("EDEN-2869".to_string())
        );
        assert_eq!(
            extract_jira_from_branch("fb-ABC-1"),
            Some("ABC-1".to_string())
        );
        assert_eq!(extract_jira_from_branch("main"), None);
        assert_eq!(extract_jira_from_branch("develop"), None);
        assert_eq!(extract_jira_from_branch("rb-PROJ-v1.0.0"), None);
        assert_eq!(extract_jira_from_branch(""), None);
    }

    #[test]
    fn test_should_skip_title() {
        assert_eq!(
            should_skip_title("fixup! Add feature"),
            Some(SkipReason::Fixup)
        );
        assert_eq!(
            should_skip_title("squash! Add feature"),
            Some(SkipReason::Squash)
        );
        assert_eq!(
            should_skip_title("amend! Add feature"),
            Some(SkipReason::Amend)
        );
        assert_eq!(
            should_skip_title("Merge branch 'main'"),
            Some(SkipReason::Merge)
        );
        assert_eq!(should_skip_title("Add feature"), None);
        assert_eq!(should_skip_title("[PROJ-123] Add feature"), None);
    }

    #[test]
    fn test_has_bracket_tag() {
        assert!(has_bracket_tag("[PROJ-123] Add feature"));
        assert!(has_bracket_tag("[ABC-1] Test"));
        assert!(has_bracket_tag("[EDEN-2869] Update"));
        assert!(!has_bracket_tag("Add feature"));
        assert!(!has_bracket_tag("PROJ-123 Add feature"));
        assert!(!has_bracket_tag("Add [PROJ-123] feature"));
    }

    #[test]
    fn test_find_old_format_tag() {
        assert_eq!(
            find_old_format_tag("Add feature\n\nDescription\n\nPROJ-123\n"),
            Some("PROJ-123".to_string())
        );
        assert_eq!(
            find_old_format_tag("Add feature\n\nPROJ-123"),
            Some("PROJ-123".to_string())
        );
        assert_eq!(
            find_old_format_tag("Add feature\n\nDescription\n\n\nPROJ-123\n"),
            Some("PROJ-123".to_string())
        );
        assert_eq!(find_old_format_tag("Add feature\n"), None);
        assert_eq!(find_old_format_tag("Add feature\n\nPROJ-123 extra"), None);
    }

    #[test]
    fn test_remove_trailing_tag() {
        assert_eq!(
            remove_trailing_tag("Add feature\n\nDescription\n\nPROJ-123\n", "PROJ-123"),
            "Add feature\n\nDescription\n"
        );
        assert_eq!(
            remove_trailing_tag("Add feature\n\nPROJ-123\n", "PROJ-123"),
            "Add feature\n"
        );
        assert_eq!(
            remove_trailing_tag("Add feature\n\n\nPROJ-123\n", "PROJ-123"),
            "Add feature\n"
        );
    }

    #[test]
    fn test_process_commit_message_add_tag() {
        let (result, status) = process_commit_message("Add new feature\n", "fb-PROJ-123-feature");
        assert_eq!(result, "[PROJ-123] Add new feature\n");
        assert_eq!(status, ProcessResult::Added("PROJ-123".to_string()));
    }

    #[test]
    fn test_process_commit_message_with_body() {
        let input = "Add new feature\n\nThis is the description.\n";
        let (result, status) = process_commit_message(input, "fb-PROJ-123-feature");
        assert_eq!(
            result,
            "[PROJ-123] Add new feature\n\nThis is the description.\n"
        );
        assert_eq!(status, ProcessResult::Added("PROJ-123".to_string()));
    }

    #[test]
    fn test_process_commit_message_old_format() {
        let input = "Add new feature\n\nSome description\n\nPROJ-123\n";
        let (result, status) = process_commit_message(input, "fb-PROJ-123-feature");
        assert!(result.starts_with("[PROJ-123] Add new feature"));
        assert!(!result.contains("PROJ-123\n\n"));
        assert_eq!(status, ProcessResult::Converted("PROJ-123".to_string()));
    }

    #[test]
    fn test_process_commit_message_already_tagged() {
        let input = "[PROJ-123] Add new feature\n";
        let (result, status) = process_commit_message(input, "fb-PROJ-123-feature");
        assert_eq!(result, input);
        assert_eq!(status, ProcessResult::AlreadyTagged);
    }

    #[test]
    fn test_process_commit_message_fixup() {
        let input = "fixup! Add new feature\n";
        let (result, status) = process_commit_message(input, "fb-PROJ-123-feature");
        assert_eq!(result, input);
        assert!(matches!(status, ProcessResult::Skipped(_)));
    }

    #[test]
    fn test_process_commit_message_non_matching_branch() {
        let input = "Add feature\n";
        let (result, status) = process_commit_message(input, "main");
        assert_eq!(result, input);
        assert!(matches!(status, ProcessResult::Skipped(_)));
    }

    #[test]
    fn test_process_commit_message_different_tag_preserved() {
        let input = "Add feature\n\nSome description\n\nOTHER-456\n";
        let (result, status) = process_commit_message(input, "fb-PROJ-123-feature");
        assert!(result.starts_with("[OTHER-456] Add feature"));
        assert_eq!(status, ProcessResult::Converted("OTHER-456".to_string()));
    }

    #[test]
    fn test_process_commit_message_both_formats() {
        let input = "[PROJ-123] Add new feature\n\nDescription\n\nPROJ-123\n";
        let (result, status) = process_commit_message(input, "fb-PROJ-123-feature");
        assert_eq!(result, "[PROJ-123] Add new feature\n\nDescription\n");
        assert_eq!(status, ProcessResult::Converted("PROJ-123".to_string()));
    }

    #[test]
    fn test_process_commit_message_tag_in_middle_of_body() {
        let input = "Add feature\n\nRelated to PROJ-123 issue.\nPROJ-123\nMore text.\n";
        let (result, _) = process_commit_message(input, "fb-PROJ-123-feature");
        // The tag at the end should be detected and converted
        assert!(result.starts_with("[PROJ-123] Add feature"));
        // Middle mention should be preserved
        assert!(result.contains("Related to PROJ-123 issue."));
    }
}
