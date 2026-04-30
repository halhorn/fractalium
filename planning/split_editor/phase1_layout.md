---
# Phase 1: レイアウト分割

## 目的

PlacementCamera を追加し、`ui.rs` のビューポート計算を変更して
Edit を左上・Placement を左下・Result を右に配置する。

## Tasks

### Task 1: PlacementCamera の追加（main.rs）

- `PlacementCamera` マーカーコンポーネントを定義
- `placement_layer()` 関数を追加（RenderLayers::layer(3)）
- `spawn_world_cameras` に PlacementCamera を追加（order: 2）
- `spawn_canvas_decor` を Placement レイヤにも呼ぶ

### Task 2: ビューポート計算の変更（ui.rs）

`compute_canvas_viewports` を変更する：

現在：左右 2 分割（Edit 左, Result 右）
変更後：左を上下 2 分割（Edit 上, Placement 下）、Result は右

```
canvas_w = win_w - panel_phys
left_w   = canvas_w / 2
right_w  = canvas_w - left_w
left_h_top = win_h / 2
left_h_bot = win_h - left_h_top

Edit      : position=(0,0),         size=(left_w, left_h_top)
Placement : position=(0,left_h_top), size=(left_w, left_h_bot)
Result    : position=(left_w,0),    size=(right_w, win_h)
```

- `params_panel` / `apply_canvas_viewports` の引数に `placement_cam` を追加
- `PlacementCamera` を Query に加える

### Task 3: placement モジュールの雛形（src/placement.rs）

- `PlacementPlugin` を定義（`Update` に空の system を登録するだけ）
- `main.rs` に `mod placement` と `PlacementPlugin` を追加

## 受け入れ条件

- [ ] ビルドが通る
- [ ] 起動時に左上に Edit、左下に Placement（まだ何も描画されない）、右に Result が表示される
- [ ] 既存の Edit 機能（線描画）が正常動作する
- [ ] Result のフラクタル表示が正常動作する
