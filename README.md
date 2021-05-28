# Ouch!

<!-- ![ouch_image](https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcR5ilNDTFZZ-Vy_ctm2YyAe8Yk0UT7lB2hIhg&usqp=CAU)  -->

`ouch` loosely stands for Obvious Unified Compression files Helper.

It is an easy and painless way of compressing and decompressing files in the terminal.

Works in `Linux`, `Mac OS` and `Windows`.

<!--     - [Listing files](#Listing-the-elements-of-an-archive) -->

- [Usage](#Usage)
    - [Decompressing files](#Decompressing-files)
    - [Compressing files/directories](#Compressing-files-and-directories)
- [Installation](#Installation)
- [Supported Formats](#Supported-formats)
- [Supported operating systems](#Supported-operating-systems)

## Usage

### Decompressing

Run `ouch` and pass compressed files as arguments.

```sh
# Decompress `a.zip`
ouch a.zip

# Decompress multiple files
ouch a.zip b.tar.gz
```

Use the `-o/--output` flag to redirect the output of decompressions to a folder.

```sh
# Decompress multiple files but inside new_folder
ouch a.zip  b.tar.gz  c.tar.bz2 -o new_folder
```

### Compressing

Use the `compress` subcommand.

Accepts files and folders, and the **last** argument shall be the **output file**.

```sh
# Compress four files into `archive.zip`
ouch compress 1 2 3 4 archive.zip
```

The supplied **output file** shall have a supported compression format, [see the list](#Supported-formats).

You can also use the `c` alias for this subcommand.

```sh
# Compress a folder and a file into `videos.tar.xz`
ouch c Videos/ funny_meme.mp4 videos.tar.xz

# Compress three files into a `.tar.bz2` archive
ouch c a.mp4 b.jpg c.png files.tar.bz2

# Compress two folders into a lzma file
ouch c src/ target/ build.tar.lz
```

<!-- ### Listing the elements of an archive

* **Upcoming feature**

```
# Shows the files and folders contained in videos.tar.xz
ouch list videos.tar.xz
``` -->

## Installation

### Installing a binary

This script downloads the latest binary and copies it to `/usr/bin`.
```sh
curl -s https://raw.githubusercontent.com/vrmiguel/ouch/master/install.sh | sh
```

### Compiling
Install [Rust](rust-lang.org) and [Cargo](https://doc.rust-lang.org/cargo/) via [rustup.rs](https://rustup.rs/).

From latest official release:
```sh
cargo install ouch
```

From repository source code:

```sh
git clone https://github.com/vrmiguel/ouch
cargo build
```

## Supported formats

|               | .tar | .zip | .tar.\*¹ | .zip.\*² | .bz, .bz2 | .gz | .xz, .lz, .lzma | .7z |
|:-------------:|:----:|:----:|:--------:|:--------:|:---------:| --- |:---------------:| --- |
| Decompression |  ✓   |  ✓   |    ✓     |    ✓     |     ✓     | ✓   |        ✓        | ✗   |
|  Compression  |  ✓   |  ✓   |    ✓     |    ✓     |     ✓     | ✓   |        ✓        | ✗   |

```
Note: .tar.*¹: .tar.gz, .tar.bz, .tar.bz2, .tar.xz, .tar.lz, .tar.lzma, .tar.zip
      .zip.*²: .zip.gz, .zip.bz, .zip.bz2, .zip.xz, .zip.lz, .zip.lzma, .zip.zip
```

<!-- ## Supported operating systems

`ouch` runs on Linux, macOS and Windows 10. Binaries are available on our [Releases](https://github.com/vrmiguel/ouch/releases) page.

Binaries are also available at the end of each (successful) [GitHub Actions](https://github.com/vrmiguel/ouch/actions) run for these targets:

* Linux x86-64 statically linked (musl libc) 
* macOS x86-64 dynamically linked
* Windows 10
* Linux ARMv7 dynamically linked (glibc)

One must be logged into GitHub to access build artifacts. -->
