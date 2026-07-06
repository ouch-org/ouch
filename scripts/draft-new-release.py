#!/usr/bin/env python3
import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")


def die(message: str) -> None:
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(1)


def run(*args: str, capture: bool = False) -> str:
    result = subprocess.run(
        args,
        text=True,
        stdout=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        die(f"Command failed: {' '.join(args)}")
    return result.stdout.strip() if capture else ""


def succeeds(*args: str) -> bool:
    return (
        subprocess.run(
            args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        ).returncode
        == 0
    )


def repo_root() -> Path:
    return Path(run("git", "rev-parse", "--show-toplevel", capture=True))


def ensure_on_origin_main() -> None:
    branch = run("git", "branch", "--show-current", capture=True)
    if branch != "main":
        die(f"Must be on main branch; currently on '{branch}'")

    run("git", "fetch", "origin", "main")

    if not succeeds("git", "rev-parse", "--verify", "origin/main"):
        die("Could not find origin/main")

    if not succeeds("git", "merge-base", "--is-ancestor", "origin/main", "HEAD"):
        die(
            "HEAD is behind or has diverged from origin/main. Pull/rebase before bumping the version."
        )


def update_cargo_toml(version: str) -> None:
    path = Path("Cargo.toml")
    text = path.read_text()
    new_text, count = re.subn(
        r'(?s)(\[package\]\n.*?^version = ")[^"]+(")',
        rf"\g<1>{version}\2",
        text,
        count=1,
        flags=re.MULTILINE,
    )
    if count != 1:
        die("Could not update package version in Cargo.toml")
    path.write_text(new_text)


def ensure_no_tracked_changes() -> None:
    status = run("git", "status", "--short", capture=True)
    tracked_changes = [
        line for line in status.splitlines() if not line.startswith("?? ")
    ]
    if tracked_changes:
        print("\n".join(tracked_changes))
        die(
            "Working tree has staged or unstaged tracked changes. Commit or stash them before drafting a release."
        )


def remote_tags(pattern: str) -> list[str]:
    refs = run(
        "git", "ls-remote", "--tags", "origin", pattern, capture=True
    ).splitlines()
    tags = []

    for ref in refs:
        tag = ref.rsplit("refs/tags/", maxsplit=1)[-1]
        if tag.endswith("^{}"):
            tag = tag[:-3]
        tags.append(tag)

    return tags


def next_rc_tag(version: str) -> str:
    pattern = f"{version}-rc*"
    tags = set(run("git", "tag", "--list", pattern, capture=True).splitlines())
    tags.update(remote_tags(pattern))
    rc_numbers = []
    rc_re = re.compile(rf"^{re.escape(version)}-rc([0-9]+)$")

    for tag in tags:
        match = rc_re.fullmatch(tag)
        if match:
            rc_numbers.append(int(match.group(1)))

    return f"{version}-rc{max(rc_numbers, default=0) + 1}"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("version", help="version like 1.0.0")
    args = parser.parse_args()

    if not VERSION_RE.fullmatch(args.version):
        die(f"Invalid version '{args.version}'. Expected format like 1.0.0")

    return args


def main() -> None:
    args = parse_args()
    root = repo_root()
    os.chdir(root)

    ensure_on_origin_main()
    ensure_no_tracked_changes()
    tag = next_rc_tag(args.version)
    update_cargo_toml(args.version)
    run("cargo", "test", "--profile", "fast")
    run("git", "add", "Cargo.lock", "Cargo.toml")
    run("git", "commit", "-m", f"bump version {args.version}")
    run("git", "tag", tag)
    run("git", "push", "origin", tag)
    print(f"Pushed tag: {tag}")
    print("GitHub Actions: https://github.com/ouch-org/ouch/actions")


if __name__ == "__main__":
    main()
