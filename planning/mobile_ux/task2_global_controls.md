# Task 2: グローバル操作パネル

## 目的

undo, redo, snap grid, depth, show generations を一列に並べた全幅の操作バーを
ナローレイアウトの Edit/Placement パネルの直上に配置する。

## パネル仕様

- 配置: `TopBottomPanel::bottom("global_controls")`（Edit/Placement の上、Task 1 の bottom より先に push）
- 高さ: 固定（例: 40px）
- 幅: 画面全幅（SidePanel::right の parameters が確保した後の残り幅）
- レイアウト: 横一列 `ui.horizontal(...)`

## ボタン仕様

| ボタン | 動作 | 備考 |
|--------|------|------|
| `↩ Undo` | `undo_stack.undo(state)` | スタック空なら無効化 |
| `↪ Redo` | `undo_stack.redo(state)` | Redo 機能の実装が必要（後述）|
| `Snap [on/off]` | `state.snap_grid` トグル | 現在の `snap_toggle_buttons` と同等、アクティブ時に色変更 |
| Depth `-` / 数値 / `+` | `state.depth` 増減 (1〜12) | スライダーではなくボタン式（タッチしやすい） |
| `Gen [on/off]` | `state.show_all_generations` トグル | アクティブ時に色変更 |

## Redo 機能の追加

現状 `UndoStack` は Undo のみ対応。Redo を実装するには `state.rs` を変更する:

- `UndoStack` に `redo_stack: Vec<FractalState>` フィールドを追加
- `push()` 時に redo_stack をクリア
- `undo()` 時に現在 state を redo_stack に push
- `redo()` メソッドを追加

## 実装対象ファイル

- `src/state.rs`: `UndoStack` への redo_stack 追加
- `src/ui.rs`: `global_controls_panel()` ヘルパー関数を追加、`layout_narrow` から呼ぶ

## 受け入れ条件

- [ ] グローバル操作パネルが Edit/Placement パネルのすぐ上に全幅で表示される
- [ ] Undo ボタンで操作を一つ戻せる
- [ ] Redo ボタンで戻した操作を再実行できる
- [ ] Snap grid ボタンがオン/オフで色が変わる
- [ ] Depth の `-` / `+` ボタンで深さを変更できる（1〜12 範囲でクランプ）
- [ ] Show generations ボタンで世代表示を切り替えられる
