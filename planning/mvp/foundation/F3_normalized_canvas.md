# F3: 正規化座標キャンバス

## 目的

操作キャンバスと結果表示キャンバスの両方で、座標系 [-1, 1] × [-1, 1] の正方領域を「歪まずに」表示する。後続の Features がこの正規化座標で点・線・図形を配置すれば、ウィンドウサイズや形状によらず正しい位置に描かれる状態にする。

## 設計方針

- 両キャンバスとも、ワールド座標 [-1, 1] × [-1, 1] が「正方形として」見えるようにする（アスペクト比による歪みなし）
- viewport が縦長か横長かによって、長い方の軸では [-1, 1] の外側が「余白」として見える（letterbox 的）
- 「キャンバスの境界」を示す枠線（unit square）を視覚化する。後続でこの枠の内側だけが「キャンバス」として扱われる
- F2 で世界座標 (10000, 0) に置いていた Result コンテンツの空間オフセットを廃止し、`RenderLayers` でコンテンツを分離する。両カメラの Transform は (0, 0) で揃える

## 計画概要

| ID | 内容 |
|----|------|
| T1 | コンテンツ分離方式を空間オフセットから `RenderLayers` に切替（EDIT_LAYER / RESULT_LAYER 定数を定義） |
| T2 | 両カメラに正規化用の `Projection`（`OrthographicProjection`、`ScalingMode::AutoMin { min_width: 2.0, min_height: 2.0 }`）を設定 |
| T3 | 両カメラの位置を (0, 0) に揃え、各キャンバス内のラベル等もそれぞれの RenderLayer 上に配置 |
| T4 | 各キャンバスに「単位正方形（[-1, 1] × [-1, 1]）」の枠線を描画（Bevy gizmos 利用） |
| T5 | 視認用の補助要素として、原点マーカ（小さな十字 or 円）と軸線を gizmos で描画 |
| T6 | "Edit" / "Result" ラベルのスケールをワールド単位（[-1, 1] 内）に合わせて再調整 |

## 計画詳細

### T1: RenderLayers でのコンテンツ分離

```rust
const EDIT_LAYER: usize = 1;
const RESULT_LAYER: usize = 2;
```

- EditCamera に `RenderLayers::layer(EDIT_LAYER)`、ResultCamera に `RenderLayers::layer(RESULT_LAYER)` を付与
- 各キャンバスに置くエンティティに対応する `RenderLayers::layer(...)` を付与
- UI カメラは引き続き `RenderLayers::none()`（egui 専用）

これにより、両カメラとも世界座標の同じ場所を見ても、自分のレイヤーのコンテンツしか映さない。F2 で使った `RESULT_AREA_OFFSET_X = 10_000.0` は撤去。

### T2: ScalingMode で歪み無し正規化

```rust
Projection::Orthographic(OrthographicProjection {
    scaling_mode: ScalingMode::AutoMin { min_width: 2.0, min_height: 2.0 },
    ..OrthographicProjection::default_2d()
})
```

- `AutoMin { min_width: 2.0, min_height: 2.0 }` は「幅と高さの少なくとも一方を 2.0 にし、もう一方はアスペクト比に応じて 2.0 以上にする」モード
- これにより [-1, 1] は常に viewport 内に収まり、 viewport が長方形なら長辺方向に余白ができる
- アスペクト比による歪みは発生しない

[Question]
Bevy 0.18 の `OrthographicProjection` / `ScalingMode` の正確な API パスとデフォルト 2D ヘルパは要確認。実装時に `bevy::camera` 配下の最新シグネチャを参照。
[/Question]

[Draft]
T2 着手時に `cargo doc --open` または docs.rs で確認。問題があれば本計画書を更新する。基本的な構造は維持。
[/Draft]

[Answer]
（実装時に確定）
[/Answer]

### T4: 単位正方形の枠線

- Bevy 標準の `Gizmos` SystemParam を使い、`Update` 内で毎フレーム描画
- 各キャンバスごとに、対応する `RenderLayers` 上で枠線を描画する必要がある
- `Gizmos<DefaultGizmoConfigGroup>` ではレイヤー指定ができないため、`GizmoConfig` を 2 種類（EDIT 用 / RESULT 用）登録し、それぞれを別の `RenderLayers` にバインドする方式を採る

[Question]
Bevy 0.18 の Gizmos に `RenderLayers` をバインドする最新 API を実装時に確認する。代替: gizmos でなく `Mesh2d` で枠線エンティティを spawn し、`RenderLayers::layer(...)` を直接付与する方式（こちらの方が単純なので、複雑なら採用）。
[/Question]

[Draft]
シンプル路線で `Mesh2d` 方式を第一候補にする。Rectangle メッシュではなく `Annulus` でもなく、線の集合（4 辺）を `Polyline` 風にするか、`Rectangle` の輪郭描画モードがあれば採用。最終的には gizmos を使えれば最も簡潔（線描画特化）。
[/Draft]

[Answer]
（実装時に確定）
[/Answer]

### T5: 原点マーカ・軸線

- 原点に小さな十字（±0.05 程度の長さの 2 線）を描画
- x 軸 (y=0) の [-1, 1]、y 軸 (x=0) の [-1, 1] を薄い色で描画（任意）

これは F3 単体の検証目的でもあり、Features 着手後に視認性に応じて削除/調整可能。

### T6: ラベルのワールド単位化

F2 では `font_size: 48.0` のテキストを world 上に置いていたが、座標が [-1, 1] になると 48 単位は巨大すぎる。世界座標で 0.2 〜 0.3 程度の高さに見えるようにスケールするか、フォントサイズを下げて Transform.scale で調整する。

実装案: `Transform::from_scale(Vec3::splat(0.005))` 程度で 48px → 約 0.24 ワールド単位。ラベルは画面確認の暫定要素なので Feat1 着手時に撤去予定。

## 受け入れ条件

- [x] EditCamera / ResultCamera の Transform.translation がいずれも (0, 0, ...) である（空間オフセット撤去）
- [x] EDIT_LAYER のエンティティは Edit キャンバスにのみ、 RESULT_LAYER のエンティティは Result キャンバスにのみ表示される
- [x] 両キャンバスに [-1, 1] × [-1, 1] の正方枠が描画され、 viewport の縦横比に関わらず「正方形」として見える
- [ ] ウィンドウをリサイズしても枠が正方形のまま、viewport 中央付近に配置される（**ユーザー目視確認推奨**）
- [x] 原点マーカが両キャンバスの中心に表示される
- [x] `cargo clippy -- -D warnings` が通る
- [x] `cargo fmt --check` が差分なしで通る
- [x] screencapture で自走確認 OK

## 範囲外

- マウス座標 → 正規化座標 への逆変換 → Feat1（描画機能）で実装
- ズーム / パン → 必要なら Phase 4
- グリッド / スナップ → Phase 4
