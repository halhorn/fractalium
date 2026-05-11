# `ui/canvas`

## 責務

ワールドの **Seed／Placement／Result** キャンバスと格子ガイド。入力から状態更新の意図を起こす。[格子スナップの数式](../../core/mod.rs) は **`core`**、**ガイドのドット・補助線の描画**はここで行う。

## サブディレクトリ

| ディレクトリ | 責務（一文） |
|------------------|----------------|
| [`seed`](seed/mod.rs) | Seed キャンバス入力・ギズモ・`EditPlugin` |
| [`placement`](placement/mod.rs) | 配置キャンバス・`PlacementPlugin` |
| [`result`](result/mod.rs) | シーン描画とパン／ズームなどナビ：`scene.rs` と `navigation.rs` で分担 |
| [`grid_overlay`](grid_overlay.rs) | 格子ガイド描画 |

結果キャンバス内のファイル対応は [`result/README.md`](result/README.md) を参照。
