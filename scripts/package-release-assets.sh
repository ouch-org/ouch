#!/usr/bin/env bash
set -e

cd downloaded_artifacts
mkdir ../assets

for input_dir in ouch-*; do
    cp ../{README.md,LICENSE,CHANGELOG.md} "$input_dir"
    mkdir "$input_dir/man"
    mkdir "$input_dir/artifacts"

    mv "$input_dir"/artifacts/*.1 "$input_dir/man"
    mv "$input_dir"/artifacts/* "$input_dir/completions"
    rm -r "$input_dir/artifacts"

    if [[ "$input_dir" = *.exe ]]; then
        target=${input_dir%.exe}
        mv "$input_dir/target/${target/ouch-/}/release/ouch.exe" "$input_dir"
        rm -r "$input_dir/target"
        mv "$input_dir" "$target"
        zip -r "../assets/$target.zip" "$target"
    else
        mv "$input_dir/target/${input_dir/ouch-/}/release/ouch" "$input_dir"
        rm -r "$input_dir/target"
        chmod +x "$input_dir/ouch"
        tar czf "../assets/$input_dir.tar.gz" "$input_dir"
    fi
done
