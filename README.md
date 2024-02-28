# add-jira-tag-pre-commit

Adds Jira issue key as a tag in commit messages if a Jira key is found in the branch's name.

---
## Usage

Add the following to the `.pre-commit-config.yaml` file in you repo. 

``` yaml
-   repo: https://github.com/ESSS/add-jira-tag-pre-commit
    rev: v0.1.0
    hooks:
        -   id: add-jira-tag
            name: add-jira-tag
            stages: [ prepare-commit-msg ]    
```    

Then, install `pre-commit` hooks (usually `pre-commit install`). 

> :warning: **`add-jira-tag` is a `prepare-commit-msg` hook**, so passing additional arguments `--install-hooks -t prepare-commit-msg` may be required during hooks installation. 