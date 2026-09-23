#!/usr/bin/env python3
"""
test_release.py - Unit tests for release automation engine
"""

from pathlib import Path
import tempfile
import unittest

from release import (
    Commit,
    calculate_next_version,
    categorize_commits,
    generate_release_notes,
    parse_semver,
    update_cargo_lock,
    update_cargo_toml,
    update_changelog,
)


class TestReleaseEngine(unittest.TestCase):
    def test_parse_semver_valid(self):
        self.assertEqual(parse_semver("1.2.3"), (1, 2, 3))
        self.assertEqual(parse_semver("v1.2.3"), (1, 2, 3))
        self.assertEqual(parse_semver("  0.10.4  "), (0, 10, 4))

    def test_parse_semver_invalid(self):
        with self.assertRaises(ValueError):
            parse_semver("invalid")
        with self.assertRaises(ValueError):
            parse_semver("1.2")

    def test_calculate_next_version_explicit(self):
        self.assertEqual(calculate_next_version("1.3.0", "patch", []), "1.3.1")
        self.assertEqual(calculate_next_version("1.3.0", "minor", []), "1.4.0")
        self.assertEqual(calculate_next_version("1.3.0", "major", []), "2.0.0")

    def test_calculate_next_version_auto(self):
        # Patch only
        commits_patch = [
            Commit("1111111", "fix: resolve edge case", "", "fix", None, False, "resolve edge case"),
            Commit("2222222", "docs: update readme", "", "docs", None, False, "update readme"),
        ]
        self.assertEqual(calculate_next_version("1.3.0", "auto", commits_patch), "1.3.1")

        # Minor bump when feat is present
        commits_minor = [
            Commit("1111111", "fix: resolve bug", "", "fix", None, False, "resolve bug"),
            Commit("3333333", "feat: add automatic restart hook", "", "feat", None, False, "add automatic restart hook"),
        ]
        self.assertEqual(calculate_next_version("1.3.0", "auto", commits_minor), "1.4.0")

        # Major bump when breaking change is present
        commits_major_bang = [
            Commit("4444444", "feat!: overhaul api interface", "", "feat", None, True, "overhaul api interface"),
        ]
        self.assertEqual(calculate_next_version("1.3.0", "auto", commits_major_bang), "2.0.0")

        commits_major_body = [
            Commit("5555555", "fix: rework arguments", "BREAKING CHANGE: changes flag syntax", "fix", None, True, "rework arguments"),
        ]
        self.assertEqual(calculate_next_version("1.3.0", "auto", commits_major_body), "2.0.0")

    def test_categorize_commits(self):
        commits = [
            Commit("aaa1111", "feat(cli): add status command", "", "feat", "cli", False, "add status command"),
            Commit("bbb2222", "fix: memory leak in ffi", "", "fix", None, False, "memory leak in ffi"),
            Commit("ccc3333", "perf: optimize snapshot query", "", "perf", None, False, "optimize snapshot query"),
            Commit("ddd4444", "chore(deps): update actions", "", "chore", "deps", False, "update actions"),
            Commit("eee5555", "random non-conventional commit", "", None, None, False, "random non-conventional commit"),
        ]
        cat = categorize_commits(commits)
        self.assertEqual(len(cat["Features"]), 1)
        self.assertEqual(len(cat["Bug Fixes"]), 1)
        self.assertEqual(len(cat["Performance Improvements"]), 1)
        self.assertEqual(len(cat["Maintenance & CI"]), 1)
        self.assertEqual(len(cat["Other Changes"]), 1)

    def test_generate_release_notes(self):
        commits = [
            Commit("1234567890abcdef", "feat(audio): support voicemeeter potato", "", "feat", "audio", False, "support voicemeeter potato", "John Gallagher", "178059587+johnathangallagher@users.noreply.github.com"),
            Commit("abcdef1234567890", "fix: prevent task XML entity injection", "", "fix", None, False, "prevent task XML entity injection", "John Gallagher", "johnathangallagher@users.noreply.github.com"),
        ]
        notes = generate_release_notes("1.4.0", "v1.3.0", commits, repo="johnathangallagher/earplugger")
        self.assertNotIn("## What's Changed in", notes)
        self.assertIn("### Features", notes)
        self.assertIn("- **audio**: support voicemeeter potato ([`1234567`](https://github.com/johnathangallagher/earplugger/commit/1234567890abcdef))", notes)
        self.assertIn("### Bug Fixes", notes)
        self.assertIn("- prevent task XML entity injection ([`abcdef1`](https://github.com/johnathangallagher/earplugger/commit/abcdef1234567890))", notes)
        self.assertIn("### Contributors", notes)
        self.assertIn("@johnathangallagher", notes)
        self.assertIn("https://github.com/johnathangallagher/earplugger/compare/v1.3.0...v1.4.0", notes)

    def test_update_cargo_toml(self):
        sample = """[package]
name = "earplugger"
version = "1.3.0"
edition = "2024"
"""
        with tempfile.TemporaryDirectory() as tmpdir:
            p = Path(tmpdir) / "Cargo.toml"
            p.write_text(sample, encoding="utf-8")
            update_cargo_toml(p, "1.4.0")
            updated = p.read_text(encoding="utf-8")
            self.assertIn('version = "1.4.0"', updated)
            self.assertNotIn('version = "1.3.0"', updated)
            self.assertIn('edition = "2024"', updated)

    def test_update_cargo_lock(self):
        sample = """# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "earplugger"
version = "1.3.0"
"""
        with tempfile.TemporaryDirectory() as tmpdir:
            p = Path(tmpdir) / "Cargo.lock"
            p.write_text(sample, encoding="utf-8")
            update_cargo_lock(p, "1.4.0")
            updated = p.read_text(encoding="utf-8")
            self.assertIn('version = "1.4.0"', updated)
            self.assertNotIn('version = "1.3.0"', updated)

    def test_update_changelog(self):
        sample = """# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- Pending changes here.

## [1.3.0] - 2026-09-18

### Features
- Initial feature.
"""
        with tempfile.TemporaryDirectory() as tmpdir:
            p = Path(tmpdir) / "CHANGELOG.md"
            p.write_text(sample, encoding="utf-8")
            body = "### Features\n- New capability added."
            update_changelog(p, "1.4.0", body)
            updated = p.read_text(encoding="utf-8")
            self.assertIn("## [1.4.0] - ", updated)
            self.assertIn("### Features\n- New capability added.", updated)
            # Ensure it is inserted before [1.3.0]
            pos_140 = updated.find("## [1.4.0]")
            pos_130 = updated.find("## [1.3.0]")
            self.assertTrue(pos_140 < pos_130)


if __name__ == "__main__":
    unittest.main()
