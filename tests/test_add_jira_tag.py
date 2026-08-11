"""Tests for the add_jira_tag.sh script."""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest


SCRIPT_PATH = Path(__file__).parent.parent / "add_jira_tag.sh"


def _get_git_bash() -> str:
    """Find Git bash executable, prefer it over WSL bash on Windows."""
    if sys.platform != "win32":
        return "bash"

    # Try to find git and derive bash path from it.
    git_path = shutil.which("git")
    if git_path:
        git_dir = Path(git_path).parent
        # git.exe is usually in cmd/ or bin/, bash is in bin/.
        for bash_candidate in [
            git_dir.parent / "bin" / "bash.exe",  # If git is in cmd/.
            git_dir / "bash.exe",  # If git is in bin/.
        ]:
            if bash_candidate.exists():
                return str(bash_candidate)

    # Common Git bash locations as fallback.
    for path in [
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ]:
        if Path(path).exists():
            return path

    return "bash"  # Fall back to PATH.


def _run_hook(
    tmp_path: Path,
    commit_msg: str,
    branch_name: str = "fb-PROJ-123-feature",
    args: list[str] | None = None,
) -> tuple[str, str, str, int]:
    """
    Run the hook script with a mocked git branch command.

    ``args`` are extra hook options (e.g. ``["--prefix", "feat-"]``) injected
    before git's positional commit-message argument, mirroring pre-commit's
    ``args:`` behaviour.

    Returns (stdout, stderr, result_msg, returncode).
    """
    msg_file = tmp_path / "COMMIT_EDITMSG"
    msg_file.write_text(commit_msg)

    extra_args = " ".join(f'"{arg}"' for arg in (args or []))

    wrapper_script = f'''
git() {{
    if [[ "$1" == "branch" && "$2" == "--show-current" ]]; then
        echo "{branch_name}"
        return
    fi
    command git "$@"
}}
export -f git

source "{SCRIPT_PATH.as_posix()}" {extra_args} "$@"
'''
    wrapper_file = tmp_path / "wrapper.sh"
    wrapper_file.write_text(wrapper_script)

    result = subprocess.run(
        [_get_git_bash(), wrapper_file.as_posix(), msg_file.as_posix()],
        capture_output=True,
        text=True,
        cwd=tmp_path,
    )

    result_msg = msg_file.read_text()
    return result.stdout, result.stderr, result_msg, result.returncode


def test_fresh_commit_adds_tag_to_title(tmp_path: Path) -> None:
    """A fresh commit without any tag gets [TAG] prepended to title."""
    stdout, stderr, result_msg, code = _run_hook(tmp_path, "Add new feature\n")
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == "[PROJ-123] Add new feature\n"
    assert "Adding tag" in stdout


def test_fresh_commit_with_body_adds_tag_to_title(tmp_path: Path) -> None:
    """A fresh commit with body gets [TAG] prepended only to title."""
    input_msg = "Add new feature\n\nThis is the description.\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == "[PROJ-123] Add new feature\n\nThis is the description.\n"


def test_old_format_converts_to_new(tmp_path: Path) -> None:
    """Old format with tag at end is converted to new format with tag in title."""
    input_msg = "Add new feature\n\nSome description\n\nPROJ-123\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg.startswith("[PROJ-123] Add new feature")
    assert "PROJ-123\n" not in result_msg.split("\n", 1)[1]  # Not in body.
    assert "Converting old format tag" in stdout


def test_old_format_with_trailing_blank_lines(tmp_path: Path) -> None:
    """Old format with blank lines before tag is cleaned up."""
    input_msg = "Add new feature\n\nDescription\n\n\nPROJ-123\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg.startswith("[PROJ-123] Add new feature")
    # Should not have multiple trailing newlines.
    assert not result_msg.endswith("\n\n\n")


def test_already_has_bracket_tag_skips(tmp_path: Path) -> None:
    """If tag is already in title with brackets, script skips."""
    input_msg = "[PROJ-123] Add new feature\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.
    assert "already in title" in stdout


def test_already_has_bracket_tag_with_body_skips(tmp_path: Path) -> None:
    """If tag is already in title, body is preserved and nothing changes."""
    input_msg = "[PROJ-123] Add new feature\n\nDescription here.\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.


def test_amend_with_branch_in_comments_adds_tag(tmp_path: Path) -> None:
    """Amend scenario: branch name in comments doesn't count as having tag."""
    input_msg = "Add feature\n\n# On branch fb-PROJ-123-feature\n# Changes to be committed:\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg.startswith("[PROJ-123] Add feature")


def test_non_matching_branch_skips(tmp_path: Path) -> None:
    """Non-matching branch name causes script to skip without changes."""
    input_msg = "Add feature\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg, branch_name="main")
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.
    assert "Skipping" in stdout


def test_release_branch_skips(tmp_path: Path) -> None:
    """Release branch pattern doesn't match, so script skips."""
    input_msg = "Add feature\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg, branch_name="rb-PROJ-v1.0.0")
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.


def test_different_jira_project(tmp_path: Path) -> None:
    """Different JIRA project keys work correctly."""
    input_msg = "Add feature\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg, branch_name="fb-EDEN-2869-some-feature")
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == "[EDEN-2869] Add feature\n"


def test_idempotent_double_run(tmp_path: Path) -> None:
    """Running the script twice produces the same result (idempotent)."""
    input_msg = "Add new feature\n"

    # First run.
    _, stderr1, result1, code1 = _run_hook(tmp_path, input_msg)
    assert code1 == 0, f"Exit code {code1}, stderr: {stderr1}"

    # Second run on result.
    _, stderr2, result2, code2 = _run_hook(tmp_path, result1)
    assert code2 == 0, f"Exit code {code2}, stderr: {stderr2}"
    assert result2 == result1  # Same result.


