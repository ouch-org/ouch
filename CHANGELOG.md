# Changelog

All notable user-facing changes to Ouch should be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Categories Used:

- New Features - new features added to ouch itself, not CI
- Bug Fixes
- Improvements - general enhancements
- Tweaks - anything that doesn't fit into other categories, small typo fixes, most CI stuff,
  meta changes (e.g. README updates), etc.
- Removals - removal of a feature (and most likely a breaking change)

**Bullet points in chronological order by PR**

## [Unreleased](https://github.com/ouch-org/ouch/compare/0.6.1...HEAD)

### Removals

- Remove archive auto-flattening on decompression (formerly known as "smart unpack") (https://github.com/ouch-org/ouch/pull/907)

### New Features

- Merge folders in decompression (https://github.com/ouch-org/ouch/pull/798)
- Provide Nushell completions (packages still need to install them) (https://github.com/ouch-org/ouch/pull/827)
- Add aliases for comic book archives (https://github.com/ouch-org/ouch/pull/835)
- Support `.lz` decompression (https://github.com/ouch-org/ouch/pull/838)
- Support `.lzma` decompression (and fix `.lzma` being a wrong alias for `.xz`) (https://github.com/ouch-org/ouch/pull/838)
- Support `.lz` compression (https://github.com/ouch-org/ouch/pull/867)
- Support `.lzma` compression (https://github.com/ouch-org/ouch/pull/867)

### Improvements

- Give better error messages when archive extensions are invalid (https://github.com/ouch-org/ouch/pull/817)
- Improve misleading error message (https://github.com/ouch-org/ouch/pull/818)
- Add aliases for `--password` flag (`--pass` and `--pw`) (https://github.com/ouch-org/ouch/pull/847)
- Avoid loading entire 7z archive into memory when listing (https://github.com/ouch-org/ouch/pull/860)
- Use `lzma-rust2` crate instead of `liblzma` crate (https://github.com/ouch-org/ouch/pull/867)
- Add `info!`, `info_accessible!` and `warning!` macros (https://github.com/ouch-org/ouch/pull/874)
- Handle --quiet logic in logger (https://github.com/ouch-org/ouch/pull/885)
- Refactor/simplify logger (https://github.com/ouch-org/ouch/pull/888)

### Bug Fixes

- Fix 7z BadSignature error when compressing and then listing (https://github.com/ouch-org/ouch/pull/819)
- Fix tar extraction count when --quiet (https://github.com/ouch-org/ouch/pull/824)
- Fix unpacking with merge flag failing without --dir flag (https://github.com/ouch-org/ouch/pull/826)
- Handle broken symlinks in zip archives and normalize path separators (https://github.com/ouch-org/ouch/pull/841)
- Fix folder softlink is not preserved after packing (https://github.com/ouch-org/ouch/pull/850)
- Handle read-only directories in tar extraction (https://github.com/ouch-org/ouch/pull/873)
- Fix tar hardlink is not preserved after decompressing or compressing (https://github.com/ouch-org/ouch/pull/879)
- Fix enable gitignore flag should work without git (https://github.com/ouch-org/ouch/pull/881)
- Fix .7z always being fully loaded to memory (https://github.com/ouch-org/ouch/pull/905)

### Tweaks

- Make `.bz3` opt-out (https://github.com/ouch-org/ouch/pull/814)
- Adjust INFO color to green (https://github.com/ouch-org/ouch/pull/858)
- Sevenz rust2 bump to 0.19.1 (https://github.com/ouch-org/ouch/pull/875)
- Bump gzp from 0.11.3 to 2.0.0 (https://github.com/ouch-org/ouch/pull/876)
- Rename output when input is stdin (https://github.com/ouch-org/ouch/pull/903)

## [0.6.1](https://github.com/ouch-org/ouch/compare/0.6.0...0.6.1)

- Fix .zip crash when file mode isn't present (https://github.com/ouch-org/ouch/pull/804)

## [0.6.0](https://github.com/ouch-org/ouch/compare/0.5.1...0.6.0)

### New Features

- Add multithreading support for `zstd` compression (https://github.com/ouch-org/ouch/pull/689)
- Add `bzip3` support (https://github.com/ouch-org/ouch/pull/522)
- Add `--remove` flag for decompression subcommand to remove files after successful decompression (https://github.com/ouch-org/ouch/pull/757)
- Add `br` (Brotli) support (https://github.com/ouch-org/ouch/pull/765)
- Add rename option in overwrite menu (https://github.com/ouch-org/ouch/pull/779)
- Store symlinks by default and add `--follow-symlinks` to store the target files (https://github.com/ouch-org/ouch/pull/789)

### Bug Fixes

- Fix output corrupted on parallel decompression (https://github.com/ouch-org/ouch/pull/642)

### Tweaks

- CI refactor (https://github.com/ouch-org/ouch/pull/578)
- Use a prefix `tmp-ouch-` for temporary decompression path name to avoid conflicts (https://github.com/ouch-org/ouch/pull/725 and https://github.com/ouch-org/ouch/pull/788)
- Ignore `.git/` when `-g/--gitignore` is set (https://github.com/ouch-org/ouch/pull/507)
- Run clippy for tests too (https://github.com/ouch-org/ouch/pull/738)
- Sevenz-rust is unmaintained, switch to sevenz-rust2 (https://github.com/ouch-org/ouch/pull/796)

### Improvements

- Fix logging IO bottleneck (https://github.com/ouch-org/ouch/pull/642)
- Support decompression over stdin (https://github.com/ouch-org/ouch/pull/692)
- Make `--format` more forgiving with the formatting of the provided format (https://github.com/ouch-org/ouch/pull/519)
- Use buffered writer for list output (https://github.com/ouch-org/ouch/pull/764)
- Disable smart unpack when `--dir` flag is provided in decompress command (https://github.com/ouch-org/ouch/pull/782)
- Align file sizes at left for each extracted file to make output clearer (https://github.com/ouch-org/ouch/pull/792)

## [0.5.1](https://github.com/ouch-org/ouch/compare/0.5.0...0.5.1)

### Improvements

- Explicitly declare feature flags `use_zlib` & `use_zstd_thin` (https://github.com/ouch-org/ouch/pull/564)

### Tweaks

- Mention support for `7z` and `rar` in help message.

## [0.5.0](https://github.com/ouch-org/ouch/compare/0.4.2...0.5.0)

### New Features

- Add support for listing and decompressing `.rar` archives (https://github.com/ouch-org/ouch/pull/529)
- Add support for 7z (https://github.com/ouch-org/ouch/pull/555) ([Flat](https://github.com/flat))

### Bug Fixes

- Fix mime type detection (https://github.com/ouch-org/ouch/pull/529)
- Fix size unit inconsistency (https://github.com/ouch-org/ouch/pull/502)

### Improvements

- Hint completions generator to expand file paths (https://github.com/ouch-org/ouch/pull/508)

## [0.4.2](https://github.com/ouch-org/ouch/compare/0.4.1...0.4.2)

### New Features

- Add flags to configure the compression level
  - `--level` to precisely set the compression level (https://github.com/ouch-org/ouch/pull/372)
  - `--fast` and `--slow` (https://github.com/ouch-org/ouch/pull/374)
- Add `--format` option (https://github.com/ouch-org/ouch/pull/341)

### Improvements

- Multi-threaded compression for gzip and snappy using gzp (https://github.com/ouch-org/ouch/pull/348)
- Add `ls` as an alternative alias for listing (https://github.com/ouch-org/ouch/pull/360)

### Bug Fixes

- Fix decompression of zip archives with files larger than 4GB (https://github.com/ouch-org/ouch/pull/354)
- Fix handling of unknown extensions during decompression (https://github.com/ouch-org/ouch/pull/355)
- Remove remaining mentions of `.lz` that refers to the LZMA format (https://github.com/ouch-org/ouch/pull/344)
- Handle Zip when modification times are missing (https://github.com/ouch-org/ouch/pull/433)

## [0.4.1](https://github.com/ouch-org/ouch/compare/0.4.0...0.4.1)

### New Features

- Add cli option to (de)compress quietly (https://github.com/ouch-org/ouch/pull/325)

### Improvements

- Allow ouch to decompress archive into existing folder (https://github.com/ouch-org/ouch/pull/321)
- Accept inserting subcommand-independent flags in any position (https://github.com/ouch-org/ouch/pull/329)
- Improve extension parsing logic (https://github.com/ouch-org/ouch/pull/330)
- Slight refactor when ensuring archive-only inputs (https://github.com/ouch-org/ouch/pull/331)
- Use BStr to display possibly non-UTF8 byte sequences (https://github.com/ouch-org/ouch/pull/332)
- Use ubyte instead of humansize (https://github.com/ouch-org/ouch/pull/333)
- Stop keeping track of the names of unpacked files (https://github.com/ouch-org/ouch/pull/334)
- Clean up (https://github.com/ouch-org/ouch/pull/335)

### Bug fixes

- Stop incorrectly asking to remove the parent dir (https://github.com/ouch-org/ouch/pull/321)

### Tweaks

- Add scoop install instructions to readme (https://github.com/ouch-org/ouch/pull/323)

## [0.4.0](https://github.com/ouch-org/ouch/compare/0.3.1...0.4.0) (2022-11-20)

### New Features

- Add release-helper.sh to make github releases easier (https://github.com/ouch-org/ouch/pull/146)
- Add support for lz4 (https://github.com/ouch-org/ouch/pull/150)
- add supported formats to help message (https://github.com/ouch-org/ouch/pull/189)
- add link to github to help message (https://github.com/ouch-org/ouch/pull/191)
- Update to Rust 2021 edition (https://github.com/ouch-org/ouch/pull/192)
- Implement accessibility mode (https://github.com/ouch-org/ouch/pull/197)
- Add heuristics to decompressing archives (https://github.com/ouch-org/ouch/pull/209)
- Add progress bar to compressing/decompressing (https://github.com/ouch-org/ouch/pull/210)
- Support snappy format (https://github.com/ouch-org/ouch/pull/215)
- Allow ignoring hidden files and files matched by .gitignore files (https://github.com/ouch-org/ouch/pull/245)
- Automatically generate man pages with clap_mangen (https://github.com/ouch-org/ouch/pull/273)
- Set last modified time during zip compression (https://github.com/ouch-org/ouch/pull/279)

### Bug Fixes

- Perform exhaustive matching on error variants (https://github.com/ouch-org/ouch/pull/147)
- Fix short flag for the --dir flag (https://github.com/ouch-org/ouch/pull/149)
- Rewrite tests (https://github.com/ouch-org/ouch/pull/163)
- switch from lz4_flex to lzzzz, enable lz4 tests (https://github.com/ouch-org/ouch/pull/173)
- Fix error message panic when cannot list non-archive files (https://github.com/ouch-org/ouch/pull/182)
- Fix not overwriting files/dirs when trying to create a dir (https://github.com/ouch-org/ouch/pull/190)
- Skip compressing file if its the same file as the output (https://github.com/ouch-org/ouch/pull/193)
- Fix warnings in doc comments (https://github.com/ouch-org/ouch/pull/196)
- Remove Lzip because its incorrect, and improve extension comparison (https://github.com/ouch-org/ouch/pull/198)
- Fix error with format infer (https://github.com/ouch-org/ouch/pull/205)
- Truncate long messages in the progress bar (https://github.com/ouch-org/ouch/pull/214)
- Fix zip memory warnings (https://github.com/ouch-org/ouch/pull/217)
- Fix the hint suggestion for compressing multiple files (https://github.com/ouch-org/ouch/pull/219)
- Simple eprintln fixes (https://github.com/ouch-org/ouch/pull/226)
- Actually use relative paths when extracting (https://github.com/ouch-org/ouch/pull/229)
- Mark directories when compressing to zip regardless of their contents (https://github.com/ouch-org/ouch/pull/230)
- Recover last modified time when unpacking zip archives (https://github.com/ouch-org/ouch/pull/250)
- Remove single quotes from clap doc comments (https://github.com/ouch-org/ouch/pull/251)
- Fix incorrect warnings for decompression (https://github.com/ouch-org/ouch/pull/270)
- Fix infinite compression if output file is inside the input folder (https://github.com/ouch-org/ouch/pull/288)
- Fix not overwriting a folder when compressing (https://github.com/ouch-org/ouch/pull/295)
- Check for EOF when asking questions (https://github.com/ouch-org/ouch/pull/311)

### Improvements

- Infer file extension when decompressing (https://github.com/ouch-org/ouch/pull/154)
- Extension: Use hardcoded slices instead of `Vecs` when creating an `Extension` (https://github.com/ouch-org/ouch/pull/155)
- Avoid allocating in `nice_directory_display` when possible, make `Extension` non-exhaustive (https://github.com/ouch-org/ouch/pull/156)
- Optimize `strip_cur_dir` (https://github.com/ouch-org/ouch/pull/167)
- Improve zip errors when paths are not utf8 valid (https://github.com/ouch-org/ouch/pull/181)
- Simplify/optimize several file inferring functions (https://github.com/ouch-org/ouch/pull/204)
- List command: print file immediately after it is processed (https://github.com/ouch-org/ouch/pull/225)
- Use `Cow<'static, str>` in `FinalError` (https://github.com/ouch-org/ouch/pull/246)
- Don't allocate when possible in `to_utf`, `nice_directory_display` (https://github.com/ouch-org/ouch/pull/249)
- Allow overriding the completions output directory (https://github.com/ouch-org/ouch/pull/251)
- Use Lazy to optimize env::current_dir repeated call (https://github.com/ouch-org/ouch/pull/261)
- Apply clippy lints and simplify smart_unpack (https://github.com/ouch-org/ouch/pull/267)
- Respect file permissions when compressing zip files (https://github.com/ouch-org/ouch/pull/271)
- Apply clippy lints (https://github.com/ouch-org/ouch/pull/273)
- Warn user if file extension is passed as file name (https://github.com/ouch-org/ouch/pull/277)
- Check for errors when setting the last modified time (https://github.com/ouch-org/ouch/pull/278)
- Use to the humansize crate for formatting human-readable file sizes (https://github.com/ouch-org/ouch/pull/281)
- Reactivate CI targets for ARM Linux and Windows MinGW (https://github.com/ouch-org/ouch/pull/289)
- Improve error message when compressing folder with single-file formats (https://github.com/ouch-org/ouch/pull/303)

### Tweaks

- Updating rustfmt (https://github.com/ouch-org/ouch/pull/144)
- Remove import comments (https://github.com/ouch-org/ouch/pull/162)
- Refactor utils into a module (https://github.com/ouch-org/ouch/pull/166)
- README update (https://github.com/ouch-org/ouch/pull/161 and https://github.com/ouch-org/ouch/pull/175)
- Organizing utils (https://github.com/ouch-org/ouch/pull/179)
- Update issue templates (https://github.com/ouch-org/ouch/pull/186)
- put compression backends behind features, clean up Cargo.toml (https://github.com/ouch-org/ouch/pull/187)
- remove trailing blank lines in error messages (https://github.com/ouch-org/ouch/pull/188)
- Improve/fix issue & question templates (https://github.com/ouch-org/ouch/pull/199 and https://github.com/ouch-org/ouch/pull/200)
- Simplify decompress function (https://github.com/ouch-org/ouch/pull/206)
- Add redundant check for --yes and --no flags conflict (https://github.com/ouch-org/ouch/pull/221)
- Ignore broken symlinks when compressing (https://github.com/ouch-org/ouch/pull/224)
- Remove redundant user_wants_to_continue function (https://github.com/ouch-org/ouch/pull/227)
- Fix missing `#[must_use]` attribute on a method returning `Self` (https://github.com/ouch-org/ouch/pull/243)
- Update dependencies (https://github.com/ouch-org/ouch/pull/253)
- Update dependencies (https://github.com/ouch-org/ouch/pull/257)
- Add pull request template (https://github.com/ouch-org/ouch/pull/263)
- Clean up the description for the `-d/--dir` argument to `decompress` (https://github.com/ouch-org/ouch/pull/264)
- Show subcommand aliases on --help (https://github.com/ouch-org/ouch/pull/275)
- Update dependencies (https://github.com/ouch-org/ouch/pull/276)
- Rewrite progress module (https://github.com/ouch-org/ouch/pull/280)
- Create scripts for benchmarking ouch (https://github.com/ouch-org/ouch/pull/280)

### Removals

- Remove automatic detection for partial compression (https://github.com/ouch-org/ouch/pull/286)
- Remove progress feature (https://github.com/ouch-org/ouch/pull/300)

## [0.3.1](https://github.com/ouch-org/ouch/compare/0.3.0...0.3.1) (2021-11-02)

### Tweaks

- Version bump

## [0.3.0](https://github.com/ouch-org/ouch/compare/0.2.0...0.3.0) (2021-11-02)

### New Features

- Properly detect if we are compressing a partially compressed file (https://github.com/ouch-org/ouch/pull/91)
- Support `.tgz` (https://github.com/ouch-org/ouch/pull/85)
- Add support for short tar archive extensions (https://github.com/ouch-org/ouch/issues/101)
- Migrate from `oof` to `clap` for argument parsing (https://github.com/ouch-org/ouch/pull/108)
- Shell completions & man page (https://github.com/ouch-org/ouch/pull/122)
- Implement command 'list' to show archive contents (https://github.com/ouch-org/ouch/pull/129)
- Print number of unpacked files by (https://github.com/ouch-org/ouch/pull/130)

### Bug Fixes

- Empty folders are ignored in archive compression formats (https://github.com/ouch-org/ouch/issues/41)
- fix macOS executable paths (https://github.com/ouch-org/ouch/pull/69)
- Print the format type when the format is in an incorrect position (https://github.com/ouch-org/ouch/pull/84)
- Compressing a single file to a single format that's not `tar` or `zip` panics (https://github.com/ouch-org/ouch/pull/89)
- Compression flag `--output` not working with single file compression (https://github.com/ouch-org/ouch/pull/93)
- Fix NO_COLOR issues, remove some dead code (https://github.com/ouch-org/ouch/pull/95)
- Add proper error message when using conflicting flags (e.g., `--yes --no`) (https://github.com/ouch-org/ouch/pull/99)
- Fix wrong archive format detection patterns (https://github.com/ouch-org/ouch/pull/125)
- Decompressing file without extension gives bad error message (https://github.com/ouch-org/ouch/issues/137)
- Fix decompression overwriting files without asking and failing on directories (https://github.com/ouch-org/ouch/pull/141)

### Improvements

- Add tests to check the resulting compressed files through MIME types (https://github.com/ouch-org/ouch/pull/74)
- Add proper error message when adding several files to a non-archive format such as bzip or gzip (https://github.com/ouch-org/ouch/pull/79)
- Apply clippy lints and small refactors (https://github.com/ouch-org/ouch/pull/86)
- Use `fs-err` crate instead of `std::fs` (https://github.com/ouch-org/ouch/pull/94)
- Change FinalError builder pattern to take and give ownership of self (https://github.com/ouch-org/ouch/issues/97)
- Omit "./" at the start of the path (https://github.com/ouch-org/ouch/pull/109 and https://github.com/ouch-org/ouch/pull/116)
- Introduce new enum for policy on how to handle y/n questions (https://github.com/ouch-org/ouch/issues/124)
- Add missing docs (https://github.com/ouch-org/ouch/pull/128)
- CI: Check the format with Github Action (https://github.com/ouch-org/ouch/pull/126)
- CI: Rewrite (https://github.com/ouch-org/ouch/pull/135)
- Improving error messages and removing dead error treatment code (https://github.com/ouch-org/ouch/pull/140)

### Tweaks

- CI: don't upload unused artifacts (https://github.com/ouch-org/ouch/pull/75)
- Compression info lines should use the \[INFO\] formatting like when decompressing (https://github.com/ouch-org/ouch/issues/76)
- CI: bump VM's Ubuntu version to 20 (https://github.com/ouch-org/ouch/pull/81)
- CI: stop building for ARM and Windows MinGW (https://github.com/ouch-org/ouch/pull/82)
- Updating Cargo.lock to newer dependencies (https://github.com/ouch-org/ouch/pull/92)
- Create CONTRIBUTING.md (https://github.com/ouch-org/ouch/pull/98)
- Minor cleanups and refactors (https://github.com/ouch-org/ouch/pull/100)
- Readme revision (https://github.com/ouch-org/ouch/pull/102)
- Fix README small markdown error (https://github.com/ouch-org/ouch/pull/104)
- Escaping pipes in installation commands (https://github.com/ouch-org/ouch/pull/106)
- Add 'Packaging Status' badge to README / note about installing on NixOS (https://github.com/ouch-org/ouch/issues/107)
- Change decompress command INFO messages (https://github.com/ouch-org/ouch/pull/117 and https://github.com/ouch-org/ouch/pull/119)
- Change decompress flag `--output` to `--dir` (https://github.com/ouch-org/ouch/pull/118)
- Updating CONTRIBUTING.md (https://github.com/ouch-org/ouch/pull/132)
- Remove tar combinations from compression format (https://github.com/ouch-org/ouch/pull/133)
- Simplify cli canonicalize implementation (https://github.com/ouch-org/ouch/pull/139)

## [0.2.0](https://github.com/ouch-org/ouch/compare/0.1.6...0.2.0) (2021-10-06)

### New Features

- Add Cargo lock file (https://github.com/ouch-org/ouch/pull/46)
- Allow compression of empty folders (https://github.com/ouch-org/ouch/pull/57)
- Make decompress command explicit (https://github.com/ouch-org/ouch/pull/61)
- Add support for Zstd (https://github.com/ouch-org/ouch/pull/64)

### Bug Fixes

- Fix download script, download from new linux urls (https://github.com/ouch-org/ouch/issues/40)

### Improvements

- Don't use colors when `stdout` or `stderr` are being redirected (https://github.com/ouch-org/ouch/pull/60)
- Making an error message for running decompress without arguments (https://github.com/ouch-org/ouch/issues/63)
- Increasing read and writer buffers capacity (https://github.com/ouch-org/ouch/pull/65)

## [0.1.6](https://github.com/ouch-org/ouch/compare/0.1.5...0.1.6) (2021-09-17)

### New Features

- Extension detection method supports more than 2 format suffixes (https://github.com/ouch-org/ouch/issues/28)
- Change Display implementation of crate::Error to an more structured FinalUserError (https://github.com/ouch-org/ouch/pull/39)
- Actions: new targets: Linux ARM64 (glibc), x86-64 (musl), Windows (MinGW) (https://github.com/ouch-org/ouch/pull/43)

### Improvements

- Further testing to oof cli (https://github.com/ouch-org/ouch/pull/38)
- Reuse Confirmation struct when checking for overwrite permission (https://github.com/ouch-org/ouch/pull/42)

## [0.1.5](https://github.com/ouch-org/ouch/compare/0.1.5-rc...0.1.5) (2021-05-27)

### New Features

- Add support for dot-dot (`..`) in output file/directory (https://github.com/ouch-org/ouch/issues/4)
- Add install.sh script (https://github.com/ouch-org/ouch/issues/37)
- Add checking for typos on the compression subcommand (https://github.com/ouch-org/ouch/pull/21)

### Bug Fixes

- Fix the -n, --no flag usage and add an alias for the compress subcommand (https://github.com/ouch-org/ouch/pull/22)

### Improvements

- Added compression and decompression tests for each current supported format (https://github.com/ouch-org/ouch/pull/24)
- Add tests to oof (https://github.com/ouch-org/ouch/pull/27)

### Tweaks

- Switch panics to errors (https://github.com/ouch-org/ouch/pull/21)
