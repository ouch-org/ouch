<p align="center">
  <a href="https://crates.io/crates/ouch">
    <img src="https://img.shields.io/crates/v/ouch?color=6090FF&style=flat-square" alt="Crates.io link">
  </a>
  <a href="https://github.com/ouch-org/ouch/blob/main/LICENSE">
    <img src="https://img.shields.io/crates/l/ouch?color=6090FF&style=flat-square" alt="License">
  </a>
</p>

# Ouch!

`ouch` stands for **Obvious Unified Compression Helper**.

It's a CLI tool for compressing and decompressing for various formats.

- [Features](#features)
- [Usage](#usage)
- [Installation](#installation)
- [Supported Formats](#supported-formats)
- [Benchmarks](#benchmarks)
- [Contributing](#contributing)

# Features

1. Easy to use.
2. Fast.
3. Great error message feedback.
4. No runtime dependencies required (for _Linux x86_64_).
5. Accessibility mode ([see more](https://github.com/ouch-org/ouch/wiki/Accessibility)).
6. Shell completions and man pages.

# Usage

Ouch has three main subcommands:

- `ouch decompress` (alias `d`)
- `ouch compress` (alias `c`)
- `ouch list` (alias `l` or `ls`)

To see `help` for a specific command:

```sh
ouch help <COMMAND>
ouch <COMMAND> --help  # equivalent
```

## Shell Completions

You can generate shell completion scripts using the `--completions` flag:

```sh
# Generate bash completions
ouch --completions bash

# Generate zsh completions
ouch --completions zsh
```

## Decompressing

Use the `decompress` subcommand, `ouch` will detect the extensions automatically.

```sh
ouch decompress a.zip

# Decompress multiple files
ouch decompress a.zip b.tar.gz c.tar
```

The `-d/--dir` flag can be used to redirect decompression results to another directory.

```sh
# Decompress 'summer_vacation.zip' inside of new folder 'pictures'
ouch decompress summer_vacation.zip --dir pictures
```

## Compressing

Pass input files to the `compress` subcommand, add the **output file** at the end.

```sh
# Compress two files into `archive.zip`
ouch compress one.txt two.txt archive.zip

# Compress file.txt using .lz4 and .zst
ouch compress file.txt file.txt.lz4.zst
```

`ouch` detects the extensions of the **output file** to decide what formats to use.

## Listing

```sh
ouch list archive.zip

# Example with tree formatting
ouch list source-code.zip --tree
```

Output:

```
└── src
   ├── archive
   │  ├── mod.rs
   │  ├── tar.rs
   │  └── zip.rs
   ├── utils
   │  ├── colors.rs
   │  ├── formatting.rs
   │  ├── mod.rs
   │  └── fs.rs
   ├── commands
   │  ├── list.rs
   │  ├── compress.rs
   │  ├── decompress.rs
   │  └── mod.rs
   ├── accessible.rs
   ├── error.rs
   ├── cli.rs
   └── main.rs
```

# Supported formats

| Format    | `.tar` | `.zip` | `7z` | `.gz` | `.xz` | `.lzma` | `.lz` | `.bz`, `.bz2` | `.bz3` | `.lz4` | `.sz` (Snappy) | `.zst` | `.rar` | `.br` |
|:---------:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| Supported | ✓ | ✓¹ | ✓¹ | ✓² | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓² | ✓² | ✓³ | ✓ |

✓: Supports compression and decompression.

✓¹: Due to limitations of the compression format itself, (de)compression can't be done with streaming.

✓²: Supported, and compression runs in parallel.

✓³: Due to RAR's restrictive license, only decompression and listing can be supported.

If you wish to exclude non-free code from your build, you can disable RAR support
by building without the `unrar` feature.

Aliases for these formats are also supported:
- `tar`: `tgz`, `tbz`, `tbz2`, `tlz4`, `txz`, `tlzma`, `tsz`, `tzst`, `tlz`, `cbt`
- `zip`: `cbz`
- `7z`: `cb7`
- `rar`: `cbr`

Formats can be chained:

- `.tar.gz`
- `.tar.gz.xz.zst.gz.lz4.sz`

If the filename has no extensions, `Ouch` will try to infer the format by the [file signature](https://en.wikipedia.org/wiki/List_of_file_signatures) and ask the user for confirmation.

# Installation

<a href="https://repology.org/project/ouch/versions">
  <img align="right" src="https://repology.org/badge/vertical-allrepos/ouch.svg" alt="Packaging status" />
</a>

Refer to the packages list on the right.

The most commonly used installation methods:

## On Arch Linux

```bash
pacman -S ouch
```

## On MacOS via homebrew

```cmd
brew install ouch
```

## On Windows via Scoop

```cmd
scoop install ouch
```

## From crates.io

```bash
cargo install ouch
```

(If you're in Ubuntu, you might need to install `clang` to build it from crates.io.)

## Download the latest release bundle

Check the [releases page](https://github.com/ouch-org/ouch/releases).

## Compiling from source code

Check the [wiki guide on compiling](https://github.com/ouch-org/ouch/wiki/Compiling-and-installing-from-source-code).

# Runtime Dependencies

If running `ouch` results in a linking error, it means you're missing a runtime dependency.

If you're downloading binaries from the [releases page](https://github.com/ouch-org/ouch/releases), try the `musl` variants, those are static binaries that require no runtime dependencies.

Otherwise, you'll need these libraries installed on your system:

* [libbz2](https://www.sourceware.org/bzip2)
* [libbz3](https://github.com/kspalaiologos/bzip3)
* [libz](https://www.zlib.net)

These should be available in your system's package manager.

# Benchmarks

Benchmark results are available [here](benchmarks/results.md).
Performance of compressing and decompressing
[Rust](https://github.com/rust-lang/rust) source code are measured and compared with
[Hyperfine](https://github.com/sharkdp/hyperfine).
The values presented are the average (wall clock) elapsed time.

Note: `ouch` focuses heavily on usage ergonomics and nice error messages, but
we plan on doing some optimization in the future.

Versions used:

- `ouch` _0.4.0_
- [`tar`] _1.34_
- [`unzip`][infozip] _6.00_
- [`zip`][infozip] _3.0_

# Contributing

`ouch` is made out of voluntary work, contributors are very welcome! Contributions of all sizes are appreciated.

- Open an [issue](https://github.com/ouch-org/ouch/issues).
- Package it for your favorite distribution or package manager.
- Share it with a friend!
- Open a pull request.

If you're creating a Pull Request, check [CONTRIBUTING.md](./CONTRIBUTING.md).

[`tar`]: https://www.gnu.org/software/tar/
[infozip]: http://www.info-zip.org/
