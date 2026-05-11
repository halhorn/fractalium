# Phase 2: 画面差し替えのプリセット選択 UI

## 目的

**`AppScreen::PresetPicker`** のときだけ egui 上に全面 UI を描画する。**上部は Fractalium ロゴ（実装初期はプレースホルダー）、その下を角丸タイルの格子**（新規作成を含む各プリセット：サムネイル＋名前）と「閉じる」で構成する。メイン編集 UI は **`AppScreen::Editing` のときのみ** 構築する。`global_controls_bar` の `Preset` はメニューではなく **`NextState(AppScreen::PresetPicker)`** の導線に変更する。カード選択後は既存のプリセット適用パスを実行し、**`NextState(AppScreen::Editing)`** で戻す。

## 計画概要

| ID | 内容 |
|----|------|
| T1 | メイン UI のルート（`UiPlugin` / `params_panel`）で **`Res<bevy::prelude::State<AppScreen>>` を読み**、`PresetPicker` のときだけ **全面プリセット UI** を描画する。`Editing` のときだけ既存の `layout_wide` / `narrow` を実行する。 |
| T2 | **ヘッダ**: 画面上部に Fractalium ブランド領域を置く。ロゴ画像は Phase 2 ではプレースホルダー（テキスト・簡易図形・後から差し替え可能なスロット）とし、正規ロゴアセットは後続で差し替え可能にする。 |
| T3 | **タイル格子**: ヘッダの下に、新規作成を含む各項目を **角丸のカードタイル** として並べる。列数はウィンドウ幅に応じて折り返し、タイル内はサムネイル領域（上または上段）とラベル（下）の縦並びを基本とする。 |
| T4 | 先頭タイルを「新規／空」、続けて `FractalPreset::ALL` の順。タイル同士・画面端に一貫した余白と最小タッチ領域を確保する。 |
| T5 | プリセット画面表示直後、キーボードフォーカスまたは視覚選択を「新規」タイルに置く。 |
| T6 | タイルクリックで `FractalState` を更新したあと **`NextState(AppScreen::Editing)`**。 |
| T7 | 「閉じる」は **`NextState(AppScreen::Editing)`** のみ（`FractalState` は Phase 1 の方針どおり変更しない場合あり）。ヘッダ近傍または画面上部隅などに配置。 |
| T8 | ワイド／ナローで同じ画面構成が破綻しないようにする。項目数が増えてもスクロール可能なコンテンツ領域とし、ヘッダは視認しやすく固定または短く追随するなど方針を一つ決める。 |

## レイアウト・デザイン方針

- **ブランドヘッダ**: 上から「Fractalium」識別（ロゴまたはプレースホルダー）→区切り（余白または細いディバイダ）→格子。ナロー幅ではロゴをやや縮小しても読みやすさを優先する。中央寄せまたは左右余白固定的な **最大幅** などで、リサイズ時もヘッダが破綻しにくくする。
- **タイル見た目**: 角丸矩形のフレーム。背景はメインのアプリ調子（ダークベース）と諧調するが、選択・ホバー（またはフォーカス）で区別できるコントラストを付ける。タイル直下の名前は中央寄せまたは左寄せで一貫させる。
- **プレースホルダー**: ロゴは画像が無い間は文言「Fractalium」または仮のアイコン枠でも可。タイル内サムネイルは Phase 3 まで灰色プレースホルダー等でサイズだけ揃える。

## 計画詳細

- **適用処理の共通化**: 現状 `global_controls_bar` 内に閉じたループがあるため、プリセット ID（および新規）を引数に取り、同一の副作用を実行する関数へ寄せるか、プリセット選択 UI からその関数を呼ぶ。シグネチャ・責務は実装時に決定（コードは本計画に書かない）。
- **新規選択時の Undo**: 既に空の既定から変わっていない場合に Undo を積むかどうかは、既存の「Clear」や Shape プリセットのパターンに合わせて Phase 2 の実装時に決める。
- **メインから Preset**: **`AppScreen::Editing` のフレームでのみ** `global_controls_bar` が描画されるため、Preset ボタンは **`NextState(AppScreen::PresetPicker)`**。

