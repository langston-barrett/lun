#!/usr/bin/env python3
"""Automate semantic version releases for Rust projects."""

from argparse import ArgumentParser
from dataclasses import dataclass
from datetime import date
from enum import Enum
from pathlib import Path
import re
import subprocess
import sys
import webbrowser


class BumpType(Enum):
    MAJOR = "major"
    MINOR = "minor"
    PATCH = "patch"


@dataclass
class Version:
    major: int
    minor: int
    patch: int

    def __str__(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"

    def bump(self, bump_type: BumpType) -> "Version":
        if bump_type == BumpType.MAJOR:
            return Version(self.major + 1, 0, 0)
        elif bump_type == BumpType.MINOR:
            return Version(self.major, self.minor + 1, 0)
        else:
            return Version(self.major, self.minor, self.patch + 1)


def run(
    cmd: list[str], check: bool = True, capture: bool = False
) -> subprocess.CompletedProcess:
    """Run a command, logging to stderr."""
    print(f"+ {' '.join(cmd)}", file=sys.stderr)
    return subprocess.run(cmd, check=check, capture_output=capture, text=True)


def get_project_root() -> Path:
    """Get the project root directory (where Cargo.toml is)."""
    result = run(["git", "rev-parse", "--show-toplevel"], capture=True)
    return Path(result.stdout.strip())


def get_project_name(root: Path) -> str:
    """Get project name from the root directory name."""
    return root.name


def parse_version(cargo_toml: str) -> Version:
    """Parse version from Cargo.toml content using string processing."""
    for line in cargo_toml.splitlines():
        line = line.strip()
        if line.startswith("version") and "=" in line:
            # Extract version string: version = "X.Y.Z"
            match = re.search(r'version\s*=\s*"(\d+)\.(\d+)\.(\d+)"', line)
            if match:
                return Version(
                    int(match.group(1)),
                    int(match.group(2)),
                    int(match.group(3)),
                )
    raise ValueError("Could not find version in Cargo.toml")


def update_cargo_toml(
    cargo_path: Path, old_version: Version, new_version: Version
) -> None:
    """Update version in Cargo.toml using string replacement."""
    content = cargo_path.read_text()
    old_pattern = f'version = "{old_version}"'
    new_pattern = f'version = "{new_version}"'
    updated = content.replace(old_pattern, new_pattern, 1)
    if content == updated:
        raise ValueError(f"Failed to update version in {cargo_path}")
    cargo_path.write_text(updated)


def update_changelog(
    changelog_path: Path, new_version: Version, project_name: str
) -> None:
    """Update CHANGELOG.md: replace '## next' with version header and add link."""
    content = changelog_path.read_text()
    today = date.today().isoformat()
    version_header = f"## [{new_version}] - {today}"
    link = f"[{new_version}]: https://github.com/langston-barrett/{project_name}/releases/tag/v{new_version}"

    # Replace ## next (case insensitive) with version header
    # Also handle ## Next, ## NEXT, etc.
    updated = re.sub(
        r"^## [Nn]ext\s*$",
        f"{version_header}\n\n{link}",
        content,
        count=1,
        flags=re.MULTILINE,
    )

    if content == updated:
        raise ValueError("Could not find '## next' section in CHANGELOG.md")

    changelog_path.write_text(updated)


def run_checks() -> None:
    """Run cargo clippy and cargo test."""
    run(["cargo", "clippy", "--", "--all-targets", "--deny", "warnings"])
    run(["cargo", "test"])


def git_prepare_branch() -> None:
    """Check out main, pull, and create release branch."""
    run(["git", "checkout", "main"])
    run(["git", "pull"])
    run(["git", "checkout", "-b", "release"])


def git_commit(version: Version, cargo_path: Path, changelog_path: Path) -> None:
    """Stage changes and commit with version message."""
    run(["git", "add", "--", str(cargo_path), str(changelog_path)])
    run(["git", "commit", "-m", f"v{version}"])


def create_pr(version: Version) -> str:
    """Create PR using gh CLI and return the PR URL."""
    run(["git", "push", "-u", "origin", "release"])
    result = run(
        [
            "gh",
            "pr",
            "create",
            "--title",
            f"v{version}",
            "--body",
            f"Release v{version}",
        ],
        capture=True,
    )
    pr_url = result.stdout.strip()
    return pr_url


def open_pr_in_browser(pr_url: str) -> None:
    """Open the PR URL in the default browser."""
    webbrowser.open(pr_url)


def wait_for_user_confirmation() -> None:
    """Wait for user to press ENTER."""
    input("\nPress ENTER if the PR looks good to continue with merge...")


def wait_for_ci(pr_url: str) -> None:
    """Wait for all CI checks to pass on the PR."""
    # Extract PR number from URL
    pr_number = pr_url.rstrip("/").split("/")[-1]
    # Wait for all checks (not just required)
    run(["gh", "pr", "checks", pr_number, "--watch"])


def merge_pr(pr_url: str) -> None:
    """Merge the PR using gh CLI."""
    pr_number = pr_url.rstrip("/").split("/")[-1]
    run(["gh", "pr", "merge", pr_number, "--merge", "--delete-branch"])


def checkout_main_and_pull() -> None:
    """Check out main and pull the merged changes."""
    run(["git", "checkout", "main"])
    run(["git", "pull"])


def create_and_push_tag(version: Version) -> None:
    """Create annotated git tag and push it."""
    tag = f"v{version}"
    run(["git", "tag", "-a", tag, "-m", tag])
    run(["git", "push", "origin", tag])


def wait_for_tag_ci(version: Version) -> None:
    """Wait for CI on the tag to complete."""
    tag = f"v{version}"
    # Use gh to wait for workflow runs on the tag
    # We need to watch for the release workflow triggered by the tag
    print(f"\nWaiting for CI on tag {tag}...", file=sys.stderr)
    # Poll for the workflow run associated with the tag
    run(["gh", "run", "watch", "--exit-status"], check=True)


def publish_draft_release(version: Version) -> None:
    """Publish the draft release created by CI."""
    tag = f"v{version}"
    # The CI creates a draft release; we just need to publish it
    run(["gh", "release", "edit", tag, "--draft=false"])
    print(f"\nRelease v{version} published!", file=sys.stderr)


def main() -> int:
    parser = ArgumentParser(description=__doc__)
    parser.add_argument(
        "bump_type",
        type=str,
        choices=["major", "minor", "patch"],
        help="Version bump type",
    )
    args = parser.parse_args()

    bump_type = BumpType(args.bump_type)
    root = get_project_root()
    project_name = get_project_name(root)
    cargo_path = root / "Cargo.toml"
    changelog_path = root / "CHANGELOG.md"

    # Parse current version
    cargo_content = cargo_path.read_text()
    current_version = parse_version(cargo_content)
    new_version = current_version.bump(bump_type)

    print(f"Project: {project_name}", file=sys.stderr)
    print(f"Current version: {current_version}", file=sys.stderr)
    print(f"New version: {new_version}", file=sys.stderr)
    print(file=sys.stderr)

    # Prepare release branch
    git_prepare_branch()

    # Update files
    update_cargo_toml(cargo_path, current_version, new_version)
    update_changelog(changelog_path, new_version, project_name)

    # Run checks
    run_checks()

    # Commit changes
    git_commit(new_version, cargo_path, changelog_path)

    # Create PR
    pr_url = create_pr(new_version)
    print(f"\nPR created: {pr_url}", file=sys.stderr)

    # Open in browser
    open_pr_in_browser(pr_url)

    # Wait for user confirmation
    wait_for_user_confirmation()

    # Wait for CI and merge
    wait_for_ci(pr_url)
    merge_pr(pr_url)

    # Update local main
    checkout_main_and_pull()

    # Tag and push
    create_and_push_tag(new_version)

    # Wait for tag CI
    wait_for_tag_ci(new_version)

    # Publish release
    publish_draft_release(new_version)

    return 0


if __name__ == "__main__":
    sys.exit(main())
