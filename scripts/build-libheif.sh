#!/usr/bin/env bash
# Builds a static libheif that can only decode HEVC (via libde265) into PREFIX,
# for libheif-sys to find through pkg-config.
#
# libheif's CMake enables every codec it can find on the build machine, so a
# plain build links Homebrew/apt dylibs (x265, x264, aom, ...) that end users
# don't have, and x265/x264 are GPL. Every codec except libde265 is switched
# off here so the result does not depend on what the runner has installed.
#
# Usage: scripts/build-libheif.sh PREFIX
# Env:   OSX_ARCH (macOS only) - arm64 or x86_64, for cross-building the x64 dmg
#        on an arm64 runner.
set -euo pipefail

LIBDE265_VERSION=1.1.3
LIBDE265_SHA256=554228bd17788c99a7e63b37ab5634722190e6e2bf60c1dcb01cef328e133905
LIBHEIF_VERSION=1.23.7
LIBHEIF_SHA256=f7483e818503ea6224c8863e4a56ac02c7089021a98472c811c12aa19faf2475

prefix=${1:?usage: build-libheif.sh PREFIX}
mkdir -p "$prefix"
prefix=$(cd "$prefix" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

fetch() {
  local url=$1 sha=$2 out=$work/$(basename "$1")
  curl -fsSL --retry 3 -o "$out" "$url"
  local got
  got=$(sha256 "$out")
  if [ "$got" != "$sha" ]; then
    echo "checksum mismatch for $url: expected $sha, got $got" >&2
    exit 1
  fi
  tar -xzf "$out" -C "$work"
}

fetch "https://github.com/strukturag/libde265/releases/download/v${LIBDE265_VERSION}/libde265-${LIBDE265_VERSION}.tar.gz" "$LIBDE265_SHA256"
fetch "https://github.com/strukturag/libheif/releases/download/v${LIBHEIF_VERSION}/libheif-${LIBHEIF_VERSION}.tar.gz" "$LIBHEIF_SHA256"

common=(
  -DCMAKE_BUILD_TYPE=Release
  -DCMAKE_INSTALL_PREFIX="$prefix"
  -DCMAKE_INSTALL_LIBDIR=lib
  -DCMAKE_PREFIX_PATH="$prefix"
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON
  -DBUILD_SHARED_LIBS=OFF
)
de265_extra=()
if [ "$(uname -s)" = Darwin ]; then
  # Must match minimum-system-version in crates/imechen-gpui/Cargo.toml.
  export MACOSX_DEPLOYMENT_TARGET=12.0
  osx_arch=${OSX_ARCH:-$(uname -m)}
  common+=(-DCMAKE_OSX_ARCHITECTURES="$osx_arch")
  if [ "$osx_arch" = x86_64 ]; then
    # The AVX2/AVX-512 kernels pick themselves via __builtin_cpu_supports,
    # whose __cpu_model lives in clang's compiler-rt. rustc links with
    # -nodefaultlibs, so the symbol stays undefined. The SSE4.1 kernels remain.
    de265_extra+=(-DENABLE_AVX2=OFF -DENABLE_AVX512=OFF)
  fi
fi

cmake -S "$work/libde265-${LIBDE265_VERSION}" -B "$work/build-de265" "${common[@]}" \
  ${de265_extra[@]+"${de265_extra[@]}"} \
  -DENABLE_DECODER=OFF \
  -DENABLE_ENCODER=OFF \
  -DENABLE_SDL=OFF
cmake --build "$work/build-de265" --parallel
cmake --install "$work/build-de265"

codecs_off=()
for codec in X265 KVAZAAR UVG266 VVDEC VVENC X264 OpenH264_DECODER DAV1D AOM_DECODER AOM_ENCODER \
  SvtEnc RAV1E JPEG_DECODER JPEG_ENCODER OpenJPEG_ENCODER OpenJPEG_DECODER FFMPEG_DECODER OPENJPH_ENCODER; do
  codecs_off+=("-DWITH_${codec}=OFF")
done

PKG_CONFIG_PATH="$prefix/lib/pkgconfig" \
cmake -S "$work/libheif-${LIBHEIF_VERSION}" -B "$work/build-heif" "${common[@]}" \
  -DWITH_LIBDE265=ON \
  -DWITH_LIBDE265_PLUGIN=OFF \
  "${codecs_off[@]}" \
  -DENABLE_PLUGIN_LOADING=OFF \
  -DWITH_LIBSHARPYUV=OFF \
  -DWITH_UNCOMPRESSED_CODEC=OFF \
  -DWITH_HEADER_COMPRESSION=OFF \
  -DWITH_EXAMPLES=OFF \
  -DWITH_GDK_PIXBUF=OFF \
  -DBUILD_TESTING=OFF \
  -DBUILD_DOCUMENTATION=OFF
cmake --build "$work/build-heif" --parallel
cmake --install "$work/build-heif"

if ! grep -q 'LIBDE265_FOUND:INTERNAL=1\|LIBDE265_FOUND:BOOL=TRUE' "$work/build-heif/CMakeCache.txt" \
  && ! grep -q 'libde265' "$prefix/lib/pkgconfig/libheif.pc"; then
  echo "libheif was built without libde265, so it could not decode HEIC" >&2
  exit 1
fi
