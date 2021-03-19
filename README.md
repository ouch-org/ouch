# ouch

`ouch` is the Obvious Unified Compression (and decompression) Helper. 

## How does it work?

`ouch` infers commands from the extensions of its command-line options.

```
ouch 0.1.0
ouch is a unified compression & decompression utility

USAGE:
    ouch [OPTIONS] --input <input>...

FLAGS:
    -h, --help       Displays this message and exits
    -V, --version    Prints version information

OPTIONS:
    -i, --input <input>...    Input files (TODO description)
    -o, --output <output>     Output file (TODO description)
```

### Examples

#### Decompressing a bunch of files

```bash
$ ouch -i file{1..5}.zip
info: attempting to decompress input files into single_folder
info: done!
```

When no output file is supplied, `ouch` infers that it must decompress all of its input files. This will error if any of the input files are not decompressable.

#### Decompressing a bunch of files into a folder

```bash
$ ouch -i file{1..5}.tar.gz -o some-folder
info: attempting to decompress input files into single_folder
info: done!
```

When the output file is not a compressed file, `ouch` will check if all input files are decompressable and infer that it must decompress them into the output file.

#### Compressing files 

```bash
$ ouch -i file{1..20} -o archive.tar
info: trying to compress input files into 'archive.tar'
info: done!
```

### Error scenarios

#### No clear decompression algorithm

```bash
$ ouch -i some-file -o some-folder
error: file 'some-file' is not decompressable.
```

`ouch` might (TODO!) be able to sniff a file's compression format if it isn't supplied in the future, but that is not currently implemented.



