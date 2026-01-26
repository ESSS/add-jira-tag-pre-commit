use add_jira_tag::{get_current_branch, process_commit_message, ProcessResult};
use std::env;
use std::fs;
use std::process;

fn main() {
    // Get commit message file path from command line argument
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: add-jira-tag <commit-msg-file>");
        process::exit(1);
    }

    let commit_msg_filepath = &args[1];

    // Get current branch
    let branch = match get_current_branch() {
        Some(b) => b,
        None => {
            println!("Branch is not in format 'fb-{{JIRA_TAG}}'. Skipping...");
            process::exit(0);
        }
    };

    // Read commit message file
    let content = match fs::read_to_string(commit_msg_filepath) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading commit message file: {}", e);
            process::exit(1);
        }
    };

    // Process the commit message
    let (new_content, result) = process_commit_message(&content, &branch);

    // Print status message
    match result {
        ProcessResult::Added(tag) => {
            println!("Adding tag '[{}]' to commit message title", tag);
        }
        ProcessResult::Converted(tag) => {
            println!("Converting old format tag '{}' to new format...", tag);
        }
        ProcessResult::AlreadyTagged => {
            println!("JIRA tag already in title. Skipping...");
        }
        ProcessResult::Skipped(reason) => {
            println!("{}", reason);
        }
    }

    // Write modified content back to file (if changed)
    if new_content != content {
        if let Err(e) = fs::write(commit_msg_filepath, new_content) {
            eprintln!("Error writing commit message file: {}", e);
            process::exit(1);
        }
    }

    // Always exit with success (0) - this is a prepare-commit-msg hook
    process::exit(0);
}
