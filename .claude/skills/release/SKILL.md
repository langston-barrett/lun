---
name: release
description: Cut and publish a release. Does CHANGELOG prep, version bumps, PR creation, CI, tagging, and publishing.
---

# Release

## Overview

1. Analyze CHANGELOG and git history to prepare release
2. Suggest missing CHANGELOG entries from git log
3. Determine version bump type (major/minor/patch)
4. Run `scripts/release.py` with the determined bump type

## Step 0: Check out `main`

```sh
git checkout main && git pull
```

## Step 1: Ensure CHANGELOG Has `## next` Section

If no `## next` section exists, create one at the top of the changelog (after the header):

```markdown
## next

- [entries go here]
```

## Step 2: Analyze CHANGELOG for Missing Entries

Before releasing, review git history for commits since the last release that aren't in CHANGELOG.

```bash
# Show commits since last tag
git log $(git describe --tags --abbrev=0)..HEAD --oneline
```

For each important change not already mentioned in CHANGELOG under `## next`:

1. Summarize the change in user-facing terms
2. Ask the user: "Add to CHANGELOG: [summary]? (y/n)"
3. If confirmed, add the entry under `## next` in CHANGELOG.md

## Step 3: Determine Version Bump Type

Analyze the `## next` section of CHANGELOG.md to determine the appropriate bump:

**MAJOR** (breaking changes):
- "BREAKING" keyword in any entry
- Removed features or APIs
- Changed behavior that breaks existing usage

**MINOR** (new features):
- "Add" keyword starting an entry
- New commands, flags, or configuration options
- New capabilities that don't break existing usage

**PATCH** (bug fixes):
- "Fix" keyword starting an entry
- Performance improvements
- Documentation updates
- Internal refactoring without user-facing changes

Apply the highest applicable bump type. Present the suggestion to the user:
"Based on CHANGELOG entries, suggesting **[type]** bump. Proceed? (y/n/major/minor/patch)"

## Step 5: Run the Release Script

Execute the release script with the determined bump type:

```bash
python3 .claude/skills/release/scripts/release.py [major|minor|patch]
```

The script handles:
1. Checkout main, pull, create `release` branch
2. Update version in Cargo.toml
3. Update CHANGELOG.md (replace `## next` with version header and link)
4. Run `cargo clippy -- --all-targets --deny warnings` and `cargo test`
5. Commit changes with message `vX.Y.Z`
6. Create PR with title `vX.Y.Z`
7. Open PR in browser
8. Wait for user to press ENTER
9. Wait for all CI checks to pass
10. Merge PR
11. Checkout main, pull
12. Create and push annotated tag `vX.Y.Z`
13. Wait for tag CI to complete
14. Publish draft GitHub release

## Example Session

```
User: cut a release

Claude:
1. Checks git log for interesting commits
2. "Add to CHANGELOG: Add support for YAML configs? (y/n)"
3. User: "y"
4. Claude adds entry to CHANGELOG
5. "Based on CHANGELOG (new 'Add' entry), suggesting minor bump. Proceed?"
6. User: "y"
7. Claude runs: python3 .claude/skills/release/scripts/release.py minor
```
