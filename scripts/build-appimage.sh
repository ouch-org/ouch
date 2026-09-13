#!/usr/bin/env bash
set -e

# Builds a single-file AppImage from a compiled ouch binary.
# Usage: build-appimage.sh <binary> <arch:x86_64|aarch64> <output>

BINARY="$1"
APPIMAGE_ARCH="$2"
OUTPUT="$3"

if [ -z "$BINARY" ] || [ -z "$APPIMAGE_ARCH" ] || [ -z "$OUTPUT" ]; then
    echo "usage: build-appimage.sh <binary> <arch> <output>" >&2
    exit 1
fi

case "$APPIMAGE_ARCH" in
    x86_64 | aarch64) ;;
    *)
        echo "unsupported AppImage arch: $APPIMAGE_ARCH" >&2
        exit 1
        ;;
esac

APPIMAGETOOL_VERSION="1.9.0"

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

APPDIR="$WORKDIR/ouch.AppDir"
mkdir -p "$APPDIR/usr/bin"

cp "$BINARY" "$APPDIR/usr/bin/ouch"
chmod +x "$APPDIR/usr/bin/ouch"

cat > "$APPDIR/ouch.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=ouch
Comment=A command-line utility for easily compressing and decompressing files and directories
Exec=ouch
Icon=ouch
Categories=Utility;
Terminal=true
EOF

# dummy 1x1 transparent PNG
base64 -d > "$APPDIR/ouch.png" <<'EOF'
iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGNgAAIAAAUAAeImBZsAAAAASUVORK5CYII=
EOF
ln -s ouch.png "$APPDIR/.DirIcon"

cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/ouch" "$@"
EOF
chmod +x "$APPDIR/AppRun"

# appimagetool runs on the build host, so download it for the host arch
case "$(uname -m)" in
    x86_64) TOOL_ARCH="x86_64" ;;
    aarch64 | arm64) TOOL_ARCH="aarch64" ;;
    *)
        echo "unsupported host arch for appimagetool: $(uname -m)" >&2
        exit 1
        ;;
esac

APPIMAGETOOL="$WORKDIR/appimagetool"
wget -q -O "$APPIMAGETOOL" \
    "https://github.com/AppImage/appimagetool/releases/download/${APPIMAGETOOL_VERSION}/appimagetool-${TOOL_ARCH}.AppImage"
chmod +x "$APPIMAGETOOL"

RUNTIME="$WORKDIR/runtime-$APPIMAGE_ARCH"
wget -q -O "$RUNTIME" \
    "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-$APPIMAGE_ARCH"

# no FUSE needed in CI; unset SOURCE_DATE_EPOCH or mksquashfs aborts (appimagetool sets its own timestamp)
env -u SOURCE_DATE_EPOCH \
    ARCH="$APPIMAGE_ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL" \
    --no-appstream \
    --runtime-file "$RUNTIME" \
    "$APPDIR" "$OUTPUT"

echo "Created $OUTPUT"
