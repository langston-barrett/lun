#!/usr/bin/env python3
"""Bump lun and install-lun action versions."""

from argparse import ArgumentParser
from pathlib import Path
import re
import subprocess
import sys


def run(
    cmd: list[str], check: bool = True, capture: bool = False
) -> subprocess.CompletedProcess:
    """Run a command, logging to stderr."""
    print(f"+ {' '.join(cmd)}", file=sys.stderr)
    return subprocess.run(cmd, check=check, capture_output=capture, text=True)


def get_project_root() -> Path:
    """Get the project root directory."""
    result = run(["git", "rev-parse", "--show-toplevel"], capture=True)
    return Path(result.stdout.strip())


def get_latest_version() -> str:
    """Get the latest release version from GitHub."""
    result = run(
        ["gh", "release", "view", "--json", "tagName", "--jq", ".tagName"],
        capture=True,
    )
    tag = result.stdout.strip()
    # Remove 'v' prefix if present
    return tag.lstrip("v")


def get_head_sha() -> str:
    """Get the current HEAD commit SHA."""
    result = run(["git", "rev-parse", "HEAD"], capture=True)
    return result.stdout.strip()


def update_install_lun_version(action_path: Path, version: str) -> None:
    """Update the default version in install-lun action.yml."""
    content = action_path.read_text()
    # Match the version default line: default: 'X.Y.Z'
    updated = re.sub(
        r"(default:\s*')[0-9]+\.[0-9]+\.[0-9]+(')",
        rf"\g<1>{version}\2",
        content,
        count=1,
    )
    if content == updated:
        raise ValueError(f"Failed to update version in {action_path}")
    action_path.write_text(updated)


def update_lun_action(action_path: Path, sha: str, version: str) -> None:
    """Update the install-lun SHA reference and version in lun action.yml."""
    content = action_path.read_text()

    # Update SHA reference: langston-barrett/lun/.github/actions/install-lun@SHA
    updated = re.sub(
        r"(langston-barrett/lun/\.github/actions/install-lun@)[a-f0-9]+",
        rf"\g<1>{sha}",
        content,
        count=1,
    )
    if content == updated:
        raise ValueError(f"Failed to update install-lun SHA in {action_path}")

    # Update version default: default: 'X.Y.Z'
    updated = re.sub(
        r"(default:\s*')[0-9]+\.[0-9]+\.[0-9]+(')",
        rf"\g<1>{version}\2",
        updated,
        count=1,
    )

    action_path.write_text(updated)


def update_workflows(workflows_dir: Path, sha: str) -> list[Path]:
    """Update workflow files that reference the lun action to use the new SHA.

    Returns list of modified workflow files.
    """
    modified = []

    for workflow_file in workflows_dir.glob("*.yml"):
        content = workflow_file.read_text()

        # Update lun action SHA references
        updated = re.sub(
            r"(langston-barrett/lun/\.github/actions/lun@)[a-f0-9]+",
            rf"\g<1>{sha}",
            content,
        )

        if content != updated:
            workflow_file.write_text(updated)
            modified.append(workflow_file)

    return modified


def git_commit(message: str, files: list[Path]) -> None:
    """Stage files and commit with the given message."""
    run(["git", "add", "--"] + [str(f) for f in files])
    run(["git", "commit", "-m", message])


def create_branch(version: str) -> None:
    """Create a new branch for the version bump."""
    branch_name = f"bump-actions-v{version}"
    print(f"Creating branch: {branch_name}", file=sys.stderr)
    run(["git", "checkout", "-b", branch_name])


def push_and_wait_for_ci() -> None:
    """Push the branch and wait for CI to pass."""
    run(["git", "push", "--set-upstream", "origin", "HEAD", "--force-with-lease"])
    # Wait for CI checks on the branch
    print("\nWaiting for CI checks to pass...", file=sys.stderr)
    run(["gh", "run", "watch", "--exit-status"])


def main() -> int:
    parser = ArgumentParser(description=__doc__)
    parser.add_argument(
        "--version",
        type=str,
        help="Version to bump to (without 'v' prefix). Defaults to latest release.",
    )
    parser.add_argument(
        "--no-push",
        action="store_true",
        help="Skip pushing and waiting for CI.",
    )
    args = parser.parse_args()

    root = get_project_root()
    install_lun_path = root / ".github" / "actions" / "install-lun" / "action.yml"
    lun_action_path = root / ".github" / "actions" / "lun" / "action.yml"
    workflows_dir = root / ".github" / "workflows"

    # Get version to bump to
    version = args.version or get_latest_version()
    print(f"Bumping to version: {version}", file=sys.stderr)

    # Create a new branch for this bump
    create_branch(version)

    # Step 1: Bump install-lun version
    print("\n=== Step 1: Bump install-lun version ===", file=sys.stderr)
    update_install_lun_version(install_lun_path, version)
    git_commit(f"chore(actions): Bump `lun` version to v{version}", [install_lun_path])

    # Step 2: Get the new HEAD SHA (after the first commit)
    head_sha = get_head_sha()
    print(f"New HEAD SHA: {head_sha}", file=sys.stderr)

    # Step 3: Bump lun action with new SHA and version
    print("\n=== Step 2: Bump lun action ===", file=sys.stderr)
    update_lun_action(lun_action_path, head_sha, version)

    # Get new HEAD SHA after lun action update
    git_commit("chore(actions): Bump `install-lun` version", [lun_action_path])
    lun_action_sha = get_head_sha()
    print(f"Lun action SHA: {lun_action_sha}", file=sys.stderr)

    # Step 4: Update workflows to use new lun action SHA
    print("\n=== Step 3: Update workflows ===", file=sys.stderr)
    modified_workflows = update_workflows(workflows_dir, lun_action_sha)
    if modified_workflows:
        print(f"Updated {len(modified_workflows)} workflow(s)", file=sys.stderr)
        git_commit(
            "chore(ci): Update workflows to use new lun action SHA", modified_workflows
        )
    else:
        print("No workflows needed updating", file=sys.stderr)

    # Step 5: Push and wait for CI
    if not args.no_push:
        print("\n=== Step 4: Push and wait for CI ===", file=sys.stderr)
        push_and_wait_for_ci()
    else:
        print("\nSkipping push (--no-push specified)", file=sys.stderr)

    print("\nDone!", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
