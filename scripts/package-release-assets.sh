#!/usr/bin/env bash
set -e

mkdir release
cd downloaded_artifacts

for dir in ouch-*; do
    mkdir "$dir/man"
    mkdir "$dir/completions"
    mv "$dir/artifacts/*.1" "$dir/man"
    mv "$dir/artifacts/*" "$dir/completions"
    rm -r "$dir/artifacts"

    cp ../{README.md,LICENSE,CHANGELOG.md} "$dir"
    rm -r "$dir/artifacts"

    if [[ "$dir" == *-pc-windows-* ]]; then
        binary_path="$dir/target/${target/ouch-/}/release/ouch.exe"
        rm -r "$dir/target"
        mv "$dir" "$target"
    else
        binary_path="$dir/target/${dir/ouch-/}/release/ouch"
        rm -r "$dir/target"
        chmod +x "$dir/ouch"
        tar czf "../release/$dir.tar.gz" "$dir"
    fi

    if [[ "$dir" == *-pc-windows-* ]]; then
        zip -r "../release/$dir.zip" "$dir"
    else
        chmod +x "$binary_path"
        tar czf "../release/$dir.tar.gz" "$dir"
    fi
done
