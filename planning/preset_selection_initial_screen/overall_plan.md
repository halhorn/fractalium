# プリセット選択画面と起動時フロー

## 目的

起動直後や「Preset」操作で、名前付きフラクタルプリセットを視覚的に選べる画面を用意する。URL 共有やブート用プリセットなど「表示すべき状態が既にある」場合は従来どおりメインのフラクタル編集 UI から始める。メニュー一覧だけの現行 `Preset` サブメニューを、格子＋サムネイル＋閉じる導線へ置き換える（または段階的に置き換える）。

## UI 実装方針（確定）

- **画面上位モード**: 差し替えは **Bevy の `States`** で表す。列挙型名は **`AppScreen`**（例: `Editing`＝既存メイン編集、`PresetPicker`＝全面プリセット一覧）とし、[`FractalState`](../../src/app/session/fractal_state.rs)・`DrawState` 等の **ワークスペース／キャンバス用 state とは名前で区別**する。コードでは `Res<bevy::prelude::State<AppScreen>>` と書き、単に `State` と略さない運用を推奨。
- **画面差し替え**: egui のルート付近で **`AppScreen` を読み**、**プリセット選択 UI と既存のメイン編集 UI を同一フレーム内で排他的に描画する**。メイン側のパネルを裏で組み立てないため、入力が下層に抜ける問題を避けやすい。
- **見た目**: 全面の別画面として振る舞う。メインの上に `Window` や `Area` で重ねる「真のオーバーレイ」は採用しない。

## 現状の整理（参考）

- 全体プリセットは `FractalPreset` と `build()`、適用は `replace_fractal_state` と Undo・Edit / Placement リセット・Result フィット等が `global_controls_bar` の Preset メニューから実行されている。
- 起動時の `FractalState` は `bootstrap::initial_fractal_state`（環境変数 `FRACTALIUM_BOOT_PRESET` または既定の空に近い `FractalState::default()`）。
- WASM では `hydrate_from_url` がフラグメントに共有クエリがあれば状態を復元する。ネイティブでは URL ハイドレーションはない。

## 計画概要

| Phase | 内容 |
|-------|------|
| [Phase 1: 表示条件と `AppScreen`](phase1_flow_and_state.md) | `AppScreen` の登録・起動時初期値、URL・ブートプリセットとの優先、`NextState` で `Editing`／`PresetPicker` を遷移 |
| [Phase 2: 画面差し替えのプリセット選択 UI](phase2_screen_swap_ui.md) | 上部に Fractalium ロゴ領域（プレースホルダー可）、その下角丸タイルの格子（新規＋各プリセット）、閉じる、選択でメインへ遷移 |
| [Phase 3: サムネイル](phase3_thumbnails.md) | 各プリセットのサンプル画像（事前 PNG 同梱）の配置と表示 |

Phase 1〜2 ではプレースホルダー（単色・ラベルのみ等）でも受け入れ可能にする。

## モジュール構成の俯瞰（責務の線引き）

| 層 | 置き場の考え方 | Phase |
|----|----------------|-------|
| **画面上位モード**（`AppScreen`） | `app/mode_state/` — **`FractalState` や `DrawState` とは別**に、「いまメイン編集か／全面プリセットか」など **排他画面**だけを表す | [Phase 1](phase1_flow_and_state.md) |
| **プリセット選択の見た目** | `ui/preset_picker/` — egui の全面画面。ドメインの載せ替えは呼び出し元が `session_rules` に委譲 | [Phase 2](phase2_screen_swap_ui.md)／[Phase 3](phase3_thumbnails.md) |
| **フラクタル状態の適用** | `app/session_rules.rs` — `replace_fractal_state` 周辺の **一括適用の単一入口** に寄せ、UI と共有 | [Phase 2](phase2_screen_swap_ui.md) |
| **起動順・プラグイン列** | `bootstrap/mod.rs` — `init_state`、プラグイン順（`hydrate_from_url` との前後）は Phase 1 | [Phase 1](phase1_flow_and_state.md) |
| **共有 URL と初回 `AppScreen`** | `app/share/sync.rs`（WASM）— 復元成否を **`app/mode_state`** の起動解決ロジックが読める形で渡す **境界**のみ。どの `AppScreen` で起動するかの解釈は `app/mode_state/` に集約 | [Phase 1](phase1_flow_and_state.md) |

`ui/frame/` は従来どおり **`AppScreen::Editing`（メイン編集）専用**の枠・パラメータ・ツールバーに留め、全面プリセットは **`ui/preset_picker/` に分離**して責務を混ぜない。

## 受け入れ条件（全体）

- [ ] **起動**: 共有 URL 等で復元する状態がない起動では、最初にプリセット選択画面が表示される（ネイティブ・WASM の両方で意図どおり）。
- [ ] **起動（例外）**: 復元可能な URL フラグメントがある（WASM）、または `FRACTALIUM_BOOT_PRESET` 等で初期状態が明示されている場合は、プリセット画面をスキップしてメイン UI から始まる。
- [ ] **ブランド**: プリセット選択画面上部に Fractalium の識別領域がある（Phase 2 ではプレースホルダーで可）。
- [ ] **格子 UI**: 上部ブランド領域の下に、角丸タイルで各プリセットとサンプル画像（および名前）が並ぶ（Phase 3 未完了時はプレースホルダーでよい）。
- [ ] **新規**: 格子の一项が「空／新規作成」であり、プリセット画面を開いた直後はその項が選択状態（フォーカスやハイライト）になっている。
- [ ] **確定**: 新規を含むいずれかを選ぶとメインのフラクタル編集レイアウトだけが操作対象になる（`NextState(AppScreen::Editing)` 等）。
- [ ] **Preset ボタン**: グローバルバーの Preset から同じプリセット選択画面に切り替わる（メニュー項目の羅列だけに留めない）。
- [ ] **閉じる**: 閉じる用のコントロールでメイン画面に戻る。起動直後に何も選ばず閉じた場合は、空に近い既定状態のままメインが見える（または意図した「空」定義に一致する）。
- [ ] **差し替え**: **`AppScreen::PresetPicker`** のあいだは、`Editing` 用の egui を構築しない（`AppScreen` に応じたルート分岐で排他）。
- [ ] 既存のプリセット適用時の Undo・DrawState・Placement・Result フィットなどの横断方針を破壊しない（既存計画・コードの挙動と整合）。

## 判断（確定）

- **サムネイル**: 事前に PNG を書き出してリポジトリに同梱する（`phase3_thumbnails.md` でパス・解像度・フォールバックを詰める）。
- **画面上位モード**: 列挙 **`AppScreen`** と Bevy の `States` / `NextState` で管理する（詳細は [phase1](phase1_flow_and_state.md)）。モジュールは **`mode_state`**（`app/mode_state/`）— **ディレクトリ名は mode（＋Bevy の state 機能）**。列挙は **`AppScreen`** のままとし単体ファイル **`state.rs` は避け、`routing.rs` に **`AppScreen` を置く**。
