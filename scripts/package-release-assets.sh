#!/usr/bin/env bash
set -e

mkdir output_assets
echo "created folder 'output_assets/'"
ls -lA -w 1
cd downloaded_artifacts
echo "entered 'downloaded_artifacts/'"
ls -lA -w 1

PLATFORMS=(
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
DEFAULT_FEATURES="unrar+use_zlib+use_zstd_thin+bzip3"

for platform in "${PLATFORMS[@]}"; do
    path="ouch-${platform}"
    echo "Processing $path"

    if [ ! -d "${path}-${DEFAULT_FEATURES}" ]; then
        echo "ERROR: Could not find artifact directory for $platform with default features ($path)"
        exit 1
    fi
    mv "${path}-${DEFAULT_FEATURES}" "$path" # remove the annoying suffix

    cp ../{README.md,LICENSE,CHANGELOG.md} "$path"
    mkdir -p "$path/man"
    mkdir -p "$path/completions"

    mv "$path"/man-page-and-completions-artifacts/*.1 "$path/man"
    mv "$path"/man-page-and-completions-artifacts/* "$path/completions"
    rm -r "$path/man-page-and-completions-artifacts"

    if [[ "$platform" == *"-windows-"* ]]; then
        mv "$path/target/$platform/release/ouch.exe" "$path"
        rm -rf "$path/target"

        zip -r "../output_assets/${path}.zip" "$path"
        echo "Created output_assets/${path}.zip"
    else
        mv "$path/target/$platform/release/ouch" "$path"
        rm -rf "$path/target"
        chmod +x "$path/ouch"

        tar czf "../output_assets/${path}.tar.gz" "$path"
        echo "Created output_assets/${path}.tar.gz"
    fi
done

echo "Done."
