---
name: Share サブメニュー + Result PNG 書き出し
type: task
status: planned
---

# 目的

グローバル操作バーの **Share** を、次の2アクションを持つサブメニューに分割する。

| アクション | 挙動 |
|------------|------|
| **Copy link** | 現行と同様、可読フラグメント `#v=1&…` 付き共有 URL をクリップボードへコピーしトースト表示。 |
| **Download image** | **Result のフラクタルのみ**をオフスクリーン描画し PNG としてユーザーに渡す。 |

**iOS（Safari / PWA）**では、Web 標準だけで「写真アプリに自動保存」はできない。ユーザーが **共有シート経由で「写真に保存」** できるよう、**Web Share API（`files` 付き）** を優先し、失敗時は従来の **Blob + `<a download>`** にフォールバックする方針とする。

検証用バイナリ `result_png_probe` およびその専用ワークフローは **本実装後に削除してよい**（本番はアプリ内プラグイン／システムに集約）。

# 背景・既に分かっていること（チャット検証より）

- Result 描画は `src/fractal.rs` の **`LineList` + 頂点色**（`FractalMesh`）。UI は写さない経路が必要。
- Bevy 0.18 では **`Camera` に `target` はなく、`RenderTarget` が別コンポーネント**。オフスクリーンは `RenderTarget::Image(Handle<Image>)`。
- キャプチャは `bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured}` + オブザーバーでディスク／メモリ書き出し可能（`examples/window/screenshot.rs` と同系）。
- **細線**はモバイルで暗く見える → 書き出し用は **クワッド（`TriangleList`）で論理ピクセル換算の太さ**が有効。ウィンドウ表示の Result と **同じメッシュを太くすると 1 フレーム両方に効く**ため、**別 `RenderLayers` の export 専用メッシュ**を推奨。
- 共有 URL のデコードは `src/share.rs` の **`decode_readable_share_query`**（`pub`）。エンコード・完全 URL 組み立ては `encode_state` / `share_url_from_token`。
- 出力解像度・線幅の経験値: 例として **4096²**、線幅相当 **4 px**（環境・プリセットで要調整）。

# スコープ（本タスクに含める）

- `src/ui.rs` の Share ボタン → **`menu_button("Share", …)`** 化（既存 Preset メニューと同パターン）。
- **Copy link**: 現在 `Share` 単体クリックで行っている `share::encode_state` → `share::share_url_from_token` → `egui::copy_text` を **Copy link** に移すのみでよい。
- **Download image**: アプリ内パイプライン（下記アーキテクチャ）。
- **WASM**: PNG バイト列から共有／ダウンロード（`web-sys` 機能追加が必要になる可能性大）。
- **ネイティブ（macOS 等）**: 保存先ダイアログ（`rfd` 等）または `~/Downloads` 固定など要決定。
- **削除**: `src/bin/result_png_probe.rs`、Cargo の `[[bin]]` 自動検出があれば除去、ドキュメント上の probe 用環境変数説明の整理。`FractalLineHalfWidth` が **ウィンドウメッシュにのみ**残っていないことを確認（検証で導入した場合、export 専用へ寄せる）。

# スコープ外（別タスクでよい）

- SVG 出力（計画段階で重すぎるケースが分かっている）。
- エクスポート中のプログレスバー（初版はトーストのみで可）。
- ユーザー設定で解像度／線幅スライダ（初版は定数でも可）。

# 推奨アーキテクチャ

## 1. レイヤ分離

| 要素 | レイヤ（例） | 備考 |
|------|----------------|------|
| 既存 Result フラクタル | `result_layer()` = `RenderLayers::layer(2)` | 現状維持、`LineList`。 |
| Export 専用フラクタルメッシュ | 新規 `result_export_layer()` = `layer(6)` 等 | **太線クワッド**のみ。ウィンドウ Result カメラからは見えない。 |
| Export 専用カメラ | `RenderTarget::Image`、**既定 `is_active: false`** | 書き出し中のみ有効化。`order` はウィンドウカメラと衝突しないよう調整。 |

