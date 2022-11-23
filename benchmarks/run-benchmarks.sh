#!/usr/bin/env bash
#
# Input files used:
# - `compiler` (27 MB) for compressed formats.
# - `rust`    (229 MB) for uncompressed formats.
#
# Compressed formats benchmarked:
# - .tar.gz
# - .zip
#
# Uncompressed formats benchmarked:
# - .tar

set -e

DESCOMPRESSION_CLEANUP="rm output -r"

function call_hyperfine() {
    hyperfine "$@" \
        --warmup 4 \
        --export-markdown "${FUNCNAME[1]}.md"
}

function tar_compression() {
    cleanup="rm output.tar"

    call_hyperfine \
        'ouch compress rust output.tar' \
        'tar -cvf output.tar rust' \
        --prepare "$cleanup || true"

    $cleanup
}

function tar_decompression() {
    echo "Creating tar archive to benchmark decompression..."
    ouch compress rust input.tar --yes &> /dev/null

    call_hyperfine \
        'ouch decompress input.tar --dir output' \
        'tar -xv -C output -f input.tar' \
        --prepare "$DESCOMPRESSION_CLEANUP || true" \
        --prepare "$DESCOMPRESSION_CLEANUP || true ; mkdir output"

    $DESCOMPRESSION_CLEANUP
}

function tar_gz_compression() {
    cleanup="rm output.tar.gz"

    call_hyperfine \
        'ouch compress compiler output.tar.gz' \
        'tar -cvzf output.tar.gz compiler' \
        --prepare "$cleanup || true"

    $cleanup
}

function tar_gz_decompression() {
    echo "Creating tar.gz archive to benchmark decompression..."
    ouch compress compiler input.tar.gz --yes &> /dev/null

    call_hyperfine \
        'ouch decompress input.tar.gz --dir output' \
        'tar -xvz -C output -f input.tar.gz' \
        --prepare "$DESCOMPRESSION_CLEANUP || true" \
        --prepare "$DESCOMPRESSION_CLEANUP || true ; mkdir output"

    $DESCOMPRESSION_CLEANUP
}

function zip_compression() {
    cleanup="rm output.zip"

    call_hyperfine \
        'zip output.zip -r compiler' \
        'ouch compress compiler output.zip' \
        --prepare "$cleanup || true"

    $cleanup
}

function zip_decompression() {
    echo "Creating zip archive to benchmark decompression..."
    ouch compress compiler input.zip --yes &> /dev/null

    call_hyperfine \
        'ouch decompress input.zip --dir output' \
        'unzip input.zip -d output' \
        --prepare "$DESCOMPRESSION_CLEANUP || true"

    $DESCOMPRESSION_CLEANUP
}

function run_benches() {
    tar_compression
    tar_decompression
    tar_gz_compression
    tar_gz_decompression
    zip_compression
    zip_decompression
}

function concatenate_results() {
    cat tar_compression.md <(echo) \
        tar_decompression.md <(echo) \
        tar_gz_compression.md <(echo) \
        tar_gz_decompression.md <(echo) \
        zip_compression.md <(echo) \
        zip_decompression.md > results.md
}

run_benches
concatenate_results

echo
echo "check results at results.md"
