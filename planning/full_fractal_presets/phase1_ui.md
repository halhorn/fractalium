# Phase 1: Preset 導線と一括適用

## 目的

`global_controls_bar` に「Preset」を置き、一覧から選ぶとフラクタル全体のパラメータが一括で切り替わる。Seed の「Shape」と同様に egui のメニューで操作感を揃える。

## 設計方針

- **UI ウィジェット**: `menu_button("Preset", |ui| { ... })` とメニュー項目。メニュー幅は項目名が読める程度（Shape 側の `set_min_width` と同種の配慮）。
- **配置**: Snap の後（または Undo グループと Share の間の左側グループ内）に置き、狭幅の折り返しでも利用可能な位置にする。
- **適用処理**: プリセット ID に対応する**ビルダー**が `FractalState` のスナップショットを返すか、受け取った `FractalState` を上書きする。ロジックは UI から分離し、Shape 用の `presets` モジュールに隣接させるか、サブモジュールで「全体プリセット」責務を分ける。
- **関連リソース**: `DrawState`・`PlacementState` のクリア方針は `overall_plan.md` の横断セクションに従う。`params_panel` から `global_controls_bar` を呼ぶ経路に `DrawState` / `PlacementState` の `ResMut` が渡せるよう、シグネチャ整理が必要なら Phase 1 の範囲で行う。

## 計画概要

| ID | 内容 |
|----|------|
| T1 | `global_controls_bar` に Preset を追加し、`params_panel` から必要なリソースを渡す。 |
| T2 | プリセット適用関数（Undo → 状態代入 → 補助リソースリセット）を一箇所にまとめる。 |
| T3 | ワイド／ナローで同じ操作ができることを確認する。 |

## 受け入れ条件

- [ ] `overall_plan.md` の Phase 1 チェックリストを満たす。
