# Ouch!

[![crates.io](https://img.shields.io/crates/v/ouch.svg?style=for-the-badge&logo=rust)](https://crates.io/crates/ouch) [![license](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge&logo=Open-Source-Initiative&logoColor=ffffff)](https://github.com/ouch-org/ouch/blob/main/LICENSE)

<!-- ![ouch_image](https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcR5ilNDTFZZ-Vy_ctm2YyAe8Yk0UT7lB2hIhg&usqp=CAU)  -->

`ouch` stands for **Obvious Unified Compression Helper**, and works on _Linux_, _Mac OS_ and _Windows_.

It is a CLI tool to compress and decompress files that aims on ease of usage.

<!-- TODO -->
<!--     - [Listing files](#Listing-the-elements-of-an-archive) -->

- [Usage](#usage)
    - [Decompressing](#decompressing)
    - [Compressing](#compressing)
- [Installation](#installation)
    - [Latest binary](#downloading-the-latest-binary)
    - [Compiling from source](#installing-from-source-code)
- [Supported Formats](#supported-formats)
- [Contributing](#contributing)

## Usage

### Decompressing

Run `ouch` and pass compressed files as arguments.

```sh
# Decompress 'a.zip'
ouch decompress a.zip

# Also works with the short version
ouch d a.zip

# Decompress multiple files
ouch decompress a.zip b.tar.gz
```

You can redirect the decompression results to a folder with the `-o/--output` flag.

```sh
# Create 'pictures' folder and decompress inside of it
ouch decompress a.zip -o pictures
```

### Compressing

Use the `compress` subcommand.

Accepts multiple files and folders, the **last** argument shall be the **output file**.

```sh
# Compress four files into 'archive.zip'
ouch compress 1 2 3 4 archive.zip

# Also works with the short version
ouch c 1 2 3 4 archive.zip

# Compress folder and video into 'videos.tar.gz'
ouch compress videos/ meme.mp4 videos.tar.gz

# Compress one file using 4 compression formats
ouch compress file.txt compressed.gz.xz.bz.zst

# Compress all the files in current folder
ouch compress * files.zip
```

`ouch` checks for the extensions of the **output file** to decide which formats should be used.

<!-- ### Listing the elements of an archive

* **Upcoming feature**

```
# Shows the files and folders contained in videos.tar.xz
ouch list videos.tar.xz
``` -->

## Installation

### Downloading the latest binary

Download the script with `curl` and run it.

```sh
curl -s https://raw.githubusercontent.com/ouch-org/ouch/master/install.sh | sh
```

Or with `wget`.

```sh
wget https://raw.githubusercontent.com/ouch-org/ouch/master/install.sh -O - | sh
```

The script will download the latest binary and copy it to `/usr/bin`.

### Installing from source code

For compiling, check [the wiki guide](https://github.com/ouch-org/ouch/wiki/Compiling-and-installing-from-source-code).


## Supported formats

|               | .tar | .zip | .bz, .bz2 | .gz | .xz, .lz, .lzma | .zst |
|:-------------:|:----:|:----:|:---------:| --- |:---------------:| --- |
| Decompression | ✓   | ✓   | ✓         | ✓  |   ✓            | ✗  |
|  Compression  | ✓   | ✓   | ✓         | ✓  |   ✓            | ✗  |

Note that formats can be chained:
- `.tar.gz`
- `.tar.xz`
- `.tar.gz.xz`
- `.tar.gz.gz.gz.gz.gz.gz.gz.gz.gz.gz.gz.gz.gz.gz.gz.gz.gz.gz.lz.lz.lz.lz.lz.lz.lz.lz.lz.lz.bz.bz.bz.bz.bz.bz.bz`
- `.gz.xz`
- etc...

## Contributing

`ouch` is 100% made out of voluntary work, any small contribution is welcome!

- Open an issue.
- Open a pr.
- Share it to a friend.
