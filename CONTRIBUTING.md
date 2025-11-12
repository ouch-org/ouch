Thanks for your interest in contributing to `ouch`!

# Table of contents:

- [Code of Conduct](#code-of-conduct)
- [I want to ask a question or provide feedback](#i-want-to-ask-a-question-or-provide-feedback)
- [Adding a new feature](#adding-a-new-feature)
- [PRs](#prs)
- [Dealing with UI tests](#dealing-with-ui-tests)

## Code of Conduct

We follow the [Rust Official Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## I want to ask a question or provide feedback

Create [an issue](https://github.com/ouch-org/ouch/issues) or go to [Ouch Discussions](https://github.com/ouch-org/ouch/discussions).

## Adding a new feature

Before opening the PR, open an issue to discuss your addition, this increases the chance of your PR being accepted.

## PRs

- Pass all CI checks.
- After opening the PR, add a [CHANGELOG.md] entry.

[CHANGELOG.md]: https://github.com/ouch-org/ouch

### CI Tests

The CI tests will run for a combination of features, `--no-default-features` will also be tested.

## Dealing with UI tests

We use snapshots to do UI testing and guarantee a consistent output, this way, you can catch accidental changes or see what output changed in the PR diff.

- Run tests with `cargo` normally, or with a filter:

```sh
cargo test
# Or, if you only want to run UI tests
# cargo test -- ui
```

- If some UI test failed, you should review them (requires `cargo install cargo-insta`):

```sh
cargo insta review
```

- After addressing all, you should be able to `git add` and `commit` accordingly.

NOTE: Sometimes, you'll have to run these two commands multiple times.
