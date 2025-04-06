#!/usr/bin/env bash

set -e

mkdir release
cd downloaded_artifacts

for dir in ouch-*; do
    mkdir "$dir/man"
    mv "$dir"/artifacts/*.1 "$dir/man"

    mv "$dir/artifacts" "$dir/completions"

    cp ../{README.md,LICENSE,CHANGELOG.md} "$dir"

    if [[ "$dir" = *.exe ]]; then
        target=${dir%.exe}
        mv "$dir/target/${target/ouch-/}/release/ouch.exe" "$dir"
        rm -r "$dir/target"
        mv "$dir" "$target"
        zip -r "../release/$target.zip" "$target"
    else
        mv "$dir/target/${dir/ouch-/}/release/ouch" "$dir"
        rm -r "$dir/target"
        chmod +x "$dir/ouch"
        tar czf "../release/$dir.tar.gz" "$dir"
    fi
done
