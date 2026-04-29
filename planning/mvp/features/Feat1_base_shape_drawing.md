# Feat1: 基本図形の描画（ドラッグで直線）

## 目的

操作キャンバス（Edit）の上をドラッグして直線を引ける状態を作る。引いた線は `FractalState::base_shape.lines` に蓄積され、複数本引ける。これにより、後続の Feat2（複製の配置）と Feat3（再帰描画）が「ユーザーが定義した基本図形」を入力として使えるようになる。

## 設計方針

- 入力は **マウス左ボタンのドラッグ** のみ（押下＝始点、移動＝プレビュー、離す＝確定）
- 入力を受け付けるのは **Edit カメラの viewport 内**かつ **egui がポインタを占有していないとき** のみ
- マウスの physical 座標 → Edit カメラのビュー座標 → 正規化座標 [-1, 1] への変換は、`Camera::viewport_to_world_2d` を使う
- 確定線は `FractalState::base_shape.lines` に push、ドラッグ中の線は `DrawState` Resource にだけ持つ（state にはまだ入れない）
- 描画は **Gizmos**（毎フレーム描画）。Edit レイヤー専用の `GizmoConfigGroup` を 1 つ用意する
- 「全消し（Clear）」ボタンを params パネルに追加（受け入れ必須ではないが、繰り返し試行のために実装）

## 計画概要

| ID | 内容 |
|----|------|
| T1 | `DrawState` Resource を定義（`Idle` / `Dragging { start: Vec2 }`） |
| T2 | カーソル位置 → Edit キャンバス正規化座標 への変換ヘルパを作成（viewport 外なら `None`） |
| T3 | ドラッグ入力ハンドリングシステム（押下・離す・キャンセル）を実装し、確定線を `FractalState::base_shape.lines` に push |
| T4 | Edit レイヤー用 `GizmoConfigGroup` を登録し、確定線とドラッグ中プレビュー線を gizmos で描画 |
| T5 | params パネルに "Clear lines" ボタンを追加（`base_shape.lines.clear()`） |
| T6 | F3 で置いた "Edit" ラベル / 原点マーカ等の暫定 decor を見直し（残置で良いものと撤去するもの） |
| T7 | clippy / fmt / screencapture 通過 |

## 計画詳細

### T1: DrawState

```rust
#[derive(Resource, Default)]
enum DrawState {
    #[default]
    Idle,
    Dragging { start: Vec2 }, // 正規化座標
}
```

- `App::init_resource::<DrawState>()` で登録
- Edit 専用の概念なので、`src/main.rs` 内に置くか、`features/draw.rs` のような新規モジュールに分けるかは実装時判断（モジュール 1 つで収まりそうなら main 直下で可）

### T2: カーソル → 正規化座標 変換

責務: `(window, edit_camera) -> Option<Vec2>`

- `window.cursor_position()` が `None` なら `None`
- `Camera::viewport_to_world_2d(camera_transform, cursor)` を呼ぶ
  - これはカーソルが viewport 外なら `Err` を返す（Bevy 0.18 仕様）
  - その場合 `None` を返す
- 戻り値は Edit カメラの world 座標 = 正規化座標 [-1, 1]（F3 の `ScalingMode::AutoMin { 2.0, 2.0 }` により等倍）

なお Bevy の `cursor_position` は logical coords を返すため、`Camera::viewport_to_world_2d` が期待する座標系に合わせて変換する必要があるかは実装時に確定する。

### T3: 入力ハンドリング

毎フレーム実行する `Update` システム。擬似コード:

```
let cursor = cursor_to_edit_normalized(window, edit_cam); // Option<Vec2>
let egui_wants_pointer = ctx.wants_pointer_input(); // egui 占有時はスキップ

match (*draw_state, mouse_button, cursor, egui_wants_pointer) {
    (Idle,    JustPressed, Some(p), false) => *draw_state = Dragging { start: p }
    (Dragging{start}, JustReleased, Some(end), _) => {
        if (end - start).length() >= MIN_LINE_LEN { state.base_shape.lines.push(Line { a: start, b: end }); }
        *draw_state = Idle
    }
    (Dragging{..}, JustReleased, None, _) => *draw_state = Idle  // viewport 外で離した: 破棄
    _ => {}
}
```

- `MIN_LINE_LEN = 0.01`（正規化座標で 1% 程度。クリックの弾き）
- ドラッグ中に egui に乗っても、すでに開始した操作は継続する（`JustPressed` 時のみ egui 占有チェック）

