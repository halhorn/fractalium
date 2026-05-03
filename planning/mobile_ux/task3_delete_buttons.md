# Task 3: 削除ボタンの追加

## 目的

Base Shape と Placement の各パネルヘッダに `-` ボタンを置き、タッチ操作で削除できるようにする。
併せて snap grid ボタンを Base Shape ヘッダから除去する（ワイド・ナロー両方）。

## 変更内容

### Base Shape パネル（Edit）

- **削除**: `snap_toggle_buttons()` の呼び出しを `show_canvas_block` の header_buttons から除去
  - ワイドレイアウト（`layout_wide`）と ナローレイアウト（`layout_narrow`）の両方
  - snap grid 操作は Task 2 のグローバル操作パネルに移動済みのため重複しない
- **追加**: `-` ボタン（`small_button("-")`）
  - クリック時: `undo_stack.push(state.clone())` → `state.base_shape.lines.pop()`
  - 線がない場合は無効化（`ui.add_enabled(has_lines, egui::Button::new("-").small())`）

### Placement パネル

- **追加**: `-` ボタン（`small_button("-")`）
  - クリック時: 選択中の Replica を削除
  - `placement.selected` が `Some(i)` なら `state.replicas.remove(i)`
  - 選択中 Replica がない場合は無効化

## 実装対象ファイル

- `src/ui.rs`:
  - `layout_wide` の `show_canvas_block("Base Shape", ...)` の header_buttons を変更
  - `layout_narrow` の `show_canvas_block("Base Shape", ...)` の header_buttons を変更
  - `layout_wide` の `show_canvas_block("Placement", ...)` に `-` ボタン追加
  - `layout_narrow` の `show_canvas_block("Placement", ...)` に `-` ボタン追加

## 注意点

- `snap_toggle_buttons()` 関数自体はグローバル操作パネル（Task 2）で流用するため削除しない
- Placement の `-` ボタンは右クリック削除と同等の操作（`placement.rs` 参照）

## 受け入れ条件

- [ ] Base Shape ヘッダから snap grid ボタンが消えている（ワイド・ナロー両方）
- [ ] Base Shape ヘッダの `-` ボタンで最後に描いた線を削除できる
- [ ] Base Shape ヘッダの `-` ボタンは線がないとき無効化される
- [ ] Placement ヘッダの `-` ボタンで選択中 Replica を削除できる
- [ ] Placement ヘッダの `-` ボタンは Replica 未選択時に無効化される
