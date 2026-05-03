# Mobile UX 改善計画

## 目的

スマホ（ナローレイアウト: 幅 700px 未満）での操作感を最適化する。
操作系を画面下部に集め、結果画面を広く取り、タッチ操作を充実させる。

## 計画概要

### Task 1: ナローレイアウト再設計

**現状**: 上部に Edit + Placement（横並び）→ 中央に Result → 下部に Parameters
**目標**: 上部に Result（大） → 中央に操作パネル（全幅）→ 下部に Edit + Placement（横並び）、Parameters は Edit/Placement 右端にタブとして overlay 表示

- [Task 1: ナローレイアウト再設計](task1_narrow_layout.md)

### Task 2: グローバル操作パネル

undo, redo, snap grid, depth, show generations ボタンを横一列に並べた全幅パネルを作成する。
edit/placement パネルの上に配置。

- [Task 2: グローバル操作パネル](task2_global_controls.md)

### Task 3: 削除ボタンの追加

- Base Shape（edit）パネルのヘッダから snap grid ボタンを削除し、`-`（行削除）ボタンを追加
- Placement パネルのヘッダに `-`（replica 削除）ボタンを追加
- ワイドレイアウトの Base Shape からも snap grid ボタンを削除

- [Task 3: 削除ボタン追加](task3_delete_buttons.md)

### Task 4: Result 画面のドラッグでパン

現状 Result はピンチズームのみ。ドラッグで平行移動（パン）できるようにする。
`view.rs` の `update_result_view` 内でドラッグ判定を追加する。

### Task 5: Edit / Placement 画面のピンチズーム

`view.rs` に Edit・Placement カメラ向けのピンチズーム処理を追加する。
`PinchState` の追跡対象カメラを増やし、各カメラのビューポートとタッチ座標を照合する。

## 受け入れ条件

- [ ] ナローレイアウトで Result が画面上部に大きく表示される
- [ ] 操作パネル（undo, redo, snap grid, depth, show gen）が Edit/Placement の上に全幅で表示される
- [ ] Base Shape ヘッダに `-` ボタンがあり、直前に描いた線を削除できる
- [ ] Placement ヘッダに `-` ボタンがあり、選択中の Replica を削除できる
- [ ] Result 画面でドラッグするとフラクタルがパンする
- [ ] Edit / Placement 画面でピンチイン/アウトでズームできる
- [ ] ワイドレイアウトに影響が出ていない（回帰なし）
