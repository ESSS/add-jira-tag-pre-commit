#!/usr/bin/env bash

# Git will pass the commit msg as an argument to prepare-commit-msg hook.
# See https://git-scm.com/docs/githooks#_prepare_commit_msg.
commit_msg_filepath=$1

branch_name=$(git branch --show-current)

regex="fb-([A-Z]+-[0-9]+)(-.*)?"
if [[ $branch_name =~ $regex ]]
then
    jira_issue=${BASH_REMATCH[1]}
else
    echo Branch is not in format 'fb-{JIRA_TAG}'. Skipping...
    exit 0
fi

function add_jira_tag {
    echo Adding tag \\'$jira_issue\\' to commit message
    echo -e "\\n\\n$1" >> $2
    exit 0
}

jira_issue_matches=$(grep -c $jira_issue $commit_msg_filepath)

# If tag not found, then add it
if (($(($jira_issue_matches)) == 0))
then
    add_jira_tag $jira_issue $commit_msg_filepath
    exit 0
fi


if (($(($jira_issue_matches)) == 1))
then
    jira_issue_match=$(grep $jira_issue $commit_msg_filepath)
    # Check if the hit has the same length as the sole tag. This
    # is required because if amending or commiting from the command
    # line, git will add to the commit message the full branch name,
    # which contains the issue tag. Using grep -w did not work for
    # this script.
    if (($((${#jira_issue_match} != ${#jira_issue}))))
    then
        add_jira_tag $jira_issue $commit_msg_filepath
        exit 0
    fi
fi