### T4: 描画（Gizmos）

```rust
#[derive(Default, Reflect, GizmoConfigGroup)]
struct EditGizmos;

app.init_gizmo_group::<EditGizmos>();
// GizmoConfigStore で render_layers を edit_layer() にバインド
```

- `gizmos: Gizmos<EditGizmos>` を取り、確定線リストを毎フレーム line_2d 描画
- `DrawState::Dragging { start }` なら、現在カーソル位置 (cursor_to_edit_normalized の結果) を終点としてプレビュー線を描画。色は確定線と区別（薄め or 別色）
- 線色は固定（例: 確定 = `Color::srgb(0.9, 0.9, 1.0)`、プレビュー = `Color::srgb(1.0, 0.8, 0.4)`）

[Question]
Bevy 0.18 の `GizmoConfigGroup` に `RenderLayers` をバインドする最新 API（`GizmoConfigStore::config_mut::<G>()` から `render_layers` を変更する形か、別 API か）は実装時に docs.rs で確定する。Mesh2d 方式のフォールバックも検討。
[/Question]

[Draft]
T4 着手時に `bevy::gizmos::config::GizmoConfig` の最新フィールドを確認。`render_layers` フィールドが直接生えているか、`GizmoConfig::render_layers = edit_layer()` で設定できる想定。問題があれば本計画書を更新。
[/Draft]

[Answer]
（実装時に確定）
[/Answer]

### T5: Clear ボタン

`params_panel` 内、Depth スライダの下あたりに:

```rust
if ui.button("Clear lines").clicked() {
    state.base_shape.lines.clear();
}
```

を追加。ドラッグ中の `DrawState` には触れない（仕様上問題が出たら考える）。

### T6: 暫定 decor の整理

F3 で置いた要素:
- 単位正方形の枠（4 辺の Sprite）→ **残す**（キャンバス境界の視認用）
- 原点十字マーカ → **残す**（描画位置の基準として有用）
- "Edit" / "Result" ラベル（Text2d）→ **撤去**（線描画と被ると見づらい）

Result 側の "Result" ラベルも合わせて撤去するかは Feat3 着手時に判断（Feat1 の範囲では Edit 側のみ撤去で十分）。

[Question]
"Edit" ラベルを撤去すると、Feat1 完了後の見た目で「左がどっちか」がわかりにくくなる懸念。代わりに params パネルの heading で示すか、`SidePanel` の左に小さなタイトル枠を置くか。
[/Question]

[Draft]
撤去のみで進める。Edit / Result は左右の位置と内容（手書き線 vs 再帰描画）から自明になる想定。困ったら戻す。
[/Draft]

[Answer]
（実装時に確定）
[/Answer]

## 受け入れ条件

- [x] Edit キャンバス上でマウス左ボタンをドラッグすると、押下点と離した点を結ぶ直線が描画される（ユーザー目視確認 OK）
- [x] 複数本の直線を続けて引ける（過去に引いた線は残る）（ユーザー目視確認 OK）
- [x] 引いた線は `FractalState::base_shape.lines` に追加され、params パネルの "Lines: N" の N が増える（ユーザー目視確認 OK）
- [x] ドラッグ中（押下〜離す）のプレビュー線が表示され、カーソル位置に追従する（ユーザー目視確認 OK）
- [x] Result キャンバス側や egui サイドパネル上をドラッグしても線が描画されない（ユーザー目視確認 OK）
- [x] params パネルに "Clear lines" ボタンがあり、押すと描画線が全消去される（"Lines: 0" に戻る）
- [x] ウィンドウをリサイズしても、既に引いた線の正規化座標が維持され、枠（[-1, 1]）と整合した位置に描画され続ける（ユーザー目視確認 OK）
- [x] `cargo clippy -- -D warnings` が通る
- [x] `cargo fmt --check` が差分なしで通る
- [x] screencapture で自走確認 OK（ダミー線投入で Edit 側のみに線が描画されること、Result 側に出ないこと、座標が正規化 [-1, 1] と整合することを確認済。確認後ダミー線は revert 済）

## 範囲外

- 個別の線の選択・削除・編集 → Phase 4（MVP では「全消しでやり直す」運用）
- 線の色・太さ等のスタイル編集 → Phase 4
- スナップ・グリッド・ガイド → Phase 4
- 直線以外の図形（曲線・多角形プリセット）→ Phase 2
- Undo/Redo → Phase 4
- 複製の配置・編集 → Feat2
- フラクタル再帰描画 → Feat3
