# Draft a New Release

Use `scripts/draft-new-release.py` to prepare a release-candidate tag and trigger the GitHub Actions release workflow.

## Checks

The script will fail if these requirements aren't met.

- Be on `main`.
- `main` must match `origin/main`.
- Do not have staged or unstaged tracked changes. Untracked files are ignored.
- Have Rust/Cargo available.

## Run the draft script

```sh
scripts/draft-new-release.py NEW_VERSION
```

Example:

```sh
scripts/draft-new-release.py 0.8.0
```

The version must be in `MAJOR.MINOR.PATCH` format.

The script will:

1. Update the top of `CHANGELOG.md`:
   - point `Unreleased` at `NEW_VERSION...HEAD`
   - add fresh empty sections for the next development cycle
   - create a `NEW_VERSION` changelog section comparing the previous version to `NEW_VERSION`
2. Ask you to review `CHANGELOG.md`.
   - You should enter `y` to proceed.
3. Update the package version in `Cargo.toml`.
4. Run `cargo test --profile fast`, which will also update `Cargo.lock`.
5. Commit `CHANGELOG.md`, `Cargo.toml`, and `Cargo.lock` with:
   - Message: `"bump version NEW_VERSION"`.
6. Create new release candidate tag, like `NEW_VERSION-rc1`, `NEW_VERSION-rc2`, etc.
7. Push tags.
8. Print the GitHub Actions URL.

## After the script runs

1. Go to GitHub Actions:
   <https://github.com/ouch-org/ouch/actions>
2. Wait for the release workflow triggered by the RC tag.
3. Go to GitHub Releases:
   <https://github.com/ouch-org/ouch/releases>
4. Open the drafted release for the RC tag.
5. Continue polishing the release notes/changelog if needed.
6. Publish to crates.io:
   ```sh
   cargo publish
   ```
7. Push the version bump commit to `main` with `git push` (the script creates the commit, but only pushes the RC tag).
8. In GitHub, edit the release:
   - mark it as the final release instead of a pre-release
   - confirm the title/body/assets are correct
9. Click **Release**.
