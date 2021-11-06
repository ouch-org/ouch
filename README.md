<p align="center">
  <a href="https://crates.io/crates/ouch">
    <img src="https://img.shields.io/crates/v/ouch?color=6090FF&style=flat-square" alt="Crates.io link">
  </a>
  <a href="https://docs.rs/ouch">
    <img src="https://img.shields.io/docsrs/ouch?color=6090FF&style=flat-square" alt="Docs.rs link">
  </a>
  <a href="https://github.com/ouch/ouch-org/blob/master/LICENSE">
    <img src="https://img.shields.io/crates/l/ouch?color=6090FF&style=flat-square" alt="License">
  </a>
</p>

# Ouch!

`ouch` stands for **Obvious Unified Compression Helper** and is a CLI tool to help you compress and decompress files of several formats.

- [Features](#features)
- [Usage](#usage)
- [Installation](#installation)
- [Supported Formats](#supported-formats)
- [Benchmarks](#benchmarks)
- [Contributing](#contributing)

# Features

1. Easy to use.
2. Infers expression formats automatically.
3. Uses the same usage syntax for all supported formats.
4. Achieves great performance through encoding and decoding streams.
5. No runtime dependencies (for _Linux x86_64_).
6. Listing archive contents with tree formatting (in next release!).

# Usage

## Decompressing

Use the `decompress` subcommand and pass the files.

```sh
# Decompress a file
ouch decompress a.zip

# Decompress multiple files
ouch decompress a.zip b.tar.gz c.tar

# Short alternative
ouch d a.zip
```

The `-d/--dir` flag can be used to redirect decompression results to another directory.

```sh
# Decompress 'summer_vacation.zip' inside of new folder 'pictures'
ouch decompress summer_vacation.zip --dir pictures
```

## Compressing

Use the `compress` subcommand, pass the files and the **output file** at the end.

```sh
# Compress four files/folders
ouch compress 1 2 3 4 archive.zip

# Short alternative
ouch c file.txt file.zip
```

`ouch` detects the extensions of the **output file** to decide what formats to use.

# Supported formats

| Format    | `.tar` | `.zip` | `.bz`, `.bz2` | `.gz` | `.lz4` | `.xz`, `.lz`, `.lzma` | `.zst` |
|:---------:|:------:|:------:|:-------------:|:-----:|:------:|:---------------------:|:------:|
| Supported | ✓     | ✓      | ✓            | ✓     | ✓     | ✓                     | ✓     |

And the aliases: `tgz`, `tbz`, `tbz2`, `tlz4`, `txz`, `tlz`, `tlzma`, `tzst`.

Formats can be chained (`ouch` keeps it _fast_):

- `.gz.xz.bz.zst`
- `.tar.gz.xz.bz.zst`
- `.tar.gz.gz.gz.gz.xz.xz.xz.xz.bz.bz.bz.bz.zst.zst.zst.zst`

# Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/ouch.svg)](https://repology.org/project/ouch/versions)

## Downloading the latest binary

Compiled for `x86_64` on _Linux_, _Mac OS_ and _Windows_, run with `curl` or `wget`.

| Method    | Command                                                                             |
|:---------:|:------------------------------------------------------------------------------------|
| **curl**  | `curl -s https://raw.githubusercontent.com/ouch-org/ouch/master/install.sh \| sh`   |
| **wget**  | `wget https://raw.githubusercontent.com/ouch-org/ouch/master/install.sh -O - \| sh` |

The script will copy the [latest binary](https://github.com/ouch-org/ouch/releases) to `/usr/local/bin`.

## Installing from source code

Check the [wiki guide](https://github.com/ouch-org/ouch/wiki/Compiling-and-installing-from-source-code).

# Dependencies

When built dynamically linked, you'll need these libraries to be available on your system:

* [liblzma](https://www.7-zip.org/sdk.html)
* [libbz2](https://www.sourceware.org/bzip2/)
* [libz](https://www.zlib.net/)

Thankfully these are all very common libraries that _should_ already be available on all mainstream Linux distributions and on macOS.

`ouch` is also easily built with MUSL, in which case it's _statically linked_ and therefore has no runtime dependencies. 

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
