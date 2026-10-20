#!/bin/bash
# Build a Tabletist AppImage from a release binary, on a Linux machine of the
# binary's architecture (as the release builds each one).
#
#   packaging/linux/appimage.sh <binary> <output.AppImage> <version>
#
# The AppImage holds the binary, the .desktop file and the icon, and no
# libraries: like the .tar.gz it needs glibc 2.39 or newer, and winit loads
# GL, Wayland, X11 and xkbcommon from the system, where the GPU driver is.
# appimagetool and the runtime it puts in front are pinned releases, checked
# against their SHA-256 before either is used. Needs curl and
# desktop-file-validate (desktop-file-utils); appimagetool fails without it.
set -euo pipefail

binary="$1"
output="$2"
version="$3"
here="$(cd "$(dirname "$0")" && pwd)"

tool_version=1.9.1
runtime_version=20251108
arch="$(uname -m)"
case "$arch" in
x86_64)
    tool_sha256=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
    runtime_sha256=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
    ;;
aarch64)
    tool_sha256=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
    runtime_sha256=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444
    ;;
*)
    echo "no AppImage tools pinned for $arch" >&2
    exit 1
    ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fetch() {
    local url="$1" file="$2" sha256="$3"
    curl --fail --location --retry 3 --silent --show-error "$url" --output "$file"
    echo "$sha256  $file" | sha256sum --check --strict
}
fetch "https://github.com/AppImage/appimagetool/releases/download/${tool_version}/appimagetool-${arch}.AppImage" \
    "$work/appimagetool" "$tool_sha256"
fetch "https://github.com/AppImage/type2-runtime/releases/download/${runtime_version}/runtime-${arch}" \
    "$work/runtime" "$runtime_sha256"
chmod 755 "$work/appimagetool"

# The binary and icon where a package would put them, linked from the
# AppDir's root, where appimagetool and the runtime look. The .desktop file is
# a copy: appimagetool rewrites it in place, which would replace a link.
appdir="$work/Tabletist.AppDir"
id=dev.tabletist.Tabletist
install -Dm755 "$binary" "$appdir/usr/bin/tabletist"
install -Dm644 "$here/$id.svg" "$appdir/usr/share/icons/hicolor/scalable/apps/$id.svg"
install -m644 "$here/$id.desktop" "$appdir/$id.desktop"
ln -s usr/bin/tabletist "$appdir/AppRun"
ln -s "usr/share/icons/hicolor/scalable/apps/$id.svg" "$appdir/$id.svg"

# appimagetool is an AppImage itself: extracted, it needs no FUSE here.
# VERSION goes into the .desktop file as X-AppImage-Version.
rm -f "$output"
APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$arch" VERSION="$version" \
    "$work/appimagetool" --no-appstream --runtime-file "$work/runtime" "$appdir" "$output"
echo "$output"
