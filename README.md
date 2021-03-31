# Ouch!

<!-- ![ouch_image](https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcR5ilNDTFZZ-Vy_ctm2YyAe8Yk0UT7lB2hIhg&usqp=CAU)  -->

`ouch` loosely stands for Obvious Unified Compression (ᵃⁿᵈ ᵈᵉᶜᵒᵐᵖʳᵉˢˢᶦᵒⁿ) Helper and aims to be an easy and intuitive way of compressing and decompressing files on the command-line.

- [Usage](#Usage)
    - [Decompressing files](#Decompressing-files)
    - [Compressing files/directories](#Compressing-files-and-directories)
    - [Listing files](#Listing-the-elements-of-an-archive)
- [Supported Formats](#Supported-formats)
- [Installation](#Installation)
- [Supported operating systems](#Supported-operating-systems)

**Note** 
   * This README represents the new, but not yet implemented, interface that `ouch` will use.
   * For current usage instructions, check [the old README](https://github.com/vrmiguel/ouch/blob/0f453e9dfc70066056b9cc40e8032dcc6ee703bc/README.md).

## Usage

### Decompressing files

To decompress any number of files, just supply them to `ouch`.

Use the `-o, --output` flag to redirect the output of decompressions to a folder.

```bash
# Decompress `a.zip`
ouch a.zip

# Decompress multiple files
ouch a.zip b.tar.gz

# Decompress multiple files but inside new_folder
ouch a.zip  b.tar.gz  c.tar.bz2 -o new_folder
```

### Compressing files and directories

The `compress` subcommand is able to compress files and folders. The **last** argument will be the **output file**. 

The compression format employed will be defined according to the output file's extension.

```bash
# Compress four files into `archive.zip`
ouch compress a b c d archive.zip

# Compress three files into a `.tar.bz2` archive
ouch compress a.mp4 b.jpg c.png files.tar.bz2

# Compress a folder and a file into `videos.tar.xz`
ouch compress Videos/ funny_meme.mp4 videos.tar.xz
```

### Listing the elements of an archive

(TODO -- not implemented at all)

```
# Shows the files and folders contained in videos.tar.xz
ouch list videos.tar.xz
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


## Installation

### Getting a pre-compiled binary

```bash
curl -s https://raw.githubusercontent.com/vrmiguel/ouch/master/install.sh | bash
```

### Building

A recent [Rust](rust-lang.org) toolchain is needed to build `ouch`. You can install it following the instructions at [rustup.rs](https://rustup.rs/).

Once [Cargo](https://doc.rust-lang.org/cargo/) is installed, run:

```bash
cargo install ouch
# or 
git clone https://github.com/vrmiguel/ouch
cargo install --path ouch
# or
git clone https://github.com/vrmiguel/ouch
cd ouch && cargo run --release
```

## Supported operating systems

`ouch` runs on Linux, macOS and Windows 10. Binaries are available on our [Releases](https://github.com/vrmiguel/ouch/releases) page.
Binaries are also available at the end of each (successful) [GitHub Actions](https://github.com/vrmiguel/ouch/actions) run. 

**Note on Windows**: colors are currently messed up on PowerShell but work fine on [ConEmu](https://conemu.github.io/). A feature for disabling colors is planned.


## Limitations

`ouch` does encoding and decoding in-memory, so decompressing very large files with it is not advisable.
