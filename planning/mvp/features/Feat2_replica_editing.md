# Feat2: 複製の配置・編集・削除

## 目的

「基本図形をどう変換して再配置するか」を定義する `Replica`（位置・回転・等方スケール）をユーザーが追加・編集・削除できるようにする。これにより Feat3 の再帰描画が「ユーザー定義の変換ルール集合」を入力として動作可能になる。

MVP overall_plan の「複製を 1 つ以上追加できる / 各複製の位置・角度・スケールを **数値入力もしくはドラッグ** で編集できる / 複製を削除できる」を満たす。MVP では **数値入力（egui）** に絞る（キャンバス上ドラッグは範囲外）。

## 設計方針

- 編集 UI は **egui の Parameters パネル内に「Replicas」セクション** として配置（Depth スライダの下、Clear lines の下）
- 各複製は折りたたみ可能な行（CollapsingHeader）で、TX / TY / Rotation (deg) / Scale を `DragValue` か小型 Slider で編集
- 角度はユーザー向けは「度 (deg)」、内部は radians（state は radians のまま）
- **スケールは厳密に > 0**（0 以下は再帰計算が破綻するため、最小値で clamp）
- **複製のキャンバス上プレビュー**: 統合キャンバス方針通り、基本図形のバウンディングボックスを各複製の affine 変換で配置した「半透明矩形」を Edit キャンバスに重ねて描画
- 基本図形が空（lines = 0）でも複製は追加できる（プレビュー枠が単に点になるだけ）
- 「Add replica」ボタンで初期値（translation = (0, 0), rotation = 0, scale = 0.5）の複製を 1 つ追加
- 各複製行に「×」削除ボタン

## 計画概要

| ID | 内容 |
|----|------|
| T1 | params パネルに "Replicas" セクションを追加（heading + Add ボタン） |
| T2 | 各複製の編集 UI（CollapsingHeader: TX / TY / Rotation deg / Scale + Delete） |
| T3 | 削除ロジック（`Vec::remove` を後段で実行できるよう、UI ループ内では削除 index をバッファ） |
| T4 | 基本図形のバウンディングボックス計算ヘルパ（`BaseShape::bbox() -> Option<Rect>` 相当） |
| T5 | Edit キャンバス上に各複製の bbox を変換した半透明矩形を gizmos で描画 |
| T6 | clippy / fmt / screencapture（複製 2 つ追加 → bbox 枠が 2 つ表示される） |

## 計画詳細

### T1-T2: Parameters パネルの拡張

```text
[Parameters]
  Depth: [slider 1..=7]
  Lines: N
  Replicas: M
  [Clear lines]
  ── Replicas ──
  [+ Add replica]
  ▼ Replica 0                     [×]
     TX [DragValue -2..=2]
     TY [DragValue -2..=2]
     Rot [DragValue -180..=180] deg
     Scale [DragValue 0.05..=2.0]
  ▼ Replica 1                     [×]
     ...
```

- `egui::CollapsingHeader::new(format!("Replica {i}"))` で各行を畳めるようにする
- `DragValue::new(...).speed(0.01).range(...)` で数値編集（連続的）。MVP では Slider より省スペース
- 削除ボタンは collapsing の右端（`ui.horizontal` 内で配置するか、ヘッダ内で工夫）

[Question]
egui の `CollapsingHeader` の右端にボタンを置くのは API 上やや手間（カスタム `header_response` が必要）。MVP では各 Replica の中身領域の最後に "Delete" ボタンを置くシンプル方式で開始する案で良いか。
[/Question]

[Draft]
シンプル路線: collapsing の中身に "Delete this replica" ボタンを置く。視認性が悪ければ後で改善。
[/Draft]

[Answer]
（実装時に確定）
[/Answer]

### T3: 削除のタイミング

UI 構築中に `state.replicas.remove(i)` を呼ぶと、その後のループのインデックスが破綻する。`to_delete: Option<usize>` をループ前に用意し、ループ後に `if let Some(i) = to_delete { state.replicas.remove(i); }` で実行する。

### T4: バウンディングボックス

```rust
impl BaseShape {
    pub fn bbox(&self) -> Option<Rect2d> {
        // lines が空なら None
        // 全 line 端点 (a, b) から min/max を取る
    }
}
```

- 戻り値は新規型 `Rect2d { min: Vec2, max: Vec2 }` か、Bevy の `bevy::math::Rect` を流用
- Feat3 でも使うので `state.rs` か `geometry.rs` 等の共有モジュールに置く

[Question]
Bevy 0.18 の `bevy::math::Rect` を流用するか、自前で `Rect2d` を定義するか。流用が自然だが、後で「複製変換後の 4 頂点」を扱う際に AABB の拡張が必要なので、まずは Vec2 の min/max ペアで充分。
[/Question]

[Draft]
`bevy::math::Rect` を流用。
[/Draft]

[Answer]
（実装時に確定）
[/Answer]

### T5: 複製プレビュー枠の描画

各 `Replica` について:
1. base bbox の 4 頂点 `[(min.x, min.y), (max.x, min.y), (max.x, max.y), (min.x, max.y)]` を取得
2. 各頂点に scale → rotate → translate を適用（`Vec2` 演算で書く、または `Affine2` を使う）
3. 4 頂点を結ぶ 4 本の線分を `EditGizmos` で半透明色描画（例: `Color::srgba(0.4, 0.7, 1.0, 0.6)`）

実装は `draw.rs` の `draw_lines` 系統に並べる新システム `draw_replicas` を追加。

### T6: 受け入れ確認

- screenshot 用に「ダミーで 2 つ Replica を default_mvp に投入 → bbox 枠 2 つが Edit に出る」確認 → ダミーは revert
- ユーザー目視確認: 数値編集の挙動・削除動作・追加動作

## 受け入れ条件

- [x] Parameters パネルに "Add replica" ボタンがあり、押すと初期値の複製が 1 つ追加される（ユーザー確認 OK）
- [x] 追加された複製は CollapsingHeader で表示され、TX / TY / Rotation (deg) / Scale を編集できる（ユーザー確認 OK）
- [x] 各複製に "Delete this replica" ボタンがあり、押すと state から削除される（ユーザー確認 OK）
- [x] 基本図形（直線が 1 本以上）が存在する状態で複製を追加すると、Edit キャンバス上に基本図形の bbox を変換した半透明矩形がプレビュー表示される（screenshot 確認済）
- [x] 数値を編集すると、プレビュー矩形がリアルタイムに位置・回転・スケールに追従する（ユーザー確認 OK）
- [x] 基本図形が空（lines = 0）でも複製は追加・編集・削除できる（プレビュー矩形は出ない or 点）
- [x] スケールに 0 以下が入らないよう、UI 側で最小値 clamp されている（range 0.05..=2.0）
- [x] Result キャンバスには複製プレビュー矩形が出ない（Edit レイヤー限定）（screenshot 確認済）
- [x] `cargo clippy -- -D warnings` が通る
- [x] `cargo fmt --check` が差分なしで通る
- [x] screencapture で自走確認 OK（ダミー 1 線 + 2 複製投入 → Edit に変換済み bbox 線 2 本が描画、Result は空）

## 範囲外

- キャンバス上ドラッグでの位置・回転・スケール編集 → Phase 4
- 複製の複製（コピペ）→ Phase 4
- 複製のグルーピング・対称配置プリセット・パラメータ同期 → Phase 3
- 反転・剪断などの非 affine 変換 → Phase 2 以降
- フラクタル再帰描画（複製を depth 回適用）→ Feat3
- リアルタイム反映の体感確認 → Feat4
