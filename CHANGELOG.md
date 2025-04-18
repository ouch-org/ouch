# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

_This changelog was created after v0.3.1. As a result, there may be slight inaccuracies with prior versions._

Categories Used:

- New Features - new features added to ouch itself, not CI
- Bug Fixes
- Improvements - general enhancements
- Tweaks - anything that doesn't fit into other categories, small typo fixes, most CI stuff,
  meta changes (e.g. README updates), etc.
- Regression - removal of a feature (that might be readded, or reworked in the future)

**Bullet points in chronological order by PR**

## [Unreleased](https://github.com/ouch-org/ouch/compare/0.6.0...HEAD)

### New Features
### Improvements
### Bug Fixes
### Tweaks

## [0.6.0](https://github.com/ouch-org/ouch/compare/0.5.1...0.6.0)

### New Features

- Add multithreading support for `zstd` compression [\#689](https://github.com/ouch-org/ouch/pull/689) ([nalabrie](https://github.com/nalabrie))
- Add `bzip3` support [\#522](https://github.com/ouch-org/ouch/pull/522) ([freijon](https://github.com/freijon))
- Add `--remove` flag for decompression subcommand to remove files after successful decompression [\#757](https://github.com/ouch-org/ouch/pull/757) ([ttys3](https://github.com/ttys3))
- Add `br` (Brotli) support [\#765](https://github.com/ouch-org/ouch/pull/765) ([killercup](https://github.com/killercup))
- Add rename option in overwrite menu [\#779](https://github.com/ouch-org/ouch/pull/779) ([talis-fb](https://github.com/talis-fb))
- Store symlinks by default and add `--follow-symlinks` to store the target files [\#789](https://github.com/ouch-org/ouch/pull/789) ([tommady](https://github.com/tommady))

### Bug Fixes

- Fix output corrupted on parallel decompression [\#642](https://github.com/ouch-org/ouch/pull/642) ([AntoniosBarotsis](https://github.com/AntoniosBarotsis))

### Tweaks

- CI refactor [\#578](https://github.com/ouch-org/ouch/pull/578) ([cyqsimon](https://github.com/cyqsimon))
- Use a prefix `tmp-ouch-` for temporary decompression path name to avoid conflicts [\#725](https://github.com/ouch-org/ouch/pull/725) ([valoq](https://github.com/valoq)) & [\#788](https://github.com/ouch-org/ouch/pull/788) ([talis-fb](https://github.com/talis-fb))
- Ignore `.git/` when `-g/--gitignore` is set [\#507](https://github.com/ouch-org/ouch/pull/507) ([talis-fb](https://github.com/talis-fb))
- Run clippy for tests too [\#738](https://github.com/ouch-org/ouch/pull/738) ([marcospb19](https://github.com/marcospb19))
- Sevenz-rust is unmaintained, switch to sevenz-rust2 [\#796](https://github.com/ouch-org/ouch/pull/796) ([tommady](https://github.com/tommady))

### Improvements

- Fix logging IO bottleneck [\#642](https://github.com/ouch-org/ouch/pull/642) ([AntoniosBarotsis](https://github.com/AntoniosBarotsis))
- Support decompression  over stdin [\#692](https://github.com/ouch-org/ouch/pull/692) ([rcorre](https://github.com/rcorre))
- Make `--format` more forgiving with the formatting of the provided format [\#519](https://github.com/ouch-org/ouch/pull/519) ([marcospb19](https://github.com/marcospb19))
- Use buffered writer for list output [\#764](https://github.com/ouch-org/ouch/pull/764) ([killercup](https://github.com/killercup))
- Disable smart unpack when `--dir` flag is provided in decompress command [\#782](https://github.com/ouch-org/ouch/pull/782) ([talis-fb](https://github.com/talis-fb))
- Align file sizes at left for each extracted file to make output clearer [\#792](https://github.com/ouch-org/ouch/pull/792) ([talis-fb](https://github.com/talis-fb))

## [0.5.1](https://github.com/ouch-org/ouch/compare/0.5.0...0.5.1)

### Improvements

- Explicitly declare feature flags `use_zlib` & `use_zstd_thin` [\#564](https://github.com/ouch-org/ouch/pull/564) ([cyqsimon](https://github.com/cyqsimon))

### Tweaks

- Mention support for `7z` and `rar` in help message.

## [0.5.0](https://github.com/ouch-org/ouch/compare/0.4.2...0.5.0)

### New Features

- Add support for listing and decompressing `.rar` archives [\#529](https://github.com/ouch-org/ouch/pull/529) ([lmkra](https://github.com/lmkra))
- Add support for 7z [\#555](https://github.com/ouch-org/ouch/pull/555) ([Flat](https://github.com/flat) & [MisileLab](https://github.com/MisileLab))

### Bug Fixes

- Fix mime type detection [\#529](https://github.com/ouch-org/ouch/pull/529) ([lmkra](https://github.com/lmkra))
- Fix size unit inconsistency [\#502](https://github.com/ouch-org/ouch/pull/502) ([marcospb19](https://github.com/marcospb19))

### Improvements

- Hint completions generator to expand file paths [\#508](https://github.com/ouch-org/ouch/pull/508) ([marcospb19](https://github.com/marcospb19))

## [0.4.2](https://github.com/ouch-org/ouch/compare/0.4.1...0.4.2)

### New Features

- Add flags to configure the compression level
  - `--level` to precisely set the compression level [\#372](https://github.com/ouch-org/ouch/pull/372) ([xgdgsc](https://github.com/xgdgsc))
  - `--fast` and `--slow` [\#374](https://github.com/ouch-org/ouch/pull/374) ([figsoda](https://github.com/figsoda))
- Add `--format` option [\#341](https://github.com/ouch-org/ouch/pull/341) ([figsoda](https://github.com/figsoda))

### Improvements

- Multi-threaded compression for gzip and snappy using gzp [\#348](https://github.com/ouch-org/ouch/pull/348) ([figsoda](https://github.com/figsoda))
- Add `ls` as an alternative alias for listing [\#360](https://github.com/ouch-org/ouch/pull/360) ([orhun](https://github.com/orhun))

### Bug Fixes

- Fix decompression of zip archives with files larger than 4GB [\#354](https://github.com/ouch-org/ouch/pull/354) ([figsoda](https://github.com/figsoda))
- Fix handling of unknown extensions during decompression [\#355](https://github.com/ouch-org/ouch/pull/355) ([figsoda](https://github.com/figsoda))
- Remove remaining mentions of `.lz` that refers to the LZMA format [\#344](https://github.com/ouch-org/ouch/pull/344) ([marcospb19](https://github.com/marcospb19))
- Handle Zip when modification times are missing [\#433](https://github.com/ouch-org/ouch/pull/433) ([marcospb19](https://github.com/marcospb19))

## [0.4.1](https://github.com/ouch-org/ouch/compare/0.4.0...0.4.1)

### New Features

- Add cli option to (de)compress quietly [\#325](https://github.com/ouch-org/ouch/pull/325) ([a-moreira](https://github.com/a-moreira))

### Improvements

- Allow ouch to decompress archive into existing folder [\#321](https://github.com/ouch-org/ouch/pull/321) ([a-moreira](https://github.com/a-moreira))
- Accept inserting subcommand-independent flags in any position [\#329](https://github.com/ouch-org/ouch/pull/329) ([marcospb19](https://github.com/marcospb19))
- Improve extension parsing logic [\#330](https://github.com/ouch-org/ouch/pull/330) ([figsoda](https://github.com/figsoda))
- Slight refactor when ensuring archive-only inputs [\#331](https://github.com/ouch-org/ouch/pull/331) ([vrmiguel](https://github.com/vrmiguel))
- Use BStr to display possibly non-UTF8 byte sequences[\#332](https://github.com/ouch-org/ouch/pull/332) ([vrmiguel](https://github.com/vrmiguel))
- Use ubyte instead of humansize #333 [\#333](https://github.com/ouch-org/ouch/pull/333) ([vrmiguel](https://github.com/vrmiguel))
- Stop keeping track of the names of unpacked files [\#334](https://github.com/ouch-org/ouch/pull/334) ([vrmiguel](https://github.com/vrmiguel))
- Clean up [\#335](https://github.com/ouch-org/ouch/pull/335) ([figsoda](https://github.com/figsoda))

### Bug fixes

- Stop incorrectly asking to remove the parent dir [\#321](https://github.com/ouch-org/ouch/pull/321) ([a-moreira](https://github.com/a-moreira))

### Tweaks

- Add scoop install instructions to readme [\#323](https://github.com/ouch-org/ouch/pull/323) ([rasa](https://github.com/rasa))

## [0.4.0](https://github.com/ouch-org/ouch/compare/0.3.1...0.4.0) (2022-11-20)

### New Features

- Add release-helper.sh to make github releases easier [\#146](https://github.com/ouch-org/ouch/pull/146) ([marcospb19](https://github.com/marcospb19))
- Add support for lz4 [\#150](https://github.com/ouch-org/ouch/pull/150) ([figsoda](https://github.com/figsoda))
- add supported formats to help message [\#189](https://github.com/ouch-org/ouch/pull/189) ([figsoda](https://github.com/figsoda))
- add link to github to help message [\#191](https://github.com/ouch-org/ouch/pull/191) ([figsoda](https://github.com/figsoda))
- Update to Rust 2021 edition [\#192](https://github.com/ouch-org/ouch/pull/192) ([marcospb19](https://github.com/marcospb19))
- Implement accessibility mode [\#197](https://github.com/ouch-org/ouch/pull/197) ([AntonHermann](https://github.com/AntonHermann))
- Add heuristics to decompressing archives [\#209](https://github.com/ouch-org/ouch/pull/209) ([sigmaSd](https://github.com/sigmaSd))
- Add progress bar to compressing/decompressing [\#210](https://github.com/ouch-org/ouch/pull/210) ([sigmaSd](https://github.com/sigmaSd))
- Support snappy format [\#215](https://github.com/ouch-org/ouch/pull/215) ([figsoda](https://github.com/figsoda))
- Allow ignoring hidden files and files matched by .gitignore files [\#245](https://github.com/ouch-org/ouch/pull/245) ([vrmiguel](https://github.com/vrmiguel))
- Automatically generate man pages with clap_mangen [\#273](https://github.com/ouch-org/ouch/pull/273) ([figsoda](https://github.com/figsoda))
- Set last modified time during zip compression [\#279](https://github.com/ouch-org/ouch/pull/279) ([figsoda](https://github.com/figsoda))

### Bug Fixes

- Perform exhaustive matching on error variants [\#147](https://github.com/ouch-org/ouch/pull/147) ([marcospb19](https://github.com/marcospb19))
- Fix short flag for the --dir flag [\#149](https://github.com/ouch-org/ouch/pull/149) ([marcospb19](https://github.com/marcospb19))
- Rewrite tests [\#163](https://github.com/ouch-org/ouch/pull/163) ([figsoda](https://github.com/figsoda))
- switch from lz4_flex to lzzzz, enable lz4 tests [\#173](https://github.com/ouch-org/ouch/pull/173) ([figsoda](https://github.com/figsoda))
- Fix error message panic when cannot list non-archive files [\#182](https://github.com/ouch-org/ouch/pull/182) ([marcospb19](https://github.com/marcospb19))
- Fix not overwriting files/dirs when trying to create a dir [\#190](https://github.com/ouch-org/ouch/pull/190) ([SpyrosRoum](https://github.com/SpyrosRoum))
- Skip compressing file if its the same file as the output [\#193](https://github.com/ouch-org/ouch/pull/193) ([sigmaSd](https://github.com/sigmaSd))
- Fix warnings in doc comments [\#196](https://github.com/ouch-org/ouch/pull/196) ([AntonHermann](https://github.com/AntonHermann))
- Remove Lzip because its incorrect, and improve extension comparison [\#198](https://github.com/ouch-org/ouch/pull/198) ([sigmaSd](https://github.com/sigmaSd))
- Fix error with format infer [\#205](https://github.com/ouch-org/ouch/pull/205) ([marcospb19](https://github.com/marcospb19))
- Truncate long messages in the progress bar [\#214](https://github.com/ouch-org/ouch/pull/214) ([sigmaSd](https://github.com/sigmaSd))
- Fix zip memory warnings [\#217](https://github.com/ouch-org/ouch/pull/217) ([Crypto-Spartan](https://github.com/Crypto-Spartan))
- Fix the hint suggestion for compressing multiple files [\#219](https://github.com/ouch-org/ouch/pull/219) ([Crypto-Spartan](https://github.com/Crypto-Spartan))
- Simple eprintln fixes [\#226](https://github.com/ouch-org/ouch/pull/226) ([Crypto-Spartan](https://github.com/Crypto-Spartan))
- Actually use relative paths when extracting [\#229](https://github.com/ouch-org/ouch/pull/229) ([sigmaSd](https://github.com/sigmaSd))
- Mark directories when compressing to zip regardless of their contents [\#230](https://github.com/ouch-org/ouch/pull/230) ([sigmaSd](https://github.com/sigmaSd))
- Recover last modified time when unpacking zip archives [\#250](https://github.com/ouch-org/ouch/pull/250) ([vrmiguel](https://github.com/vrmiguel))
- Remove single quotes from clap doc comments [\#251](https://github.com/ouch-org/ouch/pull/251) ([jcgruenhage](https://github.com/jcgruenhage))
- Fix incorrect warnings for decompression [\#270](https://github.com/ouch-org/ouch/pull/270) ([figsoda](https://github.com/figsoda))
- Fix infinite compression if output file is inside the input folder [\#288](https://github.com/ouch-org/ouch/pull/288) ([figsoda](https://github.com/figsoda))
- Fix not overwriting a folder when compressing [\#295](https://github.com/ouch-org/ouch/pull/295) ([marcospb19](https://github.com/marcospb19))
- Check for EOF when asking questions [\#311](https://github.com/ouch-org/ouch/pull/311) ([marcospb19](https://github.com/marcospb19))

### Improvements

- Infer file extension when decompressing [\#154](https://github.com/ouch-org/ouch/pull/154) ([sigmaSd](https://github.com/sigmaSd))
- Extension: Use hardcoded slices instead of `Vecs` when creating an `Extension` [\#155](https://github.com/ouch-org/ouch/pull/155) ([vrmiguel](https://github.com/vrmiguel))
- Avoid allocating in `nice_directory_display` when possible, make `Extension` non-exhaustive [\#156](https://github.com/ouch-org/ouch/pull/156) ([vrmiguel](https://github.com/vrmiguel))
- Optimize `strip_cur_dir` [\#167](https://github.com/ouch-org/ouch/pull/167) ([vrmiguel](https://github.com/vrmiguel))
- Improve zip errors when paths are not utf8 valid [\#181](https://github.com/ouch-org/ouch/pull/181) ([marcospb19](https://github.com/marcospb19))
- Simplify/optimize several file inferring functions [\#204](https://github.com/ouch-org/ouch/pull/204) ([vrmiguel](https://github.com/vrmiguel))
- List command: print file immediately after it is processed [\#225](https://github.com/ouch-org/ouch/pull/225) ([sigmaSd](https://github.com/sigmaSd))
- Use `Cow<'static, str>` in `FinalError` [\#246](https://github.com/ouch-org/ouch/pull/246) ([vrmiguel](https://github.com/vrmiguel))
- Don't allocate when possible in `to_utf`, `nice_directory_display` [\#249](https://github.com/ouch-org/ouch/pull/249) ([vrmiguel](https://github.com/vrmiguel))
- Allow overriding the completions output directory [\#251]](https://github.com/ouch-org/ouch/pull/251) ([jcgruenhage](https://github.com/jcgruenhage))
- Use Lazy to optimize env::current_dir repeated call [\#261]](https://github.com/ouch-org/ouch/pull/261) ([marcospb19](https://github.com/marcospb19))
- Apply clippy lints and simplify smart_unpack [\#267](https://github.com/ouch-org/ouch/pull/267) ([figsoda](https://github.com/figsoda))
- Respect file permissions when compressing zip files [\#271](https://github.com/ouch-org/ouch/pull/271) ([figsoda](https://github.com/figsoda))
- Apply clippy lints [\#273](https://github.com/ouch-org/ouch/pull/273) ([figsoda](https://github.com/figsoda))
- Warn user if file extension is passed as file name [\#277](https://github.com/ouch-org/ouch/pull/277) ([marcospb19](https://github.com/marcospb19))
- Check for errors when setting the last modified time [\#278](https://github.com/ouch-org/ouch/pull/278) ([marcospb19](https://github.com/marcospb19))
- Use to the humansize crate for formatting human-readable file sizes [\#281](https://github.com/ouch-org/ouch/pull/281) ([figsoda](https://github.com/figsoda))
- Reactivate CI targets for ARM Linux and Windows MinGW [\#289](https://github.com/ouch-org/ouch/pull/289) ([figsoda](https://github.com/figsoda))
- Improve error message when compressing folder with single-file formats [\#303](https://github.com/ouch-org/ouch/pull/303) ([marcospb19](https://github.com/marcospb19))

### Tweaks

- Updating rustfmt [\#144](https://github.com/ouch-org/ouch/pull/144) ([marcospb19](https://github.com/marcospb19))
- Remove import comments [\#162](https://github.com/ouch-org/ouch/pull/162) ([marcospb19](https://github.com/marcospb19))
- Refactor utils into a module [\#166](https://github.com/ouch-org/ouch/pull/166) ([vrmiguel](https://github.com/vrmiguel))
- README update [\#161](https://github.com/ouch-org/ouch/pull/161) & [\#175](https://github.com/ouch-org/ouch/pull/175) ([marcospb19](https://github.com/marcospb19))
- Fix typo [\#153](https://github.com/ouch-org/ouch/pull/153) ([figsoda](https://github.com/figsoda)) & [\#176](https://github.com/ouch-org/ouch/pull/176) ([marcospb19](https://github.com/marcospb19))
- Organizing utils [\#179](https://github.com/ouch-org/ouch/pull/179) ([marcospb19](https://github.com/marcospb19))
- Update issue templates [\#186](https://github.com/ouch-org/ouch/pull/186) ([marcospb19](https://github.com/marcospb19))
- put compression backends behind features, clean up Cargo.toml [\#187](https://github.com/ouch-org/ouch/pull/187) ([figsoda](https://github.com/figsoda))
- remove trailing blank lines in error messages [\#188](https://github.com/ouch-org/ouch/pull/188) ([figsoda](https://github.com/figsoda))
- Improve/fix issue & question templates [\#199](https://github.com/ouch-org/ouch/pull/199) & [\#200](https://github.com/ouch-org/ouch/pull/200) ([figsoda](https://github.com/figsoda))
- Simplify decompress function [\#206](https://github.com/ouch-org/ouch/pull/206) ([sigmaSd](https://github.com/sigmaSd))
- Add redundant check for --yes and --no flags conflict [\#221](https://github.com/ouch-org/ouch/pull/221) ([marcospb19](https://github.com/marcospb19))
- Ignore broken symlinks when compressing [\#224](https://github.com/ouch-org/ouch/pull/224) ([sigmaSd](https://github.com/sigmaSd))
- Remove redundant user_wants_to_continue function [\#227](https://github.com/ouch-org/ouch/pull/227) ([Crypto-Spartan](https://github.com/Crypto-Spartan))
- Fix missing \#\[must_use\] attribute on a method returning `Self` [\#243](https://github.com/ouch-org/ouch/pull/243) ([vrmiguel](https://github.com/vrmiguel))
- Update dependencies [\#253](https://github.com/ouch-org/ouch/pull/253) ([Crypto-Spartan](https://github.com/Crypto-Spartan))
- Update dependencies [\#257](https://github.com/ouch-org/ouch/pull/257) ([Artturin](https://github.com/Artturin))
- Add pull request template [\#263](https://github.com/ouch-org/ouch/pull/263) ([figsoda](https://github.com/figsoda))
- Clean up the description for the `-d/--dir` argument to `decompress` [\#264](https://github.com/ouch-org/ouch/pull/264) ([hivehand](https://github.com/hivehand))
- Show subcommand aliases on --help [\#275](https://github.com/ouch-org/ouch/pull/275) ([marcospb19](https://github.com/marcospb19))
- Update dependencies [\#276](https://github.com/ouch-org/ouch/pull/276) ([figsoda](https://github.com/figsoda))
- Rewrite progress module [\#280](https://github.com/ouch-org/ouch/pull/280) ([figsoda](https://github.com/figsoda))
- Create scripts for benchmarking ouch [\#280](https://github.com/ouch-org/ouch/pull/280) ([figsoda](https://github.com/figsoda))

### Regression

- Remove automatic detection for partial compression [\#286](https://github.com/ouch-org/ouch/pull/286) ([marcospb19](https://github.com/marcospb19))
- Remove progress feature [\#300](https://github.com/ouch-org/ouch/pull/300) ([figsoda](https://github.com/figsoda))

### New Contributors

- [@sigmaSd](https://github.com/sigmaSd) made their first contribution in [\#154](https://github.com/ouch-org/ouch/pull/154)
- [@Crypto-Spartan](https://github.com/Crypto-Spartan) made their first contribution in [\#217](https://github.com/ouch-org/ouch/pull/217)
- [@Artturin](https://github.com/Artturin) made their first contribution in [\#257](https://github.com/ouch-org/ouch/pull/257)

## [0.3.1](https://github.com/ouch-org/ouch/compare/0.3.0...0.3.1) (2021-11-02)

### Tweaks

- Version bump

## [0.3.0](https://github.com/ouch-org/ouch/compare/0.2.0...0.3.0) (2021-11-02)

### New Features

- Properly detect if we are compressing a partially compressed file [\#54](https://github.com/ouch-org/ouch/issues/54) & [\#91](https://github.com/ouch-org/ouch/pull/91) ([SpyrosRoum](https://github.com/SpyrosRoum))
- Support `.tgz` [\#47](https://github.com/ouch-org/ouch/issues/47) & [\#85](https://github.com/ouch-org/ouch/pull/85) ([figsoda](https://github.com/figsoda))
- Add support for short tar archive extensions [\#101](https://github.com/ouch-org/ouch/issues/101) ([dnaka91](https://github.com/dnaka91))
- Migrate from `oof` to `clap` for argument parsing [\#105](https://github.com/ouch-org/ouch/issues/105) & [\#108](https://github.com/ouch-org/ouch/pull/108) ([SpyrosRoum](https://github.com/SpyrosRoum))
- Shell completions & man page [\#122](https://github.com/ouch-org/ouch/pull/122) ([figsoda](https://github.com/figsoda))
- Implement command 'list' to show archive contents [\#129](https://github.com/ouch-org/ouch/pull/129) ([AntonHermann](https://github.com/AntonHermann))
- Print number of unpacked files by [\#130](https://github.com/ouch-org/ouch/pull/130) ([boozec](https://github.com/boozec))

**Disclaimer: _Our installation script does not support installing man pages and shell completions yet, but PRs are welcome!_**

### Bug Fixes

- Empty folders are ignored in archive compression formats [\#41](https://github.com/ouch-org/ouch/issues/41) ([GabrielSimonetto](https://github.com/GabrielSimonetto))
- fix macOS executable paths [\#69](https://github.com/ouch-org/ouch/pull/69) ([vrmiguel](https://github.com/vrmiguel))
- Print the format type when the format is in an incorrect position [\#84](https://github.com/ouch-org/ouch/pull/84) ([boozec](https://github.com/boozec))
- Compressing a single file to a single format that's not `tar` or `zip` panics [\#87](https://github.com/ouch-org/ouch/issues/87) & [\#89](https://github.com/ouch-org/ouch/pull/89) ([marcospb19](https://github.com/marcospb19))
- Compression flag `--output` not working with single file compression [\#90](https://github.com/ouch-org/ouch/issues/90) & [\#93](https://github.com/ouch-org/ouch/pull/93) ([figsoda](https://github.com/figsoda))
- Fix NO_COLOR issues, remove some dead code [\#66](https://github.com/ouch-org/ouch/issues/66), [\#62](https://github.com/ouch-org/ouch/issues/62), & [\#95](https://github.com/ouch-org/ouch/pull/95) ([figsoda](https://github.com/figsoda))
- Add proper error message when using conflicting flags \(e.g. `--yes --no`\) [\#55](https://github.com/ouch-org/ouch/issues/55) & [\#99](https://github.com/ouch-org/ouch/pull/99) ([SpyrosRoum](https://github.com/SpyrosRoum))
- Fix wrong archive format detection patterns [\#125](https://github.com/ouch-org/ouch/pull/125) ([SpyrosRoum](https://github.com/SpyrosRoum))
- Decompressing file without extension gives bad error message [\#137](https://github.com/ouch-org/ouch/issues/137) ([marcospb19](https://github.com/marcospb19))
- Fix decompression overwriting files without asking and failing on directories [\#141](https://github.com/ouch-org/ouch/pull/141) ([SpyrosRoum](https://github.com/SpyrosRoum))

### Improvements

- Add tests to check the resulting compressed files through MIME types [\#72](https://github.com/ouch-org/ouch/issues/72) & [\#74](https://github.com/ouch-org/ouch/pull/74) ([vrmiguel](https://github.com/vrmiguel))
- Add proper error message when adding several files to a non-archive format such as bzip or gzip [\#78](https://github.com/ouch-org/ouch/issues/78) & [\#79](https://github.com/ouch-org/ouch/pull/79) ([vrmiguel](https://github.com/vrmiguel))
- Apply clippy lints and small refactors [\#86](https://github.com/ouch-org/ouch/pull/86) ([figsoda](https://github.com/figsoda))
- Use `fs-err` crate instead of `std::fs` [\#56](https://github.com/ouch-org/ouch/issues/56) & [\#94](https://github.com/ouch-org/ouch/pull/94) ([GabrielSimonetto](https://github.com/GabrielSimonetto))
- Change FinalError builder pattern to take and give ownership of self [\#97](https://github.com/ouch-org/ouch/issues/97) ([SpyrosRoum](https://github.com/SpyrosRoum))
- Omit "./" at the start of the path [\#109](https://github.com/ouch-org/ouch/pull/109) & [\#116](https://github.com/ouch-org/ouch/pull/116) ([exoego](https://github.com/exoego))
- Introduce new enum for policy on how to handle y/n questions [\#124](https://github.com/ouch-org/ouch/issues/124) ([AntonHermann](https://github.com/AntonHermann))
- Add missing docs [\#128](https://github.com/ouch-org/ouch/pull/128) ([GabrielSimonetto](https://github.com/GabrielSimonetto))
- CI: Check the format with Github Action [\#126](https://github.com/ouch-org/ouch/pull/126) ([boozec](https://github.com/boozec))
- CI: Rewrite [\#135](https://github.com/ouch-org/ouch/pull/135) ([figsoda](https://github.com/figsoda))
- Improving error messages and removing dead error treatment code [\#140](https://github.com/ouch-org/ouch/pull/140) ([marcospb19](https://github.com/marcospb19))

### Tweaks

- CI: don't upload unused artifacts [\#75](https://github.com/ouch-org/ouch/pull/75) ([marcospb19](https://github.com/marcospb19))
- Compression info lines should use the \[INFO\] formatting like when decompressing [\#76](https://github.com/ouch-org/ouch/issues/76) ([vrmiguel](https://github.com/vrmiguel))
- CI: bump VM's Ubuntu version to 20 [\#81](https://github.com/ouch-org/ouch/pull/81) ([vrmiguel](https://github.com/vrmiguel))
- CI: stop building for ARM and Windows MinGW [\#82](https://github.com/ouch-org/ouch/pull/82) ([vrmiguel](https://github.com/vrmiguel))
- Updating Cargo.lock to newer dependencies [\#92](https://github.com/ouch-org/ouch/pull/92) ([marcospb19](https://github.com/marcospb19))
- Create CONTRIBUTING.md [\#98](https://github.com/ouch-org/ouch/pull/98) ([marcospb19](https://github.com/marcospb19))
- Minor cleanups and refactors [\#100](https://github.com/ouch-org/ouch/pull/100) ([figsoda](https://github.com/figsoda))
- Readme revision [\#102](https://github.com/ouch-org/ouch/pull/102) ([marcospb19](https://github.com/marcospb19))
- Fix README small markdown error [\#104](https://github.com/ouch-org/ouch/pull/104) ([marcospb19](https://github.com/marcospb19))
- Escaping pipes in installation commands [\#106](https://github.com/ouch-org/ouch/pull/106) ([marcospb19](https://github.com/marcospb19))
- Add 'Packaging Status' badge to README / note about installing on NixOS [\#107](https://github.com/ouch-org/ouch/issues/107) ([figsoda](https://github.com/figsoda))
- Change decompress command INFO messages [\#117](https://github.com/ouch-org/ouch/pull/117) & [\#119](https://github.com/ouch-org/ouch/pull/119) ([exoego](https://github.com/exoego))
- Change decompress flag `--output` to `--dir` [\#118](https://github.com/ouch-org/ouch/pull/118) ([khubo](https://github.com/khubo))
- Updating CONTRIBUTING.md [\#132](https://github.com/ouch-org/ouch/pull/132) ([marcospb19](https://github.com/marcospb19))
- Remove tar combinations from compression format [\#133](https://github.com/ouch-org/ouch/pull/133) ([SpyrosRoum](https://github.com/SpyrosRoum))
- Simplify cli canonicalize implementation [\#139](https://github.com/ouch-org/ouch/pull/139) ([marcospb19](https://github.com/marcospb19))

### New Contributors

- [@figsoda](https://github.com/figsoda) made their first contribution in #86
- [@boozec](https://github.com/boozec) made their first contribution in #84
- [@SpyrosRoum](https://github.com/SpyrosRoum) made their first contribution in #97
- [@dnaka91](https://github.com/dnaka91) made their first contribution in #101
- [@exoego](https://github.com/exoego) made their first contribution in #109
- [@AntonHermann](https://github.com/AntonHermann) made their first contribution in #124
- [@khubo](https://github.com/khubo) made their first contribution in #118

## [0.2.0](https://github.com/ouch-org/ouch/compare/0.1.6...0.2.0) (2021-10-06)

### New Features

- Add Cargo lock file [\#46](https://github.com/ouch-org/ouch/pull/46) ([psibi](https://github.com/psibi))
- Allow compression of empty folders [\#57](https://github.com/ouch-org/ouch/pull/57) ([GabrielSimonetto](https://github.com/GabrielSimonetto))
- Make decompress command explicit [\#61](https://github.com/ouch-org/ouch/pull/61) ([GabrielSimonetto](https://github.com/GabrielSimonetto))
- Add support for Zstd [\#64](https://github.com/ouch-org/ouch/pull/64) ([vrmiguel](https://github.com/vrmiguel))

### Bug Fixes

- Fix download script, download from new linux urls [\#40](https://github.com/ouch-org/ouch/issues/40)

### Improvements

- Don't use colors when `stdout` or `stderr` are being redirected [\#60](https://github.com/ouch-org/ouch/pull/60) ([vrmiguel](https://github.com/vrmiguel))
- Making an error message for running decompress without arguments [\#63](https://github.com/ouch-org/ouch/issues/63)
- Increasing read and writer buffers capacity [\#65](https://github.com/ouch-org/ouch/pull/65) ([marcospb19](https://github.com/marcospb19))

### New Contributors

- [@psibi](https://github.com/psibi) made their first contribution in [\#46](https://github.com/ouch-org/ouch/pull/46)
- [@GabrielSimonetto](https://github.com/GabrielSimonetto) made their first contribution in [\#57](https://github.com/ouch-org/ouch/pull/57)

## [0.1.6](https://github.com/ouch-org/ouch/compare/0.1.5...0.1.6) (2021-09-17)

### New Features

- Extension detection method supports more than 2 format suffixes. [\#28](https://github.com/ouch-org/ouch/issues/28)
- Change Display implementation of crate::Error to an more structured FinalUserError [\#39](https://github.com/ouch-org/ouch/pull/39) ([marcospb19](https://github.com/marcospb19))
- Actions: new targets: Linux ARM64 \(glibc\), x86-64 \(musl\), Windows \(MinGW\) [\#43](https://github.com/ouch-org/ouch/pull/43) ([vrmiguel](https://github.com/vrmiguel))

### Improvements

- Further testing to oof cli [\#38](https://github.com/ouch-org/ouch/pull/38) ([demfabris](https://github.com/demfabris))
- Reuse Confirmation struct when checking for overwrite permission [\#42](https://github.com/ouch-org/ouch/pull/42) ([vrmiguel](https://github.com/vrmiguel))

## [0.1.5](https://github.com/ouch-org/ouch/compare/0.1.5-rc...0.1.5) (2021-05-27)

### New Features

- Add support for dot-dot \(`..`\) in output file/directory [\#4](https://github.com/ouch-org/ouch/issues/4)
- Add install.sh script [\#37](https://github.com/ouch-org/ouch/issues/37)
- Add checking for typos on the compression subcommand [\#21](https://github.com/ouch-org/ouch/pull/21) ([vrmiguel](https://github.com/vrmiguel))

### Bug Fixes

- Fix the -n, --no flag usage and add an alias for the compress subcommand [\#22](https://github.com/ouch-org/ouch/pull/22) ([vrmiguel](https://github.com/vrmiguel))

### Improvements

- Added compression and decompression tests for each current supported format [\#24](https://github.com/ouch-org/ouch/pull/24) ([marcospb19](https://github.com/marcospb19))
- Add tests to oof [\#27](https://github.com/ouch-org/ouch/pull/27) ([demfabris](https://github.com/demfabris))

### Tweaks

- Switch panics to errors [\#21](https://github.com/ouch-org/ouch/pull/21) ([vrmiguel](https://github.com/vrmiguel))

### New Contributors

- [@demfabris](https://github.com/demfabris) made their first contribution in [\#27](https://github.com/ouch-org/ouch/pull/27)

## [0.1.5-rc](https://github.com/ouch-org/ouch/compare/0.1.4...0.1.5-rc) (2021-04-07)

### New Features

- Better error messages ([vrmiguel](https://github.com/vrmiguel))
- New `--help` message [df1bc87](https://github.com/ouch-org/ouch/commit/df1bc879cbfc91286f0570e944df113b28b638db) ([marcospb19](https://github.com/marcospb19))
- Create subproject `oof`, a thin argparsing lib [\#12](https://github.com/ouch-org/ouch/pull/12) ([marcospb19](https://github.com/marcospb19))
- Pretty-printing for bytes [\#17](https://github.com/ouch-org/ouch/pull/17) ([vrmiguel](https://github.com/vrmiguel))
- Verify inputs when decompressing [\#18](https://github.com/ouch-org/ouch/pull/18) ([vrmiguel](https://github.com/vrmiguel))
- CI: use MUSL when compiling for Linux [8e6804](https://github.com/ouch-org/ouch/commit/8e680402a929986796c9418605b2d84314fd2684) ([vrmiguel](https://github.com/vrmiguel))
- CI: build and test for Linux ARMv7 [\#19](https://github.com/ouch-org/ouch/pull/19) ([vrmiguel](https://github.com/vrmiguel))

### Bug Fixes

- Argparsing: problems with the `-o, --output` flag [\#13](https://github.com/ouch-org/ouch/issues/13) ([marcospb19](https://github.com/marcospb19))
- Short flags not receiving values [\#15](https://github.com/ouch-org/ouch/pull/15) ([vrmiguel](https://github.com/vrmiguel))

## [0.1.4](https://github.com/ouch-org/ouch/compare/08489b028c8e85176bf5d55576f75c1817df9019...0.1.4) (2021-03-29)

### New Features

- confirmation dialogs for file overwriting [\#2](https://github.com/ouch-org/ouch/pull/2) ([vrmiguel](https://github.com/vrmiguel))
- `-y, --yes` and `-n, --no` flags for automatic answering of confirmation dialogs [\#7](https://github.com/ouch-org/ouch/issues/7) ([vrmiguel](https://github.com/vrmiguel))
