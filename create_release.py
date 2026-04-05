#!/usr/bin/env python3
"""
Create a GitHub release for MiiCut.

- Reads version from index.html (line `<div id="menuTitle">…MiiCut X.Y.Z</div>`)
- Checks that the release vX.Y.Z does not already exist on GitHub
- Creates it with auto-generated notes and attaches index.html.gz

Usage:
    python3 create_release.py
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parent
INDEX_HTML = ROOT / "index.html"
BUNDLE = ROOT / "index.html.gz"


def read_version() -> str:
    if not INDEX_HTML.exists():
        sys.exit(f"ERROR: {INDEX_HTML} not found")
    text = INDEX_HTML.read_text(encoding="utf-8")
    m = re.search(r'id="menuTitle"[^>]*>.*?MiiCut\s+(\d+\.\d+\.\d+)', text)
    if not m:
        sys.exit(f"ERROR: could not find MiiCut version in {INDEX_HTML}")
    return m.group(1)


def release_exists(tag: str) -> bool:
    result = subprocess.run(
        ["gh", "release", "view", tag],
        capture_output=True,
        text=True,
    )
    return result.returncode == 0


def main() -> None:
    version = read_version()
    tag = f"v{version}"
    print(f"Version detected: {version}  →  tag {tag}")

    if release_exists(tag):
        sys.exit(f"ERROR: release {tag} already exists on GitHub")

    print("Rebuilding bundle (index.html.gz) …")
    subprocess.run(
        ["python3", str(ROOT / "deploy_to_grblhal.py"), "--bundle"],
        check=True,
    )
    if not BUNDLE.exists():
        sys.exit(f"ERROR: bundle step did not produce {BUNDLE}")

    print(f"Creating release {tag} with {BUNDLE.name} …")
    subprocess.run(
        [
            "gh", "release", "create", tag,
            "--title", tag,
            "--generate-notes",
            str(BUNDLE),
        ],
        check=True,
    )
    print("Done.")


if __name__ == "__main__":
    main()
