# 全体フラクタルプリセット（Shape / Placement / Depth）

## 目的

基図形・複製（Placement に相当する `replicas`）・再帰の深さをひとまとめにした**状態プリセット**を用意し、文献で知られる代表的なフラクタルをワン選択で再現できるようにする。既存の「Seed の Shape」は**線分の追加**に留まるのに対し、本機能は**フラクタル全体を置き換える**（または同効な初期化を行う）導線とする。

## 計画概要

| Phase | 内容 |
|-------|------|
| [Phase 1: UI と適用の共通枠](phase1_ui.md) | グローバル操作バーに「Preset」ボタンを追加し、`menu_button` ＋一覧から選択すると `FractalState`（および必要な関連リソース）が一括更新される。Undo・ワイド／ナロー共通。 |
| [Phase 2: プリセット目録と有名フラクタル 8 種](phase2_catalog.md) | 各プリセットが設定する `base_shape`・`replicas`・`depth`（と必要なら `show_all_generations`）を定義する。学術・一般向けで名前の通じるものを優先する。 |

**実装スタックの前提**: UI は `bevy_egui`。Seed の「Shape」と同様に **egui の `menu_button` ＋ メニュー項目**で「押下 → 一覧 → 選択」を実現する。

## 計画詳細（横断）

- **データモデル**: 既存の `FractalState`（`base_shape` / `replicas` / `depth` / `show_all_generations` / `snap_grid`）で表現できる範囲に収める。相似変換のリストのみが使えるため、バーンスレイのような**非相似アフィン**は本アプリの式では厳密には再現できない——近似・縮小スコープは Phase 2 で明示する。
- **適用範囲**: プリセット選択時は少なくとも `base_shape`、`replicas`、`depth` をセットする。`snap_grid` は**変更しない**（ユーザーの作業モードを維持）でよい unless Phase 2 で別方針に確定する。`show_all_generations` はプリセットごとに推奨値を持ってもよい。
- **Undo**: 適用直前に現状の `FractalState` を `UndoStack::push` する（Seed の Shape と同パターン）。
- **Edit 側**: `base_shape` が入れ替わるため、`DrawState` は範囲外・不整合を避けるため **Idle 等へリセット**する（Clear 時と同様の方針）。
- **Placement 側**: `PlacementState` の選択・ドラッグは**リセット**し、無効インデックスを残さない。
- **正規化座標**: 既存の `[-1, 1]²` に合わせ、基図形とレプリカの翻訳・スケールが極端に画面外に出ないよう、Phase 2 で各プリセットのスケールを揃える。

## 受け入れ条件

### Phase 1

- [ ] グローバル操作バー（Undo / Snap と同一の `global_controls_bar`）に「Preset」導線がある（ワイド・ナロー同一コードパス）。
- [ ] 「Preset」からサブメニューが開き、Phase 2 で定義した各項目を選べる（実装途中はプレースホルダー 1 件でも可だが、完了時はカタログ分すべて）。
- [ ] 選択後、`base_shape`・`replicas`・`depth` がプリセット定義どおりになり、Result に反映される。
- [ ] 適用前に `UndoStack` に積まれ、適用後に Edit / Placement の一時状態が破綻しない（上記のリセット方針を満たす）。
- [ ] コンパイル・`clippy`・致命的なリグレッションがない。

### Phase 2

- [ ] [Phase 2 カタログ](phase2_catalog.md) に沿い、**有名フラクタル 8 種**がメニューから選べる。
- [ ] 各項目に**通称・メニューラベル**が付き、意図したパターンが深さを変えても破綻しにくい（必要なら推奨 `depth` 範囲をドキュメントとコメントで明記）。
