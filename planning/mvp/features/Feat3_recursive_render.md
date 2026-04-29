# Feat3: フラクタル再帰描画（Result キャンバス）

## 目的

Result キャンバスに、`FractalState`（基本図形・複製・深さ）から再帰的に展開したフラクタル図形を描画する。これにより、ユーザーが Edit キャンバスで定義した基本図形と複製ルールが、深さに応じてどのようなフラクタルになるかを即座に視認できる。

MVP overall_plan の「結果表示部に少なくとも深さ N=4 までのフラクタルが描画される / 深さを UI から変更できる」を満たす。深さスライダ自体は F4 で実装済み。

## 設計方針

### 深さの意味

**`depth = N` は「N 階層分のフラクタル」を意味する。**

- `depth = 1`: 基本図形そのもの（複製を一度も適用しない）
- `depth = 2`: 各複製で 1 回変換した基本図形 → R 個の図形（R = 複製数）
- `depth = N`: 複製を `N-1` 回再帰適用 → `R^(N-1)` 個の基本図形が描画される

この定義により、深さ 1 は Edit キャンバスの基本図形と一致するため、深さを上げると変化が直感的に追える。

### 描画方式

- **毎フレーム再計算（素朴実装）**: MVP overall_plan のとおり、キャッシュは入れない
- **Gizmos ベース**: `ResultGizmos` という新しい `GizmoConfigGroup` を `result_layer()` にバインドし、Result キャンバスにのみ描画
- **再帰アルゴリズム**: 累積変換 `Replica` を引数に持つ再帰関数で、葉ノードに到達したら基本図形を変換適用して描画

```text
draw_fractal(depth, transform):
    if depth == 1:
        for line in base_shape.lines:
            draw transform.apply(line.a) → transform.apply(line.b)
    else:
        for replica in replicas:
            draw_fractal(depth - 1, transform.compose(replica))
```

`transform.compose(replica)` は「replica を内側に適用し、その結果を transform で変換する」合成。IFS 標準の `f_{i_1} ∘ f_{i_2} ∘ ... ∘ f_{i_{N-1}}` を構築する。

### 計算量と性能上の制約

- 葉ノード数は `R^(N-1)`。R=2, N=7 → 64、R=4, N=7 → 4,096、R=10, N=7 → 約 100 万
- MVP では最適化しない方針。深さ上限 7 / 複製数の暗黙制約はユーザー判断に委ねる
- 著しく遅くなるケース（複製 5+ × 深さ 7+）の対策は Phase 4

### Edge case

- `replicas` が空 / `depth >= 2` → 何も描画されない（再帰のループ本体が空のため）
  - ユーザーは深さを 1 に下げれば基本図形を確認できる（Edit 側でも見えている）
- `base_shape.lines` が空 → 葉に到達しても線が 0 本なので何も描画されない

## 計画概要

| ID | 内容 |
|----|------|
| T1 | `Replica::compose(self, other) -> Replica` を `state.rs` に追加（合成: self ∘ other） |
| T2 | `ResultGizmos` GizmoConfigGroup を定義し、`result_layer()` にバインド |
| T3 | 再帰描画関数 `draw_fractal_recursive` を実装し、`Update` システムから呼ぶ |
| T4 | `DrawPlugin` に Result 系のシステムを追加 |
| T5 | clippy / fmt / screencapture（Y 字フラクタル: 1 線 + 2 複製 で「樹木」が深さ 4 で描画されることを確認） |

## 計画詳細

### T1: Replica::compose

