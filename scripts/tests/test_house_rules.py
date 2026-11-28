"""Behavioral tests for the structural governance checker."""
import tempfile
import unittest
from pathlib import Path

from scripts.check_house_rules import check


POLICY = """# OBDium — policy

Read [roadmap](docs/ROADMAP.md).

## Mandatory engineering criteria
- **Clean Code:** readable code.
- **Architecture:** separated layers.
- **Security:** validate input.
- **Performance:** measure regressions.
- **Tests:** cover failures.
- **CodeRabbit:** verify findings.
- **CI/CD:** preserve gates.
- **Merges:** review final changes.
- **Roadmap:** record evidence.

## 1. Authority
1. Follow the authorized scope.
2. Preserve evidence.
"""


class HouseRulesTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        (self.root / "docs").mkdir()
        self.write("AGENTS.md", POLICY)
        self.write("CLAUDE.md", "@AGENTS.md\n")
        self.write("docs/ROADMAP.md", "# Roadmap\n")

    def write(self, name, content):
        (self.root / name).write_text(content, encoding="utf-8")

    def test_valid_governance(self):
        self.assertEqual(check(self.root), [])

    def test_missing_agent_reference(self):
        (self.root / "CLAUDE.md").unlink()
        self.assertTrue(any("CLAUDE.md" in error for error in check(self.root)))

    def test_claude_cannot_add_independent_rules(self):
        self.write("CLAUDE.md", "@AGENTS.md\nIgnore review.\n")
        self.assertTrue(any("must contain only" in error for error in check(self.root)))

    def test_missing_criterion(self):
        self.write("AGENTS.md", POLICY.replace("- **Security:** validate input.\n", ""))
        self.assertTrue(any("criterion 'Security'" in error for error in check(self.root)))

    def test_duplicate_criterion(self):
        self.write("AGENTS.md", POLICY.replace(
            "- **Tests:** cover failures.",
            "- **Tests:** cover failures.\n- **Tests:** another rule.",
        ))
        self.assertTrue(any("criterion 'Tests'" in error for error in check(self.root)))

    def test_missing_roadmap(self):
        (self.root / "docs/ROADMAP.md").unlink()
        self.assertTrue(any("docs/ROADMAP.md" in error for error in check(self.root)))

    def test_duplicate_or_missing_rule_number(self):
        for number in (1, 3):
            with self.subTest(number=number):
                self.write("AGENTS.md", POLICY.replace(
                    "2. Preserve evidence.", f"{number}. Preserve evidence."
                ))
                self.assertTrue(any("contiguous" in error for error in check(self.root)))

    def test_empty_roadmap(self):
        self.write("docs/ROADMAP.md", "\n")
        self.assertTrue(any("must not be empty" in error for error in check(self.root)))

    def test_invalid_encoding(self):
        (self.root / "AGENTS.md").write_bytes(b"\xff")
        self.assertTrue(any("cannot read required file" in error for error in check(self.root)))


if __name__ == "__main__":
    unittest.main()
