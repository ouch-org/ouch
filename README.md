# ouch (_work in progress_)

`ouch` is the Obvious Unified Compression (and decompression) Helper. 


| Supported formats | .tar | .zip | .tar.{.lz,.gz, .bz}          | .zip.{.lz, .gz, .bz, .bz2}   | .bz | .gz | .lz, .lzma |
|-------------------|------|------|------------------------------|------------------------------|-----|-----|------------|
| Decompression     |   ✓  |   ✓  |               ✓              |               ✓              |  ✓  |  ✓  |      ✓     |
| Compression       |   ✓  |   ✓  |               ✗              |               ✗              |  ✗  |  ✗  |      ✗     |

## How does it work?

`ouch` infers commands from the extensions of its command-line options.

```
ouch 0.1.0
Vinícius R. Miguel
ouch is a unified compression & decompression utility

USAGE:
    ouch [OPTIONS] --input <input>...

FLAGS:
    -h, --help       Displays this message and exits
    -V, --version    Prints version information

OPTIONS:
    -i, --input <input>...    The input files or directories.
    -o, --output <output>     The output directory or compressed file.
```

### Examples

#### Decompressing a bunch of files

```bash
$ ouch -i file{1..5}.zip another_file.tar.gz yet_another_file.tar.bz
```

When no output file is supplied, `ouch` infers that it must decompress all of its input files. This will error if any of the input files are not decompressible.

#### Decompressing a bunch of files into a folder

```bash
$ ouch -i file{1..5}.tar.gz -o some-folder
# Decompresses file1.tar.gz, file2.tar.gz, file3.tar.gz, file4.tar.gz and file5.tar.gz to some-folder
# The folder `ouch` saves to will be created if it doesn't already exist
```

When the output file is not a compressed file, `ouch` will check if all input files are decompressible and infer that it must decompress them into the output file.

#### Compressing files 

```bash
$ ouch -i file{1..20} -o archive.tar
```

### Error scenarios

#### No clear decompression algorithm

```bash
$ ouch -i some-file -o some-folder
error: file 'some-file' is not decompressible.
```

`ouch` cannot infer `some-file`'s compression format since it lacks an extension. Likewise, `ouch` cannot infer that the output file given is a compressed file, so it shows the user an error.

## Installation

### Runtime dependencies

`ouch` depends on a few widespread libraries:
* libbz2
* liblzma

Both should be already installed in any mainstream Linux distribution.

If they're not, then:

* On Debian-based distros

`sudo apt install liblzma-dev libbz2-dev`

* On Arch-based distros

`sudo pacman -S xz bzip2`

The last dependency is a recent [Rust](https://www.rust-lang.org/) toolchain. If you don't have one installed, follow the instructions at [rustup.rs](https://rustup.rs/).

### Build process

Once the dependency requirements are met:

```bash
git clone https://github.com/vrmiguel/jacarex   # Clone the repo.
cargo install --path ouch # .. and install it 
```
