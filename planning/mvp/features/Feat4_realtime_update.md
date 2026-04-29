# Feat4: リアルタイム反映

## 目的

操作部（Edit キャンバス・params パネル）での変更が結果表示部（Result キャンバス）に 1 秒以内（体感的に即時）に反映されることを確認・保証する。

MVP overall_plan の「操作部での変更が結果表示部に 1 秒以内（体感的に即時）に反映される」を満たす。

## 設計方針

現実装で既に要件を満たしている。

- `FractalState` は Bevy `Resource`。Feat1〜3 の変更（lines push / replicas 更新 / depth 変更）は全て同一フレーム内でこの Resource を書き換える
- `draw_fractal_result` は `Update` スケジュールで毎フレーム `Res<FractalState>` を読み直し、`ResultGizmos` に全ラインを送出する（gizmos は毎フレームクリアされ再描画）
- `params_panel` は `EguiPrimaryContextPass` で実行され depth / replicas を書き換え、同フレームまたは翌フレームの `Update` で gizmos に反映される

## 計画概要

新規コードは不要。ユーザー目視確認とスクリーンショット自走確認のみ。

| ID | 内容 |
|----|------|
| T1 | ダミーデータを投入した状態で起動し、screencapture で Result 描画を確認 |
| T2 | ユーザー目視確認: 以下の操作で即時反映されるかチェック |
| T3 | 受け入れ条件チェックボックスを更新 |

## 受け入れ条件

- [ ] Edit キャンバスで線を引くと、Result に即座にフラクタル変化が反映される（**ユーザー目視確認**）
- [ ] params パネルで複製の TX/TY/Rotation/Scale を変更すると、Result が即座に追従する（**ユーザー目視確認**）
- [ ] Depth スライダを変更すると、Result の描画深さが即座に変化する（**ユーザー目視確認**）
- [x] `cargo clippy -- -D warnings` が通る（Feat1〜3 で確認済み）
- [x] `cargo fmt --check` が差分なしで通る（Feat1〜3 で確認済み）
- [x] screencapture で自走確認 OK（depth=5、2 複製の樹形フラクタルが Result に正しく描画されることを確認）

## 範囲外

- 差分更新・描画キャッシュ（フレームレートが低い環境での最適化）→ Phase 4
- undo/redo → Phase 4
