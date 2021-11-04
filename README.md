<p align="center">
  <a href="https://crates.io/crates/ouch">
    <img src="https://img.shields.io/crates/v/ouch?color=6090FF" alt="Crates.io link">
  </a>
  <a href="https://docs.rs/ouch">
    <img src="https://img.shields.io/docsrs/ouch?color=6090FF" alt="Docs.rs link">
  </a>
  <a href="https://github.com/ouch/ouch-org/blob/master/LICENSE">
    <img src="https://img.shields.io/crates/l/ouch?color=6090FF" alt="License">
  </a>
</p>

# Ouch!

`ouch` stands for **Obvious Unified Compression Helper**, it's a CLI tool to compress and decompress files.

- [Features](#features)
- [Usage](#usage)
- [Installation](#installation)
- [Supported Formats](#supported-formats)
- [Contributing](#contributing)

## Features

1. Easy to use.
2. Automatic format detection.
3. Same syntax, various formats.
4. Encoding and decoding streams, it's fast. <!-- We should post benchmarks in our wiki and link them here -->
5. No runtime dependencies (for _Linux x86_64_).
6. Listing archive contents with tree formatting (in next release!).

## Usage

### Decompressing

Use the `decompress` subcommand and pass the files.

```sh
# Decompress one
ouch decompress a.zip

# Decompress multiple
ouch decompress a.zip b.tar.gz c.tar

# Short alternative
ouch d a.zip
```

You can redirect the decompression results to another folder with the `-d/--dir` flag.

```sh
# Decompress 'summer_vacation.zip' inside of new folder 'pictures'
ouch decompress summer_vacation.zip -d pictures
```

### Compressing

Use the `compress` subcommand, pass the files and the **output file** at the end.

```sh
# Compress four files/folders
ouch compress 1 2 3 4 archive.zip

# Short alternative
ouch c file.txt file.zip

# Compress everything in the current folder again and again
ouch compress * everything.tar.gz.xz.bz.zst.gz.gz.gz.gz.gz
```

`ouch` checks for the extensions of the **output file** to decide which formats should be used.

## Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/ouch.svg)](https://repology.org/project/ouch/versions)

### Downloading the latest binary

Compiled for `x86_64` on _Linux_, _Mac OS_ and _Windows_, run with `curl` or `wget`.

| Method    | Command                                                                             |
|:---------:|:------------------------------------------------------------------------------------|
| **curl**  | `curl -s https://raw.githubusercontent.com/ouch-org/ouch/master/install.sh \| sh`   |
| **wget**  | `wget https://raw.githubusercontent.com/ouch-org/ouch/master/install.sh -O - \| sh` |

The script will download the [latest binary](https://github.com/ouch-org/ouch/releases) and copy it to `/usr/bin`.

### Installing from source code

For compiling, check the [wiki guide](https://github.com/ouch-org/ouch/wiki/Compiling-and-installing-from-source-code).

## Supported formats

| Format    | `.tar` | `.zip` | `.bz`, `.bz2` | `.gz` | `.lz4` | `.xz`, `.lz`, `.lzma` | `.zst` |
|:---------:|:------:|:------:|:-------------:|:-----:|:------:|:---------------------:|:------:|
| Supported | ✓     | ✓      | ✓            | ✓     | ✓     | ✓                     | ✓     |

And the aliases: `tgz`, `tbz`, `tbz2`, `tlz4`, `txz`, `tlz`, `tlzma`, `tzst`.

Formats can be chained (`ouch` keeps it _fast_):

- `.gz.xz.bz.zst`
- `.tar.gz.xz.bz.zst`
- `.tar.gz.gz.gz.gz.xz.xz.xz.xz.bz.bz.bz.bz.zst.zst.zst.zst`

## Contributing

`ouch` is 100% made out of voluntary work, any small contribution is welcome!

- Open an issue.
- Open a pull request.
- Share it to a friend!
