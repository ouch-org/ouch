#! /usr/bin/sh
#
# Small script to help decompressing files from the CI to make manual releases

set -e

ouch --version

rm release -r || true

ouch decompress ouch-x86_64-pc-windows-msvc.exe.zip --dir release
mv release/ouch.exe release/ouch-x86_64-pc-windows-msvc.exe

ouch decompress ouch-x86_64-apple-darwin.zip --dir release
mv release/ouch release/ouch-x86_64-apple-darwin

ouch decompress ouch-x86_64-unknown-linux-musl.zip --dir release
mv release/ouch release/ouch-x86_64-linux-musl

dragon-drag-and-drop release/*

