#!/bin/bash
set -euo pipefail

APP_NAME="imechen"
REPO="manomekun/imechen"

echo "=== ${APP_NAME} インストーラー ==="

# Get latest version from GitHub
echo "最新バージョンを確認中..."
VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": "//;s/".*//')

# The tag is interpolated into a URL and file paths, so reject anything unexpected.
if ! [[ "$VERSION" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "エラー: バージョンの取得に失敗しました"
    exit 1
fi

echo "バージョン: ${VERSION}"

# Detect architecture
ARCH=$(uname -m)
if [ "$ARCH" = "arm64" ]; then
    DMG_NAME="${APP_NAME}_${VERSION}_aarch64.dmg"
elif [ "$ARCH" = "x86_64" ]; then
    DMG_NAME="${APP_NAME}_${VERSION}_x64.dmg"
else
    echo "エラー: サポートされていないアーキテクチャ: ${ARCH}"
    exit 1
fi

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${DMG_NAME}"
TMP_DIR=$(mktemp -d)
TMP_DMG="${TMP_DIR}/${DMG_NAME}"
MOUNT_POINT="${TMP_DIR}/mnt"
cleanup() {
    hdiutil detach "${MOUNT_POINT}" -quiet 2>/dev/null || true
    rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

# Download
echo "ダウンロード中: ${DMG_NAME}..."
if ! curl -fL -o "${TMP_DMG}" "${DOWNLOAD_URL}"; then
    echo "エラー: ダウンロードに失敗しました"
    exit 1
fi

# Mount DMG
echo "インストール中..."
if ! hdiutil attach "${TMP_DMG}" -nobrowse -noautoopen -mountpoint "${MOUNT_POINT}" -quiet; then
    echo "エラー: DMGのマウントに失敗しました"
    exit 1
fi

# Remove the existing app only once the new one is ready to copy.
if [ -d "/Applications/${APP_NAME}.app" ]; then
    echo "既存のアプリを削除中..."
    rm -rf "/Applications/${APP_NAME}.app"
fi

# Copy app
cp -R "${MOUNT_POINT}/${APP_NAME}.app" /Applications/

# Remove quarantine attribute
xattr -cr "/Applications/${APP_NAME}.app" 2>/dev/null || true

echo ""
echo "=== インストール完了 ==="
echo "${APP_NAME} ${VERSION} を /Applications/ にインストールしました"
echo "Launchpad または Finder から起動できます"
