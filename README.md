# Ouch!

[![crates.io](https://img.shields.io/crates/v/ouch.svg?style=for-the-badge&logo=rust)](https://crates.io/crates/ouch) [![license](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge&logo=Open-Source-Initiative&logoColor=ffffff)](https://github.com/ouch-org/ouch/blob/main/LICENSE)

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

You can redirect the decompression results to another folder with the `-o/--output` flag.

```sh
# Decompress 'summer_vacation.zip' inside of new folder 'pictures'
ouch decompress summer_vacation.zip -o pictures
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

### Downloading the latest binary

Compiled for `x86_64` on _Linux_, _Mac OS_ and _Windows_, run with `curl` or `wget`.

| Method    | Command                                                                            |
|:---------:|:-----------------------------------------------------------------------------------|
| **curl**  | `curl -s https://raw.githubusercontent.com/ouch-org/ouch/master/install.sh | sh`   |
| **wget**  | `wget https://raw.githubusercontent.com/ouch-org/ouch/master/install.sh -O - | sh` |


The script will download the [latest binary](https://github.com/ouch-org/ouch/releases) and copy it to `/usr/bin`.

### Installing from source code

For compiling, check the [wiki guide](https://github.com/ouch-org/ouch/wiki/Compiling-and-installing-from-source-code).

## Supported formats

| Format        | .tar, .tgz | .zip | .bz, .bz2 | .gz | .xz, .lz, .lzma | .zst |
|:-------------:|:----:|:----:|:---------:| --- |:---------------:| --- |
| Supported | ✓   | ✓   | ✓         | ✓  |   ✓            | ✓  |

Formats can be chained (`ouch` keeps it _fast_):
- `.gz.xz.bz.zst`
- `.tar.gz.xz.bz.zst`
- `.tar.gz.gz.gz.gz.xz.xz.xz.xz.bz.bz.bz.bz.zst.zst.zst.zst`
- 
## Contributing

`ouch` is 100% made out of voluntary work, any small contribution is welcome!

- Open an issue.
- Open a pr.
- Share it to a friend!