```rust
impl Replica {
    /// Composition: returns `r` such that `r.apply(p) == self.apply(other.apply(p))`.
    /// Equivalent to `self ∘ other` (other applied first, then self).
    pub fn compose(self, other: Replica) -> Replica {
        // (self ∘ other)(p) = self(other(p))
        //   = self(scale_o * R_o(p) + translation_o)
        //   = scale_s * R_s(scale_o * R_o(p) + translation_o) + translation_s
        //   = (scale_s * scale_o) * R_{s+o}(p)
        //     + scale_s * R_s(translation_o) + translation_s
        let cos_s = self.rotation.cos();
        let sin_s = self.rotation.sin();
        let rot_t = Vec2::new(
            other.translation.x * cos_s - other.translation.y * sin_s,
            other.translation.x * sin_s + other.translation.y * cos_s,
        );
        Replica {
            translation: self.translation + self.scale * rot_t,
            rotation: self.rotation + other.rotation,
            scale: self.scale * other.scale,
        }
    }

    pub fn identity() -> Replica {
        Replica { translation: Vec2::ZERO, rotation: 0.0, scale: 1.0 }
    }
}
```

`identity()` は再帰の初期値として使う。

### T2-T4: ResultGizmos と描画システム

`draw.rs` に追加:

```rust
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct ResultGizmos;

const FRACTAL_COLOR: Color = Color::srgb(0.85, 0.95, 0.9);

fn draw_fractal_result(state: Res<FractalState>, mut gizmos: Gizmos<ResultGizmos>) {
    draw_fractal_recursive(
        state.depth,
        Replica::identity(),
        &state.base_shape.lines,
        &state.replicas,
        &mut gizmos,
    );
}

fn draw_fractal_recursive(
    depth: u32,
    transform: Replica,
    lines: &[Line],
    replicas: &[Replica],
    gizmos: &mut Gizmos<ResultGizmos>,
) {
    if depth <= 1 {
        for line in lines {
            gizmos.line_2d(transform.apply(line.a), transform.apply(line.b), FRACTAL_COLOR);
        }
    } else {
        for replica in replicas {
            draw_fractal_recursive(
                depth - 1,
                transform.compose(*replica),
                lines,
                replicas,
                gizmos,
            );
        }
    }
}
```

`DrawPlugin::build` に:
- `insert_gizmo_config(ResultGizmos, GizmoConfig { render_layers: result_layer(), ..default() })`
- `add_systems(Update, draw_fractal_result)`

### T5: 動作確認シナリオ

screencapture 用ダミー（Y 字樹木フラクタル）:
- 基本図形: 縦 1 本線 `(0, -0.5) → (0, 0.5)`
- 複製 0: 上端から 30° 左に分岐。translation=(−0.25, 0.5) 程度、rotation=+30°、scale=0.5
- 複製 1: 上端から 30° 右に分岐。translation=(+0.25, 0.5) 程度、rotation=−30°、scale=0.5
- depth=4 で 8 本の終端線が「樹形」を描く

確認後ダミーは revert。

## 受け入れ条件

- [x] Result キャンバスに、基本図形と複製ルールから展開したフラクタルが描画される（screenshot 確認済）
- [x] 深さスライダの値を変えると、Result キャンバスの描画が即座に追従する（ユーザー確認 OK）
- [x] 深さ N=4 で `R^(N-1)` 本の終端基本図形が出る（R=複製数）（screenshot で 2^3=8 枝を確認）
- [x] 既知のフラクタル例（Y 字樹木）が再現できる（ユーザー確認 OK）
- [x] Edit キャンバス側にはフラクタル描画が出ない（Result レイヤー限定）（screenshot 確認済）
- [x] 複製が 0 個 / 深さ >= 2 の組み合わせでは何も描画されない（バグではない）
- [x] `cargo clippy -- -D warnings` が通る
- [x] `cargo fmt --check` が差分なしで通る
- [x] screencapture で自走確認 OK（Y 字フラクタルが Result に描画され、Edit には出ないことを確認。ただしデモデータの配置でキャンバス外にはみ出す部分あり — 動作は正常）

## 範囲外

- 描画キャッシュ・差分更新 → Phase 4
- 線スタイル（色・太さ）の編集 → Phase 4
- 深さ 8 以上 → Phase 4 で性能見直し後
- 着色・グラデーション → Phase 4
- リアルタイム反映の体感確認 → Feat4