## 2. パイプライン（概念的）

1. UI が **Download image** でイベント（例 `RequestResultImageExport`）を送信。
2. 専用システムが `FractalState` から **AABB + `result_projection_fit`**（`fractal::result_projection_fit`）で正射影 `side` と中心を算出。
3. 目標ピクセル `N`（例 4096）と **線幅 px**（例 4）から **半幅ワールド** `half = line_px * side / (2 * N)` を計算。
4. Export メッシュを `collect_fractal_segments_thick_quads` 系で再構築 → `Image` を `N×N` にリサイズ／生成 → Export カメラをオン。
5. 数フレーム待機後 `Screenshot::image` を spawn、コールバックで RGB PNG バイトを取得。
6. プラットフォーム別保存処理 → カメラオフ／メッシュ空クリアなど後始末。
7. トースト（成功／失敗）。

`PendingShareUrlSync` や URL フラッシュとは独立でよい。**メインワールドの同期負荷**に注意（深いフラクタルは GPU メモリ・頂点数が増える）。

## 3. WASM（iOS 含む）

- **第一選択**: `navigator.share({ files: [File], title: … })`（ユーザーが共有シートで「写真に保存」可能）。
- **フォールバック**: `Blob` + 一時 `object URL` + 非表示 `<a download>` クリック（iOS は挙動が端末・バージョン依存）。
- 必要になる `web-sys` / `js-sys` の feature は実装時に列挙（`Blob`, `File`, `Url`, `Navigator`, Share 関連など）。
- Clipboard API は **HTTPS** 前提（既存サイトは GitHub Pages なので問題にならない想定）。

## 4. ネイティブ

- **`rfd`**: 「名前を付けて保存」が素直。**optional dependency** で `cfg(not(target_arch = "wasm32"))` に限定するとよい。
- または初版は Downloads への固定ファイル名保存（UX は劣る）。

# 関連ファイル（実装時の起点）

| ファイル | 内容 |
|---------|------|
| `src/ui.rs` | `global_controls_bar` の Share 部分（現状 ~486 行付近）、`Preset` と同様の `menu_button` パターン。 |
| `src/share.rs` | `encode_state`, `share_url_from_token`, `decode_readable_share_query`。 |
| `src/fractal.rs` | `FractalPlugin`, メッシュ構築、**太線クワッド**関数、（既存なら）`result_projection_fit`。 |
| `src/lib.rs` | `add_plugins`、`result_layer`、Export レイヤ関数の公開先。 |

# 受け入れ条件（チェックリスト）

- [ ] Share がサブメニューになり **Copy link** / **Download image** が並ぶ。
- [ ] Copy link で従来と同等のトースト付きコピー。
- [ ] Download で Result のみが写った PNG が取得できる（解像度・線幅は定数でよい）。
- [ ] **WASM でビルドが通り**、iOS Safari で共有シートまたはダウンロードのいずれかでユーザーが画像を保存できる。
- [ ] ネイティブでファイルがディスクに保存できる（または保存ダイアログ）。
- [ ] `result_png_probe` 削除済み、デッドコード／未使用リソースなし。
- [ ] export 処理中でもウィンドウ上の Result 見た目が **常に細線のまま**（export 用メッシュが別レイヤに分離されていること）。

# リスク・注意

- **depth / 複製数** が大きいとオフスクリーン **メモリとキャプチャ時間**が増える。`max_depth_for_budget` で clamp 済みでも、export バッファ `N²` が支配的になり得る。
- **同色の大量 TriangleList** のオーバードロー／タイムアウト → 必要なら `N` を 2048 に下げる、またはトーストで「処理に時間がかかります」を検討。
- Fractal 状態変更と export の **競合**（連打）→ export 中はボタン無効化またはキュー 1 本。

# 参照

- 共有 URL 現仕様コメント: `src/share.rs` 先頭。
- Bevy screenshot 例: クレート内 `examples/window/screenshot.rs`。
- モバイル配信コンテキスト: `planning/web_wasm/overall_plan.md`、`planning/mobile_ux/overall_plan.md`。
