#!/usr/bin/env python3
# pyright: reportUnusedCallResult=false

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import NoReturn, cast

VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")

def die(message: str) -> NoReturn:
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(1)

def run(*args: str, capture: bool = False) -> str:
    result = subprocess.run(
        args,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        die(f"Command failed: {' '.join(args)}")
    return result.stdout.strip() if capture else ""

def succeeds(*args: str) -> bool:
    return (
        subprocess.run(
            args,
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode
        == 0
    )

def repo_root() -> Path:
    return Path(run("git", "rev-parse", "--show-toplevel", capture=True))

def confirm(message: str) -> bool:
    try:
        answer = input(f"{message} [y/N]: ").strip().lower()
    except EOFError:
        return False
    return answer in {"y", "yes"}

def create_release_branch(version: str, rc_number: int) -> str:
    current_branch = run("git", "branch", "--show-current", capture=True)
    if not current_branch:
        die("Must be on a branch before creating a temporary release branch")

    release_branch = f"tmp/release/{version}-rc{rc_number}"
    print(f"Temporary release branch: {release_branch}")
    if not confirm(
        f"Create '{release_branch}' from '{current_branch}' and switch to it"
    ):
        die("Release branch creation aborted")

    if succeeds("git", "ls-remote", "--exit-code", "--heads", "origin", release_branch):
        die(f"Remote branch '{release_branch}' already exists")

    run("git", "switch", "--create", release_branch)
    return release_branch

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

def confirm_working_tree_changes() -> None:
    status = run("git", "status", "--short", capture=True)
    if not status:
        return

    print("Working tree has staged, unstaged, or untracked changes:")
    print(status)
    if not confirm("Continue with these changes present"):
        die("Release creation aborted")

def confirm_release_script_changes() -> None:
    diff = run(
        "git", "diff", "--no-ext-diff", "--no-color", "origin/main", "--",
        ".github", "scripts", capture=True,
    )
    if not diff:
        return

    print("There is a diff in .github or scripts compared to origin/main:")
    print(diff)
    if not confirm("Continue with these differences present"):
        die("Release creation aborted")

def remote_tags(pattern: str) -> list[str]:
    refs = run(
        "git", "ls-remote", "--tags", "origin", pattern, capture=True
    ).splitlines()
    tags: list[str] = []

    for ref in refs:
        tag = ref.rsplit("refs/tags/", maxsplit=1)[-1].removesuffix("^{}")
        tags.append(tag)

    return tags

def remote_branches(pattern: str) -> list[str]:
    refs = run(
        "git", "ls-remote", "--heads", "origin", pattern, capture=True
    ).splitlines()
    return [ref.rsplit("refs/heads/", maxsplit=1)[-1] for ref in refs]

def final_tag_exists(version: str) -> bool:
    return bool(remote_tags(version)) or succeeds(
        "git", "rev-parse", "--verify", "--quiet", f"refs/tags/{version}"
    )

def next_rc_number(version: str) -> int:
    branch_prefix = f"tmp/release/{version}-rc"
    branches = set(
        run(
            "git",
            "for-each-ref",
            "--format=%(refname:short)",
            f"refs/heads/{branch_prefix}*",
            capture=True,
        ).splitlines()
    )
    branches.update(remote_branches(f"{branch_prefix}*"))

    tag_prefix = f"{version}-rc"
    tags = set(run("git", "tag", "--list", f"{tag_prefix}*", capture=True).splitlines())
    tags.update(remote_tags(f"{tag_prefix}*"))

    rc_numbers: list[int] = []
    for refs, prefix in ((branches, branch_prefix), (tags, tag_prefix)):
        for ref in refs:
            match = re.fullmatch(rf"{re.escape(prefix)}([0-9]+)", ref)
            if match:
                rc_numbers.append(int(match.group(1)))

    return max(rc_numbers, default=0) + 1

def parse_args() -> str:
    parser = argparse.ArgumentParser()
    parser.add_argument("version", help="version like 1.0.0")
    version = cast(str, parser.parse_args().version)

    if not VERSION_RE.fullmatch(version):
        die(f"Invalid version '{version}'. Expected format like 1.0.0")

    return version

def main() -> None:
    version = parse_args()
    root = repo_root()
    os.chdir(root)

    confirm_working_tree_changes()
    confirm_release_script_changes()
    if final_tag_exists(version):
        die(f"Final release tag '{version}' already exists")

    rc_number = next_rc_number(version)
    release_branch = create_release_branch(version, rc_number)
    update_cargo_toml(version)
    run("cargo", "test", "--profile", "fast")

    run("git", "add", "Cargo.lock", "Cargo.toml")

    if succeeds("git", "diff", "--cached", "--quiet", "--", "Cargo.lock", "Cargo.toml"):
        die("Version bump produced no changes to commit")

    run(
        "git",
        "commit",
        "-m",
        f"bump version {version}",
        "--",
        "Cargo.lock",
        "Cargo.toml",
    )

    run("git", "push", "--set-upstream", "origin", release_branch)
    tag = f"{version}-rc{rc_number}"
    run("git", "tag", tag)
    run("git", "push", "origin", tag)
    print(f"Pushed branch: {release_branch}")
    print(f"Pushed tag: {tag}")
    print("GitHub Actions: https://github.com/ouch-org/ouch/actions")

if __name__ == "__main__":
    main()
