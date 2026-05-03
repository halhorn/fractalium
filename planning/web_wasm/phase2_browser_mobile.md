---
name: Phase 2 - ブラウザ／モバイル最適化
type: project
---

# 目的

デスクトップとモバイル（iOS Safari / Android Chrome）の両方で、Fractalium が快適に操作できる状態にする。タッチ入力・レスポンシブレイアウト・パフォーマンス・配信サイズの 4 つを同時に整える。

# 計画概要

Phase 1 でブラウザ起動が成立した後に着手する。タッチ入力対応はインタラクション層の追加が必要で、本 Phase の中心タスク。

| Task | 概要 |
|------|------|
| T2.1 | レスポンシブレイアウト（ウィンドウサイズ・縦長画面に追従、モバイルでも 3 領域がアクセス可能） |
| T2.2 | タッチ入力対応（1 本指で線描画、2 本指でピンチズーム、タップで複製選択） |
| T2.3 | 修飾キー UI 代替（Ctrl/Shift スナップを画面上のトグルボタンで操作可能にする） |
| T2.4 | ブラウザのデフォルト動作抑止（ピンチズーム・長押しメニュー・スクロールバウンス） |
| T2.5 | バンドルサイズ削減（`wasm-release` プロファイル + `wasm-opt` の組み込み） |
| T2.6 | パフォーマンス検証（depth 大／複製多／モバイル端末での実機計測） |

# 計画詳細

## T2.1: レスポンシブレイアウト

現行は `1280x720` 固定の bevy_egui レイアウト。これをウィンドウサイズに追従させ、縦長画面（モバイル）でも全機能にアクセスできるようにする。

- 横幅の閾値（例: 900px 未満）でレイアウトを 2 段組みに切り替える方式を検討
  - デスクトップ: 横並び（Edit | Placement | Result | Parameters）
  - モバイル: タブ切替もしくは縦スクロール
- bevy_egui の `SidePanel` / `TopBottomPanel` の組み合わせで実装可能
- 既存の `state::CanvasLayout` / `UiLayout` の責務を見直し、画面サイズ依存の値はリソースから注入する

## T2.2: タッチ入力対応

Bevy 0.18 の `Touches` リソースおよび `TouchInput` イベントを利用する。マウス入力と排他ではなく **両方サポート**する（同じハンドラから抽象化された Pointer イベントとして扱う）。

実装観点：
- 1 本指ドラッグ → Edit キャンバス上での線描画（マウス左ドラッグと等価）
- 1 本指ドラッグ → Placement キャンバスでの複製選択／移動
- 2 本指ピンチ → Result キャンバスのズーム（既存ホイールズームと等価）
- 2 本指ドラッグ → Result のパン（任意。優先度低）
- ロングタップ／ダブルタップ → 必要に応じて検討（最初は採用しない）

`src/edit.rs` `src/placement.rs` `src/view.rs` のマウス入力処理に Touch 入力ブランチを追加する。

## T2.3: 修飾キー UI 代替

モバイルには Ctrl / Shift がないため、既存のスナップ修飾を画面上の **トグルボタン**で代替する。

- グリッドスナップ（Ctrl 相当）: トグルボタン
- 角度スナップ（Shift 相当）: トグルボタン
- デスクトップでは修飾キーとトグルが OR で動作する（既存ユーザーへの影響なし）
- 実装は `state::FractalState` あたりに `snap_grid: bool` `snap_angle: bool` を追加し、入力検知側で `key OR resource` を見る

## T2.4: ブラウザデフォルト動作抑止

`index.html` の `<meta>` および canvas 要素に対して以下を設定する。

- `<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">`
- `touch-action: none;` を canvas に CSS で指定（ピンチズーム・スクロールを Bevy 側で受け取る）
- iOS Safari の overscroll / バウンススクロール抑止（`overscroll-behavior: none`）

`Window::prevent_default_event_handling = true` だけでは不十分なケースがあるため、HTML 側でも明示的に抑止する。

## T2.5: バンドルサイズ削減

`Cargo.toml` に `wasm-release` プロファイルを追加：

- `inherits = "release"`
- `opt-level = "s"`（または `"z"`）
- `lto = true`
- `codegen-units = 1`
- `strip = true`

ビルドコマンドは `trunk build --release`（trunk が `wasm-opt` を自動適用）。サイズ目標は **`.wasm` < 15 MB**。閾値超過時は Bevy の不要 feature を間引く。

## T2.6: パフォーマンス検証

検証マトリクス：

| 環境 | 確認項目 |
|------|----------|
| デスクトップ Chrome | depth 8 / 複製 4 で 60 FPS、depth 10 / 複製 4 で 30 FPS 以上 |
| iOS Safari（実機） | depth 6 / 複製 3 で 30 FPS 以上、初回ロード 5 秒以内 |
| Android Chrome（実機） | iOS と同等 |

不足する場合は `state::FractalState::depth` のモバイル時上限を引き下げる、複製数に応じた warning 表示を追加する等で UX を担保する。

# 受け入れ条件

## レイアウト

- [ ] デスクトップでウィンドウをリサイズしても 3 キャンバス + パネルが破綻しない
- [ ] iOS Safari（縦持ち）で全機能（線描画・複製編集・depth 変更）にアクセス可能

## タッチ入力

- [ ] iOS Safari（実機）で 1 本指ドラッグして線が引ける
- [ ] iOS Safari でグリッドスナップ／角度スナップのトグルが効く
- [ ] iOS Safari で Result キャンバスをピンチでズームできる
- [ ] Android Chrome（実機）でも上記すべてが動作する
- [ ] デスクトップでマウス操作の挙動に回帰がない

## ブラウザ挙動

- [ ] モバイルでピンチしてもブラウザのページズームが発火しない
- [ ] iOS Safari でロングタップしても画像保存メニューが出ない
- [ ] スクロールバウンス（overscroll）が発生しない

## パフォーマンス／サイズ

- [ ] `trunk build --release` 後の `.wasm` が 15 MB 以下
- [ ] デスクトップ Chrome で depth 8 / 複製 4 が 30 FPS 以上
- [ ] モバイル実機で初回ロード 5 秒以内、depth 6 / 複製 3 が 30 FPS 以上