## ファイル・ディレクトリと責務

**新規ディレクトリ `src/ui/preset_picker/`**（全面プリセット画面のみ。`ui/frame/` のメインレイアウトと混在させない）

| 追加 | 責務 |
|------|------|
| `ui/preset_picker/mod.rs` | サブモジュール集約。外部から呼ぶ **唯一の入口**（例: `paint_preset_picker_screen` のような 1 関数）を宣言 |
| `ui/preset_picker/screen.rs` | **1 画面の組み立て**: 閉じる・スクロール領域・ヘッダ＋格子の縦並び。`CentralPanel` 等の粒度は実装時 |
| `ui/preset_picker/header.rs` | ブランド列（プレースホルダー／将来ロゴ）。**タイルとは独立**させ、差し替えしやすくする |
| `ui/preset_picker/tiles.rs` | 角丸タイル格子、フォーカス・「新規」先頭順。Phase 3 までサムネ枠はプレースホルダーのみ |
| `ui/mod.rs`（変更） | `params_panel` 先頭で **`AppScreen` に match**。「`PresetPicker` → `preset_picker` のみ」「`Editing` → 既存 `layout_wide` / `narrow`」の **排他分岐**。Result カメラ `viewport_bridge` は **`PresetPicker` 時も安全な viewport** にするか別途明示 |

**既存ファイルの変更**

| パス | 責務 |
|------|------|
| `ui/frame/global_bar.rs` | `Preset` のメニューをやめ、`NextState(AppScreen::PresetPicker)`。**`AppScreen`** は `crate::app::mode_state::AppScreen` のように明示 import。適用は **`session_rules` の単一関数**のみ |
| `app/session_rules.rs`（または隣接の小さい `apply` モジュール） | メニュー／タイル共通の **`apply_whole_workspace_preset`**（仮称）:**Undo push → replace_fractal_state → DrawState / Placement / pending_result_fit`** を一括。引数は「新規」かどうかまたは `Option<FractalPreset>` 程度に留める |

**データアセット（Phase 2）**

- ロゴ PNG は **必須にしない**（プレースホルダーで完了）。Phase 3 のサムネイル用 `assets/` は Phase 3 で追加。

**置かないもの**

- **`AppScreen` 列挙** → **`app/mode_state/routing.rs`**（単体ファイル **`state.rs` は使わない）。
- **起動時の `AppScreen` 決定** → **`app/mode_state/startup.rs`**。

## 受け入れ条件

- [ ] **`AppScreen::PresetPicker`** のとき、**画面上部に Fractalium のブランド領域**（ロゴまたはプレースホルダー）と**角丸タイル格子**があり、**同じフレームで `Editing` 用パラメータパネル等が egui に存在しない**。
- [ ] タイルがナロー／ワイドの両方で破綻せず、必要なら格子部分のみスクロールできる。
- [ ] 開いた直後、「新規」タイルが選択／フォーカス表示される。
- [ ] プリセットまたは「新規」確定後 **`AppScreen::Editing`** になり、Result / 編集 UI が従来どおり使える（`FractalState` は仕様どおり）。
- [ ] 「閉じる」で **`AppScreen::Editing`** に戻り、Phase 1 の `FractalState` 方針を満たす。
- [ ] Preset から **`NextState(AppScreen::PresetPicker)`** で遷移（メニューのテキスト一覧だけにしない）。
- [ ] プリセット適用後の Undo・編集状態の安定性が既存と同等であることが手動確認または既存テストで担保される。
- [ ] `ui/preset_picker/` が上表どおり分離され、`global_bar` のプリセット適用ロジックが `session_rules`（仮称）に集約されている。
