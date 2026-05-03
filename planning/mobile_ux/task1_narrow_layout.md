# Task 1: ナローレイアウト再設計

## 目的

スマホでの操作性向上のため、操作系（Edit/Placement）を画面下部へ移動し、
Result を上部に大きく配置する。

## 現状のレイアウト構造

```
[Top panel]   Edit | Placement   (上部、固定高さ)
[Central]     Result             (中央、残り領域)
[Bottom]      Parameters         (下部、折りたたみ可)
```

## 目標のレイアウト構造

```
[Top / Central]  Result                         (上部、残り領域を大きく取る)
[Bottom-mid]     Global Controls (全幅)          (Task 2 で実装)
[Bottom]         Edit | Placement (横並び)       (下部、固定高さ)
[Overlay-right]  Parameters タブ → 展開時オーバーレイ  (下半分の右端)
```

Parameters は **Edit/Placement 領域の右端に `◀` タブとして常時表示** し、
タップすると Edit/Placement 領域の上に **重なって（overlay）** 展開する。
展開しても Result / Global Controls のビューポートは変化しない。

## 実装方針（`src/ui.rs` の `layout_narrow`）

### パネル配置順

docked パネル（領域を押しのけるもの）は Parameters を含まない:

1. `TopBottomPanel::bottom("bottom_canvases")` — Edit + Placement を下部に確保
2. `TopBottomPanel::bottom("global_controls")` — グローバル操作パネルを Edit/Placement の上に確保（Task 2）
3. 残り領域 = Result（自動計算）
4. `egui::Area("params_tab")` — Parameters タブ／パネルを overlay で描画（最後に）

### Parameters の Area 実装

`egui::Area` を使い、docked パネルではなく浮かせて描画する。

**折りたたみ時（タブのみ）**:
```rust
egui::Area::new("params_tab")
    .fixed_pos(egui::pos2(win_w - 28.0, bottom_area_top))
    .show(ctx, |ui| {
        if ui.button("◀").clicked() { ui_layout.params_collapsed = false; }
    });
```

**展開時（Edit/Placement 領域の上に重なる）**:
```rust
egui::Area::new("params_tab")
    .fixed_pos(egui::pos2(win_w - 200.0, bottom_area_top))
    .show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_width(192.0);
            if ui.button("▶").clicked() { ui_layout.params_collapsed = true; }
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(bottom_area_h - 16.0)
                .show(ui, |ui| {
                    draw_params_controls(ui, state, undo_stack, placement, buttons);
                });
        });
    });
```

- `bottom_area_top` = Global Controls パネルの下端 y 座標
- `bottom_area_h` = Edit/Placement パネルの高さ（＋ Global Controls 高さ）

### Result 領域

Parameters が `Area`（overlay）になるため、Result のビューポートは Parameters の展開・折りたたみで変化しない:

```rust
let result_rect = egui::Rect::from_min_max(
    egui::pos2(0.0, 0.0),
    egui::pos2(win_w, global_controls_resp.response.rect.min.y),
);
```

### Bottom Canvases の仕様

- 高さ: `canvas_side + 52.0`（現在の top_panel_h と同じ計算式）
- Edit・Placement を横並びで同サイズ配置（現在と同じ `columns(2, ...)` 構造）
- ヘッダボタンは Task 2・Task 3 の変更を反映

## タスクの詳細手順

1. `UiLayout` のデフォルト値で `params_collapsed = true` に変更（`state.rs`）
2. `layout_narrow` を再構成:
   - Edit + Placement を `TopBottomPanel::bottom` に移動
   - Global Controls のプレースホルダーを追加（Task 2 で実装）
   - Parameters を `egui::Area` による overlay に変更
   - Result 矩形の再計算ロジックを更新

## 受け入れ条件

- [ ] ナローレイアウトで Result が画面上部に大きく表示される
- [ ] Edit + Placement が画面下部に横並びで表示される
- [ ] Parameters の `◀` タブが Edit/Placement 領域の右端に常時表示される
- [ ] タブをタップすると Parameters パネルが Edit/Placement 領域の上に重なって展開する
- [ ] Parameters の展開・折りたたみで Result のビューポートサイズが変化しない
- [ ] ワイドレイアウト（700px 以上）は変更なし
