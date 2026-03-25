#!/bin/bash
set -euo pipefail

APP_NAME="imechen"
REPO="manomekun/imechen"

echo "=== ${APP_NAME} インストーラー ==="

# Get latest version from GitHub
echo "最新バージョンを確認中..."
VERSION=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": "//;s/".*//')

if [ -z "$VERSION" ]; then
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
TMP_DMG="/tmp/${DMG_NAME}"

# Download
echo "ダウンロード中: ${DMG_NAME}..."
curl -L -o "${TMP_DMG}" "${DOWNLOAD_URL}"

if [ ! -f "${TMP_DMG}" ]; then
    echo "エラー: ダウンロードに失敗しました"
    exit 1
fi

# Remove existing app
if [ -d "/Applications/${APP_NAME}.app" ]; then
    echo "既存のアプリを削除中..."
    rm -rf "/Applications/${APP_NAME}.app"
fi

# Mount DMG
echo "インストール中..."
MOUNT_POINT=$(hdiutil attach "${TMP_DMG}" -nobrowse -noautoopen | grep '/Volumes/' | awk '{print $NF}')

if [ -z "${MOUNT_POINT}" ]; then
    echo "エラー: DMGのマウントに失敗しました"
    rm -f "${TMP_DMG}"
    exit 1
fi

# Copy app
cp -R "${MOUNT_POINT}/${APP_NAME}.app" /Applications/

# Unmount
hdiutil detach "${MOUNT_POINT}" -quiet

# Remove quarantine attribute
xattr -cr "/Applications/${APP_NAME}.app" 2>/dev/null || true

# Cleanup
rm -f "${TMP_DMG}"

echo ""
echo "=== インストール完了 ==="
echo "${APP_NAME} ${VERSION} を /Applications/ にインストールしました"
echo "Launchpad または Finder から起動できます"
