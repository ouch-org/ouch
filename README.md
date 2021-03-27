# ouch (_work in progress_)

`ouch` is the Obvious Unified Compression (_and decompression_) Helper. 


| Supported formats | .tar | .zip | .tar.{.lz*,.gz, .bz}         | .zip.{.lz*, .gz, .bz*}       | .bz | .gz | .lz, .lzma |
|-------------------|------|------|------------------------------|------------------------------|-----|-----|------------|
| Decompression     |   ✓  |   ✓  |               ✓              |               ✓              |  ✓  |  ✓  |      ✓     |
| Compression       |   ✓  |   ✓  |               ✓              |               ✓              |  ✓  |  ✓  |      ✓     |

## How does it work?

`ouch` infers commands from the extensions of its command-line options.

```
ouch 0.1.3
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

When no output file is supplied, `ouch` infers that it must decompress all of its input files into the current folder. This will error if any of the input files are not decompressible.

#### Decompressing a bunch of files into a folder

```bash
$ ouch -i file{1..3}.tar.gz videos.tar.bz2 -o some-folder
# Decompresses file1.tar.gz, file2.tar.gz, file3.tar.gz and videos.tar.bz2 to some-folder
# The folder `ouch` saves to will be created if it doesn't already exist
```

When the output file is not a compressed file, `ouch` will check if all input files are decompressible and infer that it must decompress them into the output folder.

#### Compressing files 

```bash
$ ouch -i file{1..20} -o archive.tar
$ ouch -i Videos/ Movies/ -o media.tar.lzma
$ ouch -i src/ Cargo.toml Cargo.lock -o my_project.tar.gz
```

### Error scenarios

#### No clear decompression algorithm

```bash
$ ouch -i some-file -o some-folder
error: file 'some-file' is not decompressible.
```

`ouch` cannot infer `some-file`'s compression format since it lacks an extension. Likewise, `ouch` cannot infer that the output file given is a compressed file, so it shows the user an error.

```bash
$ ouch -i file other-file -o files.gz
error: cannot compress multiple files directly to Gzip.
       Try using an intermediate archival method such as Tar.
       Example: filename.tar.gz
```

Similar errors are shown if the same scenario is applied to `.lz/.lzma` and `.bz/.bz2`.

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

* Installing from [Crates.io](https://crates.io)

```bash
cargo install ouch
```

* Cloning and building

```bash
git clone https://github.com/vrmiguel/ouch
cargo install --path ouch
# or
cd ouch && cargo run --release
```

I also recommend stripping the release binary. `ouch`'s release binary (at the time of writing) only takes up a megabyte in space when stripped.

## Supported operating systems

`ouch` _should_ be cross-platform but is currently only tested (and developed) on Linux, on both x64-64 and ARM.

## Limitations

`ouch` does encoding and decoding in-memory, so decompressing very large files with `ouch` is not advisable.

## Contributions

Any contributions and suggestions are welcome!