def test_both_formats_keeps_only_title_tag(tmp_path: Path) -> None:
    """If both new format (title) and old format (bottom) exist, remove the old."""
    input_msg = "[PROJ-123] Add new feature\n\nDescription\n\nPROJ-123\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == "[PROJ-123] Add new feature\n\nDescription\n"
    # Old format tag should be removed.
    assert "PROJ-123\n" not in result_msg.split("\n", 1)[1]


def test_fixup_commit_not_modified(tmp_path: Path) -> None:
    """Fixup commits should not be modified."""
    input_msg = "fixup! Add new feature\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.


def test_squash_commit_not_modified(tmp_path: Path) -> None:
    """Squash commits should not be modified."""
    input_msg = "squash! Add new feature\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.


def test_amend_commit_not_modified(tmp_path: Path) -> None:
    """Amend commits (git commit --fixup=amend:) should not be modified."""
    input_msg = "amend! Add new feature\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.


def test_detached_head_skips(tmp_path: Path) -> None:
    """Detached HEAD (empty branch) causes script to skip."""
    input_msg = "Add feature\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg, branch_name="")
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.


def test_merge_commit_not_modified(tmp_path: Path) -> None:
    """Merge commits should not be modified."""
    input_msg = "Merge branch 'feature' into main\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.


def test_tag_in_body_not_removed(tmp_path: Path) -> None:
    """Tag mentioned in body (not at end) should not be removed."""
    input_msg = "Add feature\n\nRelated to PROJ-123 issue.\nPROJ-123\nMore text.\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    # The mid-body mention should remain.
    assert "PROJ-123\nMore text." in result_msg
    # Title should have tag added.
    assert result_msg.startswith("[PROJ-123] Add feature")


def test_different_project_tag_not_modified(tmp_path: Path) -> None:
    """Commit with different project's tag should not be modified (rebase scenario)."""
    input_msg = "[OTHER-456] Add feature from other project\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.
    assert "already in title" in stdout


def test_same_project_different_id_not_modified(tmp_path: Path) -> None:
    """Commit with same project but different ID should not be modified (rebase scenario)."""
    input_msg = "[PROJ-456] Add feature from different task\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.
    assert "already in title" in stdout


def test_old_format_different_tag_preserved(tmp_path: Path) -> None:
    """Old format tag from different issue is preserved, not replaced with branch tag."""
    input_msg = "Add feature\n\nSome description\n\nOTHER-456\n"
    stdout, stderr, result_msg, code = _run_hook(tmp_path, input_msg)
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    # Should use OTHER-456, not PROJ-123 from branch.
    assert result_msg.startswith("[OTHER-456] Add feature")
    assert "OTHER-456\n" not in result_msg.split("\n", 1)[1]  # Old tag removed from end.
    assert "Converting old format tag" in stdout


def test_custom_prefix_adds_tag(tmp_path: Path) -> None:
    """A custom --prefix matches a branch with that prefix and tags the title."""
    stdout, stderr, result_msg, code = _run_hook(
        tmp_path,
        "Add feature\n",
        branch_name="feat-PROJ-123-some-feature",
        args=["--prefix", "feat-"],
    )
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == "[PROJ-123] Add feature\n"
    assert "Adding tag" in stdout


def test_custom_prefix_equals_form(tmp_path: Path) -> None:
    """The --prefix=<value> form is also supported."""
    stdout, stderr, result_msg, code = _run_hook(
        tmp_path,
        "Add feature\n",
        branch_name="feat-PROJ-123-some-feature",
        args=["--prefix=feat-"],
    )
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == "[PROJ-123] Add feature\n"


def test_default_prefix_still_works_with_no_args(tmp_path: Path) -> None:
    """With no args the default 'fb-' prefix keeps working (regression)."""
    stdout, stderr, result_msg, code = _run_hook(tmp_path, "Add feature\n")
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == "[PROJ-123] Add feature\n"


def test_custom_prefix_skips_default_fb_branch(tmp_path: Path) -> None:
    """When a custom prefix is set, a default 'fb-' branch no longer matches."""
    input_msg = "Add feature\n"
    stdout, stderr, result_msg, code = _run_hook(
        tmp_path,
        input_msg,
        branch_name="fb-PROJ-123-feature",
        args=["--prefix", "feat-"],
    )
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.
    assert "Skipping" in stdout


@pytest.mark.parametrize("branch_prefix", ["task", "feat", "fix"])
def test_alternation_prefix_triggers_for_each(tmp_path: Path, branch_prefix: str) -> None:
    """An alternation prefix '(task|feat|fix)-' matches any of the three."""
    stdout, stderr, result_msg, code = _run_hook(
        tmp_path,
        "Add feature\n",
        branch_name=f"{branch_prefix}-PROJ-1-some-feature",
        args=["--prefix", "(task|feat|fix)-"],
    )
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == "[PROJ-1] Add feature\n"
    assert "Adding tag" in stdout


def test_alternation_prefix_skips_non_member(tmp_path: Path) -> None:
    """A branch prefix outside the alternation set is skipped."""
    input_msg = "Add feature\n"
    stdout, stderr, result_msg, code = _run_hook(
        tmp_path,
        input_msg,
        branch_name="chore-PROJ-1-some-feature",
        args=["--prefix", "(task|feat|fix)-"],
    )
    assert code == 0, f"Exit code {code}, stderr: {stderr}"
    assert result_msg == input_msg  # Unchanged.
    assert "Skipping" in stdout
