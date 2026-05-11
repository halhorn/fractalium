# `ui/canvas/result`

## 責務

結果ビューを **シーン側** と **ビューナビ側** で分離する親モジュール。

| ファイル | 責務 |
|----------|------|
| [`scene`](scene.rs) | メッシュ・再帰同期・線分描画、`FractalPlugin` の中心となる **見え方**。 |
| [`navigation`](navigation.rs) | パン／ズーム／タッチ、結果カメラのフィット要求の消化。**ワールド入力で動かす**役割（[`viewport_bridge`](../../viewport_bridge.rs) と役割分担）。 |
| [`mod`](mod.rs) | `scene` と `navigation` のプラグイン登録などの束ね。 |
