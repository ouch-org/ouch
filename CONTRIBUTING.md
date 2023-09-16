Thanks for your interest in contributing to `ouch`!

# Code of Conduct

We follow the [Rust Official Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

# I want to ask a question or provide feedback

Create [an issue](https://github.com/ouch-org/ouch/issues) or go to [Ouch Discussions](https://github.com/ouch-org/ouch/discussions).

# Adding a brand new feature

Before opening the PR, open an issue to discuss your addition, this increases the chance of your PR being accepted.

# PRs

- Pass all CI checks.
- After opening the PR, add a [CHANGELOG.md] entry.

# Updating UI tests

In case you need to update the UI tests.

- Run tests with `insta` to create the new snapshots:

```sh
cargo insta review # or
cargo insta review -- ui # useful filter
```

- Now, review the diffs you just generated.

```sh
cargo insta review
```

- You can commit them now.

[CHANGELOG.md]: https://github.com/ouch-org/ouch
