# Phase 3: Base Shape の線編集

## 目的

Base Shape パネル上で「すでに引いた線」を後から編集できるようにする。

- 線をクリックして選択（色変化＋両端マーカー表示）
- 選択線を本体ドラッグで平行移動
- 両端の◯マーカーをドラッグして始点・終点をそれぞれ移動
- 上記の移動操作にも Ctrl（グリッドスナップ）／Shift（45° スナップ）を適用

## 想定インタラクション

| カーソル位置（左クリック押下時） | 動作 |
|--------------------------------|------|
| 線分本体に近接 | その線を選択。引き続きドラッグで本体を平行移動 |
| 選択中線分の端点◯マーカー上 | その端点をドラッグ移動（始点 or 終点） |
| いずれにもヒットしない空白部 | 既存どおり「新しい線を引く」開始 |
| 選択中線以外の線をクリック | 選択切り替え |
| Escape | 選択解除 |

修飾キーは既存ロジックと整合させる：
- Ctrl: 終点位置・両端ハンドル位置を `snap_to_grid` でグリッド吸着
- Shift: 平行移動／端点移動とも、ドラッグ開始点を起点に `snap_to_45` 相当の方向制約をかける
    - 本体平行移動: `start_translation + snap_to_45(start_cursor, cursor)` の差分
    - 端点移動: 反対側の端点を始点として `snap_to_45` を適用

## 状態設計

`src/state.rs` に以下を追加：

```rust
pub struct EditSelection {
    pub selected_line: Option<usize>, // base_shape.lines のインデックス
    pub drag: EditLineDrag,
}

pub enum EditLineDrag {
    Idle,
    MoveLine {
        start_cursor: Vec2,
        start_a: Vec2,
        start_b: Vec2,
    },
    MoveEndpoint {
        endpoint: Endpoint, // A or B
        fixed: Vec2,        // 反対側の端点（Shift 計算の基準）
    },
}

pub enum Endpoint { A, B }
```

`PlacementState` と類似の責務分離。新規 Bevy `Resource` として追加し、`main.rs` で `init_resource` する。

## ヒットテスト

- 線分本体: 点と線分の最短距離が一定半径（例: 0.02 ワールド単位）以内ならヒット。
- 端点ハンドル: 端点座標を中心とした半径（例: 0.03）以内の円ヒット。
- 端点ハンドルは選択中線分のみ判定対象（未選択時はヒット対象外＝そのまま新規線描画）。

優先順位（押下時）：
1. 選択中線分の端点ハンドル
2. 任意線分の本体（最後尾＝最後に引いた線が手前）
3. 空白部 → 新規線開始（既存挙動）

## Tasks

### Task 1: `EditSelection` リソース追加

`state.rs` に上記型を定義。`main.rs` で `insert_resource(EditSelection::default())`。

### Task 2: 入力処理の分岐

`handle_drag_input` を見直し：

- `MouseButton::just_pressed`：
    1. ヒットテスト（端点 → 線分本体 → 空白）
    2. 端点ヒット: `EditLineDrag::MoveEndpoint` に遷移、Undo に push
    3. 線分本体ヒット: `selected_line` 更新、`EditLineDrag::MoveLine` に遷移、Undo に push
    4. 空白: 既存どおり `DrawState::Dragging` に遷移
- `MouseButton::pressed`（ドラッグ中）：
    - `MoveLine`: `cursor - start_cursor` を `start_a` / `start_b` に加算。Ctrl→端点それぞれに `snap_to_grid`、Shift→`start_a` を起点に方向スナップして長さを保持。
    - `MoveEndpoint`: 端点位置を更新。Ctrl→`snap_to_grid`、Shift→`fixed` を始点に `snap_to_45`。
    - 既存の `DrawState::Dragging` も従来どおり。
- `MouseButton::just_released`：いずれのドラッグも終了し `Idle` に戻す（最小長チェックは新規線描画時のみ）。
- `Escape`: `selected_line = None`、`drag = Idle`。

`DrawState` と `EditLineDrag` は排他的（同時にアクティブにならない）に運用する。状態管理を簡潔に保つため、両者の共通ロジック（egui ガード、修飾キー読み出し）はそのまま使う。

### Task 3: 描画の更新

`draw_canvas`（または分割された描画関数）を更新：

- 確定済み線分のうち、選択中の線は別色（例: `Color::srgb(1.0, 0.85, 0.4)` などの強調色）で描画。
- 選択中線分の両端に `gizmos.circle_2d` でマーカー（半径 0.03、塗りつぶしまたは枠）を描画。
- ドラッグ中（MoveLine / MoveEndpoint）は更新後の値で描画されるので、専用プレビューは不要。

### Task 4: Undo 整合

- `MoveLine` / `MoveEndpoint` 開始時にのみ Undo を 1 回 push（連続編集で複数積まれない）。
- `selected_line` が消去された線を指したまま残らないよう、Clear lines / 線削除時に `EditSelection` をリセット。
- ただし Phase 3 範囲では「線の削除 UI」は導入しない（Phase 内で必要が出たら個別検討）。

### Task 5: ヘルパ関数

- `point_to_segment_distance(p, a, b) -> f32`
- `nearest_line_index(cursor, lines, max_dist) -> Option<usize>`
- `snap_endpoint_general(start, raw_end, modifiers)` … 既存 `edit.rs` の `snap_endpoint` を一般化（端点移動にも使えるように）

`edit.rs` 内に閉じる。新モジュールは作らない。

## 設計メモ

- 線の方向は `Line { a, b }` の順に意味を持たない（描画は両端等価）。端点ドラッグでは A/B の意味は内部で固定する。
- ヒット半径はワールド座標で固定値（カメラスケールに依らない）。Edit カメラは正規化投影なので問題なし。
- Ctrl / Shift は同時押下時の優先順位を既存 `snap_endpoint` に合わせる（Ctrl 優先）。
- Phase 2 で確定タイムラグが解消された前提で、`just_released` の即応性を活用する。

## 受け入れ条件

- [ ] 引いた線をクリックすると、その線の色が変わる
- [ ] 選択された線の両端に◯マーカーが表示される
- [ ] 選択された線の本体をドラッグすると、線全体が平行移動する
- [ ] ◯マーカーをドラッグすると、その端点だけが移動する
- [ ] 別の線をクリックすると選択が切り替わる
- [ ] 何もない場所をクリックすると新規線描画モードに入る（既存どおり）
- [ ] 平行移動・端点移動とも Ctrl 押下でグリッドスナップが効く
- [ ] 平行移動・端点移動とも Shift 押下で 45° 方向スナップが効く
- [ ] 平行移動・端点移動の確定後、1 操作につき Undo が 1 回戻る
- [ ] Escape で選択解除される
- [ ] Clear lines や Undo で対象線が消えた場合、選択状態が安全にリセットされる
- [ ] 既存の新規線描画動作に回帰がない
