---
# Phase 3: レプリカ操作インタラクション

## 目的

Placement パネルでレプリカをマウス操作（クリック選択・ドラッグ）で
移動・スケール・回転できるようにする。

## 状態設計

新リソース `PlacementState` を `state.rs` に追加：

```rust
pub struct PlacementState {
    pub selected: Option<usize>,   // 選択中レプリカのインデックス
    pub drag: PlacementDrag,
}

pub enum PlacementDrag {
    Idle,
    Move   { start_cursor: Vec2, start_translation: Vec2 },
    Scale  { start_cursor: Vec2, start_scale: f32, handle: ScaleHandle },
    Rotate { start_cursor: Vec2, start_rotation: f32, pivot: Vec2 },
}
```

## バウンディングボックスの計算

レプリカ `i` のバウンディングボックス：
- 基図形全線分に `replica.apply()` を適用した点の AABB（軸並行矩形）
- マージンを少量（0.05 程度）追加して選択しやすくする
- 基図形が空なら原点付近の小さなデフォルト矩形を使う

## ドラッグゾーンの判定

カーソル位置とバウンディングボックスの関係で操作を決定：

| カーソル位置 | 操作 |
|-------------|------|
| ボックス内（エッジ外）| 移動（Move） |
| ボックスのエッジ付近（幅 0.05）| スケール（Scale）|
| ボックス外 | 回転（Rotate） |

## Tasks

### Task 1: PlacementState リソースの定義

`state.rs` に `PlacementState` と `PlacementDrag` を追加。
`main.rs` で `insert_resource(PlacementState::default())` を呼ぶ。

### Task 2: 選択ヒットテスト

マウス左ボタン押下時、カーソル座標（Placement カメラで変換）に対して
各レプリカの AABB を調べ、最初にヒットしたものを `selected` にセット。
ボックス外クリックは選択解除。

### Task 3: ドラッグ処理

`handle_drag_input` に相当する system を placement.rs に実装：
- `MouseButton::Left` 押下 → `PlacementDrag` を設定
- `Update` 中（ドラッグ中）→ カーソル差分から replica の値を更新
- `MouseButton::Left` 離し → `PlacementDrag::Idle` に戻す

各操作の計算：
- **Move**: `replica.translation = start_translation + (cursor - start_cursor)`
- **Scale**: `replica.scale = start_scale * (cursor_dist / start_dist)`（pivot: AABB 中心）
- **Rotate**: `replica.rotation = start_rotation + atan2(cursor - pivot) - atan2(start_cursor - pivot)`

### Task 3.5: Ctrl スナップ

`edit.rs` の `Modifiers` と同じく Ctrl キーを判定し、ドラッグ確定値にスナップをかける。
`grid.rs` の `snap_to_grid` を流用できるものは流用する。

| 操作 | Ctrl なし | Ctrl あり |
|------|-----------|-----------|
| Move | 自由移動 | `snap_to_grid(translation)` でグリッドに吸着 |
| Scale | 自由スケール | 0.05 刻みに丸める（`(scale / 0.05).round() * 0.05`） |
| Rotate | 自由回転 | 15° 刻みに丸める（`(deg / 15.0).round() * 15.0`） |

スナップ値はプレビュー中（ドラッグ中）にも適用し、即時フィードバックを与える。

### Task 4: バウンディングボックスの描画

選択中レプリカの AABB を `PlacementGizmos` で矩形描画。
エッジ（スケールハンドル）を小さな四角で描画。

## 受け入れ条件

- [ ] レプリカをクリックすると枠線が表示される
- [ ] 非選択箇所クリックで選択解除される
- [ ] 枠内ドラッグで translation が変化し、描画に即反映される
- [ ] 枠エッジドラッグで scale が変化する
- [ ] 枠外ドラッグで rotation が変化する
- [ ] ドラッグ中も Result パネルのフラクタルがリアルタイムに更新される
- [ ] Undo で操作が戻る（Task 完了時に undo_stack に push）
- [ ] Ctrl 押下中、Move がグリッドに吸着する
- [ ] Ctrl 押下中、Scale が 0.05 刻みにスナップされる
- [ ] Ctrl 押下中、Rotate が 15° 刻みにスナップされる
