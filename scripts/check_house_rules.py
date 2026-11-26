"""Check the machine-verifiable OBDium governance contract.

This is not a semantic review, security scanner or proof of engineering quality.
"""
import re
import sys
from pathlib import Path

CRITERIA = (
    "Clean Code", "Architecture", "Security", "Performance", "Tests",
    "CodeRabbit", "CI/CD", "Merges", "Roadmap",
)


def check(root: Path) -> list[str]:
    errors = []
    documents = {}
    for name in ("AGENTS.md", "CLAUDE.md", "docs/ROADMAP.md"):
        try:
            documents[name] = (root / name).read_text(encoding="utf-8")
        except (OSError, UnicodeError) as exc:
            errors.append(f"{name}: cannot read required file: {exc}")

    if "CLAUDE.md" in documents and documents["CLAUDE.md"] != "@AGENTS.md\n":
        errors.append("CLAUDE.md must contain only @AGENTS.md and a final newline.")

    if "AGENTS.md" in documents:
        policy = documents["AGENTS.md"]
        if not policy.startswith("# OBDium"):
            errors.append("AGENTS.md must identify the OBDium policy.")
        if "(docs/ROADMAP.md)" not in policy:
            errors.append("AGENTS.md must link to the canonical roadmap.")
        sections = re.split(r"(?m)^## ", policy)
        matches = [
            section for section in sections[1:]
            if section.splitlines()[0] == "Mandatory engineering criteria"
        ]
        if len(matches) != 1:
            errors.append("AGENTS.md must have exactly one mandatory criteria section.")
        else:
            labels = re.findall(r"(?m)^- \*\*([^*]+):\*\* ", matches[0])
            for name in CRITERIA:
                if labels.count(name) != 1:
                    errors.append(f"Mandatory criterion {name!r} must occur exactly once.")
        numbers = [
            int(number) for number in re.findall(r"(?m)^(\d+)\. \S", policy)
        ]
        if not numbers or numbers != list(range(1, len(numbers) + 1)):
            errors.append("Numbered engineering rules must be contiguous, starting at 1.")

    if "docs/ROADMAP.md" in documents and not documents["docs/ROADMAP.md"].strip():
        errors.append("The canonical roadmap must not be empty.")
    return errors


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    errors = check(root)
    if errors:
        for error in errors:
            print(f"House Rules: {error}", file=sys.stderr)
        return 1
    print("House Rules: structural governance checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
