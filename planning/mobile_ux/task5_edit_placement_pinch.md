# Task 5: Edit / Placement 画面のピンチズーム

## 目的

Edit・Placement 画面でピンチイン/アウトしてズームできるようにする。

## 現状

`view.rs` の `update_result_view` のみピンチズームが実装されている（`PinchState` リソース使用）。
Edit・Placement カメラには同等の処理がない。

## 実装方針

### アーキテクチャ

`view.rs` に Edit・Placement カメラ向けのピンチ処理を追加する。
現状の `PinchState` は Result 専用の単一インスタンスなので、3 カメラ分に拡張が必要:

**方針A: カメラ別の PinchState**（推奨）
- `EditPinchState`, `PlacementPinchState`, `ResultPinchState` を個別リソースとして定義
- または `PinchState` を汎用にして、タッチ座標とビューポート矩形の照合でどのカメラか判別

**方針B: 単一 PinchState にカメラID フィールド追加**

→ 方針Aの「ビューポート照合」を採用:
1. タッチ 2 本指の中点がどのビューポートに入っているかを `CanvasLayout` で判別
2. 対応するカメラにズーム処理を適用

### ビューポート照合

`CanvasLayout` に Edit・Placement のビューポート矩形も持たせる（現状は Placement のみ）:
- `edit_min/max_x/y` フィールドを追加
- `ui.rs` の `layout_wide` / `layout_narrow` で更新

### ズーム処理

Result と同じスケール計算を Edit・Placement カメラに適用:
```
scale = prev_distance / current_distance
camera.scale *= scale
camera.scale.clamp(0.005, 8.0)
```

ズーム中心は 2 本指の中点（ワールド座標に変換してカメラ位置を補正）。

## 実装対象ファイル

- `src/state.rs`: `CanvasLayout` に edit ビューポートフィールドを追加
- `src/view.rs`: Edit・Placement カメラ向けピンチ処理を追加
- `src/ui.rs`: `CanvasLayout` の edit フィールドを更新

## 注意点

- Edit・Placement での描画ドラッグと指の競合:
  - 2 本指 → ピンチズーム（edit/placement の描画はキャンセル）
  - 1 本指 → 既存の描画・移動操作
  - この分岐は既存の `edit.rs` / `placement.rs` が「2 本指以上でドラッグキャンセル」を実装済みなので整合する

## 受け入れ条件

- [ ] Edit 画面でピンチイン/アウトするとズームする
- [ ] Placement 画面でピンチイン/アウトするとズームする
- [ ] ピンチ中は描画・移動操作がキャンセルされる
- [ ] 1 本指ドラッグでは既存の描画・操作が動く（回帰なし）
- [ ] Result のピンチズームに回帰がない
