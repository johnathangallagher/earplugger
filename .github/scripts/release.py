#!/usr/bin/env python3
"""
release.py - Single-Workflow Release Automation for earplugger

Parses Conventional Commits since the last release tag, calculates the
next SemVer version, formats grouped release notes, updates Cargo.toml,
Cargo.lock, and CHANGELOG.md, and creates the release commit and tag.
"""

from __future__ import annotations

import argparse
import datetime
import os
from pathlib import Path
import re
import subprocess
import sys
from typing import Dict, List, NamedTuple, Optional, Tuple

CONVENTIONAL_PATTERN = re.compile(
    r"^(?P<type>[a-zA-Z]+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?:\s*(?P<desc>.+)$"
)

CATEGORY_MAPPING = {
    "feat": "Features",
    "fix": "Bug Fixes",
    "perf": "Performance Improvements",
    "refactor": "Refactoring & Code Quality",
    "docs": "Documentation & Wiki",
    "style": "Code Style & Formatting",
    "test": "Tests & Verification",
    "ci": "Maintenance & CI",
    "build": "Maintenance & CI",
    "chore": "Maintenance & CI",
}

SECTION_ORDER = [
    "Breaking Changes ⚠️",
    "Features",
    "Bug Fixes",
    "Performance Improvements",
    "Refactoring & Code Quality",
    "Documentation & Wiki",
    "Code Style & Formatting",
    "Tests & Verification",
    "Maintenance & CI",
    "Other Changes",
]


class Commit(NamedTuple):
    hash: str
    subject: str
    body: str
    commit_type: Optional[str]
    scope: Optional[str]
    is_breaking: bool
    description: str


def run_git(args: List[str], cwd: Optional[Path] = None) -> str:
    res = subprocess.run(
        ["git"] + args,
        cwd=str(cwd) if cwd else None,
        capture_output=True,
        text=True,
        check=True,
    )
    return res.stdout.strip()


def get_tags(repo_root: Path) -> List[str]:
    try:
        tags = run_git(["tag", "-l", "v*", "--sort=-v:refname"], cwd=repo_root)
        if not tags:
            return []
        valid_tags: List[str] = []
        for tag in tags.splitlines():
            tag = tag.strip()
            if re.match(r"^v\d+\.\d+\.\d+$", tag):
                valid_tags.append(tag)
        return valid_tags
    except subprocess.CalledProcessError:
        return []


def get_commits_between(repo_root: Path, since_ref: Optional[str], until_ref: str = "HEAD") -> List[Commit]:
    git_range = f"{since_ref}..{until_ref}" if since_ref else until_ref
    cmd = ["log", "--no-merges", "--pretty=format:%H%x1f%s%x1f%b%x1e", git_range]
    try:
        raw = run_git(cmd, cwd=repo_root)
    except subprocess.CalledProcessError as e:
        print(f"[-] Warning: Failed to read git log: {e.stderr}", file=sys.stderr)
        return []

    if not raw:
        return []

    records = raw.split("\x1e")
    commits: List[Commit] = []

    for rec in records:
        rec = rec.strip()
        if not rec:
            continue
        parts = rec.split("\x1f")
        chash = parts[0].strip()
        subject = parts[1].strip() if len(parts) > 1 else ""
        body = parts[2].strip() if len(parts) > 2 else ""

        if subject.startswith("chore(release):") or subject.startswith("chore: release") or "[skip ci]" in subject:
            continue

        match = CONVENTIONAL_PATTERN.match(subject)
        if match:
            ctype = match.group("type").lower()
            scope = match.group("scope")
            breaking_bang = bool(match.group("breaking"))
            desc = match.group("desc").strip()
            body_breaking = "BREAKING CHANGE:" in body or "BREAKING-CHANGE:" in body
            is_breaking = breaking_bang or body_breaking
            commits.append(Commit(chash, subject, body, ctype, scope, is_breaking, desc))
        else:
            body_breaking = "BREAKING CHANGE:" in body or "BREAKING-CHANGE:" in body
            commits.append(Commit(chash, subject, body, None, None, body_breaking, subject))

    return commits


def parse_semver(version_str: str) -> Tuple[int, int, int]:
    m = re.match(r"^v?(\d+)\.(\d+)\.(\d+)$", version_str.strip())
    if not m:
        raise ValueError(f"Invalid semver string: {version_str}")
    return int(m.group(1)), int(m.group(2)), int(m.group(3))


