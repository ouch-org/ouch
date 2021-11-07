Thanks for your interest in contributing to `ouch`!

Feel free to open an issue anytime you wish to ask a question, suggest a feature, report a bug, etc.

# Requirements

1. Be nice to other people.
2. If editing the Rust source code, remember to run `rustfmt` (otherwise, CI will warn you the code was not properly formatted).
3. If new formats are added, please add the format to `tests/integration.rs`.
If it is an archive format that handles directories, it should be added to `DirectoryExtension`, otherwise it should be added to `FileExtension`.
It should be added to `mime.rs` as well if the [`infer`](https://docs.rs/infer) crate supports it.
Most tests are written with `proptest` ([book](https://altsysrq.github.io/proptest-book/), [docs](https://docs.rs/proptest)).
If you wish to improve these tests, the proptest book might help you.

Note: we are using `unstable` features of `rustfmt`! Nightly toolchain is required (will likely be installed automatically, cause the toolchain was specified in the project root).

# Suggestions

1. If you wish to, you can ask for some guidance before solving an issue.
2. Run `cargo clippy` too.
