# add-jira-tag-pre-commit

Adds Jira issue key as a tag in commit messages if a Jira key is found in the branch's name.

---
## Usage

Add the following to the `.pre-commit-config.yaml` file in you repo. 

``` yaml
-   repo: https://github.com/ESSS/add-jira-tag-pre-commit
    rev: v0.3.0
    hooks:
        -   id: add-jira-tag
            name: add-jira-tag
            stages: [ prepare-commit-msg ]    
```    

Then, install `pre-commit` hooks (usually `pre-commit install`). 

### Customizing the branch prefix

By default the hook looks for branches named `fb-{JIRA_TAG}` (e.g.
`fb-PROJ-123-some-feature`). The prefix that precedes the Jira key is
configurable via the `--prefix` argument; the Jira key pattern itself
(`[A-Z]+-[0-9]+`) is fixed.

``` yaml
        -   id: add-jira-tag
            name: add-jira-tag
            stages: [ prepare-commit-msg ]
            args: [ "--prefix", "feat-" ]
```

The prefix is a regex fragment, so an alternation works too — this matches
`task-`, `feat-`, or `fix-` branches:

``` yaml
            args: [ "--prefix", "(task|feat|fix)-" ]
```

> :warning: **`add-jira-tag` is a `prepare-commit-msg` hook**, so passing additional arguments `--install-hooks -t prepare-commit-msg` may be required during hooks installation.

---
## Development

Run tests with:

```bash
uv run --with pytest pytest
```

### Install hooks locally

To use this hook during development of this repo, with `pre-commit`:

```bash
uv run --with pre-commit pre-commit install -t prepare-commit-msg
```

or with `prek`:

```bash
uv run --with prek prek install -t prepare-commit-msg
```
