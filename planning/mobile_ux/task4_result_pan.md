# Task 4: Result 画面のドラッグでパン

## 目的

Result 画面でドラッグ（タッチ含む）すると、フラクタルが平行移動（パン）するようにする。

## 現状

`view.rs` の `update_result_view`:
- マウスホイール → ズーム（カーソル位置を中心）
- ピンチ（2 本指） → ズーム

ドラッグ（パン）は未実装。

## 実装方針

### マウスドラッグ

`view.rs` の `update_result_view` 内で `MouseButton::Left` のドラッグを検出:

```
if mouse_button.pressed(Left) && !is_pinching {
    delta = current_cursor_pos - prev_cursor_pos
    camera.translation -= delta / camera.scale
}
```

前フレームのカーソル位置を `PreviousCursorPos` リソースで保持するか、
Bevy の `CursorMoved` イベントから delta を取得する。

### タッチドラッグ（1 本指）

Result 用のビューポート内でのシングルタッチドラッグをパンに割り当てる:
- 2 本指の場合は既存のピンチズームが動作
- 1 本指のみのときにパン

`PinchState` の指追跡と整合するよう、1 本指 vs 2 本指の分岐を既存の実装に合わせる。

### カメラ座標系の注意

Result カメラは `Camera2d` + `OrthographicProjection`。
パン量 = スクリーン空間のデルタ ÷ カメラの `scale`（拡大率）。

## 実装対象ファイル

- `src/view.rs`: `update_result_view` にドラッグパン処理を追加
- 必要なら `src/state.rs` にカーソル前フレーム位置のリソースを追加

## 受け入れ条件

- [ ] Result 画面でマウスドラッグするとフラクタルがパンする
- [ ] Result 画面でタッチ 1 本指ドラッグするとフラクタルがパンする
- [ ] ピンチ操作中はパンしない（2 本指 → ズームのみ）
- [ ] ズームとパンを組み合わせても座標がずれない