def calculate_next_version(
    current_version: str,
    bump: str,
    commits: List[Commit],
) -> str:
    major, minor, patch = parse_semver(current_version)

    if bump == "major":
        return f"{major + 1}.0.0"
    elif bump == "minor":
        return f"{major}.{minor + 1}.0"
    elif bump == "patch":
        return f"{major}.{minor}.{patch + 1}"
    elif bump == "auto":
        has_breaking = any(c.is_breaking for c in commits)
        has_feat = any(c.commit_type == "feat" for c in commits)

        if has_breaking:
            return f"{major + 1}.0.0"
        elif has_feat:
            return f"{major}.{minor + 1}.0"
        else:
            return f"{major}.{minor}.{patch + 1}"
    else:
        raise ValueError(f"Unknown bump type: {bump}")


def categorize_commits(commits: List[Commit]) -> Dict[str, List[Commit]]:
    categorized: Dict[str, List[Commit]] = {sec: [] for sec in SECTION_ORDER}

    for c in commits:
        if c.is_breaking:
            categorized["Breaking Changes ⚠️"].append(c)

        if c.commit_type in CATEGORY_MAPPING:
            section = CATEGORY_MAPPING[c.commit_type]
            categorized[section].append(c)
        else:
            categorized["Other Changes"].append(c)

    return categorized


def generate_release_notes(
    new_version: str,
    prev_tag: Optional[str],
    commits: List[Commit],
    repo: str = "johnathangallagher/earplugger",
) -> str:
    lines = [f"## What's Changed in v{new_version}", ""]
    categorized = categorize_commits(commits)

    has_content = False
    for section in SECTION_ORDER:
        section_commits = categorized.get(section, [])
        if not section_commits:
            continue
        has_content = True
        lines.append(f"### {section}")
        for c in section_commits:
            short_hash = c.hash[:7]
            commit_link = f"https://github.com/{repo}/commit/{c.hash}"
            if c.scope:
                prefix = f"**{c.scope}**: "
            else:
                prefix = ""
            lines.append(f"- {prefix}{c.description} ([`{short_hash}`]({commit_link}))")
        lines.append("")

    if not has_content:
        lines.append("- Routine maintenance release and internal optimizations.")
        lines.append("")

    if prev_tag:
        compare_url = f"https://github.com/{repo}/compare/{prev_tag}...v{new_version}"
        lines.append(f"**Full Changelog**: {compare_url}")
        lines.append("")

    return "\n".join(lines)


def update_cargo_toml(cargo_path: Path, new_version: str) -> None:
    content = cargo_path.read_text(encoding="utf-8")
    pattern = r'(?m)^(\s*version\s*=\s*")[^"]+(")'
    new_content, count = re.subn(pattern, rf"\g<1>{new_version}\g<2>", content, count=1)
    if count == 0:
        raise RuntimeError("Could not find 'version = ...' in Cargo.toml")
    cargo_path.write_text(new_content, encoding="utf-8")


def update_cargo_lock(lock_path: Path, new_version: str) -> None:
    if not lock_path.is_file():
        return
    content = lock_path.read_text(encoding="utf-8")
    pattern = r'(?m)(name = "earplugger"\r?\nversion = ")[^"]+(")'
    new_content, count = re.subn(pattern, rf"\g<1>{new_version}\g<2>", content, count=1)
    if count > 0:
        lock_path.write_text(new_content, encoding="utf-8")


