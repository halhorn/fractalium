# パフォーマンス最適化：Mesh キャッシュ描画

## 目的

`FractalState` が変化したフレームだけ再計算を行い、線分集合を `Mesh`（`LineList`）として 1 ドローコールで描画する。
現状は毎フレーム R^(N-1) 本の線分を再帰計算して gizmos で 1 本ずつ描画しているため、depth が深くなるほど激重になる。

## 計画概要

### Task 1: Mesh ベースの描画に切り替える

gizmos による逐次描画を廃止し、`Mesh2d` + `MeshMaterial2d<ColorMaterial>` による一括描画に置き換える。
Result キャンバス専用の Mesh Entity を Startup 時に生成し、状態変化時に頂点バッファを書き直す。

### Task 2: 状態変化検知とキャッシュ

`FractalState::is_changed()` で変化フレームのみ再計算・Mesh 更新を走らせる。

## 計画詳細

### Mesh の構成

- `PrimitiveTopology::LineList` で頂点を `[a0, b0, a1, b1, ...]` の順に並べる
- `Mesh::ATTRIBUTE_COLOR` に `[f32; 4]`（LinearRgba）を頂点ごとに持たせる
- `ColorMaterial` は `#ifdef VERTEX_COLORS` で頂点カラーを自動適用する（カスタムシェーダー不要）
- マテリアルは `ColorMaterial::default()`（白・単色）でよい

### 色の維持

`collect_fractal_segments` の hue 計算ロジックはそのまま流用し、各線分の両端点に同じ色（`hue_to_color`）を `ATTRIBUTE_COLOR` として書き込む。外観は現状と同一になる。

### RenderLayers との統合

Mesh Entity に `result_layer()` の `RenderLayers` を付与することで、Result カメラのみに描画される。

### FractalMeshEntity マーカーコンポーネント

Startup 時に生成した Mesh Entity を後から Query するためのマーカーコンポーネントを定義する。

## 受け入れ条件

- [ ] `cargo build` が通る
- [ ] Result キャンバスにフラクタルが描画される（外観・色が旧実装と同等）
- [ ] `FractalState` を変更しないフレームでは Mesh 更新が走らない
- [ ] depth 12・replica 4 程度でアイドル時の FPS が改善される
