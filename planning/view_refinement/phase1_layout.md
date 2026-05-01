# Phase 1: レイアウト・スタイル整備

## 目的

左ペインと右パラメータパネルの見た目・配置を揃え、Base Shape / Placement / Result / Params の役割が一目でわかるレイアウトに整える。

## Tasks

### Task 1: パラメータパネル幅の調整・最小化対応

`src/ui.rs` の右ドック `egui::SidePanel::right("params")` を変更：

- 既定幅を「やや狭く」する（現状 300px → 240px 程度を目安、最終値は実装時に調整）。
- 折りたたみ状態を保持する `bool` を `state.rs` に新リソース `UiLayout` として追加（または既存 `CanvasLayout` を拡張）。
- パネル内の最上部に折りたたみトグルボタン（`◀` / `▶` 等）を配置。クリックで開閉切替。
- 折りたたみ中は本文を描画せず、極小幅（タイトルバー＋ボタンのみ、24〜32px 程度）にする。Result ビューポートは閉じた幅に合わせて広がる。

### Task 2: 描画上の外枠を暗色化

`src/main.rs` の `spawn_unit_frame` の `color` を背景色寄りに暗くする（例: `srgb(0.45, 0.45, 0.55)` → `srgb(0.18, 0.18, 0.22)` 程度）。Base Shape / Placement / Result すべてに反映される（共通関数のため）。

### Task 3: 左ペインの egui パネル化

現状、左ペインは Bevy カメラのビューポートを直接ウィンドウ左半分に張り出しているだけで、egui としては何も描いていない。
これを次の構造に変更する：

- `egui::SidePanel::left("left_pane")` を追加し、Params と同じ枠線＋背景色を持たせる。
- 左パネル内に「Base Shape ブロック」「Placement ブロック」を `egui::Frame` などで縦に並べる（上詰め）。各ブロックは `egui::Frame` で枠＋背景色＋内側パディング（6〜8px 目安）を持つ。
- 各ブロック内のヘッダ部（`egui::ui.horizontal`）に：
    - タイトルラベル（`Base Shape` / `Placement`）を **大きめのフォント**（`heading` または `RichText::heading`）で。
    - その右に並べてボタン群（Base Shape: `Clear lines` / Placement: `+ Add replica`）。
- ブロックの本体（描画領域）は `ui.allocate_space` で **正方形**（幅＝パネル内幅、縦＝同サイズ）を確保する。これがそのまま対応するキャンバスのビューポートになる。
- 描画領域の背景色は Result と同じ（`ClearColor` 相当）に塗る。`egui::Frame::fill` で指定。

ビューポート分配ロジック：

- 左パネルの実幅と、Base Shape ブロック・Placement ブロックの確保した正方形領域の `Rect` を取得。
- それらを物理ピクセルに変換して `EditCamera` / `PlacementCamera` の `Camera::viewport` に書き込む。
- Result カメラは「左パネル右端〜右パラメータパネル左端」の残り全幅を使う。
- パネルが縮んで正方形が確保できないケース（極端に狭い場合）は、最小幅を設けて確保する。確保不能時はビューポートを `None` にしておく方針でも可。

### Task 4: 既存オーバーレイ UI の整理

現状 `draw_panel_overlays` で `egui::Area` を使って Base Shape / Placement / Result のタイトル＋ボタンを「キャンバス上に直接置いている」が、Task 3 で左パネル内のヘッダに移すため不要になる。

- `Base Shape` タイトル `egui::Area` 削除
- `Edit Clear lines` ボタンの `egui::Area` 削除（左パネルヘッダへ）
- `Placement` タイトル `egui::Area` 削除
- `Placement + Add replica` ボタンの `egui::Area` 削除（左パネルヘッダへ）
- `Result` タイトル `egui::Area` 削除（要件: タイトル不要）
- `placement_overlay_ui` の選択中レプリカ `✕` ボタンは残す（描画領域内のフローティング UI のため Phase 1 では維持）。

## 設計メモ

- 左パネル内の正方形確保は `ui.available_width()` をベースに計算し、`ui.allocate_exact_size(Vec2::splat(side), Sense::hover())` で実現。`Response::rect` がそのままビューポートに使える `Rect`。
- `egui::Frame` の `stroke` / `fill` / `inner_margin` を使い、Params と Base Shape / Placement で **同じ Frame テンプレート関数** を共有する（重複防止）。
- `egui::SidePanel::left` の `resizable(false)` で固定幅にし、左パネル幅は中身（正方形側長＋パディング）に合わせて自動的に決まる構成にする（要検討：`exact_width` を中身から計算し設定）。
- Params パネルの折りたたみ状態は `Resource` で持ち、`ui.rs` 内のクロージャで読み書き。

## 受け入れ条件

- [ ] パラメータパネルの初期幅が以前より狭い
- [ ] パラメータパネルにトグルボタンがあり、押すと右端に最小化／復帰する
- [ ] パラメータパネルが最小化されると Result ビューポートがその分広がる
- [ ] Base Shape と Placement の描画上の外枠が以前より暗く目立たない
- [ ] 左ペインに Params と同じ枠線・背景色が適用されている
- [ ] 左ペインの Base Shape / Placement ブロックが内側パディング付きで縦にスタックされている
- [ ] 各ブロックのヘッダにタイトル（大きめ）と操作ボタン（Clear lines / + Add replica）が並んでいる
- [ ] 各ブロックの描画部の背景が Result と同じ色になっている
- [ ] 左ペインの Base Shape / Placement の描画部が **縦横比 1:1**（正方形）になっている
- [ ] パネルは上から詰めて配置され、下部は余白になっている
- [ ] Result パネル内に「Result」タイトルラベルが表示されない
- [ ] Base Shape の線描画／Placement のレプリカ操作／Params の編集が従来どおり動作する
