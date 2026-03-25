<div align="center">
  <img src="src/assets/icon.png" alt="imechen" height="80">
  <p>画像・動画をかんたんに変換・圧縮できるデスクトップアプリ</p>

  <a href="https://github.com/manomekun/imechen/releases/latest">
    <img src="https://img.shields.io/github/v/release/manomekun/imechen?label=%E3%83%80%E3%82%A6%E3%83%B3%E3%83%AD%E3%83%BC%E3%83%89&style=for-the-badge&color=F97316" alt="ダウンロード">
  </a>
</div>

---

## imechenとは？

imechenは、画像や動画の形式変換・圧縮をオフラインで行えるデスクトップアプリです。
ファイルを選んでボタンを押すだけで、さまざまな形式に変換できます。

インターネット接続不要で、すべての処理がローカルで完結します。

## 主な機能

- **画像変換** — JPEG, PNG, GIF, WebP, BMP, HEIF を相互変換
- **動画変換** — MP4, MOV, AVI, WebM, FLV, HEVC を相互変換
- **アニメーション作成** — 連番画像から GIF / Animated WebP / MP4 を生成
- **リサイズ** — 幅×高さ指定、縦横比維持オプション付き
- **圧縮** — JPEG/WebP は品質指定、PNG は pngquant + oxipng で最適化
- **解像度変更** — 動画の解像度をプリセットまたは自由入力で変更
- **ダーク / ライトテーマ** — OS連動を含む3モード切替
- **完全オフライン** — 外部サーバーへの通信なし

## ダウンロード

**[最新版をダウンロード](https://github.com/manomekun/imechen/releases/latest)**

| OS | ファイル |
|---|---|
| macOS（Apple Silicon） | `imechen_x.x.x_aarch64.dmg` |
| macOS（Intel） | `imechen_x.x.x_x64.dmg` |
| Windows | `imechen_x.x.x_x64-setup.exe` |
| Linux | `imechen_x.x.x_amd64.deb` / `.AppImage` |

> **Apple Siliconって何？** — 2020年以降のMac（M1/M2/M3/M4チップ搭載）はApple Silicon版を選んでください。それ以前のMacはIntel版を選んでください。

## インストール方法

### macOS（かんたん）

ターミナルを開いて、以下をコピペして Enter を押してください：

```bash
curl -fsSL https://raw.githubusercontent.com/manomekun/imechen/main/install.sh | bash
```

> **ターミナルの開き方** — Spotlight（`⌘ + Space`）で「ターミナル」と検索して開けます。

ダウンロードからインストール、セキュリティ設定まですべて自動で行われます。Apple Silicon / Intel も自動判定します。

### macOS（Homebrew）

```bash
brew install manomekun/apps/imechen
```

### macOS（手動）

1. 上のリンクから `.dmg` ファイルをダウンロード
2. ダウンロードした `.dmg` を開く
3. `imechen` を `Applications` フォルダにドラッグ
4. 初回起動時に「開発元が未確認」と表示されて開けない場合、ターミナルで以下を実行：
   ```
   xattr -cr /Applications/imechen.app
   ```
   > macOS 15 以降、署名されていないアプリの起動制限が強化されました。上記コマンドでダウンロード時に付与される隔離属性を除去することで起動できます。

### Windows

1. 上のリンクから `.exe` ファイルをダウンロード
2. ダウンロードしたファイルを実行
3. SmartScreen の警告が出た場合：「詳細情報」→「実行」をクリック

### Linux

```bash
# .deb の場合
sudo dpkg -i imechen_*.deb

# .AppImage の場合
chmod +x imechen_*.AppImage
./imechen_*.AppImage
```

## 使い方

### 画像変換

1. 「画像変換」タブを選択
2. 画像ファイルをドラッグ＆ドロップまたはクリックして選択
3. 出力形式（PNG, JPEG, WebP など）を選ぶ
4. 必要に応じてリサイズや品質を設定
5. 「変換開始」ボタンをクリック

### 動画変換

1. 「動画変換」タブを選択
2. 動画ファイルを選択
3. 出力形式と解像度・画質を設定
4. 「変換開始」ボタンをクリック

> 初回起動時に ffmpeg が自動でダウンロードされます。少し時間がかかりますが、2回目以降は即座に使えます。

### アニメーション作成

1. 「アニメーション」タブを選択
2. 連番画像を選択（自動でファイル名順にソート）
3. 出力形式（GIF / Animated WebP / MP4）と FPS を設定
4. 「生成開始」ボタンをクリック

## 対応形式

| 種類 | 入力 | 出力 |
|---|---|---|
| 画像 | JPEG, PNG, GIF, WebP, BMP, HEIF | JPEG, PNG, GIF, WebP, BMP |
| 動画 | MP4, MOV, AVI, WebM, FLV, HEVC | MP4, MOV, AVI, WebM, FLV |
| アニメーション | 連番画像（PNG, JPEG, BMP） | GIF, Animated WebP, MP4 |

## 開発者向け情報

### 技術スタック

- **Tauri v2** — デスクトップアプリフレームワーク
- **Solid.js** — フロントエンド（TypeScript）
- **Rust** — バックエンド
- **image crate** — 画像処理
- **imagequant + oxipng** — PNG 圧縮
- **webp crate** — WebP エンコード
- **ffmpeg-sidecar** — 動画処理

### 開発環境のセットアップ

```bash
# 依存関係のインストール
bun install

# 開発サーバー起動
bun run tauri dev

# プロダクションビルド
bun run tauri build
```

## ライセンス

[BSL 1.1](LICENSE.md)（Business Source License 1.1）

- ソースコードの閲覧: OK
- 商用・本番利用: NG（ライセンス購入が必要）
- 2030-03-25 以降 Apache License 2.0 に移行
