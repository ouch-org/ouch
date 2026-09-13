# Draft a New Release

Use `scripts/draft-new-release.py x.y.z` to prepare the new release.

How the release pipeline roughly works:

- A temporary release branch `tmp/release/x.y.z-rcN` is created and pushed.
  - It contains 1 extra commit, the version bump.
- We tag the bump commit and push the tag
  - CI will detect the tag, build the artifacts and create the draft release.
- We review the draft release.
  - If failed, delete and do again with `tmp/release/x.y.z-rc(N+1)`.
  - If successful, publish and fast-forward main to the bump commit to persist.

Steps that are omitted can be found in exhaustive list of steps below.

## Start a release

```sh
scripts/draft-new-release.py x.y.z
```

<details>
<summary>The script will:</summary>

1. Check if the `git` repo is dirty, if so, report and ask for confirmation before proceeding.
2. Create and check out to `tmp/release/x.y.z-rcN`
  - `N` will be incremented automatically based on local and remote branch and tag references.
  - The same `N` is used for both the branch and the RC tag.
  - The script should show the branch name and ask for confirmation before proceeding.
3. Update the package version in `Cargo.toml`.
4. Run `cargo test --profile fast`, which may update `Cargo.lock`.
5. Commit `Cargo.toml` and `Cargo.lock` with message `bump version x.y.z`.
6. Push newly created branch to `origin`.
7. Create and push the next RC tag, such as `x.y.z-rc1`.
8. Print the GitHub Actions URL.

</details>

If the release is a hotfix and shouldn't go to `main`, the difference is that you should first create another branch where the commits will live, suggested name is `hotfix/x.y.z`, notice that you'll still use the script to create `tmp/release/x.y.z-rcN`, but you'll base it on and merge-fast-forward later to `hotfix/x.y.z`, not `main`.

## Test the draft release

1. Go to [GitHub Actions](https://github.com/ouch-org/ouch/actions) and wait for the release workflow.
2. Open the draft at [GitHub Releases](https://github.com/ouch-org/ouch/releases) for editing.
3. Download and test the assets, check the asset names, signatures and package version.
4. Create the release text.
5. If anything needs fixing, go back to your base branch and run the script again.

## Finalize the release

1. `git switch BASE_BRANCH` (either `main` or `hotfix/x.y.z`)
2. `git pull` (ensure up to date)
3. `git merge --ff-only tmp/release/x.y.z-rcN` (add the bump commit to `BASE_BRANCH`)
4. In GitHub, finish preparing the release.
5. `cargo publish --locked`.
6. `git push`.
7. Press `Create Release` inside GitHub.
8. Clean up dangling temporary branches.
