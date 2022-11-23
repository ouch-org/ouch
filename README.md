<p align="center">
  <a href="https://crates.io/crates/ouch">
    <img src="https://img.shields.io/crates/v/ouch?color=6090FF&style=flat-square" alt="Crates.io link">
  </a>
  <a href="https://github.com/ouch/ouch-org/blob/master/LICENSE">
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
- `ouch list` (alias `l`)

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

| Format    | `.tar` | `.zip` | `.gz` | `.xz`, `.lzma` | `.bz`, `.bz2` | `.lz4` | `.sz` | `.zst` |
|:---------:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| Supported | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

And the aliases: `tgz`, `tbz`, `tbz2`, `tlz4`, `txz`, `tlzma`, `tsz`, `tzst`.

Formats can be chained:

- `.tar.gz`
- `.tar.gz.gz.gz.gz`
- `.tar.gz.gz.gz.gz.zst.xz.bz.lz4`

# Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/ouch.svg)](https://repology.org/project/ouch/versions)

## On Arch Linux

```bash
yay -S ouch
```

## From crates.io

```bash
cargo install ouch
```

## Download the latest release tarball

Check the [releases page](https://github.com/ouch-org/ouch/releases).

## Compiling from source code

Check the [wiki guide on compiling](https://github.com/ouch-org/ouch/wiki/Compiling-and-installing-from-source-code).

# Dependencies

If you installed `ouch` using the download script, you will need no dependencies (static MUSL binary).

Otherwise, you'll need these libraries installed on your system:

* [liblzma](https://www.7-zip.org/sdk.html)
* [libbz2](https://www.sourceware.org/bzip2/)
* [libz](https://www.zlib.net/)

These are available on all mainstream _Linux_ distributions and on _macOS_.

# Benchmarks

Comparison made decompressing `linux.tar.gz` and measured with
[Hyperfine](https://github.com/sharkdp/hyperfine) and the values presented are the average (wall clock) elapsed time.

| Tool         | `ouch` | [`tar`] | [`bsdtar`] |
|:------------:|:------:|:-------:|:----------:|
| Average time | 911 ms | 1102 ms |   829 ms   |

Note: `ouch` focuses heavily on usage ergonomics and nice error messages, but
we plan on doing some optimization in the future.

Versions used:

- `ouch` _0.3.1_
- [`tar`] _1.34_
- [`bsdtar`] _3.5.2_

# Contributing

`ouch` is made out of voluntary work, contributors are very welcome! No contribution is too small and all contributions are valued.

- Open an [issue](https://github.com/ouch-org/ouch/issues).
- Package it for your favorite distribution or package manager.
- Open a pull request.
- Share it with a friend!

[`tar`]: https://www.gnu.org/software/tar/
[`bsdtar`]: https://www.freebsd.org/cgi/man.cgi?query=bsdtar&sektion=1&format=html
