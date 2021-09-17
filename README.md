# Ouch!

<!-- ![ouch_image](https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcR5ilNDTFZZ-Vy_ctm2YyAe8Yk0UT7lB2hIhg&usqp=CAU)  -->

`ouch` stands for **Obvious Unified Compression Helper**, and works on _Linux_, _Mac OS_ and _Windows_.

It is a CLI tool to compress and decompress files that aims to be easy to use.

<!-- TODO -->
<!--     - [Listing files](#Listing-the-elements-of-an-archive) -->

- [Installation](#Installation)
- [Usage](#Usage)
    - [Decompressing files](#Decompressing-files)
    - [Compressing files/directories](#Compressing-files-and-directories)
- [Supported Formats](#Supported-formats)
- [Supported operating systems](#Supported-operating-systems)

## Usage

### Decompressing

Run `ouch` and pass compressed files as arguments.

```sh
# Decompress 'a.zip'
ouch a.zip

# Decompress multiple files
ouch a.zip b.tar.gz
```

You can redirect the decompression results to a folder with the `-o/--output` flag.

```sh
# Create 'pictures' folder and decompress inside of it
ouch 1.tar.gz 2.tar.gz -o pictures
```

### Compressing

Use the `compress` subcommand.

Accepts multiple files and folders, the **last** argument shall be the **output file**.

```sh
# Compress four files into 'archive.zip'
ouch compress 1 2 3 4 archive.zip

# Compress folder and video into 'videos.tar.gz'
ouch compress videos/ meme.mp4 videos.tar.gz

# Compress one file using 4 compression formats
ouch compress file.txt compressed.gz.xz.bz.zst

# Compress all the files in current folder
ouch compress * files.zip
```

`ouch` checks for the extensions of the **output file** to decide which formats should be used.

Check the [list of all file extensions supported](#Supported-formats).

<!-- ### Listing the elements of an archive

* **Upcoming feature**

```
# Shows the files and folders contained in videos.tar.xz
ouch list videos.tar.xz
``` -->

## Installation

### Downloading the latest binary

WARNING: SCRIPT TEMPORARILY DISABLED.

This script downloads the latest binary and copies it to `/usr/bin`.

```sh
curl -s https://raw.githubusercontent.com/vrmiguel/ouch/master/install.sh | sh
```

### Installing from source code

For compiling, check [the wiki guide](https://github.com/ouch-org/ouch/wiki/Compiling-and-installing-from-source-code).


## Supported formats

|               | .tar | .zip | .bz, .bz2 | .gz | .xz, .lz, .lzma | .7z |
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
