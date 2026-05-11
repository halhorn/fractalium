# Phase 1: 表示条件と `AppScreen`（Bevy `States`）

## 目的

「プリセット選択画面」と「メイン編集画面」のどちらを描画するかを、アプリ全体で一貫したルールにする。編集キャンバス側のロジックは極力触らず、**Bevy の `States`** で **現在の画面上位モード（列挙 `AppScreen`）** を表現し、`NextState` で遷移する。egui は `Res<bevy::prelude::State<AppScreen>>`（または相当）で分岐する。

コードベースには既に [`FractalState`](../../src/app/session/fractal_state.rs)・[`DrawState`](../../src/ui/canvas/seed/mod.rs)・[`PreparedResultImageState`](../../src/app/export/mod.rs) など **「state」という語を含む型が多数**あるため、本 Phase で導入する列挙は **`AppScreen` と名前を固定**し、単独ファイル名 `state.rs` は避け、**`routing.rs` に `AppScreen` をまとめる**（下表）。

## 計画概要

| ID | 内容 |
|----|------|
| T1 | **画面上位モード**を **`AppScreen` 列挙**とし `States` として登録する。第一候補のバリアント: **`Editing`**（既存メイン編集）、**`PresetPicker`**（全面プリセット選択）。名前は実装時に確定してよいが **「フラクタル定義」「シード選択」など下位の状態と混ざらない粒度** にする（`init_state` の既定は起動要件に沿うダミー＋ Startup で決定でも可）。 |
| T2 | 「共有状態あり／ブートプリセットあり」を判定し、それ以外では起動後すぐ **`AppScreen::PresetPicker`** が有効になるよう、`hydrate_from_url` との順序矛盾がないように整理する。 |
| T3 | 「閉じる」やプリセット確定時は **`NextState`** で **`AppScreen::Editing` へ**。プリセット未適用なら `FractalState` は起動時の既定のままとする動作を定義する。 |
| T4 | **任意（推奨）**: メイン編集だけで動かすべきシステムへ **`run_if(in_state(AppScreen::Editing))`** を付ける。Phase 1 で付けられるものから順に。 |

## 計画詳細

- **「表示すべきものがない」** の実装レベル定義を固定する。
  - WASM: フラグメントからの復号が成功し `FractalState` が適用された → 起動後は **`AppScreen::Editing`**。
  - フラグメントなし／復号失敗／空 → **`AppScreen::PresetPicker`**。
  - ネイティブ: 環境変数ブートプリセットなど「初期フラクタルが明示」のとき **`AppScreen::Editing`**、それ以外 **`AppScreen::PresetPicker`**（詳細は `startup.rs` で一本化）。
- **将来の追加画面**: 同じ **`AppScreen` にバリアント追加**するか、必要なら **`SubStates`** で整理。**画面切替は `NextState<AppScreen>` に集約**する。
- **Preset ボタン**: **`AppScreen::Editing` のときのみ**ツールバーが出る前提で、`NextState(AppScreen::PresetPicker)` で遷移。

## ファイル・ディレクトリと責務

**新規ディレクトリ `src/app/mode_state/`**（画面上位モードを Bevy **`States`** で扱う。`session/` の [`FractalState`](../../src/app/session/fractal_state.rs) および egui／キャンバスの下位状態とは分離）  
**名前**: 「**mode**」でモード切替、「**state**」は Bevy の状態機構に寄せた語であり、ワークスペースの `FractalState` 型とは **`AppScreen`** という別名で区別する。

| 追加・変更 | 責務 |
|------------|------|
| `app/mode_state/mod.rs` | モジュールドキュメント、`pub mod` の集約、**`pub use routing::AppScreen`** など外部公開 |
| **`app/mode_state/routing.rs`** | **`AppScreen` 列挙**と **`States` 実装のみ**。単独ファイル名に **`state`** を付けない（`FractalState`／`DrawState` などと検索時に混ざるため。ディレクトリ名 `mode_state` は許容） |
| `app/mode_state/plugin.rs` | **`AppScreenPlugin`**（仮称固定可）:`init_state`、Startup での **`AppScreen` 初期決定**システムを登録。`hydrate_from_url` **より後**で最終状態を確定できるよう `bootstrap` と順序整合 |
| `app/mode_state/startup.rs` | 「URL 復元成功／ブートプリセットあり → **`Editing`**」「それ以外 → **`PresetPicker`**」の **起動時一本判定**（egui・描画は持たない） |
| `app/mod.rs`（変更） | **`pub mod mode_state`** の追加 |

**既存ファイルの変更（境界のみ）**

| パス | 責務 |
|------|------|
| `bootstrap/mod.rs` | `AppScreenPlugin` の追加、`init_state`、`SharePlugin` との Startup 順 |
| `app/share/sync.rs`（WASM） | `hydrate_from_url` の成否を **`app/mode_state/startup.rs` 側が読める形で渡す**（リソースフラグ／`NextState` など）。共有ペイロードのデコード自体はこのファイル側に残す |

**置かないもの**

- **`AppScreen` 以外の列挙**を同ファイルにごちゃ混ぜにしない。
- egui のプリセット格子 → **`ui/preset_picker/`**。
- **`FractalState` の定義** → [`app/session/`](../../src/app/session/mod.rs)。

## 受け入れ条件

- [ ] **`AppScreen` が `States` として登録**され、`Editing` と `PresetPicker` の間を **`NextState<AppScreen>` で遷移**できる。
- [ ] **`AppScreen::PresetPicker` のとき**、最初の安定フレームからプリセット選択 UI のみが egui に構築される。
- [ ] **`FRACTALIUM_BOOT_PRESET` が有効**なとき、起動後は **`AppScreen::Editing`** から始まる。
- [ ] WASM で共有 URL が復元できたとき、起動後は **`AppScreen::Editing`** から始まる。
- [ ] 閉じるだけでは、ユーザーがプリセットを選んでいない限り `FractalState` をプリセットビルドで上書きしない。
- [ ] コンパイル・既存テスト・`clippy` に致命的な退行がない。
- [ ] `app/mode_state/` が上表どおりであり、**公開 API は `AppScreen` を中心に** `session` との循環依存を作らない。