def update_changelog(changelog_path: Path, new_version: str, release_notes_body: str) -> None:
    content = changelog_path.read_text(encoding="utf-8")
    today = datetime.date.today().isoformat()

    new_section_header = f"## [{new_version}] - {today}\n"
    
    body_lines = release_notes_body.splitlines()
    if body_lines and body_lines[0].startswith("## What's Changed"):
        body_lines = body_lines[1:]
    changelog_section_body = "\n".join(body_lines).strip()

    entry = f"{new_section_header}\n{changelog_section_body}\n\n"

    unreleased_idx = content.find("## [Unreleased]")
    if unreleased_idx != -1:
        insert_pos = content.find("\n", unreleased_idx)
        if insert_pos != -1:
            insert_pos += 1
            if insert_pos < len(content) and content[insert_pos] == "\n":
                insert_pos += 1
            new_content = content[:insert_pos] + entry + content[insert_pos:]
        else:
            new_content = content + "\n\n" + entry
    else:
        new_content = entry + content

    changelog_path.write_text(new_content, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Single-Workflow Release Automation Engine")
    parser.add_argument("--bump", choices=["auto", "patch", "minor", "major"], default="auto", help="SemVer bump type")
    parser.add_argument("--dry-run", action="store_true", help="Simulate release without modifying git or committing")
    parser.add_argument("--current-tag", help="Explicit current release tag if already tagged (e.g. on tag push)")
    parser.add_argument("--remote", default="origin", help="Git remote name to push release tags and commits")
    parser.add_argument("--repo-path", type=Path, default=Path("."), help="Path to repository root")
    parser.add_argument("--github-repo", default="johnathangallagher/earplugger", help="GitHub repo slug (owner/name)")
    args = parser.parse_args()

    repo_root = args.repo_path.resolve()
    cargo_toml = repo_root / "Cargo.toml"
    cargo_lock = repo_root / "Cargo.lock"
    changelog = repo_root / "CHANGELOG.md"

    if not cargo_toml.is_file():
        print(f"[-] Error: Cargo.toml not found at {cargo_toml}", file=sys.stderr)
        return 1

    cargo_content = cargo_toml.read_text(encoding="utf-8")
    m = re.search(r'(?m)^\s*version\s*=\s*"([^"]+)"', cargo_content)
    if not m:
        print("[-] Error: Could not determine current version from Cargo.toml", file=sys.stderr)
        return 1
    current_version = m.group(1)

    all_tags = get_tags(repo_root)

    if args.current_tag:
        new_tag = args.current_tag if args.current_tag.startswith("v") else f"v{args.current_tag}"
        new_version = new_tag.lstrip("v")
        # Find previous tag
        prev_tag = None
        if new_tag in all_tags:
            idx = all_tags.index(new_tag)
            if idx + 1 < len(all_tags):
                prev_tag = all_tags[idx + 1]
        elif all_tags:
            prev_tag = all_tags[0]
        commits = get_commits_between(repo_root, prev_tag, until_ref=new_tag)
        print(f"[*] Processing existing tag: {new_tag} (previous: {prev_tag or 'initial'}) with {len(commits)} commits")
    else:
        prev_tag = all_tags[0] if all_tags else None
        commits = get_commits_between(repo_root, prev_tag, until_ref="HEAD")
        print(f"[*] Current version: {current_version}")
        print(f"[*] Latest release tag: {prev_tag or 'None (initial release)'}")
        print(f"[*] Found {len(commits)} commits since {prev_tag or 'repository start'}")
        new_version = calculate_next_version(current_version, args.bump, commits)
        new_tag = f"v{new_version}"
        print(f"[*] Target release version: {new_version} (tag: {new_tag}) [bump={args.bump}]")

    release_notes = generate_release_notes(new_version, prev_tag, commits, repo=args.github_repo)

    notes_file = repo_root / "RELEASE_NOTES.md"
    notes_file.write_text(release_notes, encoding="utf-8")
    print(f"[+] Wrote release notes to {notes_file}")

    if args.dry_run or args.current_tag:
        if args.dry_run:
            print("\n=== DRY RUN: Generated Release Notes ===")
            print(release_notes)
            print("=== END DRY RUN ===")
            print("[*] Dry run enabled. Skipping file modifications, commit, tag, and push.")
    else:
        update_cargo_toml(cargo_toml, new_version)
        update_cargo_lock(cargo_lock, new_version)
        if changelog.is_file():
            update_changelog(changelog, new_version, release_notes)

        files_to_commit = ["Cargo.toml"]
        if cargo_lock.is_file():
            files_to_commit.append("Cargo.lock")
        if changelog.is_file():
            files_to_commit.append("CHANGELOG.md")

        run_git(["add"] + files_to_commit, cwd=repo_root)
        run_git(
            ["commit", "-m", f"chore: release {new_tag} [skip ci]"],
            cwd=repo_root,
        )
        print(f"[+] Committed version bump for {new_tag}")

        run_git(["tag", "-a", new_tag, "-m", f"Release {new_tag}"], cwd=repo_root)
        print(f"[+] Created git tag {new_tag}")

        try:
            run_git(["push", args.remote, "HEAD:main", "--tags"], cwd=repo_root)
            print(f"[+] Pushed commit and tag {new_tag} to {args.remote}")
        except subprocess.CalledProcessError as e:
            print(f"[-] Warning: Failed to push to remote '{args.remote}': {e.stderr}", file=sys.stderr)
            print("    Please ensure remote credentials and branch permissions are valid.")

    gh_output = os.environ.get("GITHUB_OUTPUT")
    if gh_output:
        with open(gh_output, "a", encoding="utf-8") as f:
            f.write(f"version={new_version}\n")
            f.write(f"tag={new_tag}\n")
            f.write(f"release_notes={notes_file.as_posix()}\n")
            f.write(f"dry_run={'true' if args.dry_run else 'false'}\n")

    return 0


if __name__ == "__main__":
    sys.exit(main())
