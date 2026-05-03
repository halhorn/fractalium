---
name: Phase 1 - WASM ビルド成立
type: project
---

# 目的

`cargo build --target wasm32-unknown-unknown` を成功させ、`trunk serve` で開いたローカルブラウザ上で現行のデスクトップ版と同等の機能（線描画・複製追加・depth 変更）が動作する状態をつくる。

# 計画概要

| Task | 概要 |
|------|------|
| T1.1 | Rust ツールチェーン整備（`wasm32-unknown-unknown` ターゲット追加・`trunk` インストール） |
| T1.2 | `Cargo.toml` の Bevy feature を WASM 互換に整理（`webgl2` 有効化） |
| T1.3 | `WindowPlugin` をブラウザ向けに調整（`canvas` セレクタ・`fit_canvas_to_parent`・`prevent_default_event_handling`） |
| T1.4 | `index.html` と `Trunk.toml` を追加し、`trunk serve` でローカル起動できるようにする |
| T1.5 | ブラウザ上で主要機能を一通り動かして回帰がないか確認する |

# 計画詳細

## T1.1: ツールチェーン

- `rustup target add wasm32-unknown-unknown`
- `cargo install --locked trunk`
- 任意で `cargo install wasm-bindgen-cli`（trunk が内部で利用、バージョン整合に注意）
- `.gitignore` に `dist/` を追加

## T1.2: Cargo.toml の feature 整理

- Bevy 0.18 の `default_platform` には `webgl2` が含まれるため、ネイティブ／WASM 両方で使える設定を確認する
- `bevy_egui` 0.39 はデフォルトで WASM 対応のため追加 feature 指定不要
- 必要に応じて Bevy のサブクレートを features で間引き、バンドルサイズの初期値を確認する（ただし大幅な削減は Phase 2 の T2.5 で扱う）

## T1.3: Window 設定

`src/main.rs` の `WindowPlugin` に以下のフィールドを設定する。値はネイティブビルドでも安全（非 web では no-op）。

- `canvas: Some("#fractalium-canvas".into())`
- `fit_canvas_to_parent: true`
- `prevent_default_event_handling: true`（ブラウザの戻る/タブ切替/F5 等の誤発火を抑制）

ネイティブ動作に副作用がないか `cargo run` で確認する。

## T1.4: trunk セットアップ

ファイル追加：

- `index.html`: `<canvas id="fractalium-canvas">` を含む最小の HTML。`<link data-trunk rel="rust" />` で Cargo.toml を参照
- `Trunk.toml`: `[build]` に `dist = "dist"`、`[serve]` に `address = "127.0.0.1"`、`port = 8080`
- `assets/` を `index.html` から `data-trunk rel="copy-dir"` で `dist/assets/` にコピー（現状は `assets/` がなければ不要）

## T1.5: 動作確認シナリオ

- Edit キャンバスでドラッグして線を引く（複数本）
- Ctrl ドラッグでグリッドスナップが効く
- Parameters パネルで `+ Add replica` → TX/TY/Rot/Scale を編集
- Result の depth を 1〜8 で変えてフラクタルが追従する
- Cmd+Z で undo（macOS Chrome／Safari）
- ブラウザコンソールに JS / WASM エラーが出ない

# 受け入れ条件

- [ ] `rustup target list --installed` に `wasm32-unknown-unknown` が含まれる
- [ ] `cargo build --target wasm32-unknown-unknown --release` が警告のみ（エラーなし）で成功する
- [ ] `cargo run`（ネイティブ）が引き続き正常に起動・動作する（リグレッションなし）
- [ ] `trunk serve` が `http://127.0.0.1:8080` で起動し、ブラウザで Fractalium 画面が表示される
- [ ] T1.5 の動作確認シナリオが Chrome / Safari の両方で通る
- [ ] ブラウザの DevTools Console にエラーが出力されない
- [ ] Cmd+Z（Mac）/ Ctrl+Z（Win）が undo として機能し、ブラウザの戻る等を誘発しない
