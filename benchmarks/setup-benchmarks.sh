#!/usr/bin/env bash

# Run this script inside of the folder `benchmarks` to download
# the input files to run the benchmarks.
#
# ```
# cd benchmarks
# ./setup-benchmarks.sh
# ```
#
# It will download rust-lang's source code.
#
# After this, you can run `./run-benchmarks.sh`.
#
# Input files downloaded:
# - `compiler` (27 MB) for compressed formats.
# - `rust`    (229 MB) for uncompressed formats.

set -e

function setup() {
    if [[ -d "rust" || -d "compiler" ]]; then
        echo "Input files already exist, try deleting before downloading again."
        exit 1
    fi

    # Download the Rust 1.65.0 source code
    git clone -b 1.65.0 https://github.com/rust-lang/rust --depth 1

    # Delete write-protected files to make benchmark cleanup simpler
    rm rust/.git -fr

    # Separate the compiler code
    cp rust/compiler -r compiler
}

setup

echo "tip: if you see a git warning above, you can ignore it"
