#!/usr/bin/env bash
set -e

mkdir -p output_assets
cd downloaded_artifacts

TARGETS=(
    "aarch64-pc-windows-msvc"
    "aarch64-unknown-linux-gnu"
    "aarch64-unknown-linux-musl"
    "armv7-unknown-linux-gnueabihf"
    "armv7-unknown-linux-musleabihf"
    "x86_64-apple-darwin"
    "x86_64-pc-windows-gnu"
    "x86_64-pc-windows-msvc"
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
)
DEFAULT_FEATURES="unrar+use_zlib+use_zstd_thin"

for target in "${TARGETS[@]}"; do
    input_dir="ouch-${target}-${DEFAULT_FEATURES}"

    if [ ! -d "$input_dir" ]; then
        echo "ERROR: Could not find artifact directory for $target with default features ($input_dir)"
        exit 1
    fi

    echo "Processing $input_dir"

    cp ../{README.md,LICENSE,CHANGELOG.md} "$input_dir"
    mkdir -p "$input_dir/man"
    mkdir -p "$input_dir/completions"

    mv "$input_dir"/man-page-and-completions-artifacts/*.1 "$input_dir/man"
    mv "$input_dir"/man-page-and-completions-artifacts/* "$input_dir/completions"
    rm -r "$input_dir/man-page-and-completions-artifacts"

    output_name="ouch-${target}"

    if [[ "$target" == *"-windows-"* ]]; then
        mv "$input_dir/target/$target/release/ouch.exe" "$input_dir"
        rm -rf "$input_dir/target"

        zip -r "../output_assets/${output_name}.zip" "$input_dir"
        echo "Created output_assets/${output_name}.zip"
    else
        mv "$input_dir/target/$target/release/ouch" "$input_dir"
        rm -rf "$input_dir/target"
        chmod +x "$input_dir/ouch"

        tar czf "../output_assets/${output_name}.tar.gz" "$input_dir"
        echo "Created output_assets/${output_name}.tar.gz"
    fi
done

echo "Done."
