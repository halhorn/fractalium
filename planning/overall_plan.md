# Fractalium - 全体計画

## 目的

ユーザーが基本図形と複製ルールをインタラクティブに定義することで、フラクタル図形をリアルタイムに生成・確認できるアプリケーションを開発する。

## 計画概要

### Phase 1: MVP

最小限の体験を成立させる。詳細は [`mvp/overall_plan.md`](mvp/overall_plan.md) を参照。

- 基本図形の描画（ドラッグで直線を引く）
- 複製の配置（位置・角度・スケールの変換）
- 指定深さでのフラクタル再帰描画
- 操作のリアルタイム反映

### Phase 2: 図形プリセット

四角形・三角形などの基本図形プリセットを操作部に配置できるようにする。直線以外の図形要素（多角形、円弧など）の表現と編集 UI の拡張を含む。

### Phase 3: 複製パラメータの同期

複数の複製のパラメータ（角度・スケール等）を同期して操作する仕組み。グルーピング、ロック、対称配置プリセット等を検討。

### Phase 4: 仕上げ

- エクスポート（PNG / SVG）
- 作品の保存・読込（ローカル / 共有 URL）
- パフォーマンス最適化（深い階層・多複製でも軽快に動作）
- 操作性向上（undo/redo, スナップ, ガイド等）

### Mobile UX 改善

スマホでの操作感を最適化。詳細は [`mobile_ux/overall_plan.md`](mobile_ux/overall_plan.md) を参照。

- ナローレイアウト再設計（Result 上部大・操作系下部）
- グローバル操作パネル（undo/redo/snap/depth/gen）
- 削除ボタン追加（Base Shape, Placement）
- Result ドラッグでパン
- Edit/Placement ピンチズーム

## 受け入れ条件

- [ ] Phase 1 (MVP) が完了している
- [ ] Phase 2 が完了している
- [ ] Phase 3 が完了している
- [ ] Phase 4 が完了している

## 全体方針

### 技術スタック方針

[Question]
プラットフォームは Web ブラウザを想定する（インストール不要・URL 共有が容易・グラフィック API が揃っている）。フレームワークと描画方式の選定が必要。
[/Question]

[Answer]
Rust + Bevy（`../galaxy` で採用したゲームエンジン）を採用する。

- 言語: Rust (edition 2024)
- エンジン: Bevy 0.18 系
- 描画: Bevy 標準の 2D 描画 (Sprite / Mesh2d / Gizmos 等) を利用。MVP は 2D で十分
- UI: `bevy_egui` 0.39 を採用
- 配布: ネイティブビルド（`cargo run`）と WebAssembly（`trunk serve` / GitHub Pages）の両方に対応済み。WASM ビルドは `trunk` + WebGL2 バックエンドで実現し、`https://halhorn.github.io/fractalium/` で公開中
[/Answer]

### フラクタル数学モデル方針

[Question]
複製は「基本図形をどう変換して再配置するか」を定義する。 affine 変換（位置・回転・スケール）を採用するか、それを超えた変換（剪断 / 反転）まで許すか。
[/Question]

[Answer]
MVP では「位置（平行移動）・回転・等方スケール」の 3 要素を採用する。これは反復関数系 (IFS: Iterated Function System) の標準的な部分集合で、コッホ曲線・シェルピンスキー三角形等の代表的フラクタルを表現できる。将来的に剪断・反転を Phase で拡張可能。
[/Answer]

## 範囲

- **対象**: 自己相似フラクタル（IFS 系）の対話的生成・閲覧
- **非対象**: 数式直接入力（マンデルブロ等のエスケープタイム系）、3D フラクタル
