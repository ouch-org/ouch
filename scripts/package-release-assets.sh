#!/usr/bin/env bash
set -e

# use the commit timestamp as SOURCE_DATE_EPOCH so packaging is reproducible.
# tar/gzip/zip honour this when set.
if [ -z "${SOURCE_DATE_EPOCH:-}" ]; then
    SOURCE_DATE_EPOCH=$(git -C .. log -1 --pretty=%ct)
    export SOURCE_DATE_EPOCH
fi
echo "SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH"

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

    # normalise mtimes so tar/zip output is reproducible
    find "$path" -exec touch -h -d "@${SOURCE_DATE_EPOCH}" {} +

    if [[ "$platform" == *"-windows-"* ]]; then
        mv "$path/target/$platform/release/ouch.exe" "$path"
        rm -rf "$path/target"

        # -X strips extra fields (uid/gid/timestamps) that vary between runs
        (cd "$(dirname "$path")" && find "$(basename "$path")" | LC_ALL=C sort | \
            zip -X -@ "../output_assets/${path}.zip")
        echo "Created output_assets/${path}.zip"
    else
        mv "$path/target/$platform/release/ouch" "$path"
        rm -rf "$path/target"
        chmod +x "$path/ouch"

        # --sort=name pins file order, --owner/--group/--numeric-owner pin uids,
        # --mtime pins timestamps. piping through gzip -n drops the gzip header
        # timestamp and original-name field.
        tar --sort=name \
            --owner=0 --group=0 --numeric-owner \
            --mtime="@${SOURCE_DATE_EPOCH}" \
            -cf - "$path" | gzip -n -9 > "../output_assets/${path}.tar.gz"
        echo "Created output_assets/${path}.tar.gz"
    fi
done

echo "Done."
